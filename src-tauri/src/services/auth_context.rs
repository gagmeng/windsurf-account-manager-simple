//! 统一认证上下文（AuthContext）
//!
//! 设计背景：
//! - Firebase 账号仅需 `x-auth-token` 一个 header（值 = Firebase idToken）。
//! - Devin 账号需要 5 个 header：`x-auth-token`、`x-devin-session-token`、
//!   `x-devin-account-id`、`x-devin-auth1-token`、`x-devin-primary-org-id`，
//!   其中 `x-auth-token` 与 `x-devin-session-token` 值均为带 `devin-session-token$`
//!   前缀的 session_token（后端直接返回该形态，无需应用层拼接）。
//!
//! 本模块提供一次性、类型安全的认证上下文抽象，供 `WindsurfService` 所有 gRPC
//! 方法消费。Firebase 与 Devin 分支在请求 header 构造层自动分流，业务方法内部
//! 不再需要区分 auth_provider。
//!
//! Quick-fail 原则：构造 `AuthContext::from_account` 时仅校验主 `token` 字段；
//! Devin 账号的 3 个扩展字段（account_id / auth1_token / primary_org_id）改为可选，
//! 以支持"仅 session_token 迁入"的场景（用户从浏览器直接拷贝 `devin-session-token$...`
//! 即可建号，此时扩展字段暂缺，后续如需 refresh_devin_session 可按需补齐）。
//!
//! header 注入层 `with_auth` 按字段存在性条件化写入对应 `x-devin-*` header，
//! 仅 session_token 的账号只发送 `x-auth-token` + `x-devin-session-token` 两个 header，
//! 与官网前端 `createDevinAuth1TokenInterceptor` 在 localStorage 缺失时的行为一致。

use crate::models::account::Account;

/// Devin 账号专属的扩展认证信息
///
/// 所有字段都是 gRPC 请求 header 的原样值，不做任何再加工。
/// 字段全部可选，以支持"仅 session_token 迁入"的场景——此时只发送主 `x-auth-token`
/// 和 `x-devin-session-token`，其他 3 个 `x-devin-*` header 缺省不发。
#[derive(Debug, Clone, Default)]
pub struct DevinAuthContext {
    /// `x-devin-account-id` header 值（Devin 账号 ID，形如 `account-<32 hex>`）
    pub account_id: Option<String>,
    /// `x-devin-auth1-token` header 值（Auth1 一级令牌，用于 refresh 会话）
    pub auth1_token: Option<String>,
    /// `x-devin-primary-org-id` header 值（Devin 主组织 ID）
    pub primary_org_id: Option<String>,
}

/// Windsurf gRPC 请求的认证上下文
///
/// 统一承载 `x-auth-token` 与可选的 Devin 扩展 header 集合。
/// 由 `WindsurfService::apply_auth_headers` 在请求构造阶段消费。
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// 主认证令牌：
    /// - Firebase 账号：Firebase idToken
    /// - Devin 账号：带 `devin-session-token$` 前缀的 session_token
    pub token: String,
    /// Devin 账号额外 header 集合；Firebase 账号为 `None`
    pub devin: Option<DevinAuthContext>,
}

impl AuthContext {
    /// 从裸 Firebase idToken 构造（Firebase 账号专用，或需要手动控制时）
    pub fn firebase(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            devin: None,
        }
    }

    /// 从 Account 构造，按 `auth_provider` 分流
    ///
    /// - Firebase 账号：仅要求 `token` 字段存在
    /// - Devin 账号：仅要求 `token`（即 session_token）；其余 3 个 Devin 扩展字段
    ///   （account_id / auth1_token / primary_org_id）均为**可选**。缺失时仅不发送对应
    ///   `x-devin-*` header，日常 API（GetCurrentUser、GetPlanStatus 等 session_token 驱动的
    ///   接口）仍可工作；仅 refresh_devin_session 等显式依赖 auth1_token 的操作会失败。
    pub fn from_account(account: &Account) -> Result<Self, String> {
        let token = account
            .token
            .clone()
            .ok_or_else(|| "账号缺少 token".to_string())?;

        if account.is_devin_account() {
            Ok(Self {
                token,
                devin: Some(DevinAuthContext {
                    account_id: account.devin_account_id.clone(),
                    auth1_token: account.devin_auth1_token.clone(),
                    primary_org_id: account.devin_primary_org_id.clone(),
                }),
            })
        } else {
            Ok(Self::firebase(token))
        }
    }

    /// 从一个纯 Devin `session_token` 直接构造（仅 session_token 迁入路径专用）
    ///
    /// `token` 预期已带 `devin-session-token$` 前缀。
    /// 3 个扩展字段均留空，`with_auth` 仅发 `x-auth-token` + `x-devin-session-token` 两个 header。
    pub fn devin_session_only(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            devin: Some(DevinAuthContext::default()),
        }
    }

    /// 便捷只读：判断当前上下文是否为 Devin 账号
    pub fn is_devin(&self) -> bool {
        self.devin.is_some()
    }

    /// 便捷只读：返回主认证令牌的 `&str` 视图，
    /// 保留以兼容 protobuf body 构造阶段大量使用 `&str` 的代码路径
    pub fn token_str(&self) -> &str {
        &self.token
    }
}

/// 为 `reqwest::RequestBuilder` 提供统一的认证 header 注入扩展
///
/// 使用方式：在现有 chain 中把
/// ```ignore
/// .header("x-auth-token", token)
/// ```
/// 替换为
/// ```ignore
/// .with_auth(ctx)
/// ```
/// 即可自动：
/// 1. 写入 `x-auth-token` 主 header
/// 2. 若为 Devin 账号，追加 `x-devin-account-id`、`x-devin-auth1-token`、
///    `x-devin-primary-org-id`、`x-devin-session-token` 4 个扩展 header
pub trait AuthHeaderExt {
    /// 依据 `AuthContext` 自动写入认证 header 族
    fn with_auth(self, ctx: &AuthContext) -> Self;
}

impl AuthHeaderExt for reqwest::RequestBuilder {
    fn with_auth(self, ctx: &AuthContext) -> Self {
        let mut req = self.header("x-auth-token", &ctx.token);
        if let Some(devin) = &ctx.devin {
            // x-devin-session-token 始终发送（值等于 ctx.token，即主 session_token）
            req = req.header("x-devin-session-token", &ctx.token);
            // 其余 3 个扩展 header 仅在字段存在时发送——对齐官网前端
            // `createDevinAuth1TokenInterceptor` 在 localStorage 缺失时的行为
            if let Some(account_id) = &devin.account_id {
                req = req.header("x-devin-account-id", account_id);
            }
            if let Some(auth1_token) = &devin.auth1_token {
                req = req.header("x-devin-auth1-token", auth1_token);
            }
            if let Some(primary_org_id) = &devin.primary_org_id {
                req = req.header("x-devin-primary-org-id", primary_org_id);
            }
        }
        req
    }
}
