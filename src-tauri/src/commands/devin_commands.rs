//! Devin Session 认证相关的 Tauri 命令
//!
//! 暴露给前端的命令：
//! - `devin_check_connections(email)` — 查询邮箱可用登录方式
//! - `devin_password_login(email, password)` — 仅账密登录，返回 auth1_token（底层接口）
//! - `devin_windsurf_post_auth(auth1_token, org_id)` — 换取 session_token（底层接口）
//! - `add_account_by_devin_login(...)` — 完整流程：登录 + 建账号（主流程）
//! - `devin_select_org(account_id, org_id)` — 多组织场景下的二次选择
//! - `refresh_devin_session(id)` — 用 auth1_token 重新换取 session_token

use crate::models::{Account, OperationLog, OperationStatus, OperationType};
use crate::repository::DataStore;
use crate::services::devin_auth_service::{
    CheckUserLoginMethodResult, ConnectionsResponse, DevinAuthService, DevinLoginResult,
    LoginMethodSniffResult, PasswordLoginResponse, WindsurfPostAuthResult,
};
use crate::services::WindsurfService;
use serde_json::json;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

// ==================== 底层接口（便于调试与高级用法） ====================

#[tauri::command]
pub async fn devin_check_connections(email: String) -> Result<ConnectionsResponse, String> {
    DevinAuthService::new()
        .check_connections(&email)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn devin_password_login(
    email: String,
    password: String,
) -> Result<PasswordLoginResponse, String> {
    DevinAuthService::new()
        .password_login(&email, &password)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn devin_windsurf_post_auth(
    auth1_token: String,
    org_id: Option<String>,
) -> Result<WindsurfPostAuthResult, String> {
    DevinAuthService::new()
        .windsurf_post_auth(&auth1_token, org_id.as_deref().unwrap_or(""))
        .await
        .map_err(|e| e.to_string())
}

// ==================== Devin session_token 迁入 ====================

/// 通过已有的 `devin-session-token$...` 前缀 session_token 直接导入 Devin 账号
///
/// 适用场景：用户从浏览器 localStorage / cookie 拷出有效 session_token 的迁入路径。
/// 仅需 `session_token` 即可，后端用它调 GetCurrentUser 反查 email / api_key / 配额等信息回填账号。
///
/// Devin 扩展字段（devin_account_id / devin_auth1_token / devin_primary_org_id）留空——
/// 日常 API（GetCurrentUser / GetPlanStatus / ResetCredits 等）仅需 session_token 即可工作；
/// 仅 `refresh_devin_session` 等显式依赖 auth1_token 的操作会失败（到期需用户重新获取 session_token）。
///
/// # Arguments
/// * `session_token` - 带 `devin-session-token$` 前缀的完整 token
/// * `nickname` - 可选备注名；留空则用反查到的 email 前缀
/// * `tags` - 标签列表
/// * `group` - 分组（可选）
#[tauri::command]
pub async fn add_account_by_devin_session_token(
    session_token: String,
    nickname: Option<String>,
    tags: Vec<String>,
    group: Option<String>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let token_trimmed = session_token.trim().to_string();
    if !token_trimmed.starts_with("devin-session-token$") {
        return Err(
            "session_token 必须以 `devin-session-token$` 前缀开头，当前输入无效".to_string(),
        );
    }

    // 仅带 session_token 的 AuthContext（仅发 x-auth-token + x-devin-session-token）
    let ctx = crate::services::AuthContext::devin_session_only(token_trimmed.clone());

    // 反查 GetCurrentUser 拿 email / api_key / 套餐 / 配额 等信息
    let windsurf_service = WindsurfService::new();
    let user_info_result = windsurf_service
        .get_current_user(&ctx)
        .await
        .map_err(|e| format!("反查账号信息失败（可能 session_token 已失效）: {}", e))?;

    if !user_info_result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let msg = user_info_result
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("未知错误");
        return Err(format!("反查账号信息失败：{}", msg));
    }

    let user_info = user_info_result
        .get("user_info")
        .ok_or_else(|| "GetCurrentUser 响应缺少 user_info 字段".to_string())?;

    let email = user_info
        .get("user")
        .and_then(|u| u.get("email"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "GetCurrentUser 响应未找到 email，无法建立账号".to_string())?
        .to_string();

    // 已存在检查（邮箱不区分大小写）
    let existing = store.get_all_accounts().await.map_err(|e| e.to_string())?;
    if existing
        .iter()
        .any(|acc| acc.email.to_lowercase() == email.to_lowercase())
    {
        return Err(format!("账号 {} 已存在", email));
    }

    let final_nickname = nickname
        .clone()
        .and_then(|s| {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        })
        .unwrap_or_else(|| email.split('@').next().unwrap_or(&email).to_string());

    // session_token 迁入场景无原始密码，password 字段留空
    let mut account = store
        .add_account(email.clone(), String::new(), final_nickname)
        .await
        .map_err(|e| e.to_string())?;

    account.tags = tags;
    account.group = group;
    account.status = crate::models::account::AccountStatus::Active;
    account.last_login_at = Some(chrono::Utc::now());
    account.token = Some(token_trimmed.clone());
    account.auth_provider = Some("devin".to_string());
    // devin_account_id / devin_auth1_token / devin_primary_org_id 留空（仅 session_token 路径）

    // 复用已拿到的 user_info 回填配额 / 套餐 / api_key 等字段
    apply_user_info_to_account(&mut account, user_info);

    store
        .update_account(account.clone())
        .await
        .map_err(|e| e.to_string())?;

    let log = OperationLog::new(
        OperationType::AddAccount,
        OperationStatus::Success,
        format!("通过 Devin session_token 添加账号: {}", email),
    )
    .with_account(account.id, email.clone());
    let _ = store.add_log(log).await;

    Ok(json!({
        "success": true,
        "account": account,
        "email": email,
        "plan_name": account.plan_name,
        "used_quota": account.used_quota,
        "total_quota": account.total_quota,
    }))
}

// ==================== 登录流派崇探（方案 B 核心）====================

/// 匿名调用 `CheckUserLoginMethod`，返回 Firebase(WS) 侧对该邮箱的登录方式判断
///
/// 用于调试或 UI 明细展示；日常智能登录请直接使用 `sniff_login_method` 聚合命令。
#[tauri::command]
pub async fn devin_check_user_login_method(
    email: String,
) -> Result<CheckUserLoginMethodResult, String> {
    DevinAuthService::new()
        .check_user_login_method(&email)
        .await
        .map_err(|e| e.to_string())
}

/// 登录流派智能嗅探（方案 B 的统一入口）
///
/// 并发调 `CheckUserLoginMethod`（Firebase 侧）+ `/_devin-auth/connections`（Devin 侧），
/// 聚合后返回 `recommended` 字段指示推荐的登录流派：
/// `"firebase"` / `"devin"` / `"sso"` / `"no_password"` / `"not_found"` / `"blocked"`。
///
/// 前端据此分派到对应的 `add_account_by_login` / `add_account_by_devin_login` 等命令，
/// 用户输入仅需 email + password，无需感知底层协议差异。
#[tauri::command]
pub async fn sniff_login_method(email: String) -> Result<LoginMethodSniffResult, String> {
    DevinAuthService::new()
        .sniff_login_method(&email)
        .await
        .map_err(|e| e.to_string())
}

/// 发送邮箱验证码（注册 / 无密码登录 共用）
///
/// - `mode`：`"signup"` 或 `"login"`，默认 `"signup"`
/// - `product`：默认 `None` 时服务端同样返回验证码；显式传 `Some("windsurf")` 与网页端一致
#[tauri::command]
pub async fn devin_email_start(
    email: String,
    mode: Option<String>,
    product: Option<String>,
) -> Result<crate::services::devin_auth_service::EmailStartResponse, String> {
    DevinAuthService::new()
        .email_start(
            &email,
            mode.as_deref().unwrap_or("signup"),
            product.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())
}

/// 提交邮箱验证码 + 可选凭证完成邮件流程
///
/// - `mode == "signup"`：需传 `password` + `name`
/// - `mode == "login"`：无需 `password` / `name`
///
/// 响应体结构与 `/password/login` 一致：`{ auth1_token, account_id, email, ... }`
#[tauri::command]
pub async fn devin_email_complete(
    email_verification_token: String,
    code: String,
    mode: String,
    password: Option<String>,
    name: Option<String>,
) -> Result<PasswordLoginResponse, String> {
    DevinAuthService::new()
        .email_complete(
            &email_verification_token,
            &code,
            &mode,
            password.as_deref(),
            name.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())
}

/// 发起“忘记密码”流程：服务端向 `email` 发送重置验证码
#[tauri::command]
pub async fn devin_password_reset_start(
    email: String,
    product: Option<String>,
) -> Result<crate::services::devin_auth_service::EmailStartResponse, String> {
    DevinAuthService::new()
        .password_reset_start(&email, product.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 完成“忘记密码”流程：提交验证码 + 新密码
#[tauri::command]
pub async fn devin_password_reset_complete(
    email_verification_token: String,
    code: String,
    new_password: String,
) -> Result<(), String> {
    DevinAuthService::new()
        .password_reset_complete(&email_verification_token, &code, &new_password)
        .await
        .map_err(|e| e.to_string())
}

// ==================== 主业务命令 ====================

/// 完整的 Devin 账密登录 + 建账号流程
///
/// 行为：
/// 1. `password_login` 得到 auth1_token
/// 2. `windsurf_post_auth(auth1_token, org_id="")` 得到 session_token + orgs
/// 3. 如果 orgs > 1 且未传 org_id，则**不**立即落库，返回 `requires_org_selection=true` + orgs
/// 4. 否则拉取用户信息并持久化账号
#[tauri::command]
pub async fn add_account_by_devin_login(
    email: String,
    password: String,
    nickname: Option<String>,
    tags: Vec<String>,
    group: Option<String>,
    org_id: Option<String>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let auth = DevinAuthService::new();

    // Step 1+2: 登录并换取 session_token
    let login = auth
        .login_with_password(&email, &password, org_id.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    // 多组织分支：要求 UI 二次选择
    if login.requires_org_selection {
        return Ok(json!({
            "success": false,
            "requires_org_selection": true,
            "auth1_token": login.auth1_token,
            "orgs": login.orgs,
            "email": email,
            "message": "检测到多个组织，请选择一个继续"
        }));
    }

    // 已存在检查
    let existing = store
        .get_all_accounts()
        .await
        .map_err(|e| e.to_string())?;
    if existing
        .iter()
        .any(|acc| acc.email.to_lowercase() == email.to_lowercase())
    {
        return Err(format!("账号 {} 已存在", email));
    }

    // Step 3: 创建账号骨架
    let final_nickname = nickname
        .unwrap_or_else(|| email.split('@').next().unwrap_or(&email).to_string());

    // 直接将用户输入的账密 password 落库，保证后续账号卡可始终回显、完整导出
    let mut account = store
        .add_account(email.clone(), password.clone(), final_nickname)
        .await
        .map_err(|e| e.to_string())?;

    // 基础字段
    account.tags = tags;
    account.group = group;
    account.status = crate::models::account::AccountStatus::Active;
    account.last_login_at = Some(chrono::Utc::now());

    // Devin 凭证
    account.token = Some(login.session_token.clone()); // session_token 放入 token 字段，保持下游透明
    account.devin_auth1_token = Some(login.auth1_token.clone());
    account.devin_account_id = login.account_id.clone();
    account.devin_primary_org_id = login.primary_org_id.clone();
    account.auth_provider = Some("devin".to_string());
    // Devin session_token 不像 Firebase id_token 那样有显式过期时间，这里不设置 expires_at

    // Step 4: 拉取用户详情（使用 session_token 作为 auth_token）
    enrich_account_with_user_info(&mut account, &login.session_token).await;

    store
        .update_account(account.clone())
        .await
        .map_err(|e| e.to_string())?;

    // 日志
    let log = OperationLog::new(
        OperationType::AddAccount,
        OperationStatus::Success,
        format!("通过 Devin 账密添加账号: {}", email),
    )
    .with_account(account.id, email.clone());
    let _ = store.add_log(log).await;

    Ok(json!({
        "success": true,
        "requires_org_selection": false,
        "account": account,
        "email": email,
        "plan_name": account.plan_name,
        "used_quota": account.used_quota,
        "total_quota": account.total_quota,
        "devin_account_id": account.devin_account_id,
        "primary_org_id": account.devin_primary_org_id,
    }))
}

/// 多组织场景下的二次选择：使用已有的 auth1_token 选择具体 org 并完成账号创建
///
/// `password` 为可选参数：
/// - 账密流注册/登录后的二次选 org 场景，前端请传入用户原始密码，使账号卡可回显密码
/// - 无密流（邮箱无密登录）或纯凭证迁入场景可传 None
#[tauri::command]
pub async fn add_account_by_devin_with_org(
    email: String,
    auth1_token: String,
    org_id: String,
    nickname: Option<String>,
    tags: Vec<String>,
    group: Option<String>,
    password: Option<String>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let auth = DevinAuthService::new();

    let post_auth = auth
        .windsurf_post_auth(&auth1_token, &org_id)
        .await
        .map_err(|e| e.to_string())?;

    // 已存在检查
    let existing = store
        .get_all_accounts()
        .await
        .map_err(|e| e.to_string())?;
    if existing
        .iter()
        .any(|acc| acc.email.to_lowercase() == email.to_lowercase())
    {
        return Err(format!("账号 {} 已存在", email));
    }

    let final_nickname = nickname
        .unwrap_or_else(|| email.split('@').next().unwrap_or(&email).to_string());

    // 同 add_account_by_devin_login：将用户原始密码落库，无密场景传 None 则保留空字段
    let stored_password = password.clone().unwrap_or_default();
    let mut account = store
        .add_account(email.clone(), stored_password, final_nickname)
        .await
        .map_err(|e| e.to_string())?;

    account.tags = tags;
    account.group = group;
    account.status = crate::models::account::AccountStatus::Active;
    account.last_login_at = Some(chrono::Utc::now());

    account.token = Some(post_auth.session_token.clone());
    account.devin_auth1_token = Some(
        post_auth
            .auth1_token
            .clone()
            .unwrap_or(auth1_token.clone()),
    );
    account.devin_account_id = post_auth.account_id.clone();
    account.devin_primary_org_id = post_auth.primary_org_id.clone().or(Some(org_id.clone()));
    account.auth_provider = Some("devin".to_string());

    enrich_account_with_user_info(&mut account, &post_auth.session_token).await;

    store
        .update_account(account.clone())
        .await
        .map_err(|e| e.to_string())?;

    let log = OperationLog::new(
        OperationType::AddAccount,
        OperationStatus::Success,
        format!("通过 Devin 账密添加账号 (org={}): {}", org_id, email),
    )
    .with_account(account.id, email.clone());
    let _ = store.add_log(log).await;

    Ok(json!({
        "success": true,
        "account": account,
        "email": email,
    }))
}

/// 通过 Devin 邮箱注册直接创建账号（signup 主流程）
///
/// 调用前提：调用方已通过 `devin_email_start(email, "signup", ...)` 拿到 `email_verification_token`，
/// 并引导用户读取邮件中的 6 位验证码。
///
/// 行为：
/// 1. `register_with_email_code(email_verification_token, code, password, name, org_id)` → `DevinLoginResult`
/// 2. 若 `requires_org_selection == true`，返回 `{requires_org_selection: true, auth1_token, orgs}`，
///    由前端引导用户选组织后调 `add_account_by_devin_with_org` 二次完成
/// 3. 否则落库为新账号
#[tauri::command]
pub async fn add_account_by_devin_register(
    email: String,
    email_verification_token: String,
    code: String,
    password: String,
    name: String,
    nickname: Option<String>,
    tags: Vec<String>,
    group: Option<String>,
    org_id: Option<String>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let auth = DevinAuthService::new();

    // Step 1+2: 注册 + 换取 session_token
    let login = auth
        .register_with_email_code(
            &email_verification_token,
            &code,
            &password,
            &name,
            org_id.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;

    // 多组织分支：要求 UI 二次选择
    if login.requires_org_selection {
        return Ok(json!({
            "success": false,
            "requires_org_selection": true,
            "auth1_token": login.auth1_token,
            "orgs": login.orgs,
            "email": email,
            "message": "检测到多个组织，请选择一个继续"
        }));
    }

    let account = persist_devin_account_from_login_result(
        &store,
        &email,
        &password,
        nickname,
        tags,
        group,
        &login,
        &format!("通过 Devin 邮箱注册添加账号: {}", email),
    )
    .await?;

    Ok(json!({
        "success": true,
        "requires_org_selection": false,
        "account": account,
        "email": email,
        "plan_name": account.plan_name,
        "used_quota": account.used_quota,
        "total_quota": account.total_quota,
        "devin_account_id": account.devin_account_id,
        "primary_org_id": account.devin_primary_org_id,
    }))
}

/// 通过 Devin 邮箱验证码登录（无密码账号）直接添加账号
///
/// 用于从 SSO 迁移且无密码的 Devin 账号。流程与 `add_account_by_devin_register` 相同，
/// 区别在 `email_complete(mode="login")`——服务端不会创建新账号，而是返回已有账号的 auth1_token。
#[tauri::command]
pub async fn add_account_by_devin_email_login(
    email: String,
    email_verification_token: String,
    code: String,
    nickname: Option<String>,
    tags: Vec<String>,
    group: Option<String>,
    org_id: Option<String>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let auth = DevinAuthService::new();

    let login = auth
        .login_with_email_code(&email_verification_token, &code, org_id.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    if login.requires_org_selection {
        return Ok(json!({
            "success": false,
            "requires_org_selection": true,
            "auth1_token": login.auth1_token,
            "orgs": login.orgs,
            "email": email,
            "message": "检测到多个组织，请选择一个继续"
        }));
    }

    // 无密登录场景：没有原始密码可落库，传空字段
    let account = persist_devin_account_from_login_result(
        &store,
        &email,
        "",
        nickname,
        tags,
        group,
        &login,
        &format!("通过 Devin 邮件验证码登录添加账号: {}", email),
    )
    .await?;

    Ok(json!({
        "success": true,
        "requires_org_selection": false,
        "account": account,
        "email": email,
        "plan_name": account.plan_name,
        "used_quota": account.used_quota,
        "total_quota": account.total_quota,
        "devin_account_id": account.devin_account_id,
        "primary_org_id": account.devin_primary_org_id,
    }))
}

/// 使用已持久化的 auth1_token 重新换取 session_token
///
/// 当 Devin session_token 失效（401）时，可用此命令刷新
#[tauri::command]
pub async fn refresh_devin_session(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut account = store.get_account(uuid).await.map_err(|e| e.to_string())?;

    let auth1_token = account
        .devin_auth1_token
        .clone()
        .ok_or_else(|| "该账号未存储 Devin auth1_token，无法刷新".to_string())?;

    let org_id = account.devin_primary_org_id.clone().unwrap_or_default();

    let auth = DevinAuthService::new();
    let post_auth = auth
        .windsurf_post_auth(&auth1_token, &org_id)
        .await
        .map_err(|e| e.to_string())?;

    account.token = Some(post_auth.session_token.clone());
    if let Some(new_a1) = post_auth.auth1_token.clone() {
        account.devin_auth1_token = Some(new_a1);
    }
    if post_auth.account_id.is_some() {
        account.devin_account_id = post_auth.account_id.clone();
    }
    if post_auth.primary_org_id.is_some() {
        account.devin_primary_org_id = post_auth.primary_org_id.clone();
    }
    account.status = crate::models::account::AccountStatus::Active;
    account.last_login_at = Some(chrono::Utc::now());

    store
        .update_account(account.clone())
        .await
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "session_token": post_auth.session_token,
        "primary_org_id": post_auth.primary_org_id,
        "message": "Devin 会话已刷新"
    }))
}

// ==================== 内部工具函数（可被同 crate 其它 commands 复用） ====================

/// 使用 Devin `auth1_token` 重新换取 session_token 并更新传入的 `account`（仅内存）
///
/// 用于 `api_commands` 中各需要刷新 token 的场景（如 `login_account` / `refresh_token` /
/// `ensure_valid_token`）对 Devin 账号的分流处理。不会落库，由调用方决定是否持久化。
pub(crate) async fn refresh_devin_session_in_memory(
    account: &mut Account,
) -> Result<String, String> {
    let auth1_token = account
        .devin_auth1_token
        .clone()
        .ok_or_else(|| "该 Devin 账号缺失 auth1_token，无法刷新 session_token".to_string())?;

    let org_id = account.devin_primary_org_id.clone().unwrap_or_default();

    let post_auth = DevinAuthService::new()
        .windsurf_post_auth(&auth1_token, &org_id)
        .await
        .map_err(|e| e.to_string())?;

    account.token = Some(post_auth.session_token.clone());
    if let Some(new_a1) = post_auth.auth1_token.clone() {
        account.devin_auth1_token = Some(new_a1);
    }
    if post_auth.account_id.is_some() {
        account.devin_account_id = post_auth.account_id.clone();
    }
    if post_auth.primary_org_id.is_some() {
        account.devin_primary_org_id = post_auth.primary_org_id.clone();
    }

    Ok(post_auth.session_token)
}

/// 为 enrich_account_with_user_info 构造合适的 AuthContext：
/// Devin 账号都走 DevinAuthContext【支持仅 session_token 的部分字段场景】，
/// Firebase 账号走 firebase 单 header。
fn build_auth_context_for_account(
    account: &Account,
    session_token: &str,
) -> crate::services::AuthContext {
    if account.is_devin_account() {
        crate::services::AuthContext {
            token: session_token.to_string(),
            devin: Some(crate::services::DevinAuthContext {
                account_id: account.devin_account_id.clone(),
                auth1_token: account.devin_auth1_token.clone(),
                primary_org_id: account.devin_primary_org_id.clone(),
            }),
        }
    } else {
        crate::services::AuthContext::firebase(session_token.to_string())
    }
}

/// 将 GetCurrentUser 返回的 `user_info` 嵌套对象的字段回填到 `Account`
///
/// 纯内存操作，不发网络请求。供 `enrich_account_with_user_info`（包含一次网络拉取）
/// 和 `add_account_by_devin_session_token`（已在构建阶段拿到 user_info，避免重复拉取）复用。
pub(crate) fn apply_user_info_to_account(
    account: &mut Account,
    user_info: &serde_json::Value,
) {
    // 基本信息（api_key、禁用状态）
    if let Some(user) = user_info.get("user") {
        if let Some(api_key) = user.get("api_key").and_then(|v| v.as_str()) {
            account.windsurf_api_key = Some(api_key.to_string());
        }
        if let Some(disabled) = user.get("disable_codeium").and_then(|v| v.as_bool()) {
            account.is_disabled = Some(disabled);
        }
    }

    // 套餐
    if let Some(plan) = user_info.get("plan") {
        if let Some(plan_name) = plan.get("plan_name").and_then(|v| v.as_str()) {
            account.plan_name = Some(plan_name.to_string());
        }
    }

    // 订阅配额
    if let Some(subscription) = user_info.get("subscription") {
        if let Some(used) = subscription.get("used_quota").and_then(|v| v.as_i64()) {
            account.used_quota = Some(used as i32);
        }
        if let Some(total) = subscription.get("quota").and_then(|v| v.as_i64()) {
            account.total_quota = Some(total as i32);
        }
        if let Some(expires_at) = subscription.get("expires_at").and_then(|v| v.as_i64()) {
            account.subscription_expires_at =
                chrono::DateTime::from_timestamp(expires_at, 0);
        }
        if let Some(active) = subscription
            .get("subscription_active")
            .and_then(|v| v.as_bool())
        {
            account.subscription_active = Some(active);
        }
    }

    account.last_quota_update = Some(chrono::Utc::now());
}

/// 使用 session_token 拉取用户信息并回填账号字段
pub(crate) async fn enrich_account_with_user_info(account: &mut Account, session_token: &str) {
    let ctx = build_auth_context_for_account(account, session_token);
    let windsurf_service = WindsurfService::new();
    let Ok(user_info_result) = windsurf_service.get_current_user(&ctx).await else {
        return;
    };
    let Some(user_info) = user_info_result.get("user_info") else {
        return;
    };
    apply_user_info_to_account(account, user_info);
}

/// 从 `DevinLoginResult` 持久化一个新账号
///
/// 封装通用的“已存在检查 → add_account 骨架 → 填字段 → enrich → update → 写日志”流程，
/// 供新增的 `add_account_by_devin_register` / `add_account_by_devin_email_login` 复用。
///
/// 调用方需自行负责多组织分支（`requires_org_selection`）的处理，本函数只做落库。
pub(crate) async fn persist_devin_account_from_login_result(
    store: &DataStore,
    email: &str,
    // password: 用户原始密码。账密注册/登录场景传入以便账号卡回显；邮箱无密等场景传 ""
    password: &str,
    nickname: Option<String>,
    tags: Vec<String>,
    group: Option<String>,
    login: &DevinLoginResult,
    log_reason: &str,
) -> Result<Account, String> {
    // 已存在检查（邮箱不区分大小写）
    let existing = store
        .get_all_accounts()
        .await
        .map_err(|e| e.to_string())?;
    if existing
        .iter()
        .any(|acc| acc.email.to_lowercase() == email.to_lowercase())
    {
        return Err(format!("账号 {} 已存在", email));
    }

    let final_nickname = nickname
        .unwrap_or_else(|| email.split('@').next().unwrap_or(email).to_string());

    let mut account = store
        .add_account(email.to_string(), password.to_string(), final_nickname)
        .await
        .map_err(|e| e.to_string())?;

    // 基础字段
    account.tags = tags;
    account.group = group;
    account.status = crate::models::account::AccountStatus::Active;
    account.last_login_at = Some(chrono::Utc::now());

    // Devin 凭证（session_token 放入 token 字段，保持下游透明）
    account.token = Some(login.session_token.clone());
    account.devin_auth1_token = Some(login.auth1_token.clone());
    account.devin_account_id = login.account_id.clone();
    account.devin_primary_org_id = login.primary_org_id.clone();
    account.auth_provider = Some("devin".to_string());

    // 拉用户详情回填套餐、配额、api_key 等
    enrich_account_with_user_info(&mut account, &login.session_token).await;

    store
        .update_account(account.clone())
        .await
        .map_err(|e| e.to_string())?;

    let log = OperationLog::new(
        OperationType::AddAccount,
        OperationStatus::Success,
        log_reason.to_string(),
    )
    .with_account(account.id, email.to_string());
    let _ = store.add_log(log).await;

    Ok(account)
}
