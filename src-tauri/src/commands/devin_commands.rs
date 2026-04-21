//! Devin Session 认证相关的 Tauri 命令
//!
//! 暴露给前端的命令：
//! - `devin_check_connections(email)` — 查询邮箱可用登录方式
//! - `devin_password_login(email, password)` — 仅账密登录，返回 auth1_token（底层接口）
//! - `devin_windsurf_post_auth(auth1_token, org_id)` — 换取 session_token（底层接口）
//! - `add_account_by_devin_login(...)` — 完整流程：登录 + 建账号（主流程）
//! - `devin_select_org(account_id, org_id)` — 多组织场景下的二次选择
//! - `refresh_devin_session(id)` — 用 auth1_token 重新换取 session_token
//! - `add_account_by_devin_auth1_token(...)` — 通过 auth1_token 直接迁入账号（与 session_token 迁入对称）

use crate::commands::api_commands::devin_session_pseudo_expires_at;
use crate::models::{Account, OperationLog, OperationStatus, OperationType};
use crate::repository::DataStore;
use crate::services::devin_auth_service::{
    CheckUserLoginMethodResult, ConnectionsResponse, DevinAuthService, DevinLoginResult,
    LoginMethodSniffResult, PasswordLoginResponse, WindsurfPostAuthResult,
};
use crate::services::{AuthContext, AuthService, WindsurfService};
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
    account.token_expires_at = Some(devin_session_pseudo_expires_at());
    account.auth_provider = Some("devin".to_string());
    // devin_account_id / devin_auth1_token / devin_primary_org_id 留空（仅 session_token 路径）

    // 复用已拿到的 user_info 回填配额 / 套餐 / api_key 等字段
    apply_user_info_to_account(&mut account, user_info);

    // 补拉 GetPlanStatus：回填 billing_strategy + daily/weekly_quota_remaining_percent
    // 等 QUOTA 模式专用字段，避免账号卡降级到 CREDITS 积分显示。
    // GetCurrentUser 已在上方完成，这里只多发一次 GetPlanStatus 网络请求。
    enrich_account_with_plan_status(&mut account, &token_trimmed).await;

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
/// - `product`：默认 `None` 时服务端同样返回验证码；显式传 `Some("Windsurf")` 与网页端一致
///   （服务端对该字段做字面值校验，只接受 `"Devin"` 或 `"Windsurf"`，传小写会 422）
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

// ==================== Devin 原生站点（app.devin.ai）底层接口 ====================

/// 查询 Devin 原生侧（app.devin.ai）邮箱可用的连接方式
///
/// 端口：`POST https://app.devin.ai/api/auth1/connections`（区别于 Windsurf 侧的
/// `https://windsurf.com/_devin-auth/connections`）。响应的 `connections` 数组中会额外
/// 包含 `windsurf-bridge` 条目，提示该邮箱可跨桥到 Windsurf。
///
/// 用途：UI 在进入"Devin 原生注册"流程前的可选预检，判断邮箱是否已注册。
#[tauri::command]
pub async fn devin_app_check_connections(email: String) -> Result<ConnectionsResponse, String> {
    DevinAuthService::new()
        .devin_app_check_connections(&email)
        .await
        .map_err(|e| e.to_string())
}

/// 向 Devin 原生侧（app.devin.ai）发送邮箱验证码
///
/// 端口：`POST https://app.devin.ai/api/auth1/email/start`
/// 请求体：`{"email":"...","mode":"signup"|"login"}`（不携带 product 字段，与 Windsurf 侧有关键差异）
///
/// 返回：`EmailStartResponse { email_verification_token, ... }`，
/// 用于后续 `devin_app_email_complete` 回传。
///
/// 注意：此处注册出的账号主归属 **Devin 产品侧**，JWT payload 中 `product` 为 `"Devin"`，
/// 后续可用 `add_account_by_devin_native_register` 一键桥接到 Windsurf 并落库。
#[tauri::command]
pub async fn devin_app_email_start(
    email: String,
    mode: Option<String>,
) -> Result<crate::services::devin_auth_service::EmailStartResponse, String> {
    DevinAuthService::new()
        .devin_app_email_start(&email, mode.as_deref().unwrap_or("signup"))
        .await
        .map_err(|e| e.to_string())
}

/// 提交验证码完成 Devin 原生侧邮件流程
///
/// 端口：`POST https://app.devin.ai/api/auth1/email/complete`
/// 请求体：`{"email_verification_token":"...","code":"...","mode":"signup"|"login"}`
/// （不携带 password / name 字段，与 Windsurf 侧有关键差异：Devin 原生注册是"纯邮箱验证码"建号）
///
/// 响应：`PasswordLoginResponse { auth1_token, account_id, email, ... }`，
/// 其中 `auth1_token` 直接即是 Devin 侧的终端凭证（格式 `auth1_<52>`）。
#[tauri::command]
pub async fn devin_app_email_complete(
    email_verification_token: String,
    code: String,
    mode: String,
) -> Result<PasswordLoginResponse, String> {
    DevinAuthService::new()
        .devin_app_email_complete(&email_verification_token, &code, &mode)
        .await
        .map_err(|e| e.to_string())
}

// ==================== Windsurf 侧忘记密码流程（保持原位） ====================

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
    // Devin session_token 本身没有显式过期时间，用 pseudo_expires_at（+32d）占位，
    // 避免 `is_token_expired` 把新建账号误判为过期。真正过期判定靠 401 触发 force_refresh。
    account.token_expires_at = Some(devin_session_pseudo_expires_at());

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
    // 与 add_account_by_devin_login 一致：初建时填 pseudo_expires_at，保证账号卡「到期时间」立即可见
    account.token_expires_at = Some(devin_session_pseudo_expires_at());

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

/// 通过 **Devin 原生站点**（app.devin.ai）完成注册 + 桥接到 Windsurf + 落库
///
/// 与 `add_account_by_devin_register` 的关键区别：
/// - 端口：`https://app.devin.ai/api/auth1/email/complete`（Devin 官方后端）
///   而非 `https://windsurf.com/_devin-auth/email/complete`（Windsurf 同源代理）
/// - 无需 `password` 与 `name` 字段（Devin 原生注册是"纯邮箱验证码"建号）
/// - 注册出的账号在 Devin 侧的 JWT payload 中 `product == "Devin"`，
///   随后通过 `WindsurfPostAuth(auth1_token, org_id)` 桥接拿到 Windsurf session_token，
///   落库后既可用 Devin 产品侧功能（auth1_token），也可用 Windsurf 产品侧 API（session_token）
///
/// 调用前提：已通过 `devin_app_email_start(email, "signup")` 拿到 `email_verification_token`，
/// 并引导用户从邮箱读取 6 位验证码。
///
/// 行为：
/// 1. `register_native_with_email_code(email_verification_token, code, org_id)` → `DevinLoginResult`
///    （内部先调 Devin 原生 `email/complete` 拿 auth1_token，再调 `WindsurfPostAuth` 换 session_token）
/// 2. 若 `requires_org_selection == true`，返回组织列表让 UI 选择，随后复用 `add_account_by_devin_with_org`
/// 3. 否则落库为新账号。由于 Devin 原生注册时用户未设定密码，`password` 字段留空
///
/// 参数语义与 `add_account_by_devin_register` 完全一致（少了 `password` 和 `name`），
/// 便于前端保留与"Windsurf 侧注册"一致的参数收集习惯
#[tauri::command]
pub async fn add_account_by_devin_native_register(
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

    // Step 1+2: Devin 原生 email/complete → auth1_token → WindsurfPostAuth → session_token
    let login = auth
        .register_native_with_email_code(&email_verification_token, &code, org_id.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    // 多组织分支：要求 UI 二次选择（与 Windsurf 侧注册一致）
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

    // Devin 原生注册不收集用户设定的密码（服务端不允许），password 字段留空
    // 落库账号后用户可在 Devin 产品侧自行设置密码（通过 password/reset-start 流程）
    let account = persist_devin_account_from_login_result(
        &store,
        &email,
        "",
        nickname,
        tags,
        group,
        &login,
        &format!("通过 Devin 原生注册（app.devin.ai）添加账号: {}", email),
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
    // 刷新命令本身也要刷新 pseudo_expires_at，避免用户手动刷新后到期时间依然是旧值
    account.token_expires_at = Some(devin_session_pseudo_expires_at());

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
///
/// 工作流：
/// 1. 调 GetCurrentUser 回填 plan_name / api_key / used_quota / total_quota / subscription_* 等基础字段
/// 2. 紧接着调 GetPlanStatus 回填 billing_strategy / daily_quota_remaining_percent /
///    weekly_quota_remaining_percent / daily/weekly_quota_reset_at_unix / overage_balance_micros
///    等"新版配额百分比模式（QUOTA）"专用字段
///
/// 这样所有走此函数的注册/登录/导入/刷新路径，建号完成后账号卡即可正确显示
/// "日配额 X% / 周配额 Y%"（当服务端账号真实为 billing_strategy=QUOTA 时），
/// 无需等待用户手动触发一次 login_account 才补齐字段。
///
/// 任意一步失败都静默跳过（保持原有"建号不因富化失败整体失败"的弱依赖语义）。
pub(crate) async fn enrich_account_with_user_info(account: &mut Account, session_token: &str) {
    let ctx = build_auth_context_for_account(account, session_token);
    let windsurf_service = WindsurfService::new();

    // Step 1: GetCurrentUser → 基础字段
    if let Ok(user_info_result) = windsurf_service.get_current_user(&ctx).await {
        if let Some(user_info) = user_info_result.get("user_info") {
            apply_user_info_to_account(account, user_info);
        }
    }

    // Step 2: GetPlanStatus → 新版 QUOTA 模式字段（billing_strategy + daily/weekly 百分比等）
    // 注意：对 billing_strategy=CREDITS 的账号，这一步只会刷新 used_quota/total_quota，
    // 不会误填日/周百分比字段（apply_plan_status_to_account 只在字段存在时才写）
    enrich_account_with_plan_status_inner(account, &ctx, &windsurf_service).await;
}

/// 仅调用 GetPlanStatus 并回填账号字段，不触发 GetCurrentUser
///
/// 供 `add_account_by_devin_session_token` / `add_account_by_devin_auth1_token`
/// 等"已在建号阶段拿到 user_info，避免重复 GetCurrentUser"的路径使用：
/// 这些路径已调用 `apply_user_info_to_account`，但同样缺 QUOTA 模式字段，
/// 因此注册入口完成 apply_user_info_to_account 之后再显式调一次本函数即可补齐。
pub(crate) async fn enrich_account_with_plan_status(account: &mut Account, session_token: &str) {
    let ctx = build_auth_context_for_account(account, session_token);
    let windsurf_service = WindsurfService::new();
    enrich_account_with_plan_status_inner(account, &ctx, &windsurf_service).await;
}

/// 共享实现：已持有 ctx + service 时，直接调 GetPlanStatus 并回填
///
/// 抽出为独立函数是为了避免 `enrich_account_with_user_info` 内部重复构造 ctx，
/// 也便于未来扩展"是否重试"等策略而不污染调用方。
async fn enrich_account_with_plan_status_inner(
    account: &mut Account,
    ctx: &crate::services::AuthContext,
    windsurf_service: &WindsurfService,
) {
    let Ok(plan_status_result) = windsurf_service.get_plan_status(ctx).await else {
        return;
    };
    if !plan_status_result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return;
    }
    let Some(plan_status) = plan_status_result.get("plan_status") else {
        return;
    };
    crate::commands::api_commands::apply_plan_status_to_account(plan_status, account);
}

// ==================== Devin auth1_token 迁入 ====================

/// 通过已有的 Devin `auth1_token` 直接导入账号
///
/// 适用场景：用户从浏览器 localStorage 拷出 `devin_auth1_token`（格式 `auth1_<52 字符>`）的迁入路径。
/// 与 `add_account_by_devin_session_token` 对称，但多保留 auth1_token，
/// 使未来 `refresh_devin_session` 能正常刷新 session。
///
/// 行为：
/// 1. `windsurf_post_auth(auth1_token, org_id.unwrap_or(""))` 换取 session_token + orgs
/// 2. 用 session_token 调 `GetCurrentUser` 反查 email / api_key / 套餐 / 配额
/// 3. 分支：
///    - `orgs > 1` 且未指定 `org_id` 且 `auto_select_primary_org != true`：
///      返回 `{ requires_org_selection: true, email, auth1_token, orgs }`，由前端引导用户选组织后
///      再调 `add_account_by_devin_with_org(email, auth1_token, chosen_org_id, ...)` 完成落库
///    - 单组织 / 已指定 org_id / 批量自动选 primary：直接落库
///
/// # Arguments
/// * `auth1_token` - 完整的 `auth1_<52字符>` 令牌
/// * `org_id` - 可选，指定要加入的组织 ID；为空时后端用用户的 primary org
/// * `nickname` - 可选备注名；留空时用反查到的 email 前缀
/// * `tags` - 标签列表
/// * `group` - 分组（可选）
/// * `auto_select_primary_org` - 可选（默认 false）；批量导入场景传 true，多组织时直接用 primary_org 落库而非返回选择需求
#[tauri::command]
pub async fn add_account_by_devin_auth1_token(
    auth1_token: String,
    org_id: Option<String>,
    nickname: Option<String>,
    tags: Vec<String>,
    group: Option<String>,
    auto_select_primary_org: Option<bool>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let token_trimmed = auth1_token.trim().to_string();
    if !token_trimmed.starts_with("auth1_") {
        return Err("auth1_token 必须以 `auth1_` 前缀开头，当前输入无效".to_string());
    }
    // auth1_ (6) + 52 字符 = 58；留一点宽容度以防官方后续调整
    if token_trimmed.len() < 20 {
        return Err(format!(
            "auth1_token 长度异常（{} 字符），请确认完整粘贴",
            token_trimmed.len()
        ));
    }

    let auth = DevinAuthService::new();
    let user_specified_org = org_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let auto_pick = auto_select_primary_org.unwrap_or(false);

    // Step 1: 用 auth1_token 换取 session_token
    let post_auth = auth
        .windsurf_post_auth(&token_trimmed, &user_specified_org)
        .await
        .map_err(|e| format!("auth1_token 无效或已过期：{}", e))?;

    let session_token = post_auth.session_token.clone();
    // 服务端若在响应中轮换了 auth1_token，以新值为准；否则沿用用户输入
    let effective_auth1_token = post_auth
        .auth1_token
        .clone()
        .unwrap_or_else(|| token_trimmed.clone());

    // Step 2: 反查 GetCurrentUser（构造完整的 Devin AuthContext，确保服务端所需头部齐全）
    let ctx = crate::services::AuthContext {
        token: session_token.clone(),
        devin: Some(crate::services::DevinAuthContext {
            account_id: post_auth.account_id.clone(),
            auth1_token: Some(effective_auth1_token.clone()),
            primary_org_id: post_auth.primary_org_id.clone(),
        }),
    };

    let windsurf_service = WindsurfService::new();
    let user_info_result = windsurf_service
        .get_current_user(&ctx)
        .await
        .map_err(|e| format!("反查账号信息失败（auth1_token 可能已失效）：{}", e))?;

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

    // Step 3: 多组织分支 — 未指定 org_id 且未开启自动选主 org 时，交给前端二次决策
    if user_specified_org.is_empty() && !auto_pick && post_auth.orgs.len() > 1 {
        return Ok(json!({
            "success": false,
            "requires_org_selection": true,
            "auth1_token": effective_auth1_token,
            "orgs": post_auth.orgs,
            "email": email,
            "message": "检测到多个组织，请选择一个继续"
        }));
    }

    // Step 4: 落库 —— 已存在检查（邮箱不区分大小写）
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

    // auth1_token 迁入场景无原始密码，password 字段留空
    let mut account = store
        .add_account(email.clone(), String::new(), final_nickname)
        .await
        .map_err(|e| e.to_string())?;

    account.tags = tags;
    account.group = group;
    account.status = crate::models::account::AccountStatus::Active;
    account.last_login_at = Some(chrono::Utc::now());
    account.token = Some(session_token);
    account.token_expires_at = Some(devin_session_pseudo_expires_at());
    account.auth_provider = Some("devin".to_string());

    // Devin 扩展字段齐全（区别于 session_token 迁入路径）
    account.devin_auth1_token = Some(effective_auth1_token);
    account.devin_account_id = post_auth.account_id.clone();
    account.devin_primary_org_id = post_auth.primary_org_id.clone().or_else(|| {
        if !user_specified_org.is_empty() {
            Some(user_specified_org.clone())
        } else {
            post_auth.orgs.first().map(|o| o.id.clone())
        }
    });

    // 复用已拿到的 user_info 回填配额 / 套餐 / api_key 等，避免重复拉取
    apply_user_info_to_account(&mut account, user_info);

    // 补拉 GetPlanStatus：回填 billing_strategy + daily/weekly_quota_remaining_percent
    // 等 QUOTA 模式专用字段，避免账号卡降级到 CREDITS 积分显示。
    // GetCurrentUser 已在上方完成，这里只多发一次 GetPlanStatus 网络请求。
    // 注意：account.token 已被 move 进 Option，不能在 &mut account 期间再借；
    // 这里复用上方构造的 ctx.token（与 session_token 同值，仍 owned 可借）。
    enrich_account_with_plan_status(&mut account, &ctx.token).await;

    store
        .update_account(account.clone())
        .await
        .map_err(|e| e.to_string())?;

    let log = OperationLog::new(
        OperationType::AddAccount,
        OperationStatus::Success,
        format!("通过 Devin auth1_token 添加账号: {}", email),
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
    // 与其它建账路径一致：初建时填 pseudo_expires_at（+32d），与刷新路径行为对齐
    account.token_expires_at = Some(devin_session_pseudo_expires_at());

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

// ==================== Firebase ↔ Devin 账号互转 ====================
//
// 背景：官方将部分老 Firebase 账号迁移到 Devin 体系（密码未变），
// 之前帐号卡的 refresh_token 刷新会失败。本模块提供两个手动切换命令，
// 复用账号已存的 email + password，一键完成登录方式切换：
//   - convert_account_to_devin：Firebase → Devin，走 /_devin-auth/password/login + WindsurfPostAuth，
//     成功后写入 devin_* 字段、清空 refresh_token、标记 auth_provider="devin"
//   - convert_account_to_firebase：Devin → Firebase，走 Firebase signInWithPassword，
//     成功后写入 id_token + refresh_token、清空 devin_* 字段、重置 auth_provider=None
//
// 设计要点：
//   - 幂等：目标体系与当前一致时直接返回 already_converted=true，不执行任何网络请求
//   - 原子性：所有网络调用在改动 account 字段前完成，中途失败原账号字段保持不变
//   - 多组织：Firebase→Devin 场景下若未传 org_id 且账号有 > 1 个 org，返回
//     requires_org_selection=true + orgs + email，令前端弹选择后再次调本命令并传入 org_id
//   - 富化：切换成功后调 GetCurrentUser + GetPlanStatus 补齐套餐/配额/api_key 等字段

/// 把 Firebase 账号转换为 Devin 登录方式
///
/// 前提：账号在服务端已被官方迁移到 Devin 体系，但本地帐号卡仍是
/// Firebase 配置（token=id_token, refresh_token 存在, auth_provider=None）。
/// 调用后使用账号已存的明文密码（经 store.get_decrypted_password 解密）走 Devin 登录流程，
/// 成功后将账号字段切换为 Devin 体系。
#[tauri::command]
pub async fn convert_account_to_devin(
    id: String,
    org_id: Option<String>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut account = store.get_account(uuid).await.map_err(|e| e.to_string())?;

    // 幂等：已是 Devin 账号，无需转换
    if account.is_devin_account() {
        return Ok(json!({
            "success": false,
            "already_converted": true,
            "message": "账号当前已是 Devin 登录体系，无需转换",
        }));
    }

    // 解密密码；password 空则转换无从谈起
    let password = store
        .get_decrypted_password(uuid)
        .await
        .map_err(|e| format!("读取账号密码失败: {}", e))?;
    if password.trim().is_empty() {
        return Err(
            "账号密码为空，无法转换。请先在「编辑账号」中补充密码后重试".to_string(),
        );
    }

    let email = account.email.clone();
    let user_specified_org = org_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_string();

    // 走完整 Devin 登录：password_login → windsurf_post_auth
    // login_with_password 内部已处理了新建账号的 404 no_eligible_organizations 退避重试逻辑
    let login_result = DevinAuthService::new()
        .login_with_password(
            &email,
            &password,
            if user_specified_org.is_empty() {
                None
            } else {
                Some(user_specified_org.as_str())
            },
        )
        .await
        .map_err(|e| format!("Devin 登录失败（密码可能已失效或服务端未迁移至 Devin 体系）: {}", e))?;

    // 多组织：未传 org_id 且账号下有多个 org 时，返回选择请求，不修改当前账号
    if login_result.requires_org_selection {
        return Ok(json!({
            "success": false,
            "requires_org_selection": true,
            "orgs": login_result.orgs,
            "email": email,
            "message": "账号关联多个组织，请选择一个继续转换",
        }));
    }

    // 原子写回：仅在 login 成功后替换账号字段
    account.token = Some(login_result.session_token.clone());
    account.token_expires_at = Some(devin_session_pseudo_expires_at());
    account.refresh_token = None; // Firebase 或通用 refresh_token 清掉
    account.devin_auth1_token = Some(login_result.auth1_token.clone());
    account.devin_account_id = login_result.account_id.clone();
    account.devin_primary_org_id = login_result.primary_org_id.clone();
    account.auth_provider = Some("devin".to_string());
    account.status = crate::models::account::AccountStatus::Active;
    account.last_login_at = Some(chrono::Utc::now());

    // 补拉 user_info + plan_status（enrich_account_with_user_info 内部已含两者）
    enrich_account_with_user_info(&mut account, &login_result.session_token).await;

    store
        .update_account(account.clone())
        .await
        .map_err(|e| e.to_string())?;

    let log = OperationLog::new(
        OperationType::EditAccount,
        OperationStatus::Success,
        format!("登录方式转换: Firebase → Devin（{}）", email),
    )
    .with_account(uuid, email.clone());
    let _ = store.add_log(log).await;

    Ok(json!({
        "success": true,
        "message": "已转换为 Devin 登录体系",
        "email": email,
        "account": account,
    }))
}

/// 把 Devin 账号转换为 Firebase 登录方式
///
/// 适用于：官方回调某些帐号到 Firebase 体系、或用户误将账号转为 Devin 后需要还原。
/// 前提：账号在服务端仍能用 Firebase signInWithPassword 登录。
#[tauri::command]
pub async fn convert_account_to_firebase(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut account = store.get_account(uuid).await.map_err(|e| e.to_string())?;

    // 幂等：已是 Firebase 账号（auth_provider 为 None 或非 "devin"）
    if !account.is_devin_account() {
        return Ok(json!({
            "success": false,
            "already_converted": true,
            "message": "账号当前已是 Firebase 登录体系，无需转换",
        }));
    }

    let password = store
        .get_decrypted_password(uuid)
        .await
        .map_err(|e| format!("读取账号密码失败: {}", e))?;
    if password.trim().is_empty() {
        return Err(
            "账号密码为空，无法转换。请先在「编辑账号」中补充密码后重试".to_string(),
        );
    }

    let email = account.email.clone();

    // Firebase 账密登录，只有登录成功后才修改账号字段
    let (token, refresh_token, expires_at) = AuthService::new()
        .sign_in(&email, &password)
        .await
        .map_err(|e| format!("Firebase 登录失败（密码可能已失效或服务端已迁移至 Devin 体系）: {}", e))?;

    // 原子写回：替换 token 类型，清理 Devin 专属字段
    account.token = Some(token.clone());
    account.token_expires_at = Some(expires_at);
    account.refresh_token = Some(refresh_token);
    account.devin_auth1_token = None;
    account.devin_account_id = None;
    account.devin_primary_org_id = None;
    account.auth_provider = None;
    account.status = crate::models::account::AccountStatus::Active;
    account.last_login_at = Some(chrono::Utc::now());

    // 补拉 user_info + plan_status（走 Firebase AuthContext）
    let ctx = AuthContext::firebase(token);
    let windsurf_service = WindsurfService::new();
    if let Ok(result) = windsurf_service.get_current_user(&ctx).await {
        if let Some(user_info) = result.get("user_info") {
            apply_user_info_to_account(&mut account, user_info);
        }
    }
    if let Ok(plan_result) = windsurf_service.get_plan_status(&ctx).await {
        if plan_result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            if let Some(plan_status) = plan_result.get("plan_status") {
                crate::commands::api_commands::apply_plan_status_to_account(
                    plan_status,
                    &mut account,
                );
            }
        }
    }

    store
        .update_account(account.clone())
        .await
        .map_err(|e| e.to_string())?;

    let log = OperationLog::new(
        OperationType::EditAccount,
        OperationStatus::Success,
        format!("登录方式转换: Devin → Firebase（{}）", email),
    )
    .with_account(uuid, email.clone());
    let _ = store.add_log(log).await;

    Ok(json!({
        "success": true,
        "message": "已转换为 Firebase 登录体系",
        "email": email,
        "account": account,
    }))
}
