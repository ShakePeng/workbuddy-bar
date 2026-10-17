use reqwest::header::{HeaderMap, COOKIE, SET_COOKIE};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(not(target_os = "windows"))]
use tauri::LogicalPosition;
use tauri::{AppHandle, Emitter, Manager, Position, State};
use url::Url;

const TRAY_ID: &str = "workbuddybar";
const EVENT_SNAPSHOT: &str = "snapshot-changed";
const CONFIG_VERSION: u32 = 4;
const AUTH_MODE_TOKEN: &str = "token";
const AUTH_MODE_PASSWORD: &str = "password";
// 管理端会话 Cookie 名称，登录后按原值回传，续期时以响应里的新值为准。
const SESSION_COOKIE_NAME: &str = "wb_session";

fn default_auth_mode() -> String {
    AUTH_MODE_TOKEN.to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppConfig {
    #[serde(default)]
    config_version: u32,
    base_url: String,
    // 登录方式：token（访问令牌）或 password（账户密码 + 会话 Cookie）。
    #[serde(default = "default_auth_mode")]
    auth_mode: String,
    // 管理端访问令牌（wbt_ 前缀），以 Authorization: Bearer 透传。
    // 老版本配置里没有这个字段，必须给默认值，否则升级时整份配置解析失败、地址被丢。
    #[serde(default)]
    token: String,
    // 账户密码方式的凭据：会话过期时用它们自动重新登录，因此需要落盘（0600）。
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    // 账户密码方式拿到的管理端会话 Cookie，服务端续期后会更新。
    #[serde(default)]
    session_cookie: String,
    refresh_interval_sec: u64,
    low_credit_threshold: f64,
    launch_at_login: bool,
    // 账号到期前 N 天在弹层账号明细中显示提醒，0 视为关闭。
    #[serde(default = "default_expiry_reminder_days")]
    expiry_reminder_days: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config_version: CONFIG_VERSION,
            base_url: String::new(),
            auth_mode: default_auth_mode(),
            token: String::new(),
            username: String::new(),
            password: String::new(),
            session_cookie: String::new(),
            refresh_interval_sec: 120,
            low_credit_threshold: 1000.0,
            launch_at_login: false,
            expiry_reminder_days: default_expiry_reminder_days(),
        }
    }
}

fn default_expiry_reminder_days() -> u64 {
    2
}

impl AppConfig {
    fn is_password_mode(&self) -> bool {
        self.auth_mode == AUTH_MODE_PASSWORD
    }

    // 凭据足够发起管理端请求即可轮询：账户密码方式即使没存密码，
    // 只要还有上一次的会话 Cookie 就继续用，避免每次启动都强制重登。
    fn can_refresh(&self) -> bool {
        if self.base_url.trim().is_empty() {
            return false;
        }
        if self.is_password_mode() {
            return !self.session_cookie.is_empty()
                || (!self.username.trim().is_empty() && !self.password.is_empty());
        }
        !self.token.trim().is_empty()
    }
}

fn normalize_auth_mode(raw: &str) -> String {
    if raw.trim() == AUTH_MODE_PASSWORD {
        AUTH_MODE_PASSWORD.to_string()
    } else {
        AUTH_MODE_TOKEN.to_string()
    }
}

#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct UsageSummary {
    today_requests: u64,
    today_tokens: u64,
    today_credit: f64,
    week_requests: u64,
    week_tokens: u64,
    week_credit: f64,
    total_requests: u64,
    total_tokens: u64,
    total_credit: f64,
    active_keys: u64,
    top_model: Option<String>,
}

#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct UsagePoint {
    day: String,
    requests: u64,
    tokens: u64,
    credit: f64,
    failed: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyUsageView {
    id: u64,
    name: String,
    prefix: String,
    enabled: bool,
    today_tokens: u64,
    total_tokens: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountView {
    file: String,
    uid: String,
    nickname: String,
    realm: String,
    credits: Option<f64>,
    status: String,
    healthy: Option<bool>,
    cooling: bool,
    disabled: bool,
    manual_disabled: bool,
    expires_at: Option<i64>,
    is_expired: bool,
    in_pool: Option<bool>,
    pool_unknown: bool,
    source: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSnapshot {
    updated_at: u64,
    last_success_at: u64,
    stale: bool,
    auth_state: String,
    username: String,
    pool_available: bool,
    pool_synced: u64,
    account_total: u64,
    enabled_accounts: u64,
    credit_synced_accounts: u64,
    credit_partial: bool,
    total_credits: Option<f64>,
    accounts: Vec<AccountView>,
    expiry_reminder_days: u64,
    today: UsageSummary,
    usage_days: Vec<UsagePoint>,
    is_admin: bool,
    key_usage: Vec<KeyUsageView>,
    error: Option<String>,
}

impl Default for AppSnapshot {
    fn default() -> Self {
        Self {
            updated_at: 0,
            last_success_at: 0,
            stale: false,
            auth_state: "login_required".into(),
            username: String::new(),
            pool_available: false,
            pool_synced: 0,
            account_total: 0,
            enabled_accounts: 0,
            credit_synced_accounts: 0,
            credit_partial: false,
            total_credits: None,
            accounts: Vec::new(),
            expiry_reminder_days: default_expiry_reminder_days(),
            today: UsageSummary::default(),
            usage_days: Vec::new(),
            is_admin: false,
            key_usage: Vec::new(),
            error: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsView {
    base_url: String,
    auth_mode: String,
    token: String,
    username: String,
    password: String,
    refresh_interval_sec: u64,
    low_credit_threshold: f64,
    launch_at_login: bool,
    expiry_reminder_days: u64,
    config_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsInput {
    base_url: String,
    token: String,
    username: String,
    password: String,
    refresh_interval_sec: u64,
    low_credit_threshold: f64,
    launch_at_login: bool,
    expiry_reminder_days: u64,
}

#[derive(Debug)]
enum ApiError {
    AuthExpired,
    // 登录被明确拒绝（401/429）：文案来自管理端，直接展示，不再追加「访问令牌」前缀。
    Rejected(String),
    Message(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthExpired => write!(f, "登录已失效，请重新登录"),
            Self::Rejected(message) => write!(f, "{message}"),
            Self::Message(message) => write!(f, "{message}"),
        }
    }
}

impl From<ApiError> for String {
    fn from(value: ApiError) -> Self {
        value.to_string()
    }
}

struct AppState {
    config: Mutex<AppConfig>,
    snapshot: Mutex<AppSnapshot>,
    http: Client,
    consecutive_errors: AtomicU32,
    quitting: AtomicBool,
}

impl AppState {
    fn new() -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent(concat!("WorkBuddyBar/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build HTTP client");

        Self {
            config: Mutex::new(AppConfig::default()),
            snapshot: Mutex::new(AppSnapshot::default()),
            http,
            consecutive_errors: AtomicU32::new(0),
            quitting: AtomicBool::new(false),
        }
    }

    fn config(&self) -> AppConfig {
        self.config.lock().expect("config lock poisoned").clone()
    }

    fn snapshot(&self) -> AppSnapshot {
        self.snapshot
            .lock()
            .expect("snapshot lock poisoned")
            .clone()
    }

    fn set_config(&self, config: AppConfig) {
        *self.config.lock().expect("config lock poisoned") = config;
    }

    fn set_snapshot(&self, snapshot: AppSnapshot) {
        *self.snapshot.lock().expect("snapshot lock poisoned") = snapshot;
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn display_username(raw: &str) -> String {
    // 令牌鉴权时管理端返回 `token:<令牌名>`；界面上只显示令牌名本身。
    raw.strip_prefix("token:").unwrap_or(raw).to_string()
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录：{error}"))
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("config.json"))
}

fn migrate_config(config: &mut AppConfig) -> bool {
    if config.config_version >= CONFIG_VERSION {
        return false;
    }

    // v3 及更早的配置没有 authMode：有访问令牌就沿用令牌方式，
    // 否则回落到账户密码（1.0.1 之前的配置本来就存着用户名/密码/会话）。
    if config.config_version < 4 {
        config.auth_mode = if !config.token.trim().is_empty() {
            AUTH_MODE_TOKEN.to_string()
        } else if !config.username.trim().is_empty() || !config.password.is_empty() {
            AUTH_MODE_PASSWORD.to_string()
        } else {
            AUTH_MODE_TOKEN.to_string()
        };
    }

    // 旧版本默认值为 60 秒；只迁移这一种情况，避免覆盖用户自定义的小间隔。
    if config.refresh_interval_sec == 60 {
        config.refresh_interval_sec = 120;
    }
    config.config_version = CONFIG_VERSION;
    true
}

fn load_config(app: &AppHandle) -> Result<AppConfig, String> {
    let path = config_path(app)?;
    if !path.exists() {
        let config = AppConfig::default();
        save_config(app, &config)?;
        return Ok(config);
    }

    let bytes = std::fs::read(&path).map_err(|error| format!("读取配置失败：{error}"))?;
    let mut config: AppConfig =
        serde_json::from_slice(&bytes).map_err(|error| format!("配置格式错误：{error}"))?;
    let mut changed = migrate_config(&mut config);

    let auth_mode = normalize_auth_mode(&config.auth_mode);
    if auth_mode != config.auth_mode {
        config.auth_mode = auth_mode;
        changed = true;
    }

    let refresh_interval_sec = config.refresh_interval_sec.clamp(30, 600);
    if refresh_interval_sec != config.refresh_interval_sec {
        config.refresh_interval_sec = refresh_interval_sec;
        changed = true;
    }

    if !config.low_credit_threshold.is_finite() || config.low_credit_threshold < 0.0 {
        config.low_credit_threshold = 1000.0;
        changed = true;
    }

    if changed {
        save_config(app, &config)?;
    }
    Ok(config)
}

fn save_config(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("创建配置目录失败：{error}"))?;
    }
    let bytes =
        serde_json::to_vec_pretty(config).map_err(|error| format!("序列化配置失败：{error}"))?;
    std::fs::write(&path, bytes).map_err(|error| format!("写入配置失败：{error}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("设置配置权限失败：{error}"))?;
    }
    Ok(())
}

// 会话 Cookie 变化时同步内存与磁盘：即使落盘失败，本次运行仍然使用新会话，
// 因此只提示错误、不让刷新流程失败。
fn store_session_cookie(app: &AppHandle, config: &mut AppConfig, cookie: String) {
    config.session_cookie = cookie;
    let state = app.state::<AppState>();
    state.set_config(config.clone());
    if let Err(error) = save_config(app, config) {
        eprintln!("保存管理端会话失败：{error}");
    }
}

fn normalize_base_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let url = Url::parse(trimmed).map_err(|_| "管理端地址格式不正确".to_string())?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("管理端地址只支持 http 或 https".into());
    }
    if url.host_str().is_none() {
        return Err("管理端地址缺少主机名".into());
    }
    Ok(trimmed.to_string())
}

fn value_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(|value| {
        value.as_f64().or_else(|| {
            value
                .as_str()
                .and_then(|raw| raw.trim().parse::<f64>().ok())
        })
    })
}

fn value_u64(value: Option<&Value>) -> u64 {
    value_f64(value)
        .filter(|number| number.is_finite() && *number > 0.0)
        .map(|number| number.round() as u64)
        .unwrap_or(0)
}

fn value_i64(value: Option<&Value>) -> Option<i64> {
    value_f64(value).map(|number| number.round() as i64)
}

fn value_bool(value: Option<&Value>) -> Option<bool> {
    value.and_then(|value| {
        value
            .as_bool()
            .or_else(|| value.as_i64().map(|number| number != 0))
            .or_else(|| {
                value
                    .as_str()
                    .and_then(|raw| match raw.trim().to_ascii_lowercase().as_str() {
                        "true" | "1" | "yes" => Some(true),
                        "false" | "0" | "no" => Some(false),
                        _ => None,
                    })
            })
    })
}

fn value_string(value: Option<&Value>) -> String {
    value
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

fn account_status(account: &Value) -> String {
    let disabled = value_bool(account.get("disabled")).unwrap_or(false)
        || value_bool(account.get("disabled_by_panel")).unwrap_or(false)
        || value_bool(account.get("manual_disabled")).unwrap_or(false);
    if disabled {
        return "disabled".into();
    }
    if value_bool(account.get("cooling")).unwrap_or(false) {
        return "cooling".into();
    }
    if value_bool(account.get("is_expired")).unwrap_or(false) {
        return "expired".into();
    }
    if value_bool(account.get("poolUnknown")).unwrap_or(false) {
        return "unknown".into();
    }
    if value_bool(account.get("in_pool")) == Some(false) {
        return "unavailable".into();
    }
    if value_bool(account.get("healthy")) == Some(false) {
        return "unavailable".into();
    }
    "healthy".into()
}

fn parse_accounts(value: &Value) -> (Vec<AccountView>, u64, u64, bool, Option<f64>) {
    let raw_accounts = value
        .get("accounts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut accounts = Vec::with_capacity(raw_accounts.len());
    let mut enabled_accounts = 0_u64;
    let mut synced_accounts = 0_u64;
    let mut total_credits = 0.0_f64;
    let mut found_credit = false;
    let mut partial = false;

    for raw in raw_accounts {
        let status = account_status(&raw);
        let disabled = status == "disabled";
        let credits = value_f64(raw.get("credits"));
        if !disabled {
            enabled_accounts += 1;
            match credits {
                Some(amount) if amount.is_finite() => {
                    synced_accounts += 1;
                    found_credit = true;
                    total_credits += amount.max(0.0);
                }
                _ => partial = true,
            }
        }

        accounts.push(AccountView {
            file: value_string(raw.get("file")),
            uid: value_string(raw.get("uid")),
            nickname: {
                let nickname = value_string(raw.get("nickname"));
                if nickname.is_empty() {
                    "未命名账号".into()
                } else {
                    nickname
                }
            },
            realm: {
                let realm = value_string(raw.get("realm"));
                if realm.is_empty() {
                    "cn".into()
                } else {
                    realm
                }
            },
            credits,
            status,
            healthy: value_bool(raw.get("healthy")),
            cooling: value_bool(raw.get("cooling")).unwrap_or(false),
            disabled,
            manual_disabled: value_bool(raw.get("manual_disabled")).unwrap_or(false),
            expires_at: value_i64(raw.get("expires_at")),
            is_expired: value_bool(raw.get("is_expired")).unwrap_or(false),
            in_pool: value_bool(raw.get("in_pool")),
            pool_unknown: value_bool(raw.get("poolUnknown")).unwrap_or(false),
            source: value_string(raw.get("source")),
        });
    }

    (
        accounts,
        enabled_accounts,
        synced_accounts,
        partial,
        found_credit.then_some((total_credits * 100.0).round() / 100.0),
    )
}

fn apply_realtime_credits(accounts: &mut [AccountView], value: &Value) -> (u64, bool, Option<f64>) {
    let credits = value.get("credits").and_then(Value::as_object);
    let mut synced_accounts = 0_u64;
    let mut total_credits = 0.0_f64;
    let mut found_credit = false;
    let mut partial = false;

    for account in accounts {
        // 实时接口按 uid 返回；null 表示本次查询失败，保留上游快照值作为兜底。
        let realtime_credits = credits
            .and_then(|map| map.get(&account.uid))
            .and_then(|raw| value_f64(Some(raw)));
        if let Some(amount) = realtime_credits.filter(|amount| amount.is_finite()) {
            account.credits = Some(amount);
        }

        if account.disabled {
            continue;
        }
        match account.credits {
            Some(amount) if amount.is_finite() => {
                synced_accounts += 1;
                found_credit = true;
                total_credits += amount.max(0.0);
            }
            _ => partial = true,
        }
    }

    (
        synced_accounts,
        partial,
        found_credit.then_some((total_credits * 100.0).round() / 100.0),
    )
}

fn parse_usage_summary(value: &Value) -> UsageSummary {
    UsageSummary {
        today_requests: value_u64(value.get("today_requests")),
        today_tokens: value_u64(value.get("today_tokens")),
        today_credit: value_f64(value.get("today_credit")).unwrap_or(0.0),
        week_requests: value_u64(value.get("week_requests")),
        week_tokens: value_u64(value.get("week_tokens")),
        week_credit: value_f64(value.get("week_credit")).unwrap_or(0.0),
        total_requests: value_u64(value.get("total_requests")),
        total_tokens: value_u64(value.get("total_tokens")),
        total_credit: value_f64(value.get("total_credit")).unwrap_or(0.0),
        active_keys: value_u64(value.get("active_keys")),
        top_model: value
            .get("top_model")
            .and_then(Value::as_str)
            .filter(|model| !model.is_empty())
            .map(str::to_string),
    }
}

fn parse_usage_days(value: &Value) -> Vec<UsagePoint> {
    let mut points: Vec<UsagePoint> = value
        .as_array()
        .into_iter()
        .flatten()
        .map(|day| UsagePoint {
            day: value_string(day.get("day")),
            requests: value_u64(day.get("requests")),
            tokens: value_u64(day.get("prompt_tokens")) + value_u64(day.get("completion_tokens")),
            credit: value_f64(day.get("credit")).unwrap_or(0.0),
            failed: value_u64(day.get("failed")),
        })
        .collect();
    points.sort_by(|left, right| left.day.cmp(&right.day));
    points
}

fn parse_key_usage(keys: &Value, today: &Value) -> Vec<KeyUsageView> {
    let mut today_by_name: HashMap<String, u64> = HashMap::new();
    for row in today.as_array().into_iter().flatten() {
        let name = value_string(row.get("name"));
        let tokens = value_u64(row.get("prompt_tokens")) + value_u64(row.get("completion_tokens"));
        *today_by_name.entry(name).or_default() += tokens;
    }

    let mut key_usage: Vec<KeyUsageView> = keys
        .as_array()
        .into_iter()
        .flatten()
        .map(|key| {
            let id = value_u64(key.get("id"));
            let raw_name = value_string(key.get("name"));
            let name = if raw_name.is_empty() {
                format!("密钥 {id}")
            } else {
                raw_name
            };
            let today_tokens = today_by_name.get(&name).copied().unwrap_or(0);
            KeyUsageView {
                id,
                name,
                prefix: value_string(key.get("prefix")),
                enabled: value_bool(key.get("enabled")).unwrap_or(true),
                today_tokens,
                total_tokens: value_u64(key.get("used_tokens")),
            }
        })
        .collect();

    key_usage.sort_by(|left, right| {
        right
            .enabled
            .cmp(&left.enabled)
            .then_with(|| right.total_tokens.cmp(&left.total_tokens))
            .then_with(|| right.today_tokens.cmp(&left.today_tokens))
            .then_with(|| left.name.cmp(&right.name))
    });
    key_usage
}

fn api_detail(status: StatusCode, raw: &str) -> String {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("detail")
                .or_else(|| value.get("error"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .filter(|message| !message.is_empty())
        .unwrap_or_else(|| {
            if raw.trim().is_empty() {
                status.to_string()
            } else {
                raw.trim().chars().take(200).collect()
            }
        })
}

fn api_message(status: StatusCode, raw: &str) -> String {
    let detail = api_detail(status, raw);
    if status == StatusCode::UNAUTHORIZED {
        return format!("访问令牌无效：{detail}");
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        return format!("请求过于频繁：{detail}");
    }
    format!("管理端返回 {status}：{detail}")
}

// 账户密码登录走 /api/login，失败文案（用户名或密码错误 / 失败次数过多）
// 由管理端给出，直接展示，不套用令牌专用前缀。
fn login_message(status: StatusCode, raw: &str) -> String {
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::TOO_MANY_REQUESTS {
        return api_detail(status, raw);
    }
    api_message(status, raw)
}

fn auth_expired_message(config: &AppConfig) -> String {
    if config.is_password_mode() {
        "登录状态已失效，请重新输入账号密码".into()
    } else {
        "访问令牌已失效，请重新填写".into()
    }
}

struct ApiReply {
    value: Value,
    // 管理端续期会话会在响应里重新下发 wb_session，必须回写配置，
    // 否则持有的 Cookie 一直停在旧 iat，闲置 12 小时后会被判失效。
    renewed_cookie: Option<String>,
}

fn apply_auth(request: reqwest::RequestBuilder, config: &AppConfig) -> reqwest::RequestBuilder {
    if config.is_password_mode() {
        if config.session_cookie.is_empty() {
            return request;
        }
        return request.header(
            COOKIE,
            format!("{}={}", SESSION_COOKIE_NAME, config.session_cookie),
        );
    }
    if config.token.is_empty() {
        return request;
    }
    request.bearer_auth(&config.token)
}

fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    let prefix = format!("{SESSION_COOKIE_NAME}=");
    for value in headers.get_all(SET_COOKIE) {
        let Ok(raw) = value.to_str() else {
            continue;
        };
        for part in raw.split(';') {
            let Some(cookie) = part.trim().strip_prefix(prefix.as_str()) else {
                continue;
            };
            if !cookie.is_empty() {
                return Some(cookie.to_string());
            }
        }
    }
    None
}

async fn send_json(request: reqwest::RequestBuilder) -> Result<ApiReply, ApiError> {
    let response = request
        .send()
        .await
        .map_err(|error| ApiError::Message(format!("无法连接管理端：{error}")))?;
    let status = response.status();
    let headers = response.headers().clone();
    let body = response
        .text()
        .await
        .map_err(|error| ApiError::Message(format!("读取管理端响应失败：{error}")))?;

    if status == StatusCode::UNAUTHORIZED {
        return Err(ApiError::AuthExpired);
    }
    if !status.is_success() {
        return Err(ApiError::Message(api_message(status, &body)));
    }
    let value = serde_json::from_str(&body)
        .map_err(|error| ApiError::Message(format!("管理端响应格式错误：{error}")))?;
    Ok(ApiReply {
        value,
        renewed_cookie: extract_session_cookie(&headers),
    })
}

async fn get_json(client: &Client, config: &AppConfig, path: &str) -> Result<ApiReply, ApiError> {
    let request = apply_auth(client.get(format!("{}{}", config.base_url, path)), config);
    send_json(request).await
}

async fn post_json(client: &Client, config: &AppConfig, path: &str) -> Result<ApiReply, ApiError> {
    let request = apply_auth(client.post(format!("{}{}", config.base_url, path)), config);
    send_json(request).await
}

async fn api_login(
    client: &Client,
    base_url: &str,
    username: &str,
    password: &str,
) -> Result<String, ApiError> {
    let response = client
        .post(format!("{base_url}/api/login"))
        .json(&serde_json::json!({ "username": username, "password": password }))
        .send()
        .await
        .map_err(|error| ApiError::Message(format!("无法连接管理端：{error}")))?;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response
        .text()
        .await
        .map_err(|error| ApiError::Message(format!("读取登录响应失败：{error}")))?;

    if !status.is_success() {
        return Err(ApiError::Rejected(login_message(status, &body)));
    }
    extract_session_cookie(&headers)
        .ok_or_else(|| ApiError::Message("管理端未返回 wb_session 会话".into()))
}

async fn fetch_snapshot(
    client: &Client,
    config: &AppConfig,
) -> Result<(AppSnapshot, Option<String>), ApiError> {
    let me = get_json(client, config, "/api/me").await?;
    let accounts = get_json(client, config, "/api/accounts").await?;
    let summary = get_json(client, config, "/api/stats/summary").await?;
    let daily = get_json(client, config, "/api/stats/daily?days=30").await?;
    let is_admin = me.value.get("role").and_then(Value::as_str) == Some("admin");
    let keys = get_json(client, config, "/api/keys").await?;
    let key_today = get_json(client, config, "/api/stats/by-key?days=1").await?;

    let (mut parsed_accounts, enabled_accounts, _, _, _) = parse_accounts(&accounts.value);
    let realtime_credits =
        post_json(client, config, "/api/accounts/refresh-credits?force=false").await?;
    let (credit_synced_accounts, credit_partial, total_credits) =
        apply_realtime_credits(&mut parsed_accounts, &realtime_credits.value);
    let now = now_secs();

    let renewed_cookie = [
        &me,
        &accounts,
        &summary,
        &daily,
        &keys,
        &key_today,
        &realtime_credits,
    ]
    .into_iter()
    .filter_map(|reply| reply.renewed_cookie.clone())
    .last();

    let snapshot = AppSnapshot {
        updated_at: now,
        last_success_at: now,
        stale: false,
        auth_state: "ok".into(),
        username: display_username(&value_string(me.value.get("username"))),
        pool_available: value_bool(accounts.value.get("pool_available")).unwrap_or(false),
        pool_synced: value_u64(accounts.value.get("pool_synced")),
        account_total: value_u64(accounts.value.get("total")).max(parsed_accounts.len() as u64),
        enabled_accounts,
        credit_synced_accounts,
        credit_partial,
        total_credits,
        accounts: parsed_accounts,
        expiry_reminder_days: config.expiry_reminder_days,
        today: parse_usage_summary(&summary.value),
        usage_days: parse_usage_days(&daily.value),
        is_admin,
        key_usage: parse_key_usage(&keys.value, &key_today.value),
        error: None,
    };
    Ok((snapshot, renewed_cookie))
}

fn publish(app: &AppHandle, snapshot: AppSnapshot) {
    let state = app.state::<AppState>();
    state.set_snapshot(snapshot.clone());
    state.consecutive_errors.store(0, Ordering::Relaxed);
    update_tray(app, &snapshot);
    let _ = app.emit(EVENT_SNAPSHOT, snapshot);
}

fn publish_error(app: &AppHandle, error: impl Into<String>) {
    let state = app.state::<AppState>();
    let mut snapshot = state.snapshot();
    snapshot.updated_at = now_secs();
    snapshot.stale = true;
    snapshot.error = Some(error.into());
    snapshot.auth_state = if snapshot.last_success_at == 0 {
        "login_required".into()
    } else {
        "error".into()
    };
    state.set_snapshot(snapshot.clone());
    state.consecutive_errors.fetch_add(1, Ordering::Relaxed);
    update_tray(app, &snapshot);
    let _ = app.emit(EVENT_SNAPSHOT, snapshot);
}

async fn refresh_state(app: &AppHandle) -> AppSnapshot {
    let state = app.state::<AppState>();
    let mut config = state.config();

    if !config.can_refresh() {
        let mut snapshot = AppSnapshot::default();
        snapshot.updated_at = now_secs();
        state.set_snapshot(snapshot.clone());
        update_tray(app, &snapshot);
        let _ = app.emit(EVENT_SNAPSHOT, snapshot.clone());
        return snapshot;
    }

    let mut outcome = fetch_snapshot(&state.http, &config).await;

    // 账户密码方式的会话有总时长与闲置限制：先凭保存的账号密码自动重新登录一次，
    // 让托盘应用可以长期无人值守；重登失败才把登录窗口叫回来。
    if matches!(outcome, Err(ApiError::AuthExpired))
        && config.is_password_mode()
        && !config.username.trim().is_empty()
        && !config.password.is_empty()
    {
        match api_login(
            &state.http,
            &config.base_url,
            &config.username,
            &config.password,
        )
        .await
        {
            Ok(cookie) => {
                store_session_cookie(app, &mut config, cookie);
                outcome = fetch_snapshot(&state.http, &config).await;
            }
            Err(error) => {
                publish_error(app, error.to_string());
                present_login(app);
                return state.snapshot();
            }
        }
    }

    match outcome {
        Ok((snapshot, cookie)) => {
            if let Some(cookie) = cookie {
                store_session_cookie(app, &mut config, cookie);
            }
            publish(app, snapshot);
            state.snapshot()
        }
        // 凭据被删除、过期或管理端重置了密钥：直接把登录窗口叫回来让用户重填。
        Err(ApiError::AuthExpired) => {
            publish_error(app, auth_expired_message(&config));
            present_login(app);
            state.snapshot()
        }
        Err(error) => {
            publish_error(app, error.to_string());
            state.snapshot()
        }
    }
}

fn tray_image(kind: &str) -> Image<'static> {
    let bytes: &'static [u8] = match kind {
        "warn" => include_bytes!("../icons/tray-warn.png"),
        "error" => include_bytes!("../icons/tray-error.png"),
        _ => include_bytes!("../icons/tray-normal.png"),
    };
    Image::from_bytes(bytes).expect("invalid tray icon")
}

#[cfg(target_os = "macos")]
extern "C" {
    fn workbuddy_set_tray_title(title: *const std::os::raw::c_char);
}

#[cfg(target_os = "macos")]
fn set_readable_tray_title(title: &str) {
    use std::ffi::CString;

    if let Ok(title) = CString::new(title) {
        unsafe { workbuddy_set_tray_title(title.as_ptr()) };
    }
}

#[cfg(not(target_os = "macos"))]
fn set_readable_tray_title(_title: &str) {}

fn add_thousands_separators(digits: &str) -> String {
    let mut output = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            output.push(',');
        }
        output.push(character);
    }
    output
}

fn tray_title(snapshot: &AppSnapshot) -> String {
    let used = compact_number(snapshot.today.today_credit);
    let remaining = snapshot
        .total_credits
        .map(compact_number)
        .unwrap_or_else(|| "--".into());
    format!("{used}\n{remaining}")
}

fn compact_number(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded.abs() >= 100_000.0 {
        return format!("{:.1}k", rounded / 1000.0);
    }

    let raw = if rounded.fract().abs() < 0.005 {
        format!("{:.0}", rounded)
    } else if rounded.abs() >= 1000.0 {
        format!("{:.1}", rounded)
    } else {
        format!("{:.2}", rounded)
    };
    let (sign, digits) = raw
        .strip_prefix('-')
        .map(|digits| ("-", digits))
        .unwrap_or(("", raw.as_str()));
    let (whole, fraction) = digits
        .split_once('.')
        .map(|(whole, fraction)| (whole, Some(fraction)))
        .unwrap_or((digits, None));
    let mut output = format!("{sign}{}", add_thousands_separators(whole));
    if let Some(fraction) = fraction {
        output.push('.');
        output.push_str(fraction);
    }
    output
}

fn update_tray(app: &AppHandle, snapshot: &AppSnapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let title = tray_title(snapshot);
    let _ = tray.set_title(Some(&title));
    set_readable_tray_title(&title);

    let tooltip = format!(
        "WorkBuddyBar · 今日已用 {} · 剩余总量 {} · 账号 {}/{}",
        compact_number(snapshot.today.today_credit),
        snapshot
            .total_credits
            .map(compact_number)
            .unwrap_or_else(|| "暂未同步".into()),
        snapshot
            .accounts
            .iter()
            .filter(|account| account.status == "healthy")
            .count(),
        snapshot.account_total
    );
    let _ = tray.set_tooltip(Some(&tooltip));

    let state = app.state::<AppState>();
    let low_credit = snapshot
        .total_credits
        .map(|value| value <= state.config().low_credit_threshold)
        .unwrap_or(false);
    let kind = if snapshot.stale && snapshot.error.is_some() && snapshot.last_success_at == 0 {
        "error"
    } else if snapshot.error.is_some()
        || !snapshot.pool_available
        || low_credit
        || snapshot.credit_partial
    {
        "warn"
    } else {
        "normal"
    };
    #[cfg(target_os = "macos")]
    let _ = tray.set_icon_with_as_template(Some(tray_image(kind)), true);
    #[cfg(not(target_os = "macos"))]
    let _ = tray.set_icon(Some(tray_image(kind)));
}

fn toggle_popover(app: &AppHandle) {
    let Some(window) = app.get_webview_window("popover") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }

    #[cfg(target_os = "windows")]
    {
        position_popover_windows(app, &window);
        return;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let Ok(cursor) = app.cursor_position() else {
            show_popover_centered(app);
            return;
        };
        let current_scale = window.scale_factor().unwrap_or(1.0);
        let primary_scale = app
            .primary_monitor()
            .ok()
            .flatten()
            .map(|monitor| monitor.scale_factor())
            .unwrap_or(1.0);

        // cursor_position 始终按主屏缩放输出，统一还原成 macOS 全局逻辑坐标后再选屏。
        let logical_x = cursor.x / primary_scale;
        let logical_y = cursor.y / primary_scale;
        let target_monitor = app
            .monitor_from_point(logical_x, logical_y)
            .ok()
            .flatten()
            .or_else(|| app.primary_monitor().ok().flatten());
        // 窗口位置统一使用逻辑坐标，避免 set_position 按隐藏窗口所在屏幕的缩放因子重复换算。
        let current_size = window
            .outer_size()
            .unwrap_or(tauri::PhysicalSize::new(400, 720));
        let logical_size = current_size.to_logical::<f64>(current_scale);
        let mut x = logical_x - logical_size.width / 2.0;
        let mut y = logical_y + 12.0;

        if let Some(monitor) = target_monitor.as_ref() {
            let area = monitor.work_area();
            let scale = monitor.scale_factor();
            let left = area.position.x as f64 / scale;
            let top = area.position.y as f64 / scale;
            let right = left + area.size.width as f64 / scale;
            let bottom = top + area.size.height as f64 / scale;
            y = top + 10.0;
            x = x.clamp(
                left + 8.0,
                (right - logical_size.width - 8.0).max(left + 8.0),
            );
            if y + logical_size.height > bottom - 8.0 {
                y = (logical_y - logical_size.height - 10.0).max(top + 8.0);
            }
        }

        let _ = window.set_position(Position::Logical(LogicalPosition::new(x, y)));
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(target_os = "windows")]
fn tray_rect_physical(rect: tauri::Rect) -> (f64, f64, f64, f64) {
    let position = rect.position.to_physical::<f64>(1.0);
    let size = rect.size.to_physical::<u32>(1.0);
    (
        position.x,
        position.y,
        size.width as f64,
        size.height as f64,
    )
}

#[cfg(target_os = "windows")]
fn position_popover_windows(app: &AppHandle, window: &tauri::WebviewWindow) {
    let tray_rect = app
        .tray_by_id(TRAY_ID)
        .and_then(|tray| tray.rect().ok().flatten());
    let cursor = app.cursor_position().ok();
    let anchor = tray_rect
        .map(|rect| {
            let (x, y, width, height) = tray_rect_physical(rect);
            (x + width / 2.0, y + height / 2.0)
        })
        .or_else(|| cursor.map(|position| (position.x, position.y)));
    let Some((anchor_x, anchor_y)) = anchor else {
        show_popover_centered(app);
        return;
    };

    let target_monitor = app
        .monitor_from_point(anchor_x, anchor_y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let fallback_scale = target_monitor
        .as_ref()
        .map(|monitor| monitor.scale_factor())
        .unwrap_or(1.0);
    let window_size = window.outer_size().unwrap_or_else(|_| {
        tauri::PhysicalSize::new(
            (400.0 * fallback_scale).round() as u32,
            (720.0 * fallback_scale).round() as u32,
        )
    });

    let mut x = anchor_x - window_size.width as f64 / 2.0;
    let mut y = anchor_y + 12.0;

    if let Some(monitor) = target_monitor.as_ref() {
        let area = monitor.work_area();
        let left = area.position.x as f64;
        let top = area.position.y as f64;
        let right = left + area.size.width as f64;
        let bottom = top + area.size.height as f64;
        let window_width = window_size.width as f64;
        let window_height = window_size.height as f64;
        let anchor_on_lower_half = anchor_y >= top + area.size.height as f64 / 2.0;

        x = x.clamp(left + 8.0, (right - window_width - 8.0).max(left + 8.0));
        y = if anchor_on_lower_half {
            tray_rect
                .map(|rect| tray_rect_physical(rect).1 - window_height - 10.0)
                .unwrap_or(anchor_y - window_height - 12.0)
        } else {
            tray_rect
                .map(|rect| {
                    let (_, y, _, height) = tray_rect_physical(rect);
                    y + height + 10.0
                })
                .unwrap_or(anchor_y + 12.0)
        }
        .clamp(top + 8.0, (bottom - window_height - 8.0).max(top + 8.0));
    }

    let position = tauri::PhysicalPosition::new(x.round() as i32, y.round() as i32);
    let _ = window.set_position(Position::Physical(position));
    let _ = window.show();
    let _ = window.set_focus();
}

fn show_popover_centered(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("popover") {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn present_login(app: &AppHandle) {
    if let Some(popover) = app.get_webview_window("popover") {
        let _ = popover.hide();
    }
    if let Some(settings) = app.get_webview_window("settings") {
        let _ = settings.hide();
    }
    if let Some(login) = app.get_webview_window("login") {
        let _ = login.center();
        let _ = login.show();
        let _ = login.set_focus();
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let refresh = MenuItemBuilder::with_id("refresh", "立即刷新").build(app)?;
    let manager = MenuItemBuilder::with_id("manager", "打开管理端").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "设置").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出 WorkBuddyBar").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&refresh, &manager, &settings, &quit])
        .build()?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_image("normal"))
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "refresh" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = refresh_state(&app).await;
                });
            }
            "manager" => {
                let _ = open_manager(app.clone());
            }
            "settings" => {
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.center();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.state::<AppState>()
                    .quitting
                    .store(true, Ordering::Relaxed);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_popover(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();

    // Windows 下该插件通过注册表 HKCU\...\CurrentVersion\Run 实现自启：
    // 当自启项不存在时，disable() 会触发 RegDeleteValueW 返回
    // ERROR_FILE_NOT_FOUND，即「系统找不到指定的文件。(os error 2)」。
    // 因此先查询真实状态，仅在需要变更时才写入，保证开关幂等。
    if matches!(manager.is_enabled(), Ok(current) if current == enabled) {
        return Ok(());
    }

    if enabled {
        manager
            .enable()
            .map_err(|error| format!("开启开机启动失败：{error}"))
    } else {
        match manager.disable() {
            Ok(()) => Ok(()),
            // 注册表中本就没有该条目，等价于已经关闭，不必报错
            Err(_) if matches!(manager.is_enabled(), Ok(false)) => Ok(()),
            Err(error) => Err(format!("关闭开机启动失败：{error}")),
        }
    }
}

#[tauri::command]
fn load_state(state: State<'_, AppState>) -> AppSnapshot {
    state.snapshot()
}

#[tauri::command]
async fn login(
    app: AppHandle,
    base_url: String,
    auth_mode: String,
    token: String,
    username: String,
    password: String,
) -> Result<AppSnapshot, String> {
    let base_url = normalize_base_url(&base_url)?;
    let state = app.state::<AppState>();

    let mut config = state.config();
    config.base_url = base_url.clone();
    config.auth_mode = normalize_auth_mode(&auth_mode);

    if config.is_password_mode() {
        let username = username.trim().to_string();
        if username.is_empty() {
            return Err("请输入用户名".into());
        }
        if password.is_empty() {
            return Err("请输入密码".into());
        }
        // 两种方式的凭据不并存：切换方式时清掉另一种。
        config.token.clear();
        config.username = username;
        config.password = password;
        let cookie = api_login(&state.http, &base_url, &config.username, &config.password).await?;
        config.session_cookie = cookie;
    } else {
        let token = token.trim().to_string();
        if token.is_empty() {
            return Err("请输入访问令牌".into());
        }
        config.token = token;
        config.username.clear();
        config.password.clear();
        config.session_cookie.clear();
    }

    // 先拿 /api/me 验活：凭据错了就当场报错，不把无效凭据写进配置。
    match get_json(&state.http, &config, "/api/me").await {
        Ok(reply) => {
            if let Some(cookie) = reply.renewed_cookie {
                config.session_cookie = cookie;
            }
        }
        Err(ApiError::AuthExpired) if config.is_password_mode() => {
            return Err("管理端未接受本次登录会话，请稍后重试".into())
        }
        Err(ApiError::AuthExpired) => return Err("访问令牌无效或已过期，请在管理端重新创建".into()),
        Err(error) => return Err(error.to_string()),
    }

    save_config(&app, &config)?;
    state.set_config(config.clone());

    let snapshot = match fetch_snapshot(&state.http, &config).await {
        Ok((snapshot, cookie)) => {
            if let Some(cookie) = cookie {
                store_session_cookie(&app, &mut config, cookie);
            }
            publish(&app, snapshot);
            state.snapshot()
        }
        Err(error) => {
            publish_error(&app, error.to_string());
            state.snapshot()
        }
    };
    if let Some(window) = app.get_webview_window("login") {
        let _ = window.hide();
    }
    show_popover_centered(&app);
    Ok(snapshot)
}

#[tauri::command]
fn logout(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut config = state.config();
    // 退出登录清空全部凭据与会话，但保留地址、监测设置和上次选择的登录方式。
    config.token.clear();
    config.username.clear();
    config.password.clear();
    config.session_cookie.clear();
    save_config(&app, &config)?;
    state.set_config(config);
    let mut snapshot = AppSnapshot::default();
    snapshot.updated_at = now_secs();
    state.set_snapshot(snapshot.clone());
    update_tray(&app, &snapshot);
    let _ = app.emit(EVENT_SNAPSHOT, snapshot);
    present_login(&app);
    Ok(())
}

#[tauri::command]
async fn refresh(app: AppHandle) -> Result<AppSnapshot, String> {
    Ok(refresh_state(&app).await)
}

#[tauri::command]
fn get_settings(app: AppHandle, state: State<'_, AppState>) -> Result<SettingsView, String> {
    let config = state.config();
    Ok(SettingsView {
        base_url: config.base_url,
        auth_mode: config.auth_mode,
        token: config.token,
        username: config.username,
        password: config.password,
        refresh_interval_sec: config.refresh_interval_sec,
        low_credit_threshold: config.low_credit_threshold,
        launch_at_login: config.launch_at_login,
        expiry_reminder_days: config.expiry_reminder_days,
        config_path: config_path(&app)
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
    })
}

#[tauri::command]
async fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: SettingsInput,
) -> Result<AppSnapshot, String> {
    let base_url = normalize_base_url(&settings.base_url)?;
    let interval = settings.refresh_interval_sec.clamp(30, 600);
    let threshold = if settings.low_credit_threshold.is_finite() {
        settings.low_credit_threshold.max(0.0)
    } else {
        1000.0
    };

    let token = settings.token.trim().to_string();
    let username = settings.username.trim().to_string();
    let password = settings.password;

    let mut config = state.config();
    config.base_url = base_url;
    // 登录方式由登录页决定，设置页只维护当前方式对应的凭据。
    if config.is_password_mode() {
        // 换账号或改密码会让旧会话立即作废，交给 refresh_state 重新登录验证。
        if username != config.username || password != config.password {
            config.session_cookie.clear();
        }
        config.username = username;
        config.password = password;
    } else {
        config.token = token;
    }
    config.refresh_interval_sec = interval;
    config.low_credit_threshold = threshold;
    config.launch_at_login = settings.launch_at_login;
    config.expiry_reminder_days = settings.expiry_reminder_days.clamp(0, 365);
    save_config(&app, &config)?;
    state.set_config(config.clone());
    apply_autostart(&app, config.launch_at_login)?;
    let snapshot = refresh_state(&app).await;
    Ok(snapshot)
}

#[tauri::command]
fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(popover) = app.get_webview_window("popover") {
        let _ = popover.hide();
    }
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "设置窗口不存在".to_string())?;
    window.center().map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn resize_popover(app: AppHandle, height: f64) -> Result<(), String> {
    let window = app
        .get_webview_window("popover")
        .ok_or_else(|| "弹层窗口不存在".to_string())?;
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let logical_size = size.to_logical::<f64>(scale);
    let max_height = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor.work_area().size.height as f64 / monitor.scale_factor() - 16.0)
        .unwrap_or(1000.0)
        .max(320.0);
    let target_height = height.clamp(320.0, max_height);
    let position = window.outer_position().ok();

    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize::new(
            logical_size.width,
            target_height,
        )))
        .map_err(|error| error.to_string())?;

    if let Some(position) = position {
        let _ = window.set_position(Position::Physical(position));
    }
    Ok(())
}

#[tauri::command]
fn open_manager(app: AppHandle) -> Result<(), String> {
    let config = app.state::<AppState>().config();
    let base_url = normalize_base_url(&config.base_url)?;
    open_external_url(&base_url)
}

#[cfg(target_os = "windows")]
fn open_external_url(url: &str) -> Result<(), String> {
    Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开管理端失败：{error}"))
}

#[cfg(target_os = "macos")]
fn open_external_url(url: &str) -> Result<(), String> {
    Command::new("open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开管理端失败：{error}"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn open_external_url(url: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开管理端失败：{error}"))
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.state::<AppState>()
        .quitting
        .store(true, Ordering::Relaxed);
    app.exit(0);
}

async fn poll_loop(app: AppHandle) {
    loop {
        let state = app.state::<AppState>();
        let config = state.config();
        let errors = state.consecutive_errors.load(Ordering::Relaxed).min(3);
        let multiplier = 1_u64 << errors;
        let wait = (config.refresh_interval_sec.clamp(30, 600) * multiplier).min(300);
        drop(state);
        tokio::time::sleep(Duration::from_secs(wait)).await;
        let _ = refresh_state(&app).await;
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_popover_centered(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .setup(|app| {
            let state = app.state::<AppState>();
            let config = load_config(app.handle()).unwrap_or_default();
            state.set_config(config.clone());
            let mut snapshot = AppSnapshot::default();
            snapshot.updated_at = now_secs();
            state.set_snapshot(snapshot);

            // 菜单栏应用：隐藏 Dock 图标，只保留状态栏入口。
            #[cfg(target_os = "macos")]
            let _ = app
                .handle()
                .set_activation_policy(tauri::ActivationPolicy::Accessory);

            setup_tray(app)?;

            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("popover") {
                let _ = window_vibrancy::apply_vibrancy(
                    &window,
                    window_vibrancy::NSVisualEffectMaterial::Popover,
                    Some(window_vibrancy::NSVisualEffectState::Active),
                    Some(14.0),
                );
            }

            if !config.can_refresh() {
                present_login(app.handle());
            } else {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = refresh_state(&handle).await;
                });
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(poll_loop(handle));
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Focused(false) if window.label() == "popover" => {
                let _ = window.hide();
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            load_state,
            login,
            logout,
            refresh,
            get_settings,
            save_settings,
            open_settings,
            resize_popover,
            open_manager,
            quit_app
        ])
        .build(tauri::generate_context!())
        .expect("error while building WorkBuddyBar");

    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if !app.state::<AppState>().quitting.load(Ordering::Relaxed) {
                api.prevent_exit();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_base_url() {
        assert_eq!(
            normalize_base_url(" http://47.99.55.3:7864/ ").unwrap(),
            "http://47.99.55.3:7864"
        );
        assert!(normalize_base_url("file:///tmp/db").is_err());
    }

    #[test]
    fn strips_token_prefix_from_username() {
        assert_eq!(display_username("token:test"), "test");
        assert_eq!(display_username("admin"), "admin");
        assert_eq!(display_username(""), "");
    }

    #[test]
    fn parses_string_numbers_and_sums_enabled_accounts() {
        let accounts = json!({
            "total": 3,
            "pool_available": true,
            "pool_synced": 2,
            "accounts": [
                {"uid": "1", "credits": "120.5", "healthy": true, "in_pool": true},
                {"uid": "2", "credits": 79.5, "healthy": true, "in_pool": true},
                {"uid": "3", "credits": 999, "disabled": true}
            ]
        });
        let (items, enabled, synced, partial, total) = parse_accounts(&accounts);
        assert_eq!(items.len(), 3);
        assert_eq!(enabled, 2);
        assert_eq!(synced, 2);
        assert!(!partial);
        assert_eq!(total, Some(200.0));
    }

    #[test]
    fn marks_partial_when_enabled_account_has_no_credit() {
        let accounts = json!({
            "accounts": [
                {"uid": "1", "credits": 10, "healthy": true, "in_pool": true},
                {"uid": "2", "credits": null, "healthy": true, "in_pool": true}
            ]
        });
        let (_, enabled, synced, partial, total) = parse_accounts(&accounts);
        assert_eq!(enabled, 2);
        assert_eq!(synced, 1);
        assert!(partial);
        assert_eq!(total, Some(10.0));
    }

    #[test]
    fn applies_realtime_credit_overrides() {
        let accounts = json!({
            "accounts": [
                {"uid": "1", "credits": 0, "healthy": true, "in_pool": true},
                {"uid": "2", "credits": 50, "healthy": true, "in_pool": true}
            ]
        });
        let (mut items, _, _, _, _) = parse_accounts(&accounts);
        let realtime = json!({"credits": {"1": 6432, "2": null}});

        let (synced, partial, total) = apply_realtime_credits(&mut items, &realtime);

        assert_eq!(items[0].credits, Some(6432.0));
        assert_eq!(items[1].credits, Some(50.0));
        assert_eq!(synced, 2);
        assert!(!partial);
        assert_eq!(total, Some(6482.0));
    }

    #[test]
    fn parses_key_usage_and_sorts() {
        let keys = json!([
            {"id": 1, "name": "alpha", "prefix": "sk-a", "enabled": true, "used_tokens": "1500000"},
            {"id": 2, "name": "beta", "prefix": "sk-b", "enabled": 0, "used_tokens": 500000},
            {"id": 3, "name": "gamma", "prefix": "sk-g", "enabled": true, "used_tokens": 200}
        ]);
        let today = json!([
            {"name": "alpha", "prompt_tokens": "1000000", "completion_tokens": 230000},
            {"name": "beta", "prompt_tokens": 1000, "completion_tokens": 2000}
        ]);

        let rows = parse_key_usage(&keys, &today);

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].name, "alpha");
        assert_eq!(rows[0].today_tokens, 1_230_000);
        assert_eq!(rows[0].total_tokens, 1_500_000);
        assert_eq!(rows[1].name, "gamma");
        assert_eq!(rows[1].today_tokens, 0);
        assert_eq!(rows[2].name, "beta");
        assert!(!rows[2].enabled);
    }

    #[test]
    fn formats_tray_title() {
        let mut snapshot = AppSnapshot::default();
        snapshot.today.today_credit = 12.34;
        snapshot.total_credits = Some(12345.0);
        assert_eq!(tray_title(&snapshot), "12.34\n12,345");

        snapshot.today.today_credit = 123456.0;
        snapshot.total_credits = Some(123456.0);
        assert_eq!(tray_title(&snapshot), "123.5k\n123.5k");

        snapshot.today.today_credit = 0.0;
        snapshot.total_credits = None;
        assert_eq!(tray_title(&snapshot), "0\n--");
    }

    #[test]
    fn migrates_v3_config_keeping_token_mode() {
        // 1.0.1 的配置只有访问令牌：升级后仍是令牌方式，地址与其它设置不丢。
        let legacy = r#"{
            "configVersion": 3,
            "baseUrl": "http://47.99.55.3:7864",
            "token": "wbt_abc",
            "refreshIntervalSec": 120,
            "lowCreditThreshold": 1000.0,
            "launchAtLogin": false
        }"#;
        let mut config: AppConfig = serde_json::from_str(legacy).unwrap();
        assert!(migrate_config(&mut config));
        assert_eq!(config.config_version, CONFIG_VERSION);
        assert_eq!(config.auth_mode, AUTH_MODE_TOKEN);
        assert_eq!(config.token, "wbt_abc");
        assert_eq!(config.base_url, "http://47.99.55.3:7864");
    }

    #[test]
    fn defaults_expiry_reminder_days_for_old_config() {
        // 1.0.5 及更早的配置没有 expiryReminderDays：解析后回落到默认 2 天。
        let legacy = r#"{
            "configVersion": 4,
            "baseUrl": "http://47.99.55.3:7864",
            "token": "wbt_abc",
            "refreshIntervalSec": 120,
            "lowCreditThreshold": 1000.0,
            "launchAtLogin": false
        }"#;
        let config: AppConfig = serde_json::from_str(legacy).unwrap();
        assert_eq!(config.expiry_reminder_days, 2);
    }

    #[test]
    fn migrates_legacy_password_config_back_to_password_mode() {
        // 1.0.0 及更早的配置存着账号/密码/会话：升级后回到账户密码方式，
        // 这样会话过期时还能自动重新登录。已删除的旧字段（密钥名单）不再写回。
        let legacy = r#"{
            "configVersion": 2,
            "baseUrl": "http://47.99.55.3:7864",
            "username": "stats",
            "password": "secret",
            "sessionCookie": "abc",
            "refreshIntervalSec": 120,
            "lowCreditThreshold": 1000.0,
            "launchAtLogin": false,
            "keyUsageNames": ["公司Mac"]
        }"#;
        let mut config: AppConfig = serde_json::from_str(legacy).unwrap();
        assert!(migrate_config(&mut config));
        assert_eq!(config.config_version, CONFIG_VERSION);
        assert_eq!(config.auth_mode, AUTH_MODE_PASSWORD);
        assert_eq!(config.username, "stats");
        assert_eq!(config.password, "secret");
        assert_eq!(config.session_cookie, "abc");
        assert!(config.can_refresh());

        let written = serde_json::to_string(&config).unwrap();
        assert!(!written.contains("keyUsageNames"));
    }

    #[test]
    fn requires_credentials_matching_auth_mode() {
        let mut config = AppConfig::default();
        assert!(!config.can_refresh());

        config.base_url = "http://host:1".into();
        assert!(!config.can_refresh());
        config.token = "wbt_x".into();
        assert!(config.can_refresh());

        config.auth_mode = AUTH_MODE_PASSWORD.into();
        config.token.clear();
        assert!(!config.can_refresh());
        config.username = "stats".into();
        config.password = "secret".into();
        assert!(config.can_refresh());

        // 只剩会话 Cookie 也继续轮询，避免每次启动都强制重新登录。
        config.username.clear();
        config.password.clear();
        assert!(!config.can_refresh());
        config.session_cookie = "sess".into();
        assert!(config.can_refresh());
    }

    #[test]
    fn extracts_renewed_session_cookie() {
        let mut headers = HeaderMap::new();
        headers.append(
            SET_COOKIE,
            "wb_session=abc123; HttpOnly; Path=/; SameSite=lax"
                .parse()
                .unwrap(),
        );
        assert_eq!(extract_session_cookie(&headers).as_deref(), Some("abc123"));

        // 退出登录时管理端会下发空值删 Cookie，不能当成新会话。
        let mut cleared = HeaderMap::new();
        cleared.append(SET_COOKIE, "wb_session=; Path=/".parse().unwrap());
        assert_eq!(extract_session_cookie(&cleared), None);
    }

    #[test]
    fn applies_auth_header_by_mode() {
        let client = Client::new();
        let mut config = AppConfig {
            base_url: "http://host:1".into(),
            token: "wbt_abc".into(),
            ..AppConfig::default()
        };

        let request = apply_auth(client.get("http://host:1/api/me"), &config)
            .build()
            .unwrap();
        assert_eq!(
            request
                .headers()
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap(),
            "Bearer wbt_abc"
        );
        assert!(request.headers().get(COOKIE).is_none());

        config.auth_mode = AUTH_MODE_PASSWORD.into();
        config.session_cookie = "sess123".into();
        let request = apply_auth(client.get("http://host:1/api/me"), &config)
            .build()
            .unwrap();
        assert_eq!(
            request.headers().get(COOKIE).unwrap().to_str().unwrap(),
            "wb_session=sess123"
        );
        assert!(request.headers().get("authorization").is_none());
    }

    #[test]
    fn keeps_manager_wording_for_password_login_failures() {
        let rejected = login_message(StatusCode::UNAUTHORIZED, r#"{"detail":"用户名或密码错误"}"#);
        assert_eq!(rejected, "用户名或密码错误");
        assert!(!rejected.contains("访问令牌"));

        let locked = login_message(
            StatusCode::TOO_MANY_REQUESTS,
            r#"{"detail":"失败次数过多，请 10 分钟后再试"}"#,
        );
        assert_eq!(locked, "失败次数过多，请 10 分钟后再试");
    }

    #[test]
    fn migrates_old_default_refresh_interval_once() {
        let mut config = AppConfig {
            config_version: 0,
            refresh_interval_sec: 60,
            ..AppConfig::default()
        };

        assert!(migrate_config(&mut config));
        assert_eq!(config.config_version, CONFIG_VERSION);
        assert_eq!(config.refresh_interval_sec, 120);

        config.refresh_interval_sec = 60;
        assert!(!migrate_config(&mut config));
        assert_eq!(config.refresh_interval_sec, 60);
    }

    #[test]
    fn formats_compact_credit_title() {
        assert_eq!(compact_number(12345.0), "12,345");
        assert_eq!(compact_number(123456.0), "123.5k");
        assert_eq!(compact_number(12.5), "12.50");
    }
}
