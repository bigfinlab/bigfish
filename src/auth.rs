//! 微软正版登录：OAuth 2.0 设备码流程 → Xbox Live → XSTS → Minecraft 服务
//!
//! 为什么用设备码流程（device code flow）：
//! - 桌面启动器不需要开本地 HTTP 服务、不需要配置重定向 URI
//! - 用户在浏览器里输入一次性代码即可授权，体验接近 HMCL/PCL2
//!
//! 注意：`client_id` 必须由开发者自己在 Azure 注册应用获得（见 README），
//! 没有配置时所有登录函数会返回明确的错误提示，不影响离线模式。

use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const SCOPE: &str = "XboxLive.signin offline_access";

/// 设备码登录的初始信息
#[derive(Debug, Clone, Serialize)]
pub struct DeviceCode {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

/// Minecraft 玩家档案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McProfile {
    /// 无横线的 32 位 UUID
    pub id: String,
    pub name: String,
}

/// 微软 access token（含 refresh token）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsToken {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix 秒
    pub expires_at: i64,
}

/// 轮询结果
pub enum PollState {
    Pending,
    SlowDown,
    Ready(MsToken),
    Failed(String),
}

fn agent() -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            // 关键：4xx/5xx 不抛错，方便读取 OAuth 的错误响应体
            .http_status_as_error(false)
            .user_agent("BigFishLauncher/0.1 (Minecraft launcher)")
            .build(),
    )
}

fn post_form(url: &str, body: &str) -> Result<(u16, String), String> {
    let resp = agent()
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send(body)
        .map_err(|e| format!("请求失败 {url}: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp
        .into_body()
        .read_to_string()
        .map_err(|e| format!("读取响应失败: {e}"))?;
    Ok((status, text))
}

fn post_json(url: &str, body: &str) -> Result<(u16, String), String> {
    let resp = agent()
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .send(body)
        .map_err(|e| format!("请求失败 {url}: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp
        .into_body()
        .read_to_string()
        .map_err(|e| format!("读取响应失败: {e}"))?;
    Ok((status, text))
}

fn get_bearer(url: &str, token: &str) -> Result<(u16, String), String> {
    let resp = agent()
        .get(url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|e| format!("请求失败 {url}: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp
        .into_body()
        .read_to_string()
        .map_err(|e| format!("读取响应失败: {e}"))?;
    Ok((status, text))
}

fn client_id_or_err(client_id: &str) -> Result<String, String> {
    let id = client_id.trim();
    if id.is_empty() {
        return Err(
            "还没有配置 Azure 应用的 client_id。\n\n\
             请先注册 Azure 应用（README 有步骤），然后在「设置 → 微软登录」里填入 \
             Application (client) ID。\n\
             没有 client_id 时仍然可以使用离线登录。"
                .to_string(),
        );
    }
    Ok(id.to_string())
}

/// 第一步：申请设备码，返回给用户去浏览器授权
pub fn start_device_flow(client_id: &str) -> Result<DeviceCode, String> {
    let cid = client_id_or_err(client_id)?;
    let body = format!(
        "client_id={cid}&scope={}",
        urlencode(SCOPE)
    );
    let (status, text) = post_form(DEVICE_CODE_URL, &body)?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("设备码响应解析失败: {e}（原文: {text}）"))?;
    if let Some(err) = v["error"].as_str() {
        return Err(format!(
            "申请设备码被拒绝(HTTP {status}): {err} {}",
            v["error_description"].as_str().unwrap_or("")
        ));
    }
    Ok(DeviceCode {
        user_code: v["user_code"].as_str().unwrap_or("").to_string(),
        device_code: v["device_code"].as_str().unwrap_or("").to_string(),
        verification_uri: v["verification_uri"]
            .as_str()
            .unwrap_or("https://microsoft.com/devicelogin")
            .to_string(),
        expires_in: v["expires_in"].as_u64().unwrap_or(900),
        interval: v["interval"].as_u64().unwrap_or(5),
        message: v["message"].as_str().unwrap_or("").to_string(),
    })
}

/// 第二步：轮询用户是否完成授权（调用方按 interval 秒循环调用）
pub fn poll_device_token(client_id: &str, device_code: &str) -> Result<PollState, String> {
    let cid = client_id_or_err(client_id)?;
    let body = format!(
        "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code&client_id={cid}&device_code={}",
        urlencode(device_code)
    );
    let (_status, text) = post_form(TOKEN_URL, &body)?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("token 响应解析失败: {e}（原文: {text}）"))?;

    if let Some(err) = v["error"].as_str() {
        return Ok(match err {
            "authorization_pending" => PollState::Pending,
            "slow_down" => PollState::SlowDown,
            "authorization_declined" => PollState::Failed("你（或操作者）拒绝了本次授权".into()),
            "expired_token" => PollState::Failed("设备码已过期，请重新发起登录".into()),
            "bad_verification_code" => PollState::Failed("设备码无效".into()),
            other => PollState::Failed(format!(
                "{other}: {}",
                v["error_description"].as_str().unwrap_or("")
            )),
        });
    }

    let access_token = v["access_token"].as_str().unwrap_or("").to_string();
    if access_token.is_empty() {
        return Err(format!("token 响应里没有 access_token（原文: {text}）"));
    }
    let refresh_token = v["refresh_token"].as_str().unwrap_or("").to_string();
    let expires_in = v["expires_in"].as_i64().unwrap_or(3600);
    Ok(PollState::Ready(MsToken {
        access_token,
        refresh_token,
        expires_at: now_unix() + expires_in,
    }))
}

// ---------------------------------------------------------------------------
// Xbox Live → XSTS → Minecraft
// ---------------------------------------------------------------------------

/// Xbox Live 认证，返回 (xbl_token, user_hash)
fn xbox_live_auth(ms_token: &str) -> Result<(String, String), String> {
    let body = format!(
        r#"{{"Properties":{{"AuthMethod":"RPS","SiteName":"user.auth.xboxlive.com","RpsTicket":"d={}"}},"RelyingParty":"http://auth.xboxlive.com","TokenType":"JWT"}}"#,
        ms_token
    );
    let (status, text) = post_json(XBL_URL, &body)?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("Xbox Live 响应解析失败(HTTP {status}): {e}"))?;
    let token = v["Token"].as_str().unwrap_or("").to_string();
    let uhs = v["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if token.is_empty() || uhs.is_empty() {
        return Err(format!(
            "Xbox Live 认证失败(HTTP {status}): {}",
            v["Message"].as_str().unwrap_or("未知错误")
        ));
    }
    Ok((token, uhs))
}

/// XSTS 授权，返回 (xsts_token, user_hash)
fn xsts_auth(xbl_token: &str) -> Result<(String, String), String> {
    let body = format!(
        r#"{{"Properties":{{"SandboxId":"RETAIL","UserTokens":["{}"]}},"RelyingParty":"rp://api.minecraftservices.com/","TokenType":"JWT"}}"#,
        xbl_token
    );
    let (status, text) = post_json(XSTS_URL, &body)?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("XSTS 响应解析失败(HTTP {status}): {e}"))?;
    let token = v["Token"].as_str().unwrap_or("").to_string();
    let uhs = v["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if token.is_empty() || uhs.is_empty() {
        let xerr = v["XErr"].as_u64().unwrap_or(0);
        let hint = match xerr {
            2148916233 => "该微软账号还没有创建 Xbox 账户（先去 xbox.com 登录一次即可）",
            2148916235 => "该账号所在地区不支持 Xbox 服务",
            2148916236 | 2148916237 => "该账号需要成年验证（家长控制）",
            2148916238 => "未成年账号需要先被添加进家庭组",
            _ => "请确认该微软账号已开通 Xbox 服务",
        };
        return Err(format!("XSTS 授权失败(HTTP {status}): {hint}（XErr={xerr}）"));
    }
    Ok((token, uhs))
}

/// 用 XSTS token 换取 Minecraft access token
fn minecraft_login(xsts_token: &str, uhs: &str) -> Result<String, String> {
    let body = format!(
        r#"{{"identityToken":"XBL3.0 x={};{}"}}"#,
        uhs, xsts_token
    );
    let (status, text) = post_json(MC_LOGIN_URL, &body)?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("Minecraft 登录响应解析失败(HTTP {status}): {e}"))?;
    let token = v["access_token"].as_str().unwrap_or("").to_string();
    if token.is_empty() {
        return Err(format!(
            "Minecraft 登录失败(HTTP {status}): {}",
            v["errorMessage"].as_str().unwrap_or("未知错误")
        ));
    }
    Ok(token)
}

/// 查询玩家档案（正版 UUID + 名字）
pub fn minecraft_profile(mc_access_token: &str) -> Result<McProfile, String> {
    let (status, text) = get_bearer(MC_PROFILE_URL, mc_access_token)?;
    if status == 404 {
        return Err("这个微软账号没有购买 Minecraft Java 版。\n可以用离线登录继续玩。".to_string());
    }
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("档案解析失败(HTTP {status}): {e}"))?;
    let id = v["id"].as_str().unwrap_or("").to_string();
    let name = v["name"].as_str().unwrap_or("").to_string();
    if id.is_empty() || name.is_empty() {
        return Err(format!(
            "无法获取玩家档案(HTTP {status}): {}",
            v["errorMessage"].as_str().unwrap_or("未知错误")
        ));
    }
    Ok(McProfile { id, name })
}

/// 完整登录链：微软 token → Xbox → XSTS → Minecraft → 档案
/// 返回 (Minecraft access token, 玩家档案)
pub fn complete_login(ms_access_token: &str) -> Result<(String, McProfile), String> {
    let (xbl, _uhs1) = xbox_live_auth(ms_access_token)?;
    let (xsts, uhs2) = xsts_auth(&xbl)?;
    let mc_token = minecraft_login(&xsts, &uhs2)?;
    let profile = minecraft_profile(&mc_token)?;
    Ok((mc_token, profile))
}

/// 用 refresh_token 刷新微软 access token（token 过期时使用）
pub fn refresh_ms_token(client_id: &str, refresh_token: &str) -> Result<MsToken, String> {
    let cid = client_id_or_err(client_id)?;
    let body = format!(
        "grant_type=refresh_token&client_id={cid}&refresh_token={}&scope={}",
        urlencode(refresh_token),
        urlencode(SCOPE)
    );
    let (_status, text) = post_form(TOKEN_URL, &body)?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("刷新响应解析失败: {e}"))?;
    if let Some(err) = v["error"].as_str() {
        return Err(format!(
            "刷新 token 失败: {err} {}",
            v["error_description"].as_str().unwrap_or("")
        ));
    }
    let access_token = v["access_token"].as_str().unwrap_or("").to_string();
    let new_refresh = v["refresh_token"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| refresh_token.to_string());
    let expires_in = v["expires_in"].as_i64().unwrap_or(3600);
    Ok(MsToken {
        access_token,
        refresh_token: new_refresh,
        expires_at: now_unix() + expires_in,
    })
}

// ---------------------------------------------------------------------------
// 工具
// ---------------------------------------------------------------------------

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// 登录状态持久化（~/.bigfish/auth.json）
// ---------------------------------------------------------------------------

/// 保存下来的完整登录状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedAuth {
    pub ms_token: MsToken,
    pub mc_token: String,
    pub profile: McProfile,
}

fn auth_path() -> std::path::PathBuf {
    crate::home_dir().join(".bigfish").join("auth.json")
}

/// 读取已保存的登录状态
pub fn load_saved_auth() -> Option<SavedAuth> {
    let text = std::fs::read_to_string(auth_path()).ok()?;
    serde_json::from_str(&text).ok()
}

/// 保存登录状态（会自动创建 ~/.bigfish 目录）
pub fn save_auth(auth: &SavedAuth) -> Result<(), String> {
    let p = auth_path();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    let text = serde_json::to_string_pretty(auth).map_err(|e| e.to_string())?;
    std::fs::write(&p, text).map_err(|e| format!("保存登录状态失败: {e}"))
}

/// 清除登录状态（退出登录）
pub fn clear_auth() -> Result<(), String> {
    let p = auth_path();
    if p.exists() {
        std::fs::remove_file(&p).map_err(|e| format!("清除登录状态失败: {e}"))?;
    }
    Ok(())
}

/// 用已保存的 refresh_token 刷新整条链路，返回新的 SavedAuth
pub fn refresh_saved_auth(client_id: &str, saved: &SavedAuth) -> Result<SavedAuth, String> {
    let ms = refresh_ms_token(client_id, &saved.ms_token.refresh_token)?;
    let (mc_token, profile) = complete_login(&ms.access_token)?;
    Ok(SavedAuth {
        ms_token: ms,
        mc_token,
        profile,
    })
}

/// token 是否即将/已经过期（提前 60 秒判定，避免边界情况）
pub fn token_expired(ms_token: &MsToken) -> bool {
    ms_token.expires_at <= now_unix() + 60
}
