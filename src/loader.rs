//! 加载器自动安装：Fabric / Quilt（Meta API）与 Forge / NeoForge（installer）
//!
//! 设计要点：
//! - Fabric/Quilt 走官方 Meta API 生成 profile JSON，纯 Rust 下载依赖，不需要 Java。
//! - Forge/NeoForge 使用官方 installer jar，需要一个可用的 Java 来执行。

use crate::{fetch, fetch_bytes, find_java_for, make_agent, save_file, Progress};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

/// 解析 maven 坐标 `group:artifact:version[:classifier]` 为仓库内相对路径
pub fn maven_path(name: &str) -> Option<String> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0];
    let artifact = parts[1];
    let version = parts[2];
    let classifier = if parts.len() >= 4 { Some(parts[3]) } else { None };
    let file = match classifier {
        Some(c) => format!("{artifact}-{version}-{c}.jar"),
        None => format!("{artifact}-{version}.jar"),
    };
    Some(format!(
        "{}/{}/{}/{}",
        group.replace('.', "/"),
        artifact,
        version,
        file
    ))
}

/// 把 profile 里所有 libraries 下载到 `libraries/` 下，返回 (成功数, 跳过数)
fn download_profile_libraries(
    root: &Path,
    profile: &serde_json::Value,
    on_progress: Progress,
) -> Result<(usize, usize), String> {
    let libs = profile["libraries"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let agent = make_agent(120);
    let mut done = 0usize;
    let mut skipped = 0usize;
    for (i, lib) in libs.iter().enumerate() {
        let Some(name) = lib["name"].as_str() else {
            continue;
        };
        let Some(rel) = maven_path(name) else {
            continue;
        };
        let dest = root.join("libraries").join(&rel);
        if dest.exists() {
            skipped += 1;
            continue;
        }
        let base = lib["url"]
            .as_str()
            .unwrap_or("https://maven.fabricmc.net/")
            .trim_end_matches('/');
        let url = format!("{base}/{rel}");
        let bytes = fetch_bytes(&agent, &url)?;
        save_file(&bytes, &dest, "")?;
        done += 1;
        if i % 5 == 0 || i + 1 == libs.len() {
            on_progress(&format!("下载库 {}/{}", i + 1, libs.len()));
        }
    }
    Ok((done, skipped))
}

// ---------------------------------------------------------------------------
// Fabric
// ---------------------------------------------------------------------------

/// 列出某游戏版本可用的 Fabric Loader 版本
pub fn list_fabric_loaders(game_version: &str) -> Result<Vec<LoaderVersion>, String> {
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{game_version}");
    let body = fetch(&url)?;
    let arr: Vec<serde_json::Value> = serde_json::from_str(&body)
        .map_err(|e| format!("Fabric Meta 返回格式异常: {e}"))?;
    let mut out: Vec<LoaderVersion> = arr
        .iter()
        .filter_map(|item| {
            let loader = item.get("loader")?;
            Some(LoaderVersion {
                version: loader["version"].as_str()?.to_string(),
                stable: loader["stable"].as_bool().unwrap_or(false),
            })
        })
        .collect();
    // 稳定的排前面
    out.sort_by(|a, b| b.stable.cmp(&a.stable));
    Ok(out)
}

/// 安装 Fabric Loader，生成 `versions/<id>/<id>.json` 并下载依赖库
pub fn install_fabric(
    game_dir: &str,
    game_version: &str,
    loader_version: &str,
    on_progress: Progress,
) -> Result<String, String> {
    let root = Path::new(game_dir);
    // 前置检查：原版必须已下载（Fabric profile 依赖 inheritsFrom）
    let vanilla_json = root
        .join("versions")
        .join(game_version)
        .join(format!("{game_version}.json"));
    if !vanilla_json.exists() {
        return Err(format!(
            "需要先下载原版 {game_version}，再安装 Fabric（现在原版还没下载）"
        ));
    }

    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{game_version}/{loader_version}/profile/json"
    );
    on_progress(&format!("获取 Fabric profile: {loader_version}"));
    let body = fetch(&url)?;
    let profile: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Fabric profile 解析失败: {e}"))?;
    let id = profile["id"]
        .as_str()
        .ok_or("Fabric profile 缺少 id 字段")?
        .to_string();

    let ver_dir = root.join("versions").join(&id);
    std::fs::create_dir_all(&ver_dir).map_err(|e| e.to_string())?;
    std::fs::write(ver_dir.join(format!("{id}.json")), &body).map_err(|e| e.to_string())?;
    on_progress("✔ 版本 JSON 已生成");

    let (done, skipped) = download_profile_libraries(root, &profile, on_progress)?;
    on_progress(&format!(
        "✔ Fabric {loader_version} 安装完成（新下载 {done} 个库，已存在 {skipped} 个）"
    ));
    Ok(id)
}

// ---------------------------------------------------------------------------
// Quilt（与 Fabric 类似，走 Quilt Meta）
// ---------------------------------------------------------------------------

/// 安装 Quilt Loader
pub fn install_quilt(
    game_dir: &str,
    game_version: &str,
    loader_version: &str,
    on_progress: Progress,
) -> Result<String, String> {
    let root = Path::new(game_dir);
    let vanilla_json = root
        .join("versions")
        .join(game_version)
        .join(format!("{game_version}.json"));
    if !vanilla_json.exists() {
        return Err(format!("需要先下载原版 {game_version}，再安装 Quilt"));
    }
    let url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{game_version}/{loader_version}/profile/json"
    );
    on_progress(&format!("获取 Quilt profile: {loader_version}"));
    let body = fetch(&url)?;
    let profile: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Quilt profile 解析失败: {e}"))?;
    let id = profile["id"]
        .as_str()
        .ok_or("Quilt profile 缺少 id 字段")?
        .to_string();
    let ver_dir = root.join("versions").join(&id);
    std::fs::create_dir_all(&ver_dir).map_err(|e| e.to_string())?;
    std::fs::write(ver_dir.join(format!("{id}.json")), &body).map_err(|e| e.to_string())?;
    on_progress("✔ 版本 JSON 已生成");
    // Quilt profile 的 url 字段可能是完整 maven 库地址，统一按 maven 处理
    let libs = profile["libraries"].as_array().cloned().unwrap_or_default();
    let agent = make_agent(120);
    let mut done = 0usize;
    for (i, lib) in libs.iter().enumerate() {
        let Some(name) = lib["name"].as_str() else { continue };
        let Some(rel) = maven_path(name) else { continue };
        let dest = root.join("libraries").join(&rel);
        if dest.exists() {
            continue;
        }
        let base = lib["url"]
            .as_str()
            .unwrap_or("https://maven.quiltmc.org/repository/release/")
            .trim_end_matches('/');
        let url = format!("{base}/{rel}");
        let bytes = fetch_bytes(&agent, &url)?;
        save_file(&bytes, &dest, "")?;
        done += 1;
        if i % 5 == 0 {
            on_progress(&format!("下载库 {}/{}", i + 1, libs.len()));
        }
    }
    on_progress(&format!("✔ Quilt {loader_version} 安装完成（新下载 {done} 个库）"));
    Ok(id)
}

// ---------------------------------------------------------------------------
// Forge
// ---------------------------------------------------------------------------

const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";

/// 列出某游戏版本可用的 Forge 版本（从 maven-metadata.xml 解析）
pub fn list_forge_versions(game_version: &str) -> Result<Vec<String>, String> {
    let url = format!("{FORGE_MAVEN}/maven-metadata.xml");
    let body = fetch(&url)?;
    let prefix = format!("{game_version}-");
    let mut out: Vec<String> = Vec::new();
    let mut rest = body.as_str();
    while let Some(start) = rest.find("<version>") {
        let after = &rest[start + "<version>".len()..];
        let Some(end) = after.find("</version>") else {
            break;
        };
        let ver = after[..end].trim();
        if ver.starts_with(&prefix) {
            // 只保留形如 1.20.1-47.2.0 的正式版
            let tail = &ver[prefix.len()..];
            if tail.chars().all(|c| c.is_ascii_digit() || c == '.') {
                out.push(ver.to_string());
            }
        }
        rest = &after[end + "</version>".len()..];
    }
    out.sort();
    out.reverse();
    Ok(out)
}

/// 安装 Forge（下载官方 installer.jar 并用 Java 执行 headless 安装）
pub fn install_forge(
    game_dir: &str,
    forge_version: &str,
    on_progress: Progress,
) -> Result<String, String> {
    let root = Path::new(game_dir);
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;

    // 找 Java：Forge installer 对 Java 版本有要求，优先用较高版本
    let major = if forge_version.contains("1.20.5") || forge_version.contains("1.21") {
        21
    } else if forge_version.contains("1.17") || forge_version.contains("1.18") || forge_version.contains("1.19") || forge_version.contains("1.20") {
        17
    } else {
        8
    };
    let java = find_java_for(major)
        .or_else(|| find_java_for(17))
        .or_else(|| find_java_for(21))
        .or_else(|| find_java_for(8))
        .ok_or_else(|| "安装 Forge 需要 Java，但本机没有找到合适的 Java".to_string())?;

    // 下载 installer
    let file = format!("forge-{forge_version}-installer.jar");
    let url = format!("{FORGE_MAVEN}/{forge_version}/{file}");
    let tmp_dir = std::env::temp_dir().join("bigfish-forge");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let installer = tmp_dir.join(&file);
    if !installer.exists() {
        on_progress(&format!("下载 Forge installer: {file}"));
        let agent = make_agent(300);
        let bytes = fetch_bytes(&agent, &url)?;
        save_file(&bytes, &installer, "")?;
    }
    on_progress(&format!("✔ installer 就绪（{}）", installer.display()));

    // 安装前记录版本目录，安装后对比找出新版本 id
    let versions_dir = root.join("versions");
    std::fs::create_dir_all(&versions_dir).map_err(|e| e.to_string())?;
    let before: std::collections::HashSet<String> = std::fs::read_dir(&versions_dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();

    on_progress(&format!("运行 Forge installer（Java {}）…", java.major));
    let output = std::process::Command::new(&java.path)
        .arg("-jar")
        .arg(&installer)
        .arg("--installClient")
        .arg(root)
        .output()
        .map_err(|e| format!("执行 Forge installer 失败: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let tail: String = format!("{stdout}\n{stderr}")
            .lines()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!("Forge installer 执行失败:\n{tail}"));
    }

    // 找出安装产生的新版本目录
    let after: Vec<String> = std::fs::read_dir(&versions_dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    let new_dirs: Vec<String> = after
        .into_iter()
        .filter(|d| !before.contains(d) && d.to_lowercase().contains("forge"))
        .collect();

    if let Some(id) = new_dirs.first() {
        on_progress(&format!("✔ Forge {forge_version} 安装完成：{id}"));
        Ok(id.clone())
    } else {
        // installer 可能没有产生新目录（已安装过），按命名规则推断
        let guess = format!("{forge_version}");
        if versions_dir.join(&guess).exists() {
            on_progress(&format!("✔ Forge {forge_version} 已是最新"));
            Ok(guess)
        } else {
            Err("Forge installer 运行成功，但没有找到生成的版本目录".to_string())
        }
    }
}
