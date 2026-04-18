//! Devin Session 认证服务
//!
//! 实现基于 Devin Session 的新认证体系，包括：
//! - `POST /_devin-auth/connections` — 查询邮箱可用登录方式
//! - `POST /_devin-auth/password/login` — 账号密码登录，返回 auth1_token
//! - `SeatManagementService/WindsurfPostAuth` — 用 auth1_token 换取 session_token
//!
//! 设计要点：
//! - 全程同源请求（windsurf.com），不涉及 Google API Key，因此不需要特殊的 Referer 处理
//! - 所有 REST 请求都附带 `Origin: https://windsurf.com` 和 `Referer: https://windsurf.com/account/login`
//!   以保持与浏览器行为一致，避免某些边缘 CSP 拒绝

use crate::utils::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Devin 账密登录的前置基础 URL（走 windsurf.com 的同源代理）
const DEVIN_AUTH_BASE_URL: &str = "https://windsurf.com/_devin-auth";
/// WindsurfPostAuth 走主后端
const WINDSURF_BACKEND_URL: &str = "https://web-backend.windsurf.com";

// ==================== 请求/响应结构 ====================

#[derive(Debug, Serialize)]
pub struct ConnectionsRequest {
    pub product: String,
    pub email: String,
}

/// `/_devin-auth/connections` 的响应（结构按观察推断，未知字段将被忽略）
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ConnectionsResponse {
    /// 可用的连接方式列表（原始 JSON 透传以容错字段变化）
    #[serde(flatten)]
    pub raw: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct PasswordLoginRequest {
    pub email: String,
    pub password: String,
}

/// `/_devin-auth/password/login` 的响应
///
/// 实测真实响应体示例：
/// ```json
/// {
///   "token": "auth1_yjk627jgorikbvs5u5r5ubsjszuuugu6uwvbecarhidxhmntl4zq",
///   "user_id": "user-5b148c75b9cb4bf2b48e1ae73ee13277",
///   "email": "aporter755@fnsdghm.shop"
/// }
/// ```
///
/// 字段别名同时兼容早期推测的命名（`auth1_token` / `account_id`）。
#[derive(Debug, Deserialize, Serialize)]
pub struct PasswordLoginResponse {
    /// 一级认证令牌（格式：`auth1_<52 字符随机>`）
    ///
    /// 真实 API 返回字段名为 `token`；此处保留 `auth1_token` 作为 Rust 侧主名，
    /// 并通过 alias 兼容 `token` / `auth1Token` / `auth_token`。
    #[serde(alias = "token", alias = "auth1Token", alias = "auth_token")]
    pub auth1_token: String,
    /// Devin 账号/用户 ID（真实返回字段为 `user_id`，格式：`user-<32 字符>`）
    #[serde(default, alias = "user_id", alias = "userId", alias = "accountId")]
    pub account_id: Option<String>,
    /// 返回的邮箱（服务端会回显，便于校验）
    #[serde(default)]
    pub email: Option<String>,
    /// 保留未知字段以备调试
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// `WindsurfPostAuth` 响应解析后的结构
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct WindsurfPostAuthResult {
    pub session_token: String,
    #[serde(default)]
    pub auth1_token: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub primary_org_id: Option<String>,
    #[serde(default)]
    pub orgs: Vec<WindsurfOrg>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct WindsurfOrg {
    pub id: String,
    pub name: String,
}

/// `CheckUserLoginMethod` 的响应解析结构
///
/// 来源：`exa.seat_management_pb.SeatManagementService/CheckUserLoginMethod`
///
/// Proto schema（字段号按官网前端 chunk 还原）：
/// - 1: `redirect_url` (string)      — 多区/安全全域重定向 URL，空表示无重定向
/// - 2: `disallow_enterprise_user_login` (bool) — 企业用户禁止普通登录
/// - 3: `user_exists` (bool)          — WS(Firebase) 侧该邮箱是否存在
/// - 4: `is_migrated` (bool)          — 是否已迁移到 Auth1
/// - 5: `has_password` (bool)         — 是否设过密码凭证
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CheckUserLoginMethodResult {
    #[serde(default)]
    pub redirect_url: String,
    #[serde(default)]
    pub disallow_enterprise_user_login: bool,
    #[serde(default)]
    pub user_exists: bool,
    #[serde(default)]
    pub is_migrated: bool,
    #[serde(default)]
    pub has_password: bool,
}

/// 登录流派崇探聚合结果
///
/// 由 [`DevinAuthService::sniff_login_method`] 生成，同时聚合两侧探测数据：
/// - Firebase(WS) 侧：`CheckUserLoginMethod`
/// - Devin 侧：  `/_devin-auth/connections`
///
/// `recommended` 字段取值与含义（与官网 reducer BOTH_CHECKS_DONE 对齐）：
/// - `"firebase"`    — 老 Firebase 账号 + 已设密码，走 `signInWithEmailAndPassword`
/// - `"devin"`       — 已迁移或新 Auth1 账号，走 Devin 账密登录
/// - `"sso"`         — 挂接企业 SSO 连接，必须在浏览器中完成 SSO 跳转
/// - `"no_password"` — 老账号仅用过 Google/GitHub，需用 OAuth 或先重置密码
/// - `"not_found"`   — 邮箱两侧都不存在，建议先注册
/// - `"blocked"`     — 企业用户被限制普通登录通道
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginMethodSniffResult {
    /// 建议的登录流派
    pub recommended: String,
    /// 面向人的理由说明（可直接展给 UI）
    pub reason: String,

    // ==== Firebase(WS) 侧原始判定 ====
    pub user_exists: bool,
    pub is_migrated: bool,
    pub has_password: bool,
    pub redirect_url: Option<String>,
    pub disallow_enterprise: bool,

    // ==== Devin 侧原始判定 ====
    /// Devin `/connections` 返回的原始 JSON，接口失败/邮箱不存在时为 `None`
    pub devin_connections: Option<serde_json::Value>,
    /// Devin 侧 `method` 字段：`"auth1"` | `"not_found"` | null
    pub devin_method: Option<String>,
    /// Devin 侧 `has_password` 字段
    pub devin_has_password: Option<bool>,
    /// Devin 侧 `sso_connections` 数组是否非空
    pub has_sso_connection: bool,
}

/// 完整登录流程的最终结果
#[derive(Debug, Serialize, Clone)]
pub struct DevinLoginResult {
    pub session_token: String,
    pub auth1_token: String,
    pub account_id: Option<String>,
    pub primary_org_id: Option<String>,
    pub orgs: Vec<WindsurfOrg>,
    /// 当返回多个组织时，需要用户二次选择再调用 windsurf_post_auth(auth1_token, chosen_org_id)
    pub requires_org_selection: bool,
}

// ==================== 邮箱注册 / 无密码登录 / 忘记密码 相关结构 ====================

/// `/email/start` 的请求
#[derive(Debug, Serialize)]
pub struct EmailStartRequest {
    pub email: String,
    /// `"signup"` | `"login"`
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
}

/// `/email/start` 与 `/password/reset-start` 的响应（两者结构一致）
///
/// 服务端会向 `email` 发送 6 位验证码，并返回 `email_verification_token`，
/// 由前端在后续 `/email/complete` 或 `/password/reset-complete` 步骤中回传。
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct EmailStartResponse {
    #[serde(default)]
    pub email_verification_token: String,
    /// 其它字段（如 `expires_at`、`max_attempts` 等）兜底透传以便调试
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// `/email/complete` 的请求
#[derive(Debug, Serialize)]
pub struct EmailCompleteRequest {
    pub email_verification_token: String,
    pub code: String,
    /// `"signup"`：注册；`"login"`：无密码邮件登录
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `/password/reset-start` 的请求
#[derive(Debug, Serialize)]
pub struct PasswordResetStartRequest {
    pub email: String,
    pub product: String,
}

/// `/password/reset-complete` 的请求
#[derive(Debug, Serialize)]
pub struct PasswordResetCompleteRequest {
    pub email_verification_token: String,
    pub code: String,
    pub new_password: String,
}

// ==================== 服务实现 ====================

pub struct DevinAuthService {
    client: Arc<reqwest::Client>,
}

impl DevinAuthService {
    pub fn new() -> Self {
        // Devin Auth 走 windsurf.com 同源代理，**不**经过 Google API Client
        // 所以这里使用全局普通 HTTP Client（支持用户代理配置但无 Google 限制）
        Self {
            client: super::get_http_client(),
        }
    }

    /// 为所有 REST 请求附加通用头（模拟浏览器行为）
    fn apply_common_headers(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder
            .header("Content-Type", "application/json")
            .header("Accept", "*/*")
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .header("Origin", "https://windsurf.com")
            .header("Referer", "https://windsurf.com/account/login")
            .header("Sec-Fetch-Dest", "empty")
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Site", "same-origin")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/147.0.0.0 Safari/537.36",
            )
    }

    /// 查询邮箱可用的连接方式（可选步骤，用于 UI 预判断）
    pub async fn check_connections(&self, email: &str) -> AppResult<ConnectionsResponse> {
        let url = format!("{}/connections", DEVIN_AUTH_BASE_URL);

        let body = ConnectionsRequest {
            product: "windsurf".to_string(),
            email: email.to_string(),
        };

        let response = Self::apply_common_headers(self.client.post(&url))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                super::report_request_failure();
                AppError::Network(format!("Devin connections 请求失败: {}", e))
            })?;

        super::report_request_success();

        let status = response.status();
        let text = response.text().await.map_err(|e| AppError::Network(e.to_string()))?;

        if !status.is_success() {
            return Err(AppError::Api(format!(
                "Devin connections 返回状态 {}: {}",
                status, text
            )));
        }

        let parsed: ConnectionsResponse = serde_json::from_str(&text)
            .unwrap_or_else(|_| ConnectionsResponse::default());

        Ok(parsed)
    }

    /// 使用邮箱密码登录，返回 `auth1_token`
    pub async fn password_login(
        &self,
        email: &str,
        password: &str,
    ) -> AppResult<PasswordLoginResponse> {
        let url = format!("{}/password/login", DEVIN_AUTH_BASE_URL);

        let body = PasswordLoginRequest {
            email: email.to_string(),
            password: password.to_string(),
        };

        let response = Self::apply_common_headers(self.client.post(&url))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                super::report_request_failure();
                AppError::Network(format!("Devin 登录请求失败: {}", e))
            })?;

        super::report_request_success();

        let status = response.status();
        let text = response.text().await.map_err(|e| AppError::Network(e.to_string()))?;

        if !status.is_success() {
            return Err(Self::parse_password_login_error(status.as_u16(), &text));
        }

        serde_json::from_str::<PasswordLoginResponse>(&text).map_err(|e| {
            AppError::Parse(format!(
                "解析 Devin 登录响应失败: {}. Body: {}",
                e, text
            ))
        })
    }

    /// 将常见的错误响应翻译为用户可读信息
    fn parse_password_login_error(status: u16, body: &str) -> AppError {
        let lower = body.to_lowercase();
        if lower.contains("invalid") && (lower.contains("password") || lower.contains("credentials"))
        {
            AppError::AuthFailed("邮箱或密码错误".to_string())
        } else if lower.contains("not found") || lower.contains("no such") {
            AppError::AuthFailed("该邮箱未注册 Devin 账号".to_string())
        } else if lower.contains("disabled") || lower.contains("suspended") {
            AppError::AuthFailed("账号已被禁用或暂停".to_string())
        } else if lower.contains("verify") && lower.contains("email") {
            AppError::AuthFailed("请先验证邮箱".to_string())
        } else if lower.contains("too many") || lower.contains("rate") || status == 429 {
            AppError::AuthFailed("尝试次数过多，请稍后再试".to_string())
        } else {
            AppError::AuthFailed(format!("Devin 登录失败({}): {}", status, body))
        }
    }

    /// 用 `auth1_token` (+ 可选 `org_id`) 换取最终 `session_token`
    ///
    /// 通过 gRPC-Web (application/proto) 调用 SeatManagementService/WindsurfPostAuth
    pub async fn windsurf_post_auth(
        &self,
        auth1_token: &str,
        org_id: &str,
    ) -> AppResult<WindsurfPostAuthResult> {
        let url = format!(
            "{}/exa.seat_management_pb.SeatManagementService/WindsurfPostAuth",
            WINDSURF_BACKEND_URL
        );

        // Protobuf 编码：field 1 (auth1_token) + field 2 (org_id)
        let mut body = Vec::with_capacity(auth1_token.len() + org_id.len() + 4);
        encode_proto_string_field(&mut body, 1, auth1_token);
        if !org_id.is_empty() {
            encode_proto_string_field(&mut body, 2, org_id);
        }

        let response = self
            .client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("origin", "https://windsurf.com")
            .header("referer", "https://windsurf.com/account/login")
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .send()
            .await
            .map_err(|e| {
                super::report_request_failure();
                AppError::Network(format!("WindsurfPostAuth 请求失败: {}", e))
            })?;

        super::report_request_success();

        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        if !status.is_success() {
            return Err(AppError::Api(format!(
                "WindsurfPostAuth 返回状态 {}: {}",
                status,
                String::from_utf8_lossy(&bytes)
            )));
        }

        parse_windsurf_post_auth_response(&bytes)
    }

    /// 一键完整登录：连通性检查 → 账密登录 → 换取 session_token
    ///
    /// 如果返回的 orgs 多于 1 个且调用方未预先指定 `org_id`，则返回的结果中
    /// `requires_org_selection = true`，需要 UI 二次调用 `windsurf_post_auth` 并传入选中的 org_id
    pub async fn login_with_password(
        &self,
        email: &str,
        password: &str,
        org_id: Option<&str>,
    ) -> AppResult<DevinLoginResult> {
        // Step 1: 账密登录，获取 auth1_token
        let login = self.password_login(email, password).await?;
        // Step 2: 用 auth1_token 换取 session_token 并构造 DevinLoginResult
        self.post_auth_to_login_result(login, org_id).await
    }

    // ==================== 登录流派崇探（方案 B 核心）====================

    /// 匿名调用 `CheckUserLoginMethod`，拿到 Firebase(WS) 侧对该邮箱的登录方式判断
    ///
    /// 端点：`POST https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/CheckUserLoginMethod`
    /// 协议：Connect-Web protobuf (binary)
    /// 请求体：field 1 = email (string)
    ///
    /// 不需任何 auth token，可任意邮箱查询。
    pub async fn check_user_login_method(
        &self,
        email: &str,
    ) -> AppResult<CheckUserLoginMethodResult> {
        let url = format!(
            "{}/exa.seat_management_pb.SeatManagementService/CheckUserLoginMethod",
            WINDSURF_BACKEND_URL
        );

        let mut body = Vec::with_capacity(email.len() + 4);
        encode_proto_string_field(&mut body, 1, email);

        let response = self
            .client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("origin", "https://windsurf.com")
            .header("referer", "https://windsurf.com/account/login")
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .send()
            .await
            .map_err(|e| {
                super::report_request_failure();
                AppError::Network(format!("CheckUserLoginMethod 请求失败: {}", e))
            })?;

        super::report_request_success();

        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        if !status.is_success() {
            return Err(AppError::Api(format!(
                "CheckUserLoginMethod 返回状态 {}: {}",
                status,
                String::from_utf8_lossy(&bytes)
            )));
        }

        parse_check_user_login_method_response(&bytes)
    }

    /// 聚合崇探：并发调 Firebase `CheckUserLoginMethod` + Devin `/_devin-auth/connections`，
    /// 给出统一登录流派建议。
    ///
    /// 决策优先级（完全对齐官网 reducer `BOTH_CHECKS_DONE`）：
    /// 1. `disallow_enterprise_user_login == true` → `blocked`
    /// 2. Devin 侧 `sso_connections[]` 非空 且 WS 侧 `!is_migrated` → `sso`
    /// 3. WS `user_exists && !is_migrated`:
    ///    - `has_password` → `firebase`
    ///    - `!has_password` → `no_password`
    /// 4. Devin `method == "auth1"` 或 WS `is_migrated` → `devin`
    /// 5. 两侧都不存在 → `not_found`
    ///
    /// Devin 侧不可用（网络异常 / 4xx）时仅基于 Firebase 侧判定，保证可用性。
    pub async fn sniff_login_method(&self, email: &str) -> AppResult<LoginMethodSniffResult> {
        // 两侧探测并发：Firebase 侧必须成功；Devin 侧允许失败
        let (ws_result, devin_result) =
            tokio::join!(self.check_user_login_method(email), self.check_connections(email));

        let ws = ws_result?;
        let connections = devin_result.ok();

        // 从 Devin connections 响应里抽出关键字段
        //
        // 官网前端实际取的是 `connections.auth_method` 子对象（见官网 chunk 1635：
        //   `let [e, n] = await Promise.all([et(r), Wk("windsurf", r)]);`
        //   `let o = n.auth_method ?? null;`
        //   `dispatch({ type: "BOTH_CHECKS_DONE", wsResult: e, devinResult: o })`
        // ）。所以 `method` / `has_password` / `sso_connections` 三个字段都挂在 `auth_method`
        // 子对象下。早期直接从 raw 顶层取会导致 Devin 侧永远读不到 → 被误判为 `not_found`。
        //
        // 兼容性兜底：若后端新版本把字段平铺到顶层，也能读到（先 auth_method 再 raw 顶层）。
        let (devin_method, devin_has_password, has_sso_connection, devin_raw) = match &connections
        {
            Some(conn) => {
                let raw = &conn.raw;
                let auth_method = raw.get("auth_method");

                let read_str = |key: &str| {
                    auth_method
                        .and_then(|v| v.get(key))
                        .or_else(|| raw.get(key))
                        .and_then(|v| v.as_str())
                        .map(String::from)
                };
                let read_bool = |key: &str| {
                    auth_method
                        .and_then(|v| v.get(key))
                        .or_else(|| raw.get(key))
                        .and_then(|v| v.as_bool())
                };
                let read_array_non_empty = |key: &str| -> bool {
                    auth_method
                        .and_then(|v| v.get(key))
                        .or_else(|| raw.get(key))
                        .and_then(|v| v.as_array())
                        .map(|arr| !arr.is_empty())
                        .unwrap_or(false)
                };

                let method = read_str("method");
                let has_password = read_bool("has_password");
                let sso_present = read_array_non_empty("sso_connections");
                (method, has_password, sso_present, Some(raw.clone()))
            }
            None => (None, None, false, None),
        };

        let (recommended, reason): (&str, String) = if ws.disallow_enterprise_user_login {
            (
                "blocked",
                "企业用户被限制普通登录，必须走 Devin Enterprise".to_string(),
            )
        } else if has_sso_connection && !ws.is_migrated {
            (
                "sso",
                "该邮箱绑定企业 SSO，请在浏览器完成 SSO 登录后再导入".to_string(),
            )
        } else if ws.user_exists && !ws.is_migrated {
            if ws.has_password {
                (
                    "firebase",
                    "老账号已设密码，走 Firebase 邮箱密码登录".to_string(),
                )
            } else {
                (
                    "no_password",
                    "老账号未设密码（仅 Google/GitHub），请先以 OAuth 登录或重置密码"
                        .to_string(),
                )
            }
        } else if ws.is_migrated || devin_method.as_deref() == Some("auth1") {
            (
                "devin",
                "已迁移或新 Auth1 账号，走 Devin 账密登录".to_string(),
            )
        } else if !ws.user_exists
            && (devin_method.is_none() || devin_method.as_deref() == Some("not_found"))
        {
            (
                "not_found",
                "该邮箱尚未注册，请先完成注册".to_string(),
            )
        } else {
            (
                "not_found",
                "无法自动判定登录方式，请手动选择模式".to_string(),
            )
        };

        Ok(LoginMethodSniffResult {
            recommended: recommended.to_string(),
            reason,
            user_exists: ws.user_exists,
            is_migrated: ws.is_migrated,
            has_password: ws.has_password,
            redirect_url: if ws.redirect_url.is_empty() {
                None
            } else {
                Some(ws.redirect_url)
            },
            disallow_enterprise: ws.disallow_enterprise_user_login,
            devin_connections: devin_raw,
            devin_method,
            devin_has_password,
            has_sso_connection,
        })
    }

    // ==================== 邮箱注册 / 无密码邮件登录 / 忘记密码 ====================

    /// 发送邮箱验证码（注册 / 无密码登录均复用，区别在 `mode`）
    ///
    /// - `mode == "signup"`：为新账号创建而发送验证码
    /// - `mode == "login"`：为无密码账号的邮件验证码登录而发送
    /// - `product`：默认 `Some("windsurf")`（大小写与网页端一致）
    ///
    /// 服务端会将 6 位数字验证码发至目标邮箱，并返回 `email_verification_token`，
    /// 供后续 `email_complete` 回传使用。
    pub async fn email_start(
        &self,
        email: &str,
        mode: &str,
        product: Option<&str>,
    ) -> AppResult<EmailStartResponse> {
        let url = format!("{}/email/start", DEVIN_AUTH_BASE_URL);

        let body = EmailStartRequest {
            email: email.to_string(),
            mode: mode.to_string(),
            product: product.map(|s| s.to_string()),
        };

        let response = Self::apply_common_headers(self.client.post(&url))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                super::report_request_failure();
                AppError::Network(format!("Devin email/start 请求失败: {}", e))
            })?;

        super::report_request_success();

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        if !status.is_success() {
            return Err(Self::parse_email_start_error(status.as_u16(), &text));
        }

        serde_json::from_str::<EmailStartResponse>(&text).map_err(|e| {
            AppError::Parse(format!(
                "解析 Devin email/start 响应失败: {}. Body: {}",
                e, text
            ))
        })
    }

    /// 提交验证码 + 可选凭证完成邮件流程
    ///
    /// - `mode == "signup"`：**需传** `password` + `name`，成功即创建新 Devin 账号
    /// - `mode == "login"`：**无需** `password` / `name`，用于无密码账号的邮件验证码登录
    ///
    /// 成功响应体结构与 `/password/login` 一致：`{ token (auth1_token), user_id, email }`
    pub async fn email_complete(
        &self,
        email_verification_token: &str,
        code: &str,
        mode: &str,
        password: Option<&str>,
        name: Option<&str>,
    ) -> AppResult<PasswordLoginResponse> {
        let url = format!("{}/email/complete", DEVIN_AUTH_BASE_URL);

        let body = EmailCompleteRequest {
            email_verification_token: email_verification_token.to_string(),
            code: code.to_string(),
            mode: mode.to_string(),
            password: password.map(|s| s.to_string()),
            name: name.map(|s| s.to_string()),
        };

        let response = Self::apply_common_headers(self.client.post(&url))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                super::report_request_failure();
                AppError::Network(format!("Devin email/complete 请求失败: {}", e))
            })?;

        super::report_request_success();

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        if !status.is_success() {
            return Err(Self::parse_email_complete_error(status.as_u16(), &text));
        }

        serde_json::from_str::<PasswordLoginResponse>(&text).map_err(|e| {
            AppError::Parse(format!(
                "解析 Devin email/complete 响应失败: {}. Body: {}",
                e, text
            ))
        })
    }

    /// 发起“忘记密码”流程：服务端向 `email` 发送重置验证码
    ///
    /// 响应体结构与 `/email/start` 一致：返回 `email_verification_token`。
    /// 部分服务端实现可能返回空 body（仅以 HTTP 200 表示成功），此时返回空 token 的响应体，
    /// 由调用方通过后续 `password_reset_complete` 的错误（如“invalid token”）感知并做对应处理。
    pub async fn password_reset_start(
        &self,
        email: &str,
        product: Option<&str>,
    ) -> AppResult<EmailStartResponse> {
        let url = format!("{}/password/reset-start", DEVIN_AUTH_BASE_URL);

        let body = PasswordResetStartRequest {
            email: email.to_string(),
            product: product.unwrap_or("Windsurf").to_string(),
        };

        let response = Self::apply_common_headers(self.client.post(&url))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                super::report_request_failure();
                AppError::Network(format!("Devin password/reset-start 请求失败: {}", e))
            })?;

        super::report_request_success();

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        if !status.is_success() {
            return Err(Self::parse_reset_error(status.as_u16(), &text));
        }

        if text.trim().is_empty() {
            return Ok(EmailStartResponse::default());
        }

        serde_json::from_str::<EmailStartResponse>(&text).map_err(|e| {
            AppError::Parse(format!(
                "解析 Devin password/reset-start 响应失败: {}. Body: {}",
                e, text
            ))
        })
    }

    /// 完成“忘记密码”流程：提交验证码 + 新密码
    pub async fn password_reset_complete(
        &self,
        email_verification_token: &str,
        code: &str,
        new_password: &str,
    ) -> AppResult<()> {
        let url = format!("{}/password/reset-complete", DEVIN_AUTH_BASE_URL);

        let body = PasswordResetCompleteRequest {
            email_verification_token: email_verification_token.to_string(),
            code: code.to_string(),
            new_password: new_password.to_string(),
        };

        let response = Self::apply_common_headers(self.client.post(&url))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                super::report_request_failure();
                AppError::Network(format!("Devin password/reset-complete 请求失败: {}", e))
            })?;

        super::report_request_success();

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        if !status.is_success() {
            return Err(Self::parse_reset_error(status.as_u16(), &text));
        }

        Ok(())
    }

    /// 注册流程组合：`/email/complete`(signup) → `WindsurfPostAuth` → `DevinLoginResult`
    ///
    /// 前置条件：调用方已通过 `email_start(email, "signup", ...)` 拿到 `email_verification_token`，
    /// 并引导用户在邮箱中读取验证码 `code`。
    pub async fn register_with_email_code(
        &self,
        email_verification_token: &str,
        code: &str,
        password: &str,
        name: &str,
        org_id: Option<&str>,
    ) -> AppResult<DevinLoginResult> {
        let complete = self
            .email_complete(
                email_verification_token,
                code,
                "signup",
                Some(password),
                Some(name),
            )
            .await?;
        self.post_auth_to_login_result(complete, org_id).await
    }

    /// 无密码邮件登录流程组合：`/email/complete`(login) → `WindsurfPostAuth` → `DevinLoginResult`
    pub async fn login_with_email_code(
        &self,
        email_verification_token: &str,
        code: &str,
        org_id: Option<&str>,
    ) -> AppResult<DevinLoginResult> {
        let complete = self
            .email_complete(email_verification_token, code, "login", None, None)
            .await?;
        self.post_auth_to_login_result(complete, org_id).await
    }

    // ==================== 内部共享 ====================

    /// 拿到 `PasswordLoginResponse`（即包含 `auth1_token` 的响应）后，
    /// 换取 session_token 并构造 `DevinLoginResult`。
    ///
    /// 被 `login_with_password` / `register_with_email_code` / `login_with_email_code` 共享，
    /// 避免 auth1→session_token 的合并逻辑重复实现。
    async fn post_auth_to_login_result(
        &self,
        login: PasswordLoginResponse,
        org_id: Option<&str>,
    ) -> AppResult<DevinLoginResult> {
        let post_auth = self
            .windsurf_post_auth(&login.auth1_token, org_id.unwrap_or(""))
            .await?;

        let final_auth1_token = post_auth
            .auth1_token
            .clone()
            .unwrap_or_else(|| login.auth1_token.clone());
        let final_account_id = post_auth.account_id.clone().or(login.account_id.clone());
        let requires_org_selection = org_id.is_none() && post_auth.orgs.len() > 1;

        Ok(DevinLoginResult {
            session_token: post_auth.session_token,
            auth1_token: final_auth1_token,
            account_id: final_account_id,
            primary_org_id: post_auth.primary_org_id,
            orgs: post_auth.orgs,
            requires_org_selection,
        })
    }

    // ==================== 错误翻译 ====================

    /// `/email/start` 错误翻译
    fn parse_email_start_error(status: u16, body: &str) -> AppError {
        let lower = body.to_lowercase();
        if lower.contains("already") && lower.contains("exist") {
            AppError::AuthFailed("该邮箱已注册，请直接登录".to_string())
        } else if lower.contains("invalid") && lower.contains("email") {
            AppError::AuthFailed("邮箱格式不正确".to_string())
        } else if lower.contains("disposable") || lower.contains("temporary") {
            AppError::AuthFailed("不支持临时 / 一次性邮箱".to_string())
        } else if lower.contains("too many") || lower.contains("rate") || status == 429 {
            AppError::AuthFailed("发送验证码过于频繁，请稍后再试".to_string())
        } else {
            AppError::AuthFailed(format!("Devin 发送验证码失败({}): {}", status, body))
        }
    }

    /// `/email/complete` 错误翻译
    fn parse_email_complete_error(status: u16, body: &str) -> AppError {
        let lower = body.to_lowercase();
        if lower.contains("invalid") && lower.contains("code") {
            AppError::AuthFailed("验证码错误".to_string())
        } else if lower.contains("expired") {
            AppError::AuthFailed("验证码已过期，请重新获取".to_string())
        } else if lower.contains("password")
            && (lower.contains("weak") || lower.contains("short") || lower.contains("strength"))
        {
            AppError::AuthFailed("密码强度不足".to_string())
        } else if lower.contains("already") && lower.contains("exist") {
            AppError::AuthFailed("该邮箱已注册".to_string())
        } else if lower.contains("too many") || lower.contains("rate") || status == 429 {
            AppError::AuthFailed("尝试次数过多，请稍后再试".to_string())
        } else {
            AppError::AuthFailed(format!("Devin 验证码提交失败({}): {}", status, body))
        }
    }

    /// `/password/reset-*` 错误翻译
    fn parse_reset_error(status: u16, body: &str) -> AppError {
        let lower = body.to_lowercase();
        if lower.contains("invalid") && lower.contains("code") {
            AppError::AuthFailed("验证码错误".to_string())
        } else if lower.contains("expired") {
            AppError::AuthFailed("验证码已过期，请重新获取".to_string())
        } else if lower.contains("not found") || lower.contains("no such") {
            AppError::AuthFailed("该邮箱未注册".to_string())
        } else if lower.contains("too many") || lower.contains("rate") || status == 429 {
            AppError::AuthFailed("操作过于频繁，请稍后再试".to_string())
        } else {
            AppError::AuthFailed(format!("Devin 密码重置失败({}): {}", status, body))
        }
    }
}

impl Default for DevinAuthService {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Protobuf 编解码（手写，保持零依赖） ====================

/// 将 varint 编码后追加到 buffer
fn encode_varint(buf: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        buf.push((value as u8 & 0x7F) | 0x80);
        value >>= 7;
    }
    buf.push(value as u8);
}

/// 编码 protobuf `string` 字段（wire type 2 = length-delimited）
fn encode_proto_string_field(buf: &mut Vec<u8>, field_no: u32, value: &str) {
    let tag = (field_no << 3) | 2;
    encode_varint(buf, tag as u64);
    let bytes = value.as_bytes();
    encode_varint(buf, bytes.len() as u64);
    buf.extend_from_slice(bytes);
}

/// 解码 varint（返回值 + 消耗字节数）
fn decode_varint(bytes: &[u8], offset: usize) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0;
    let mut i = offset;
    while i < bytes.len() {
        let b = bytes[i];
        result |= ((b & 0x7F) as u64) << shift;
        i += 1;
        if b & 0x80 == 0 {
            return Some((result, i - offset));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}

/// 解析 WindsurfPostAuthResponse
///
/// Proto 结构:
/// - field 1: session_token (string)
/// - field 2: repeated WindsurfPostAuthOrg orgs (message)
///   - WindsurfPostAuthOrg.field 1: id (string)
///   - WindsurfPostAuthOrg.field 2: name (string)
/// - field 3: optional auth1_token (string)
/// - field 4: optional account_id (string)
/// - field 5: optional primary_org_id (string)
fn parse_windsurf_post_auth_response(bytes: &[u8]) -> AppResult<WindsurfPostAuthResult> {
    let mut result = WindsurfPostAuthResult::default();
    let mut i = 0;

    while i < bytes.len() {
        let (tag, consumed) = decode_varint(bytes, i).ok_or_else(|| {
            AppError::Parse("WindsurfPostAuth 响应 tag varint 解码失败".to_string())
        })?;
        i += consumed;

        let field_no = (tag >> 3) as u32;
        let wire_type = (tag & 0x7) as u8;

        // 仅处理 wire_type = 2（length-delimited），其他类型按长度跳过
        if wire_type == 2 {
            let (len, consumed_len) = decode_varint(bytes, i).ok_or_else(|| {
                AppError::Parse("WindsurfPostAuth 响应 length varint 解码失败".to_string())
            })?;
            i += consumed_len;
            let end = i + len as usize;
            if end > bytes.len() {
                return Err(AppError::Parse(format!(
                    "WindsurfPostAuth 响应长度越界: field={} len={} i={} total={}",
                    field_no,
                    len,
                    i,
                    bytes.len()
                )));
            }
            let payload = &bytes[i..end];
            match field_no {
                1 => {
                    result.session_token =
                        String::from_utf8_lossy(payload).into_owned();
                }
                2 => {
                    if let Some(org) = parse_windsurf_org(payload) {
                        result.orgs.push(org);
                    }
                }
                3 => {
                    result.auth1_token =
                        Some(String::from_utf8_lossy(payload).into_owned());
                }
                4 => {
                    result.account_id =
                        Some(String::from_utf8_lossy(payload).into_owned());
                }
                5 => {
                    result.primary_org_id =
                        Some(String::from_utf8_lossy(payload).into_owned());
                }
                _ => {} // 未知字段忽略
            }
            i = end;
        } else {
            // 其他 wire type 暂不需要，按最小方式跳过
            match wire_type {
                0 => {
                    // varint，跳过
                    let (_, c) = decode_varint(bytes, i).ok_or_else(|| {
                        AppError::Parse("跳过 varint 失败".to_string())
                    })?;
                    i += c;
                }
                1 => i += 8, // 64-bit
                5 => i += 4, // 32-bit
                _ => {
                    return Err(AppError::Parse(format!(
                        "WindsurfPostAuth 响应出现不支持的 wire_type {}",
                        wire_type
                    )));
                }
            }
        }
    }

    if result.session_token.is_empty() {
        return Err(AppError::Api(
            "WindsurfPostAuth 响应未包含 session_token".to_string(),
        ));
    }

    Ok(result)
}

/// 解析 CheckUserLoginMethodResponse
///
/// Proto 结构：
/// - field 1: redirect_url (string, wire_type=2)
/// - field 2: disallow_enterprise_user_login (bool → varint, wire_type=0)
/// - field 3: user_exists (bool)
/// - field 4: is_migrated (bool)
/// - field 5: has_password (bool)
///
/// 未知 field 按 wire_type 跳过，保证前向兼容。
fn parse_check_user_login_method_response(
    bytes: &[u8],
) -> AppResult<CheckUserLoginMethodResult> {
    let mut result = CheckUserLoginMethodResult::default();
    let mut i = 0;

    while i < bytes.len() {
        let (tag, consumed) = decode_varint(bytes, i).ok_or_else(|| {
            AppError::Parse("CheckUserLoginMethod 响应 tag varint 解码失败".to_string())
        })?;
        i += consumed;

        let field_no = (tag >> 3) as u32;
        let wire_type = (tag & 0x7) as u8;

        match (field_no, wire_type) {
            (1, 2) => {
                let (len, cl) = decode_varint(bytes, i).ok_or_else(|| {
                    AppError::Parse(
                        "CheckUserLoginMethod redirect_url length varint 解码失败".to_string(),
                    )
                })?;
                i += cl;
                let end = i + len as usize;
                if end > bytes.len() {
                    return Err(AppError::Parse(
                        "CheckUserLoginMethod redirect_url 长度越界".to_string(),
                    ));
                }
                result.redirect_url = String::from_utf8_lossy(&bytes[i..end]).into_owned();
                i = end;
            }
            (2, 0) | (3, 0) | (4, 0) | (5, 0) => {
                let (v, c) = decode_varint(bytes, i).ok_or_else(|| {
                    AppError::Parse(format!(
                        "CheckUserLoginMethod field {} varint 解码失败",
                        field_no
                    ))
                })?;
                i += c;
                match field_no {
                    2 => result.disallow_enterprise_user_login = v != 0,
                    3 => result.user_exists = v != 0,
                    4 => result.is_migrated = v != 0,
                    5 => result.has_password = v != 0,
                    _ => {}
                }
            }
            _ => {
                // 未知字段按 wire_type 跳过，保证前向兼容
                match wire_type {
                    0 => {
                        let (_, c) = decode_varint(bytes, i).ok_or_else(|| {
                            AppError::Parse("跳过 varint 失败".to_string())
                        })?;
                        i += c;
                    }
                    2 => {
                        let (len, cl) = decode_varint(bytes, i).ok_or_else(|| {
                            AppError::Parse("跳过 length-delimited 失败".to_string())
                        })?;
                        i += cl + len as usize;
                    }
                    1 => i += 8,
                    5 => i += 4,
                    _ => {
                        return Err(AppError::Parse(format!(
                            "CheckUserLoginMethod 未支持的 wire_type {}",
                            wire_type
                        )));
                    }
                }
            }
        }
    }

    Ok(result)
}

/// 解析 WindsurfPostAuthOrg 子消息（field 1=id, field 2=name）
fn parse_windsurf_org(bytes: &[u8]) -> Option<WindsurfOrg> {
    let mut org = WindsurfOrg::default();
    let mut i = 0;

    while i < bytes.len() {
        let (tag, consumed) = decode_varint(bytes, i)?;
        i += consumed;
        let field_no = (tag >> 3) as u32;
        let wire_type = (tag & 0x7) as u8;
        if wire_type != 2 {
            // 当前结构只使用字符串字段，其它类型直接中止
            return None;
        }
        let (len, consumed_len) = decode_varint(bytes, i)?;
        i += consumed_len;
        let end = i + len as usize;
        if end > bytes.len() {
            return None;
        }
        let payload = &bytes[i..end];
        match field_no {
            1 => org.id = String::from_utf8_lossy(payload).into_owned(),
            2 => org.name = String::from_utf8_lossy(payload).into_owned(),
            _ => {}
        }
        i = end;
    }

    if org.id.is_empty() && org.name.is_empty() {
        None
    } else {
        Some(org)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_roundtrip() {
        let mut buf = Vec::new();
        encode_varint(&mut buf, 150);
        assert_eq!(buf, vec![0x96, 0x01]);

        let (v, consumed) = decode_varint(&buf, 0).unwrap();
        assert_eq!(v, 150);
        assert_eq!(consumed, 2);
    }

    #[test]
    fn test_string_field_encoding() {
        let mut buf = Vec::new();
        encode_proto_string_field(&mut buf, 1, "auth1_test");
        // tag = (1 << 3) | 2 = 10 = 0x0a
        assert_eq!(buf[0], 0x0a);
        // length = 10
        assert_eq!(buf[1], 10);
        assert_eq!(&buf[2..], b"auth1_test");
    }

    #[test]
    fn test_parse_har_auth1_token_field_encoding() {
        // 复现 HAR 中捕获的 body: "\n:auth1_4phbs32swt5f66xsq26dr4eiziz6ahm3cj6n3ehd7mx6huj4zy5a"
        let auth1_token = "auth1_4phbs32swt5f66xsq26dr4eiziz6ahm3cj6n3ehd7mx6huj4zy5a";
        let mut buf = Vec::new();
        encode_proto_string_field(&mut buf, 1, auth1_token);
        assert_eq!(buf[0], 0x0a, "field 1 tag");
        assert_eq!(buf[1], 0x3a, "length=58 (':' = 0x3A)");
        assert_eq!(&buf[2..], auth1_token.as_bytes());
    }
}
