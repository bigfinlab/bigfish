//! BigFish 核心库 —— 下载引擎 / 版本解析 / 启动命令
//!
//! CLI（src/main.rs）和桌面 UI（src-tauri/）共用这套逻辑。

use serde::{Deserialize, Serialize};

/// Mojang 官方版本清单
pub const MOJANG_MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
/// BMCLAPI 国内镜像（bangbang93 维护，HMCL/PCL 同款）
pub const BMCLAPI_MANIFEST: &str = "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json";

/// 进度回调：人类可读的一行消息
pub type Progress<'a> = &'a (dyn Fn(&str) + Send + Sync);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub release_time: String,
    pub url: String,
}

/// 用户主目录（macOS/Linux 用 HOME，Windows 用 USERPROFILE）
pub fn home_dir() -> std::path::PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// 系统版本描述（macOS: 12.7.6 / Linux: PRETTY_NAME / Windows: 空）
pub fn os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(o) = std::process::Command::new("sw_vers").arg("-productVersion").output() {
            if let Ok(s) = String::from_utf8(o.stdout) {
                return s.trim().to_string();
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(s) = std::fs::read_to_string("/etc/os-release") {
            for line in s.lines() {
                if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
                    return v.trim().trim_matches('"').to_string();
                }
            }
        }
    }
    String::new()
}

pub fn make_agent(timeout_secs: u64) -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(timeout_secs)))
            .build(),
    )
}

pub fn fetch(url: &str) -> Result<String, String> {
    let agent = make_agent(30);
    let mut resp = agent
        .get(url)
        .call()
        .map_err(|e| format!("请求失败: {e}"))?;
    resp.body_mut()
        .read_to_string()
        .map_err(|e| format!("读取失败: {e}"))
}

pub fn fetch_bytes(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, String> {
    let resp = agent.get(url).call().map_err(|e| format!("请求失败: {e}"))?;
    resp.into_body()
        .with_config()
        .limit(256 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| format!("读取失败: {e}"))
}

/// 拉版本清单（官方优先，失败回退 BMCLAPI）
pub fn load_manifest() -> Result<(&'static str, serde_json::Value), String> {
    match fetch(MOJANG_MANIFEST) {
        Ok(b) => Ok(("Mojang 官方", serde_json::from_str(&b).map_err(|e| e.to_string())?)),
        Err(e) => {
            let b = fetch(BMCLAPI_MANIFEST).map_err(|e2| format!("官方源({e})和镜像源({e2})都失败"))?;
            Ok(("BMCLAPI 镜像", serde_json::from_str(&b).map_err(|e| e.to_string())?))
        }
    }
}

/// 列出全部版本（含旧版 alpha/beta）
pub fn list_versions() -> Result<Vec<VersionInfo>, String> {
    let (_source, manifest) = load_manifest()?;
    let mut out = Vec::new();
    if let Some(versions) = manifest["versions"].as_array() {
        for v in versions {
            out.push(VersionInfo {
                id: v["id"].as_str().unwrap_or("").to_string(),
                kind: v["type"].as_str().unwrap_or("").to_string(),
                release_time: v["releaseTime"].as_str().unwrap_or("").to_string(),
                url: v["url"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    Ok(out)
}

pub fn find_version<'a>(manifest: &'a serde_json::Value, id: &str) -> Option<&'a serde_json::Value> {
    manifest["versions"]
        .as_array()?
        .iter()
        .find(|v| v["id"] == id)
}

/// 判断当前平台是否满足 libraries 的 rules（macOS 场景）
pub fn rules_allow(rules: Option<&serde_json::Value>) -> bool {
    let Some(rules) = rules.and_then(|r| r.as_array()) else {
        return true;
    };
    let mut allow = false;
    for rule in rules {
        let action = rule["action"].as_str().unwrap_or("");
        let mut matches = true;

        if let Some(os) = rule.get("os") {
            let name = os["name"].as_str().unwrap_or("");
            let platform_ok = match name {
                "osx" => cfg!(target_os = "macos"),
                "windows" => cfg!(target_os = "windows"),
                "linux" => cfg!(target_os = "linux"),
                _ => true,
            };
            if !platform_ok {
                matches = false;
            }
            if let Some(arch) = os["arch"].as_str() {
                let arch_ok = match arch {
                    "x86" => cfg!(target_arch = "x86"),
                    "x86_64" => cfg!(target_arch = "x86_64"),
                    "arm64" => cfg!(target_arch = "aarch64"),
                    _ => true,
                };
                if !arch_ok {
                    matches = false;
                }
            }
        }
        // features 条件：只有启动器主动开启的特性才匹配。
        // 我们不开任何特性（不是 demo 用户、不是 quick play），所以要求 true 的都跳过。
        if let Some(features) = rule.get("features").and_then(|f| f.as_object()) {
            for (_k, v) in features {
                let required = v.as_bool().unwrap_or(false);
                // 本启动器所有 features 都是 false
                if required {
                    matches = false;
                }
            }
        }
        if matches {
            match action {
                "allow" => allow = true,
                "disallow" => allow = false,
                _ => {}
            }
        }
    }
    allow
}

/// 写文件并校验 sha1（sha1 为空则跳过校验）
pub fn save_file(bytes: &[u8], path: &std::path::Path, expect_sha1: &str) -> Result<(), String> {
    use sha1::{Digest, Sha1};
    if !expect_sha1.is_empty() {
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        let actual: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if actual != expect_sha1 {
            return Err(format!("sha1 校验失败: {actual} != {expect_sha1}"));
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

/// 选择适合的 Java：优先 JDK 21（1.20.1 推荐 17/21；Java 25 会触发 LWJGL JNI 崩溃）
pub fn find_java() -> String {
    let candidates = [
        "/Library/Java/JavaVirtualMachines/jdk-21.jdk/Contents/Home/bin/java",
        "/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java",
        "/Library/Java/JavaVirtualMachines/zulu-21.jdk/Contents/Home/bin/java",
    ];
    for c in candidates {
        if std::path::Path::new(c).exists() {
            return c.to_string();
        }
    }
    // Windows/Linux 常见路径
    for c in [
        "C:\\Program Files\\Java\\jdk-21\\bin\\java.exe",
        "C:\\Program Files\\Eclipse Adoptium\\jdk-21\\bin\\java.exe",
        "/usr/lib/jvm/java-21-openjdk/bin/java",
    ] {
        if std::path::Path::new(c).exists() {
            return c.to_string();
        }
    }
    "java".to_string()
}

// ============================================================
// Java 管理：按版本要求选择本地 JRE，缺失时可从 Adoptium 下载
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaInfo {
    pub major: u8,
    pub version: String,
    pub path: String,
    pub source: String,
}

/// BigFish 自管的 Java 目录：~/.bigfish/java/
pub fn java_root() -> std::path::PathBuf {
    home_dir().join(".bigfish").join("java")
}

/// 解析 JDK 目录里的 release 文件，得到 (大版本号, 版本串)
fn parse_release(home: &std::path::Path) -> Option<(u8, String)> {
    let txt = std::fs::read_to_string(home.join("release")).ok()?;
    let raw = txt
        .lines()
        .find_map(|l| l.strip_prefix("JAVA_VERSION="))?
        .trim()
        .trim_matches('"')
        .to_string();
    let first: u32 = raw.split('.').next()?.parse().ok()?;
    let major = if first == 1 {
        raw.split('.').nth(1)?.parse().ok()?
    } else {
        first
    };
    Some((major as u8, raw))
}

/// 判断目录是否为 Java home，是则返回信息
fn java_home_of(p: &std::path::Path, source: &str) -> Option<JavaInfo> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let home = if p.join("bin").join(exe).exists() {
        p.to_path_buf()
    } else if p.join("Contents").join("Home").join("bin").join(exe).exists() {
        p.join("Contents").join("Home")
    } else {
        return None;
    };
    let (major, version) = parse_release(&home)?;
    Some(JavaInfo {
        major,
        version,
        path: home.join("bin").join(exe).display().to_string(),
        source: source.to_string(),
    })
}

fn scan_java_parent(root: &std::path::Path, source: &str, out: &mut Vec<JavaInfo>) {
    let Ok(rd) = std::fs::read_dir(root) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if !p.is_dir() { continue; }
        if let Some(info) = java_home_of(&p, source) {
            if !out.iter().any(|j| j.path == info.path) {
                out.push(info);
            }
        }
    }
}

/// 扫描本机所有 Java
pub fn scan_local_javas() -> Vec<JavaInfo> {
    let mut out: Vec<JavaInfo> = Vec::new();

    #[cfg(target_os = "macos")]
    {
        scan_java_parent(std::path::Path::new("/Library/Java/JavaVirtualMachines"), "系统", &mut out);
        scan_java_parent(
            &home_dir().join("Library/Java/JavaVirtualMachines"),
            "用户",
            &mut out,
        );
        for base in ["/usr/local/opt", "/opt/homebrew/opt"] {
            if let Ok(rd) = std::fs::read_dir(base) {
                for e in rd.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with("openjdk") {
                        if let Some(info) = java_home_of(&e.path().join("libexec").join("openjdk.jdk"), "Homebrew") {
                            if !out.iter().any(|j| j.path == info.path) {
                                out.push(info);
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        scan_java_parent(std::path::Path::new("/usr/lib/jvm"), "系统", &mut out);
        scan_java_parent(std::path::Path::new("/opt"), "系统", &mut out);
    }

    #[cfg(target_os = "windows")]
    {
        for base in [
            r"C:\Program Files\Java",
            r"C:\Program Files\Eclipse Adoptium",
            r"C:\Program Files\Zulu",
        ] {
            scan_java_parent(std::path::Path::new(base), "系统", &mut out);
        }
    }

    // BigFish 自己下载的
    if let Ok(rd) = std::fs::read_dir(java_root()) {
        for e in rd.flatten() {
            if let Some(info) = java_home_of(&e.path(), "BigFish") {
                out.retain(|j| j.path != info.path);
                out.push(info);
            }
        }
    }

    out.sort_by_key(|j| std::cmp::Reverse(j.major));
    out
}

/// 找满足指定大版本的 Java：优先精确匹配，其次选高一档但不太远的
pub fn find_java_for(major: u8) -> Option<JavaInfo> {
    let all = scan_local_javas();
    if let Some(j) = all.iter().find(|j| j.major == major) {
        return Some(j.clone());
    }
    all.into_iter()
        .filter(|j| j.major > major && j.major <= major + 4)
        .min_by_key(|j| j.major)
}

/// 根据版本 JSON 推断需要的 Java 大版本
pub fn java_requirement(version_id: &str, game_dir: &str) -> u8 {
    let json_path = std::path::Path::new(game_dir)
        .join("versions")
        .join(version_id)
        .join(format!("{version_id}.json"));
    if let Ok(s) = std::fs::read_to_string(&json_path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            if let Some(m) = v["javaVersion"]["majorVersion"].as_u64() {
                return m as u8;
            }
            if let Some(base) = v["inheritsFrom"].as_str() {
                return java_requirement(base, game_dir);
            }
        }
    }
    guess_java_major(version_id)
}

fn guess_java_major(id: &str) -> u8 {
    for token in id.split(|c: char| !c.is_ascii_digit() && c != '.') {
        let Some(rest) = token.strip_prefix("1.") else { continue };
        let parts: Vec<u32> = rest.split('.').filter_map(|s| s.parse().ok()).collect();
        if let Some(&minor) = parts.first() {
            let patch = parts.get(1).copied().unwrap_or(0);
            return if minor >= 21 || (minor == 20 && patch >= 5) {
                21
            } else if minor >= 17 {
                17
            } else {
                8
            };
        }
    }
    8
}

/// Adoptium Temurin JRE 下载地址
fn adoptium_url(major: u8) -> String {
    let (os, arch) = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") { ("mac", "aarch64") } else { ("mac", "x64") }
    } else if cfg!(target_os = "windows") {
        ("windows", "x64")
    } else if cfg!(target_arch = "aarch64") {
        ("linux", "aarch64")
    } else {
        ("linux", "x64")
    };
    format!("https://api.adoptium.net/v3/binary/latest/{major}/ga/{os}/{arch}/jre/hotspot/normal/eclipse")
}

/// 下载并解压指定大版本的 JRE
pub fn install_java(major: u8, on_progress: Progress) -> Result<JavaInfo, String> {
    let root = java_root();
    std::fs::create_dir_all(&root).map_err(|e| format!("创建目录失败: {e}"))?;

    if let Ok(rd) = std::fs::read_dir(&root) {
        for e in rd.flatten() {
            if let Some(info) = java_home_of(&e.path(), "BigFish") {
                if info.major == major {
                    on_progress(&format!("Java {major} 已存在：{}", info.path));
                    return Ok(info);
                }
            }
        }
    }

    let url = adoptium_url(major);
    on_progress(&format!("下载 Java {major}（Adoptium Temurin JRE）…"));
    let agent = make_agent(1800);
    let bytes = fetch_bytes(&agent, &url)?;
    on_progress(&format!("下载完成：{:.1} MB", bytes.len() as f64 / 1048576.0));

    // Windows 的 Adoptium 包是 zip，其他平台是 tar.gz
    #[cfg(windows)]
    {
        on_progress("解压中…");
        extract_zip_to_dir(&bytes, &root)?;
    }
    #[cfg(not(windows))]
    {
        let archive = root.join(format!("jre{major}.tar.gz"));
        std::fs::write(&archive, &bytes).map_err(|e| e.to_string())?;
        on_progress("解压中…");
        let status = std::process::Command::new("tar")
            .arg("-xzf")
            .arg(&archive)
            .arg("-C")
            .arg(&root)
            .status()
            .map_err(|e| format!("调用 tar 失败：{e}"))?;
        let _ = std::fs::remove_file(&archive);
        if !status.success() {
            return Err("解压失败".to_string());
        }
    }

    let mut found: Vec<JavaInfo> = std::fs::read_dir(&root)
        .map(|rd| rd.flatten().filter_map(|e| java_home_of(&e.path(), "BigFish")).collect())
        .unwrap_or_default();
    found.sort_by_key(|j| std::cmp::Reverse(j.major));
    if let Some(info) = found.into_iter().find(|j| j.major == major) {
        on_progress(&format!("Java {major} 安装完成"));
        return Ok(info);
    }
    Err("解压后没找到 java 可执行文件".to_string())
}

/// 离线账号的 UUID：MD5("OfflinePlayer:" + name)，version=3 / variant=2
pub fn offline_uuid(name: &str) -> String {
    let digest = md5::compute(format!("OfflinePlayer:{name}"));
    let mut bytes: Vec<u8> = digest.iter().copied().collect();
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// 下载并解压 natives（LWJGL 的原生库）
fn extract_natives(root: &std::path::Path, libs: &[serde_json::Value], on_progress: Progress) -> Result<(), String> {
    let natives_dir = root.join("natives");
    std::fs::create_dir_all(&natives_dir).map_err(|e| e.to_string())?;
    let agent = make_agent(60);
    for lib in libs {
        let name = lib["name"].as_str().unwrap_or("");
        if !name.contains("natives-macos") || name.contains("arm64") {
            continue;
        }
        let Some(path) = lib["downloads"]["artifact"]["path"].as_str() else { continue };
        let jar = root.join("libraries").join(path);
        if !jar.exists() {
            continue;
        }
        let bytes = std::fs::read(&jar).map_err(|e| e.to_string())?;
        extract_zip_to_dir(&bytes, &natives_dir)?;
        let _ = &agent;
    }
    on_progress(&format!("natives 解压完成 -> {}", natives_dir.display()));
    Ok(())
}

/// 极简 zip 解压（按中央目录读取，够用）
fn extract_zip_to_dir(bytes: &[u8], dir: &std::path::Path) -> Result<(), String> {
    use std::io::Read;
    let mut reader = std::io::Cursor::new(bytes);
    let mut eocd = [0u8; 22];
    let len = bytes.len();
    if len < 22 {
        return Err("zip 太小".into());
    }
    // 从尾部找 End of Central Directory（0x06054b50）
    for skip in 0..(len - 22).min(65535) {
        let pos = len - 22 - skip;
        reader.set_position(pos as u64);
        reader.read_exact(&mut eocd).map_err(|e| e.to_string())?;
        if eocd[0..4] == [0x50, 0x4b, 0x05, 0x06] {
            let cd_size = u32::from_le_bytes([eocd[12], eocd[13], eocd[14], eocd[15]]) as usize;
            let cd_offset = u32::from_le_bytes([eocd[16], eocd[17], eocd[18], eocd[19]]) as usize;
            let cd_end = cd_offset.checked_add(cd_size).ok_or("zip 损坏")?;
            if cd_end > len {
                return Err("zip 目录越界".into());
            }
            let mut p = cd_offset;
            while p + 46 <= cd_end {
                if bytes[p..p + 4] != [0x50, 0x4b, 0x01, 0x02] {
                    break;
                }
                let method = u16::from_le_bytes([bytes[p + 10], bytes[p + 11]]);
                let comp_size = u32::from_le_bytes([bytes[p + 20], bytes[p + 21], bytes[p + 22], bytes[p + 23]]) as usize;
                let name_len = u16::from_le_bytes([bytes[p + 28], bytes[p + 29]]) as usize;
                let extra_len = u16::from_le_bytes([bytes[p + 30], bytes[p + 31]]) as usize;
                let comment_len = u16::from_le_bytes([bytes[p + 32], bytes[p + 33]]) as usize;
                let local_off = u32::from_le_bytes([bytes[p + 42], bytes[p + 43], bytes[p + 44], bytes[p + 45]]) as usize;
                let name = String::from_utf8_lossy(&bytes[p + 46..p + 46 + name_len]).to_string();
                if !name.ends_with('/') {
                    // 本地头
                    let lname_len = u16::from_le_bytes([bytes[local_off + 26], bytes[local_off + 27]]) as usize;
                    let lextra_len = u16::from_le_bytes([bytes[local_off + 28], bytes[local_off + 29]]) as usize;
                    let data_off = local_off + 30 + lname_len + lextra_len;
                    let data = &bytes[data_off..(data_off + comp_size).min(len)];
                    let mut out = Vec::with_capacity(comp_size * 4);
                    if method == 0 {
                        out = data.to_vec();
                    } else if method == 8 {
                        use flate2::read::DeflateDecoder;
                        let mut decoder = DeflateDecoder::new(data);
                        decoder
                            .read_to_end(&mut out)
                            .map_err(|e| format!("解压 {name} 失败: {e}"))?;
                    } else {
                        continue;
                    }
                    let dest = dir.join(&name);
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent).ok();
                    }
                    std::fs::write(&dest, out).ok();
                }
                p += 46 + name_len + extra_len + comment_len;
            }
            return Ok(());
        }
    }
    Err("不是有效 zip".into())
}

/// 下载一个版本的完整文件（client.jar + libraries + assets + natives）
pub fn download_version(version_id: &str, game_dir: &str, on_progress: Progress) -> Result<(), String> {
    let root = std::path::Path::new(game_dir).to_path_buf();
    let agent = make_agent(120);

    let (_source, manifest) = load_manifest()?;
    let meta = find_version(&manifest, version_id).ok_or_else(|| format!("找不到版本 {version_id}"))?;
    let version_url = meta["url"].as_str().ok_or("版本 url 缺失")?;

    let body = fetch(version_url)?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    // 1. 版本 JSON
    let ver_dir = root.join("versions").join(version_id);
    std::fs::create_dir_all(&ver_dir).map_err(|e| e.to_string())?;
    std::fs::write(ver_dir.join(format!("{version_id}.json")), &body).map_err(|e| e.to_string())?;
    on_progress(&format!("[1/4] 版本 JSON 已保存"));

    // 2. client.jar
    let client = &v["downloads"]["client"];
    let jar_url = client["url"].as_str().ok_or("client url 缺失")?;
    let jar_sha1 = client["sha1"].as_str().unwrap_or("");
    let jar_path = ver_dir.join(format!("{version_id}.jar"));
    if !jar_path.exists() {
        on_progress(&format!("[2/4] 下载 client.jar ({} MB)...", client["size"].as_u64().unwrap_or(0) / 1048576));
        let bytes = fetch_bytes(&agent, jar_url)?;
        save_file(&bytes, &jar_path, jar_sha1)?;
    }
    on_progress("[2/4] client.jar ok");

    // 3. libraries
    let libs = v["libraries"].as_array().cloned().unwrap_or_default();
    let mut todo = Vec::new();
    for lib in &libs {
        if !rules_allow(lib.get("rules")) {
            continue;
        }
        if let (Some(url), Some(path)) = (
            lib["downloads"]["artifact"]["url"].as_str(),
            lib["downloads"]["artifact"]["path"].as_str(),
        ) {
            todo.push((url.to_string(), path.to_string(), lib["downloads"]["artifact"]["sha1"].as_str().unwrap_or("").to_string()));
        }
    }
    on_progress(&format!("[3/4] 下载 libraries: {} 个文件", todo.len()));
    let mut ok = 0usize;
    for (i, (url, rel, sha1)) in todo.iter().enumerate() {
        let dest = root.join("libraries").join(rel);
        if dest.exists() {
            ok += 1;
            continue;
        }
        fetch_bytes(&agent, url).and_then(|b| save_file(&b, &dest, sha1))?;
        ok += 1;
        if i % 10 == 0 {
            on_progress(&format!("[3/4] libraries {}/{}", i + 1, todo.len()));
        }
    }
    on_progress(&format!("[3/4] libraries ok ({ok}/{})", todo.len()));

    // 4. assets
    on_progress("[4/4] 下载 assets...");
    download_assets_impl(&v, &root, on_progress)?;

    // 5. natives
    extract_natives(&root, &libs, on_progress)?;

    on_progress("下载完成 ✔");
    Ok(())
}

fn download_assets_impl(v: &serde_json::Value, root: &std::path::Path, on_progress: Progress) -> Result<(), String> {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let idx = &v["assetIndex"];
    let idx_url = idx["url"].as_str().ok_or("assetIndex.url 缺失")?;
    let idx_id = idx["id"].as_str().unwrap_or("legacy");

    let idx_body = fetch(idx_url)?;
    let idx_path = root.join("assets").join("indexes").join(format!("{idx_id}.json"));
    if let Some(parent) = idx_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&idx_path, &idx_body).map_err(|e| e.to_string())?;

    let index: serde_json::Value = serde_json::from_str(&idx_body).map_err(|e| e.to_string())?;
    let objects = index["objects"].as_object().ok_or("assets objects 缺失")?;

    let mut tasks: Vec<String> = Vec::new();
    for (_name, obj) in objects {
        if let Some(hash) = obj["hash"].as_str() {
            let rel = format!("assets/objects/{}/{}", &hash[..2], hash);
            if !root.join(&rel).exists() {
                tasks.push(rel);
            }
        }
    }
    tasks.sort();
    tasks.dedup();
    let total = tasks.len();
    if total == 0 {
        on_progress("assets 已全部存在");
        return Ok(());
    }
    on_progress(&format!("assets 待下载: {total} 个文件"));

    let counter = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let tasks = std::sync::Arc::new(tasks);
    let root = root.to_path_buf();
    let n_threads = 8usize;

    std::thread::scope(|scope| {
        for t in 0..n_threads {
            let tasks = tasks.clone();
            let root = root.clone();
            let counter = &counter;
            let failed = &failed;
            let done = &done;
            scope.spawn(move || {
                let agent = make_agent(60);
                for (i, rel) in tasks.iter().enumerate() {
                    if i % n_threads != t {
                        continue;
                    }
                    let dest = root.join(rel);
                    let hash = rel.rsplit('/').next().unwrap_or(rel);
                    let official = format!("https://resources.download.minecraft.net/{}/{}", &hash[..2], hash);
                    let mirror = format!("https://bmclapi2.bangbang93.com/assets/{}/{}", &hash[..2], hash);
                    let result = fetch_bytes(&agent, &official).or_else(|_| fetch_bytes(&agent, &mirror));
                    match result.and_then(|b| save_file(&b, &dest, hash)) {
                        Ok(()) => { counter.fetch_add(1, Ordering::Relaxed); }
                        Err(_) => { failed.fetch_add(1, Ordering::Relaxed); }
                    }
                    done.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
        // 等所有线程完成（scope 自动 join）
    });

    let ok = counter.load(Ordering::Relaxed);
    let bad = failed.load(Ordering::Relaxed);
    on_progress(&format!("assets 完成: {ok} 成功, {bad} 失败"));
    Ok(())
}

/// 变量替换（${var} → value）
fn subst_vars(s: &str, vars: &[(&str, &str)]) -> String {
    let mut out = s.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("${{{k}}}"), v);
    }
    out
}

/// 解析 arguments 数组（字符串 / {rules, value} 形式），带平台过滤 + 变量替换
fn build_args(arr: &serde_json::Value, vars: &[(&str, &str)]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(items) = arr.as_array() else { return out };
    for item in items {
        if let Some(s) = item.as_str() {
            out.push(subst_vars(s, vars));
        } else if let Some(obj) = item.as_object() {
            if rules_allow(obj.get("rules")) {
                match &obj["value"] {
                    serde_json::Value::String(s) => out.push(subst_vars(s, vars)),
                    serde_json::Value::Array(a) => {
                        for v in a {
                            if let Some(s) = v.as_str() {
                                out.push(subst_vars(s, vars));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    out
}

/// 找 natives 目录（版本专属优先，其次全局）
fn find_natives_dir(root: &std::path::Path, version_id: &str) -> std::path::PathBuf {
    let ver_dir = root.join("versions").join(version_id);
    let candidates = [
        ver_dir.join("natives-macos-x86_64"),
        ver_dir.join("natives-macos-arm64"),
        ver_dir.join("natives"),
        root.join("natives/macos/x64"),
        root.join("natives"),
    ];
    for c in candidates {
        if c.is_dir() {
            return c;
        }
    }
    root.join("natives")
}

/// 收集 classpath（jar + libraries，已去重）
fn build_classpath(root: &std::path::Path, version_id: &str, v: &serde_json::Value) -> Vec<String> {
    let jar_id = v["jar"].as_str().unwrap_or(version_id);
    let mut cp: Vec<String> = Vec::new();
    let jar_path = root.join("versions").join(jar_id).join(format!("{jar_id}.jar"));
    if jar_path.exists() {
        cp.push(jar_path.display().to_string());
    }
    if let Some(libs) = v["libraries"].as_array() {
        for lib in libs {
            if !rules_allow(lib.get("rules")) {
                continue;
            }
            if let Some(path) = lib["downloads"]["artifact"]["path"].as_str() {
                let p = root.join("libraries").join(path).display().to_string();
                if !cp.contains(&p) {
                    cp.push(p);
                }
            }
        }
    }
    cp
}

/// 组装完整启动参数（jvm + game），返回 (java 参数列表, mainClass)
fn build_launch_args(
    root: &std::path::Path,
    version_id: &str,
    v: &serde_json::Value,
    username: &str,
) -> (Vec<String>, Vec<String>, std::path::PathBuf) {
    let natives = find_natives_dir(root, version_id);
    let cp = build_classpath(root, version_id, v);
    let sep = if cfg!(windows) { ";" } else { ":" };
    let cp_joined = cp.join(sep);
    let uuid = offline_uuid(username);
    let asset_index = v["assetIndex"]["id"].as_str().unwrap_or(version_id);
    let main_class = v["mainClass"].as_str().unwrap_or("net.minecraft.client.main.Main");

    let vars: Vec<(&str, &str)> = vec![
        ("natives_directory", ""),  // 下面单独处理（生命周期问题）
        ("launcher_name", "BigFish"),
        ("launcher_version", "0.1.0"),
        ("classpath", ""),
        ("classpath_separator", sep),
        ("library_directory", ""),
        ("auth_player_name", username),
        ("version_name", version_id),
        ("game_directory", ""),
        ("assets_root", ""),
        ("assets_index_name", asset_index),
        ("auth_uuid", &uuid),
        ("auth_access_token", "0"),
        ("auth_session", "0"),
        ("auth_xuid", ""),
        ("clientid", "0"),
        ("user_type", "legacy"),
        ("version_type", "release"),
        ("user_properties", "{}"),
        ("resolution_width", "854"),
        ("resolution_height", "480"),
        ("quickPlayPath", ""),
        ("quickPlaySingleplayer", ""),
        ("quickPlayMultiplayer", ""),
        ("quickPlayRealms", ""),
    ];
    // 用 owned 字符串构造最终替换表
    let natives_s = natives.display().to_string();
    let lib_s = root.join("libraries").display().to_string();
    let game_s = root.display().to_string();
    let assets_s = root.join("assets").display().to_string();
    let mut vars2: Vec<(&str, &str)> = vars.clone();
    vars2[0] = ("natives_directory", &natives_s);
    vars2[3] = ("classpath", &cp_joined);
    vars2[5] = ("library_directory", &lib_s);
    vars2[8] = ("game_directory", &game_s);
    vars2[9] = ("assets_root", &assets_s);

    let mut jvm_args: Vec<String> = Vec::new();
    if let Some(jvm) = v.get("arguments").and_then(|a| a.get("jvm")) {
        jvm_args = build_args(jvm, &vars2);
    }
    let mut game_args: Vec<String> = Vec::new();
    if let Some(game) = v.get("arguments").and_then(|a| a.get("game")) {
        game_args = build_args(game, &vars2);
    }
    // 清理：跳过空值或未替换变量（如 --quickPlayPath ""）的 flag 对
    let mut cleaned: Vec<String> = Vec::new();
    let mut i = 0;
    while i < game_args.len() {
        let a = &game_args[i];
        if a.starts_with("--") && i + 1 < game_args.len() {
            let val = &game_args[i + 1];
            if val.is_empty() || val.contains("${") {
                i += 2;
                continue;
            }
        }
        cleaned.push(a.clone());
        i += 1;
    }
    (jvm_args, cleaned, natives)
}

/// 启动游戏，返回子进程 pid（支持带完整 arguments 的版本，如 Forge）
pub fn launch_version(version_id: &str, game_dir: &str, username: &str) -> Result<u32, String> {
    let root = std::path::Path::new(game_dir).to_path_buf();
    let ver_dir = root.join("versions").join(version_id);
    let body = std::fs::read_to_string(ver_dir.join(format!("{version_id}.json")))
        .map_err(|e| format!("读不到版本 JSON（先下载）: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    let (jvm_args, game_args, _natives) = build_launch_args(&root, version_id, &v, username);
    let main_class = v["mainClass"].as_str().unwrap_or("net.minecraft.client.main.Main");

    // 按版本要求选 Java（1.16 及更早要 Java 8，1.17~1.20.4 要 17，1.20.5+ 要 21）
    let required = java_requirement(version_id, game_dir);
    let java = find_java_for(required).ok_or_else(|| {
        format!(
            "版本「{version_id}」需要 Java {required}，本机没有找到。\n请到设置 → Java 环境里下载 Java {required}。"
        )
    })?;
    let java_bin = java.path.clone();
    let cfg = load_config();
    let max_mem = cfg.max_memory_mb.max(512);
    let min_mem = cfg.min_memory_mb.min(max_mem).max(128);
    let mut cmd = std::process::Command::new(&java_bin);
    cmd.arg(format!("-Xmx{max_mem}M")).arg(format!("-Xms{min_mem}M"));
    for a in &jvm_args {
        cmd.arg(a);
    }
    // macOS 上没有 -XstartOnFirstThread 时补上（LWJGL 硬性要求）
    #[cfg(target_os = "macos")]
    if !jvm_args.iter().any(|a| a == "-XstartOnFirstThread") {
        cmd.arg("-XstartOnFirstThread");
    }
    // 离线模式兜底：没有 java.library.path 时补
    if !jvm_args.iter().any(|a| a.starts_with("-Djava.library.path")) {
        cmd.arg(format!("-Djava.library.path={}", find_natives_dir(&root, version_id).display()));
    }
    cmd.arg(main_class);
    for a in &game_args {
        cmd.arg(a);
    }

    let child = cmd.spawn().map_err(|e| format!("启动失败: {e}"))?;
    Ok(child.id())
}

/// 生成启动命令预览（不启动）
pub fn launch_preview(version_id: &str, game_dir: &str, username: &str) -> Result<String, String> {
    let root = std::path::Path::new(game_dir).to_path_buf();
    let ver_dir = root.join("versions").join(version_id);
    let body = std::fs::read_to_string(ver_dir.join(format!("{version_id}.json")))
        .map_err(|e| format!("读不到版本 JSON: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let (jvm_args, game_args, natives) = build_launch_args(&root, version_id, &v, username);
    let main_class = v["mainClass"].as_str().unwrap_or("net.minecraft.client.main.Main");
    let required = java_requirement(version_id, game_dir);
    let java = find_java_for(required)
        .map(|j| j.path)
        .unwrap_or_else(|| format!("<缺少 Java {required}>"));
    let cfg = load_config();
    let mut parts = vec![java, format!("-Xmx{}M", cfg.max_memory_mb.max(512))];
    parts.extend(jvm_args);
    parts.push(main_class.to_string());
    parts.extend(game_args);
    Ok(format!(
        "natives: {}\n\n{}",
        natives.display(),
        parts.join(" ")
    ))
}

// ============================================================
// 存档管理：扫描旧启动器目录 + 解析 level.dat (NBT)
// ============================================================

#[derive(Debug, Clone)]
pub enum Nbt {
    Byte(i8), Short(i16), Int(i32), Long(i64), Float(f32), Double(f64),
    String(String), List(Vec<Nbt>), Compound(Vec<(String, Nbt)>),
    ByteArray(Vec<u8>), IntArray(Vec<i32>), LongArray(Vec<i64>),
}

fn read_nbt_str(b: &[u8], p: &mut usize) -> Result<String, String> {
    if *p + 2 > b.len() { return Err("nbt 截断(长度)".into()); }
    let len = u16::from_be_bytes([b[*p], b[*p + 1]]) as usize;
    *p += 2;
    if *p + len > b.len() { return Err("nbt 字符串越界".into()); }
    let s = String::from_utf8_lossy(&b[*p..*p + len]).to_string();
    *p += len;
    Ok(s)
}

fn parse_nbt_payload(b: &[u8], p: &mut usize, tag: u8) -> Result<Nbt, String> {
    match tag {
        1 => { if *p + 1 > b.len() { return Err("nbt 截断".into()); } let v = b[*p] as i8; *p += 1; Ok(Nbt::Byte(v)) }
        2 => { if *p + 2 > b.len() { return Err("nbt 截断".into()); } let v = i16::from_be_bytes([b[*p], b[*p + 1]]); *p += 2; Ok(Nbt::Short(v)) }
        3 => { if *p + 4 > b.len() { return Err("nbt 截断".into()); } let v = i32::from_be_bytes([b[*p], b[*p+1], b[*p+2], b[*p+3]]); *p += 4; Ok(Nbt::Int(v)) }
        4 => { if *p + 8 > b.len() { return Err("nbt 截断".into()); } let v = i64::from_be_bytes([b[*p], b[*p+1], b[*p+2], b[*p+3], b[*p+4], b[*p+5], b[*p+6], b[*p+7]]); *p += 8; Ok(Nbt::Long(v)) }
        5 => { if *p + 4 > b.len() { return Err("nbt 截断".into()); } let v = f32::from_bits(u32::from_be_bytes([b[*p], b[*p+1], b[*p+2], b[*p+3]])); *p += 4; Ok(Nbt::Float(v)) }
        6 => { if *p + 8 > b.len() { return Err("nbt 截断".into()); } let v = f64::from_bits(u64::from_be_bytes([b[*p], b[*p+1], b[*p+2], b[*p+3], b[*p+4], b[*p+5], b[*p+6], b[*p+7]])); *p += 8; Ok(Nbt::Double(v)) }
        7 => {
            if *p + 4 > b.len() { return Err("nbt 截断".into()); }
            let len = i32::from_be_bytes([b[*p], b[*p+1], b[*p+2], b[*p+3]]);
            *p += 4;
            if len < 0 || *p + len as usize > b.len() { return Err("nbt ByteArray 越界".into()); }
            let v = b[*p..*p + len as usize].to_vec();
            *p += len as usize;
            Ok(Nbt::ByteArray(v))
        }
        8 => Ok(Nbt::String(read_nbt_str(b, p)?)),
        9 => {
            if *p + 5 > b.len() { return Err("nbt 截断".into()); }
            let elem = b[*p]; *p += 1;
            let len = i32::from_be_bytes([b[*p], b[*p+1], b[*p+2], b[*p+3]]);
            *p += 4;
            if len < 0 { return Err("nbt List 长度负数".into()); }
            let mut items = Vec::with_capacity(len as usize);
            for _ in 0..len { items.push(parse_nbt_payload(b, p, elem)?); }
            Ok(Nbt::List(items))
        }
        10 => {
            let mut items = Vec::new();
            loop {
                if *p >= b.len() { return Err("nbt Compound 未闭合".into()); }
                let t = b[*p]; *p += 1;
                if t == 0 { break; }
                let name = read_nbt_str(b, p)?;
                let val = parse_nbt_payload(b, p, t)?;
                items.push((name, val));
            }
            Ok(Nbt::Compound(items))
        }
        11 => {
            if *p + 4 > b.len() { return Err("nbt 截断".into()); }
            let len = i32::from_be_bytes([b[*p], b[*p+1], b[*p+2], b[*p+3]]);
            *p += 4;
            if len < 0 || *p + (len as usize) * 4 > b.len() { return Err("nbt IntArray 越界".into()); }
            let mut v = Vec::with_capacity(len as usize);
            for _ in 0..len { v.push(i32::from_be_bytes([b[*p], b[*p+1], b[*p+2], b[*p+3]])); *p += 4; }
            Ok(Nbt::IntArray(v))
        }
        12 => {
            if *p + 4 > b.len() { return Err("nbt 截断".into()); }
            let len = i32::from_be_bytes([b[*p], b[*p+1], b[*p+2], b[*p+3]]);
            *p += 4;
            if len < 0 || *p + (len as usize) * 8 > b.len() { return Err("nbt LongArray 越界".into()); }
            let mut v = Vec::with_capacity(len as usize);
            for _ in 0..len { v.push(i64::from_be_bytes([b[*p], b[*p+1], b[*p+2], b[*p+3], b[*p+4], b[*p+5], b[*p+6], b[*p+7]])); *p += 8; }
            Ok(Nbt::LongArray(v))
        }
        _ => Err(format!("未知 nbt tag {tag}")),
    }
}

pub fn parse_level_dat(path: &std::path::Path) -> Result<Nbt, String> {
    use std::io::Read;
    let raw = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    if raw.len() > 2 && raw[0] == 0x1f && raw[1] == 0x8b {
        use flate2::read::GzDecoder;
        let mut d = GzDecoder::new(&raw[..]);
        d.read_to_end(&mut bytes).map_err(|e| format!("gzip 解压失败: {e}"))?;
    } else {
        bytes = raw;
    }
    let mut p = 0usize;
    let tag = *bytes.first().ok_or("空文件")?;
    p += 1;
    if tag != 10 { return Err(format!("level.dat 根不是 Compound（tag={tag}）")); }
    let _name = read_nbt_str(&bytes, &mut p)?;
    parse_nbt_payload(&bytes, &mut p, 10)
}

fn nbt_get<'a>(n: &'a Nbt, key: &str) -> Option<&'a Nbt> {
    match n {
        Nbt::Compound(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

fn nbt_string(n: &Nbt) -> Option<String> {
    match n { Nbt::String(s) => Some(s.clone()), _ => None }
}

fn nbt_long(n: &Nbt) -> Option<i64> {
    match n { Nbt::Long(v) => Some(*v), Nbt::Int(v) => Some(*v as i64), _ => None }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SaveInfo {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub last_played: i64,
    pub path: String,
    pub source: String,
}

fn save_root_candidates() -> Vec<(String, std::path::PathBuf)> {
    let mut out: Vec<(String, std::path::PathBuf)> = Vec::new();
    {
        let h = home_dir();
        #[cfg(target_os = "macos")]
        {
            out.push(("官方".into(), h.join("Library/Application Support/minecraft/saves")));
            out.push(("HMCL".into(), std::path::PathBuf::from("/Applications/.minecraft/saves")));
            out.push(("HMCL".into(), h.join(".minecraft/saves")));
        }
        #[cfg(not(target_os = "macos"))]
        {
            out.push(("官方".into(), h.join(".minecraft/saves")));
        }
        out.push(("自定义".into(), h.join("MinecraftBigFish/saves")));
        out.push(("自定义".into(), h.join("Minecraft/saves")));
    }
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            out.push(("官方".into(), std::path::PathBuf::from(&appdata).join(".minecraft/saves")));
        }
    }
    out
}

fn read_save_meta(save_dir: &std::path::Path, source: &str) -> Option<SaveInfo> {
    let level = save_dir.join("level.dat");
    let name = save_dir.file_name()?.to_string_lossy().to_string();
    let mut info = SaveInfo {
        name: name.clone(),
        display_name: name.clone(),
        version: "?".into(),
        last_played: 0,
        path: save_dir.display().to_string(),
        source: source.to_string(),
    };
    if level.exists() {
        if let Ok(root) = parse_level_dat(&level) {
            let data = nbt_get(&root, "Data").unwrap_or(&root);
            if let Some(v) = nbt_get(data, "LevelName").and_then(nbt_string) { info.display_name = v; }
            if let Some(v) = nbt_get(data, "LastPlayed").and_then(nbt_long) { info.last_played = v; }
            if let Some(v) = nbt_get(data, "Version").and_then(|ver| nbt_get(ver, "Name")).and_then(nbt_string) {
                info.version = v;
            }
        }
    }
    Some(info)
}

pub fn scan_saves() -> Vec<SaveInfo> {
    let mut out = Vec::new();
    for (source, root) in save_root_candidates() {
        if !root.exists() { continue; }
        let Ok(entries) = std::fs::read_dir(&root) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                if let Some(info) = read_save_meta(&p, &source) { out.push(info); }
            }
        }
    }
    out.sort_by(|a, b| b.last_played.cmp(&a.last_played));
    out
}

pub fn import_save(source_path: &str, target_game_dir: &str) -> Result<String, String> {
    let src = std::path::Path::new(source_path);
    if !src.is_dir() { return Err(format!("存档不存在: {source_path}")); }
    let name = src.file_name().ok_or("存档名无效")?.to_string_lossy().to_string();
    let saves_dir = std::path::Path::new(target_game_dir).join("saves");
    std::fs::create_dir_all(&saves_dir).map_err(|e| e.to_string())?;
    let mut dest = saves_dir.join(&name);
    let mut i = 1;
    while dest.exists() {
        dest = saves_dir.join(format!("{name} ({i})"));
        i += 1;
    }
    copy_dir_recursive(src, &dest)?;
    Ok(dest.display().to_string())
}

fn copy_dir_recursive(src: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let p = entry.path();
        let target = dest.join(entry.file_name());
        // 用 symlink_metadata 判断本身类型：符号链接不能直接 copy（目标可能不存在）
        let meta = match std::fs::symlink_metadata(&p) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            #[cfg(unix)]
            {
                if let Ok(link) = std::fs::read_link(&p) {
                    let _ = std::fs::remove_file(&target);
                    let _ = std::os::unix::fs::symlink(&link, &target);
                }
            }
        } else if meta.is_dir() {
            copy_dir_recursive(&p, &target)?;
        } else {
            std::fs::copy(&p, &target).map_err(|e| format!("复制 {} 失败: {e}", p.display()))?;
        }
    }
    Ok(())
}

// ============================================================
// 配置管理：用户设置 + 多个游戏目录（实例）
// ============================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameDir {
    pub path: String,
    pub name: String,
    pub has_versions: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LauncherConfig {
    pub username: String,
    pub game_dirs: Vec<String>,
    pub current_dir: String,
    /// 给游戏分配的最大堆内存（MB）
    #[serde(default = "default_max_mem")]
    pub max_memory_mb: u64,
    /// 初始堆内存（MB）
    #[serde(default = "default_min_mem")]
    pub min_memory_mb: u64,
}

fn default_max_mem() -> u64 { 3072 }
fn default_min_mem() -> u64 { 512 }

impl Default for LauncherConfig {
    fn default() -> Self {
        let default_dir = format!("{}/MinecraftBigFish", home_dir().display());
        Self {
            username: "George".into(),
            game_dirs: vec![default_dir.clone()],
            current_dir: default_dir,
            max_memory_mb: default_max_mem(),
            min_memory_mb: default_min_mem(),
        }
    }
}

/// 物理内存总量（MB）
pub fn total_memory_mb() -> u64 {
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("sysctl").args(["-n", "hw.memsize"]).output() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                if let Ok(bytes) = s.trim().parse::<u64>() {
                    return bytes / 1048576;
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(s) = std::fs::read_to_string("/proc/meminfo") {
            for line in s.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    if let Ok(kb) = rest.trim().trim_end_matches("kB").trim().parse::<u64>() {
                        return kb / 1024;
                    }
                }
            }
        }
    }
    8192
}

pub fn config_path() -> std::path::PathBuf {
    home_dir().join(".bigfish").join("config.json")
}

pub fn load_config() -> LauncherConfig {
    let p = config_path();
    match std::fs::read_to_string(&p) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => LauncherConfig::default(),
    }
}

pub fn save_config(cfg: &LauncherConfig) -> Result<(), String> {
    let p = config_path();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let s = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&p, s).map_err(|e| e.to_string())
}

pub fn dir_has_versions(dir: &str) -> bool {
    std::path::Path::new(dir).join("versions").is_dir()
}

pub fn list_game_dirs() -> Vec<GameDir> {
    let cfg = load_config();
    cfg.game_dirs.iter().map(|p| {
        let name = std::path::Path::new(p).file_name()
            .map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| p.clone());
        GameDir { path: p.clone(), name, has_versions: dir_has_versions(p) }
    }).collect()
}

pub fn add_game_dir(path: &str) -> Result<LauncherConfig, String> {
    let p = std::path::Path::new(path);
    if !p.is_dir() { return Err(format!("目录不存在: {path}")); }
    let mut cfg = load_config();
    if !cfg.game_dirs.iter().any(|d| d == path) { cfg.game_dirs.push(path.to_string()); }
    cfg.current_dir = path.to_string();
    save_config(&cfg)?;
    Ok(cfg)
}

pub fn remove_game_dir(path: &str) -> Result<LauncherConfig, String> {
    let mut cfg = load_config();
    cfg.game_dirs.retain(|d| d != path);
    if cfg.current_dir == path {
        cfg.current_dir = cfg.game_dirs.first().cloned().unwrap_or_default();
    }
    save_config(&cfg)?;
    Ok(cfg)
}

pub fn set_current_dir(path: &str) -> Result<LauncherConfig, String> {
    let mut cfg = load_config();
    if !cfg.game_dirs.iter().any(|d| d == path) { cfg.game_dirs.push(path.to_string()); }
    cfg.current_dir = path.to_string();
    save_config(&cfg)?;
    Ok(cfg)
}

/// 从版本 JSON 里识别 mod 加载器（Forge / Fabric / NeoForge / 原版…）。
/// 以 mainClass + libraries 为准，目录名只作辅助，避免「1.21.5」这种
/// 目录名看不出是 Forge 的情况。
fn detect_loader(id: &str, json: &serde_json::Value) -> String {
    let main = json["mainClass"].as_str().unwrap_or("").to_lowercase();
    let libs = json["libraries"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|l| l["name"].as_str())
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
        .to_lowercase();
    let hay = format!("{main} {libs} {}", id.to_lowercase());
    if hay.contains("neoforge") || hay.contains("net.neoforged") {
        "neoforge".to_string()
    } else if hay.contains("fabric") {
        "fabric".to_string()
    } else if hay.contains("quilt") {
        "quilt".to_string()
    } else if hay.contains("bootstraplauncher")
        || hay.contains("minecraftforge")
        || hay.contains("forge")
    {
        "forge".to_string()
    } else if hay.contains("optifine") {
        "optifine".to_string()
    } else {
        "vanilla".to_string()
    }
}

/// 已安装版本：类型和加载器都从版本 JSON 读取，
/// 这样 Forge/Fabric 这类不在官方清单里的实例也能显示真实信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVersion {
    pub id: String,
    /// 版本类型：release / snapshot / old_beta / old_alpha
    #[serde(rename = "type")]
    pub kind: String,
    /// mod 加载器：forge / fabric / neoforge / quilt / optifine / vanilla
    pub loader: String,
    pub release_time: String,
}

pub fn list_installed_versions(game_dir: &str) -> Vec<InstalledVersion> {
    let dir = std::path::Path::new(game_dir).join("versions");
    let Ok(entries) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut out: Vec<InstalledVersion> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| {
            let id = e.file_name().to_string_lossy().to_string();
            let json_path = e.path().join(format!("{id}.json"));
            let parsed = std::fs::read_to_string(&json_path)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
            let (kind, release_time, loader) = match &parsed {
                Some(j) => (
                    j["type"].as_str().unwrap_or("unknown").to_string(),
                    j["releaseTime"].as_str().unwrap_or("").to_string(),
                    detect_loader(&id, j),
                ),
                None => ("unknown".to_string(), String::new(), "unknown".to_string()),
            };
            InstalledVersion { id, kind, loader, release_time }
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// 展开 `~/` 前缀为用户主目录
fn expand_home(p: &str) -> String {
    if let Some(rest) = p.strip_prefix("~/") {
        return format!("{}/{rest}", home_dir().display());
    }
    p.to_string()
}

/// 批量删除已安装版本（只删除 versions/<id> 目录，不动存档和配置）
pub fn delete_versions(game_dir: &str, ids: &[String]) -> Result<usize, String> {
    let mut removed = 0;
    for id in ids {
        let p = std::path::Path::new(game_dir).join("versions").join(id);
        if p.is_dir() {
            std::fs::remove_dir_all(&p).map_err(|e| format!("删除「{id}」失败: {e}"))?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// 批量导出已安装版本到目标目录（dest/<id>/），返回实际导出路径
pub fn export_versions(game_dir: &str, ids: &[String], dest: &str) -> Result<String, String> {
    let dest = std::path::PathBuf::from(expand_home(dest));
    std::fs::create_dir_all(&dest).map_err(|e| format!("创建导出目录失败: {e}"))?;
    for id in ids {
        let src = std::path::Path::new(game_dir).join("versions").join(id);
        if !src.is_dir() {
            return Err(format!("版本「{id}」不存在"));
        }
        copy_dir_recursive(&src, &dest.join(id)).map_err(|e| format!("导出「{id}」失败: {e}"))?;
    }
    Ok(dest.display().to_string())
}
