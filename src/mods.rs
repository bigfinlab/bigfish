//! 模组管理：扫描 mods 目录、解析 jar 元数据、启用/禁用/删除
//!
//! 支持的元数据来源：
//! - `fabric.mod.json`（Fabric / Quilt）
//! - `META-INF/neoforge.mods.toml`（NeoForge 1.20.5+）
//! - `META-INF/mods.toml`（Forge 1.13+）
//! - `mcmod.info`（旧版 Forge）

use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct ModInfo {
    /// 磁盘上的文件名（可能是 xxx.jar 或 xxx.jar.disabled）
    pub file_name: String,
    /// 展示名（来自元数据，缺省时用文件名）
    pub name: String,
    pub version: String,
    pub mod_id: String,
    /// fabric / quilt / forge / neoforge / unknown
    pub loader: String,
    pub enabled: bool,
    pub size: u64,
    pub description: String,
}

const DISABLED_SUFFIX: &str = ".disabled";

/// mods 目录路径
pub fn mods_dir(game_dir: &str) -> PathBuf {
    Path::new(game_dir).join("mods")
}

fn is_jar_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".jar") || lower.ends_with(".jar.disabled")
}

/// 从 jar 里读取一个条目的内容（读不到返回 None）
fn read_zip_entry(path: &Path, entry_name: &str) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut entry = archive.by_name(entry_name).ok()?;
    let mut buf = String::new();
    entry.read_to_string(&mut buf).ok()?;
    Some(buf)
}

/// 从 TOML 文本里提取某个 key 的第一个字符串值（容忍 `${...}` 占位符）
fn toml_first_string(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        // 跳过注释
        if line.starts_with('#') {
            continue;
        }
        let Some(rest) = line.strip_prefix(key) else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim();
        let value = rest.trim_matches('"').trim_matches('\'').trim();
        if value.is_empty() || value.contains("${") {
            continue;
        }
        return Some(value.to_string());
    }
    None
}

/// 解析单个模组 jar 的元数据
fn parse_mod_meta(path: &Path) -> (String, String, String, String, String) {
    // 返回 (name, version, mod_id, loader, description)

    // 1) Fabric / Quilt
    if let Some(json) = read_zip_entry(path, "fabric.mod.json") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
            let id = v["id"].as_str().unwrap_or("").to_string();
            let name = v["name"].as_str().unwrap_or("").to_string();
            let version = v["version"].as_str().unwrap_or("").to_string();
            let desc = v["description"].as_str().unwrap_or("").to_string();
            let loader = if json.contains("quilt_loader") {
                "quilt"
            } else {
                "fabric"
            };
            return (
                if name.is_empty() { id.clone() } else { name },
                version,
                id,
                loader.to_string(),
                desc,
            );
        }
    }

    // 2) NeoForge / Forge（TOML）
    for (entry, loader) in [
        ("META-INF/neoforge.mods.toml", "neoforge"),
        ("META-INF/mods.toml", "forge"),
    ] {
        if let Some(toml) = read_zip_entry(path, entry) {
            let id = toml_first_string(&toml, "modId").unwrap_or_default();
            let name = toml_first_string(&toml, "displayName").unwrap_or_default();
            let version = toml_first_string(&toml, "version").unwrap_or_default();
            let desc = toml_first_string(&toml, "description").unwrap_or_default();
            if !id.is_empty() || !name.is_empty() {
                return (
                    if name.is_empty() { id.clone() } else { name },
                    version,
                    id,
                    loader.to_string(),
                    desc,
                );
            }
        }
    }

    // 3) 旧版 Forge mcmod.info（JSON 数组）
    if let Some(json) = read_zip_entry(path, "mcmod.info") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
            let first = if v.is_array() {
                v.get(0).cloned().unwrap_or_default()
            } else {
                v.clone()
            };
            let id = first["modid"].as_str().unwrap_or("").to_string();
            let name = first["name"].as_str().unwrap_or("").to_string();
            let version = first["version"].as_str().unwrap_or("").to_string();
            let desc = first["description"].as_str().unwrap_or("").to_string();
            if !id.is_empty() {
                return (
                    if name.is_empty() { id.clone() } else { name },
                    version,
                    id,
                    "forge".to_string(),
                    desc,
                );
            }
        }
    }

    // 4) 兜底：文件名
    let stem = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
        .trim_end_matches(DISABLED_SUFFIX)
        .trim_end_matches(".jar")
        .to_string();
    (stem.clone(), String::new(), String::new(), "unknown".to_string(), String::new())
}

/// 扫描游戏目录下的所有模组
pub fn list_mods(game_dir: &str) -> Vec<ModInfo> {
    let dir = mods_dir(game_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<ModInfo> = entries
        .flatten()
        .filter(|e| e.path().is_file())
        .filter(|e| is_jar_name(&e.file_name().to_string_lossy()))
        .map(|e| {
            let path = e.path();
            let file_name = e.file_name().to_string_lossy().to_string();
            let enabled = !file_name.to_lowercase().ends_with(DISABLED_SUFFIX);
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            let (name, version, mod_id, loader, description) = parse_mod_meta(&path);
            ModInfo {
                file_name,
                name,
                version,
                mod_id,
                loader,
                enabled,
                size,
                description,
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

fn has_disabled_suffix(name: &str) -> bool {
    name.to_lowercase().ends_with(DISABLED_SUFFIX)
}

/// 启用/禁用一个模组（重命名 jar）
pub fn set_mod_enabled(game_dir: &str, file_name: &str, enabled: bool) -> Result<String, String> {
    let dir = mods_dir(game_dir);
    let src = dir.join(file_name);
    if !src.is_file() {
        return Err(format!("找不到模组文件: {file_name}"));
    }
    let currently_enabled = !has_disabled_suffix(file_name);
    if currently_enabled == enabled {
        return Ok(file_name.to_string());
    }
    let target_name = if enabled {
        file_name.trim_end_matches(DISABLED_SUFFIX).to_string()
    } else {
        format!("{file_name}{DISABLED_SUFFIX}")
    };
    let dst = dir.join(&target_name);
    if dst.exists() {
        return Err(format!("目标文件已存在: {target_name}"));
    }
    std::fs::rename(&src, &dst).map_err(|e| format!("重命名失败: {e}"))?;
    Ok(target_name)
}

/// 批量删除模组
pub fn delete_mods(game_dir: &str, file_names: &[String]) -> Result<usize, String> {
    let dir = mods_dir(game_dir);
    let mut removed = 0;
    for name in file_names {
        let p = dir.join(name);
        if p.is_file() {
            std::fs::remove_file(&p).map_err(|e| format!("删除「{name}」失败: {e}"))?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// 打开 mods 目录（macOS / Windows / Linux 各自用系统文件管理器）
pub fn open_mods_dir(game_dir: &str) -> Result<(), String> {
    let dir = mods_dir(game_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建 mods 目录失败: {e}"))?;
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    #[cfg(all(unix, not(target_os = "macos")))]
    let cmd = "xdg-open";
    std::process::Command::new(cmd)
        .arg(&dir)
        .spawn()
        .map_err(|e| format!("打开目录失败: {e}"))?;
    Ok(())
}
