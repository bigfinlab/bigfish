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

// ---------- 加载器（Fabric / Quilt / Forge） ----------

#[tauri::command]
fn check_vanilla_installed(game_dir: String, game_version: String) -> bool {
    let p = std::path::Path::new(&game_dir)
        .join("versions")
        .join(&game_version)
        .join(format!("{game_version}.json"));
    p.exists()
}

#[tauri::command]
fn list_fabric_loaders(game_version: String) -> Result<Vec<bigfish::loader::LoaderVersion>, String> {
    bigfish::loader::list_fabric_loaders(&game_version)
}

#[tauri::command]
async fn install_fabric(
    game_dir: String,
    game_version: String,
    loader_version: String,
    on_progress: Channel<String>,
) -> Result<String, String> {
    let ch = on_progress.clone();
    let progress = move |msg: &str| {
        let _ = ch.send(msg.to_string());
    };
    bigfish::loader::install_fabric(&game_dir, &game_version, &loader_version, &progress)
}

#[tauri::command]
async fn install_quilt(
    game_dir: String,
    game_version: String,
    loader_version: String,
    on_progress: Channel<String>,
) -> Result<String, String> {
    let ch = on_progress.clone();
    let progress = move |msg: &str| {
        let _ = ch.send(msg.to_string());
    };
    bigfish::loader::install_quilt(&game_dir, &game_version, &loader_version, &progress)
}

#[tauri::command]
fn list_forge_versions(game_version: String) -> Result<Vec<String>, String> {
    bigfish::loader::list_forge_versions(&game_version)
}

#[tauri::command]
async fn install_forge(
    game_dir: String,
    forge_version: String,
    on_progress: Channel<String>,
) -> Result<String, String> {
    let ch = on_progress.clone();
    let progress = move |msg: &str| {
        let _ = ch.send(msg.to_string());
    };
    bigfish::loader::install_forge(&game_dir, &forge_version, &progress)
}

// ---------- 微软正版登录 ----------

#[tauri::command]
fn get_auth_state() -> serde_json::Value {
    let cfg = bigfish::load_config();
    let saved = bigfish::auth::load_saved_auth();
    let logged_in = saved.is_some();
    serde_json::json!({
        "client_id_configured": !cfg.auth_client_id.trim().is_empty(),
        "client_id": cfg.auth_client_id,
        "logged_in": logged_in,
        "username": saved.as_ref().map(|s| s.profile.name.clone()).unwrap_or_default(),
        "uuid": saved.as_ref().map(|s| s.profile.id.clone()).unwrap_or_default(),
    })
}

#[tauri::command]
fn save_auth_client_id(client_id: String) -> Result<bigfish::LauncherConfig, String> {
    let mut cfg = bigfish::load_config();
    cfg.auth_client_id = client_id.trim().to_string();
    bigfish::save_config(&cfg)?;
    Ok(cfg)
}

#[tauri::command]
fn start_ms_login() -> Result<bigfish::auth::DeviceCode, String> {
    let cfg = bigfish::load_config();
    bigfish::auth::start_device_flow(&cfg.auth_client_id)
}

#[tauri::command]
fn poll_ms_login(device_code: String) -> Result<String, String> {
    let cfg = bigfish::load_config();
    match bigfish::auth::poll_device_token(&cfg.auth_client_id, &device_code)? {
        bigfish::auth::PollState::Pending => Ok("pending".to_string()),
        bigfish::auth::PollState::SlowDown => Ok("slow".to_string()),
        bigfish::auth::PollState::Failed(e) => Err(e),
        bigfish::auth::PollState::Ready(ms) => {
            let (mc_token, profile) = bigfish::auth::complete_login(&ms.access_token)?;
            let saved = bigfish::auth::SavedAuth {
                ms_token: ms,
                mc_token,
                profile,
            };
            bigfish::auth::save_auth(&saved)?;
            let mut cfg = bigfish::load_config();
            cfg.auth_username = saved.profile.name.clone();
            let _ = bigfish::save_config(&cfg);
            Ok(format!("ready:{}", saved.profile.name))
        }
    }
}

#[tauri::command]
fn logout_ms() -> Result<(), String> {
    bigfish::auth::clear_auth()
}

#[tauri::command]
fn launch_game_ms(version_id: String, game_dir: String) -> Result<String, String> {
    let saved = bigfish::auth::load_saved_auth()
        .ok_or_else(|| "还没有登录微软账号".to_string())?;
    let cfg = bigfish::load_config();
    let saved = if bigfish::auth::token_expired(&saved.ms_token) {
        bigfish::auth::refresh_saved_auth(&cfg.auth_client_id, &saved)?
    } else {
        saved
    };
    let auth = bigfish::LaunchAuth::microsoft(
        &saved.profile.name,
        &saved.profile.id,
        &saved.mc_token,
    );
    match bigfish::launch_version_with_auth(&version_id, &game_dir, &auth) {
        Ok(pid) => Ok(format!(
            "游戏已启动（正版账号 {}，pid {pid}）",
            saved.profile.name
        )),
        Err(e) => Err(e),
    }
}

// ---------- 模组管理 ----------

#[tauri::command]
fn list_mods(game_dir: String) -> Vec<bigfish::mods::ModInfo> {
    bigfish::mods::list_mods(&game_dir)
}

#[tauri::command]
fn toggle_mod(game_dir: String, file_name: String, enabled: bool) -> Result<String, String> {
    bigfish::mods::set_mod_enabled(&game_dir, &file_name, enabled)
}

#[tauri::command]
fn delete_mods(game_dir: String, file_names: Vec<String>) -> Result<usize, String> {
    bigfish::mods::delete_mods(&game_dir, &file_names)
}

#[tauri::command]
fn open_mods_dir(game_dir: String) -> Result<(), String> {
    bigfish::mods::open_mods_dir(&game_dir)
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
            default_game_dir,
            check_vanilla_installed,
            list_fabric_loaders,
            install_fabric,
            install_quilt,
            list_forge_versions,
            install_forge,
            get_auth_state,
            save_auth_client_id,
            start_ms_login,
            poll_ms_login,
            logout_ms,
            launch_game_ms,
            list_mods,
            toggle_mod,
            delete_mods,
            open_mods_dir
        ])
        .run(tauri::generate_context!())
        .expect("BigFish 启动失败");
}
