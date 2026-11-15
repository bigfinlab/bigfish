//! BigFish 桌面启动器（Tauri 后端）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bigfish::VersionInfo;
use tauri::ipc::Channel;

#[tauri::command]
fn list_versions() -> Result<Vec<VersionInfo>, String> {
    bigfish::list_versions()
}

#[tauri::command]
async fn download_version(
    version_id: String,
    game_dir: String,
    on_progress: Channel<String>,
) -> Result<(), String> {
    let ch = on_progress.clone();
    let progress = move |msg: &str| {
        let _ = ch.send(msg.to_string());
    };
    bigfish::download_version(&version_id, &game_dir, &progress)
}

#[tauri::command]
fn launch_game(version_id: String, game_dir: String, username: String) -> Result<String, String> {
    match bigfish::launch_version(&version_id, &game_dir, &username) {
        Ok(pid) => Ok(format!("游戏已启动 (pid {pid})")),
        Err(e) => Err(e),
    }
}

#[tauri::command]
fn launch_preview(version_id: String, game_dir: String, username: String) -> Result<String, String> {
    bigfish::launch_preview(&version_id, &game_dir, &username)
}

#[tauri::command]
fn get_config() -> bigfish::LauncherConfig {
    bigfish::load_config()
}

#[tauri::command]
fn save_username(username: String) -> Result<bigfish::LauncherConfig, String> {
    let mut cfg = bigfish::load_config();
    cfg.username = username;
    bigfish::save_config(&cfg)?;
    Ok(cfg)
}

#[tauri::command]
fn list_game_dirs() -> Vec<bigfish::GameDir> {
    bigfish::list_game_dirs()
}

#[tauri::command]
fn add_game_dir(path: String) -> Result<bigfish::LauncherConfig, String> {
    bigfish::add_game_dir(&path)
}

#[tauri::command]
fn set_current_dir(path: String) -> Result<bigfish::LauncherConfig, String> {
    bigfish::set_current_dir(&path)
}

#[tauri::command]
fn remove_game_dir(path: String) -> Result<bigfish::LauncherConfig, String> {
    bigfish::remove_game_dir(&path)
}

#[tauri::command]
fn list_installed_versions(game_dir: String) -> Vec<bigfish::InstalledVersion> {
    bigfish::list_installed_versions(&game_dir)
}

#[tauri::command]
fn app_info() -> serde_json::Value {
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "os_version": bigfish::os_version(),
        "changelog": include_str!("../../CHANGELOG.md"),
    })
}

#[tauri::command]
fn system_memory() -> u64 {
    bigfish::total_memory_mb()
}

#[tauri::command]
fn save_memory(max_mb: u64) -> Result<bigfish::LauncherConfig, String> {
    let mut cfg = bigfish::load_config();
    cfg.max_memory_mb = max_mb.max(512);
    bigfish::save_config(&cfg)?;
    Ok(cfg)
}

#[tauri::command]
fn list_javas() -> Vec<bigfish::JavaInfo> {
    bigfish::scan_local_javas()
}

#[tauri::command]
fn java_need(version_id: String, game_dir: String) -> u8 {
    bigfish::java_requirement(&version_id, &game_dir)
}

#[tauri::command]
async fn install_java(major: u8, on_progress: Channel<String>) -> Result<bigfish::JavaInfo, String> {
    let ch = on_progress.clone();
    let progress = move |msg: &str| {
        let _ = ch.send(msg.to_string());
    };
    bigfish::install_java(major, &progress)
}

#[tauri::command]
fn delete_versions(game_dir: String, ids: Vec<String>) -> Result<usize, String> {
    bigfish::delete_versions(&game_dir, &ids)
}

#[tauri::command]
fn export_versions(game_dir: String, ids: Vec<String>, dest: String) -> Result<String, String> {
    bigfish::export_versions(&game_dir, &ids, &dest)
}

#[tauri::command]
fn scan_saves() -> Vec<bigfish::SaveInfo> {
    bigfish::scan_saves()
}

#[tauri::command]
fn import_save(source_path: String, target_game_dir: String) -> Result<String, String> {
    bigfish::import_save(&source_path, &target_game_dir)
}

#[tauri::command]
fn offline_uuid(name: String) -> String {
    bigfish::offline_uuid(&name)
}

#[tauri::command]
fn default_game_dir() -> String {
    // 默认游戏目录：~/MinecraftBigFish
    bigfish::home_dir().join("MinecraftBigFish").display().to_string()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_versions,
            download_version,
            launch_game,
            launch_preview,
            get_config,
            save_username,
            list_game_dirs,
            add_game_dir,
            set_current_dir,
            remove_game_dir,
            list_installed_versions,
            delete_versions,
            export_versions,
            app_info,
            system_memory,
            save_memory,
            list_javas,
            java_need,
            install_java,
            scan_saves,
            import_save,
            offline_uuid,
            default_game_dir
        ])
        .run(tauri::generate_context!())
        .expect("BigFish 启动失败");
}
