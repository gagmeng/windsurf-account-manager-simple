//! 外部 HTTP API 服务（仅内部使用）
//!
//! 启停由 Settings.api_enabled 控制，监听地址/端口由 Settings.api_host / Settings.api_port 配置。
//! 复用 commands 层的业务逻辑（DataStore + ensure_valid_token + AuthContext）暴露 REST 端点
//! 给同机其它工具调用。
//!
//! 路由（v1）：
//! - GET /api/v1/health
//! - GET /api/v1/accounts                              # 分页 + 过滤（query 参数对应 AccountPageRequest）
//! - GET /api/v1/accounts/:id                          # 单条详情（含所有字段）
//! - GET /api/v1/accounts/:id/token?auto_refresh=true  # 取有效 access_token
//! - GET /api/v1/groups
//! - GET /api/v1/stats                                 # 等同 AccountAggregates
//!
//! 关于敏感字段：
//! - 仅供同机内部工具使用，所有字段（包括 password / token / refresh_token /
//!   devin_auth1_token）原样返回，**不做脱敏**。
//! - /accounts/:id/token 仍保留：会触发过期自动刷新逻辑，并以更友好的字段名返回。

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
// 用 axum_extra::Query 而非 axum::Query：后者的 serde_urlencoded 不支持 Vec<T>，
// 导致 ?plan_names[]=TRIAL / ?statuses[]=normal 这类数组参数被静默忽略。
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use crate::commands::ensure_valid_token;
use crate::repository::{AccountPageRequest, DataStore};
use crate::services::AuthContext;

// ==================== ApiServerHandle / 生命周期管理 ====================

/// API 服务运行句柄。Settings 变化时由 settings_commands 触发停止 + 重启。
pub struct ApiServerHandle {
    shutdown_tx: oneshot::Sender<()>,
    join: JoinHandle<()>,
    pub host: String,
    pub port: u16,
}

impl ApiServerHandle {
    /// 优雅停止并等待任务结束
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.join.await;
    }
}

/// 启动 API 服务。失败原因主要为：地址/端口非法、端口被占用。
///
/// 不做任何鉴权 / DNS rebind 检查 / CORS 限制（仅内部使用）。
pub async fn start_server(
    store: Arc<DataStore>,
    host: String,
    port: u16,
) -> Result<ApiServerHandle, String> {
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| format!("无效的监听地址 {}:{} - {}", host, port, e))?;

    let state = ApiState { store };
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("绑定 {} 失败：{}", addr, e))?;

    println!("[API] 外部 HTTP API 服务已启动：http://{}", addr);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let host_for_handle = host.clone();
    let join: JoinHandle<()> = tokio::spawn(async move {
        let serve = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
                println!("[API] 收到关闭信号，正在停止 HTTP 服务...");
            });
        if let Err(e) = serve.await {
            eprintln!("[API] HTTP 服务异常退出：{}", e);
        } else {
            println!("[API] HTTP 服务已停止");
        }
    });

    Ok(ApiServerHandle {
        shutdown_tx,
        join,
        host: host_for_handle,
        port,
    })
}

// ==================== State / Router ====================

#[derive(Clone)]
struct ApiState {
    store: Arc<DataStore>,
}

fn build_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/accounts", get(list_accounts))
        .route("/api/v1/accounts/:id", get(get_account_detail))
        .route("/api/v1/accounts/:id/token", get(get_account_token))
        .route("/api/v1/groups", get(list_groups))
        .route("/api/v1/stats", get(get_stats))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// ==================== Error ====================

/// 统一错误类型；映射成 4xx/5xx + JSON body
#[derive(Debug)]
enum ApiError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        let body = Json(serde_json::json!({
            "error": {
                "code": status.as_u16(),
                "message": message,
            }
        }));
        (status, body).into_response()
    }
}

// ==================== Handlers ====================

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    version: &'static str,
    name: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
        name: env!("CARGO_PKG_NAME"),
    })
}

async fn list_accounts(
    State(state): State<ApiState>,
    Query(req): Query<AccountPageRequest>,
) -> Result<Json<crate::repository::AccountPageResponse>, ApiError> {
    state
        .store
        .account_store
        .get_accounts_page(&req)
        .map(Json)
        .map_err(|e| ApiError::Internal(format!("分页查询失败：{}", e)))
}

async fn get_account_detail(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<crate::models::Account>, ApiError> {
    let uuid = Uuid::parse_str(&id).map_err(|e| ApiError::BadRequest(format!("无效 UUID：{}", e)))?;
    state
        .store
        .get_account(uuid)
        .await
        .map(Json)
        .map_err(|e| ApiError::NotFound(format!("账号不存在：{}", e)))
}

#[derive(Deserialize, Default)]
struct TokenQuery {
    /// token 过期/不存在时是否自动刷新，默认 true
    #[serde(default = "default_true")]
    auto_refresh: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
struct TokenResponse {
    /// 主认证令牌（Firebase idToken 或带前缀的 Devin session_token）
    token: String,
    /// token 过期时间（ISO 8601）；可能为空（Devin 体系无显式过期）
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 是否为 Devin 体系账号
    is_devin: bool,
}

async fn get_account_token(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Query(query): Query<TokenQuery>,
) -> Result<Json<TokenResponse>, ApiError> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| ApiError::BadRequest(format!("无效 UUID：{}", e)))?;
    let mut account = state
        .store
        .get_account(uuid)
        .await
        .map_err(|e| ApiError::NotFound(format!("账号不存在：{}", e)))?;

    if query.auto_refresh {
        // 复用 commands::api_commands 的 token 刷新逻辑（过期时自动刷新并写回 store）
        ensure_valid_token(&state.store, &mut account, uuid)
            .await
            .map_err(ApiError::Internal)?;
    }

    let ctx = AuthContext::from_account(&account)
        .map_err(|e| ApiError::Internal(format!("构造 AuthContext 失败：{}", e)))?;

    Ok(Json(TokenResponse {
        token: ctx.token,
        expires_at: account.token_expires_at,
        is_devin: account.is_devin_account(),
    }))
}

async fn list_groups(State(state): State<ApiState>) -> Result<Json<Vec<String>>, ApiError> {
    let groups = state
        .store
        .get_groups()
        .await
        .map_err(|e| ApiError::Internal(format!("读取分组失败：{}", e)))?;
    Ok(Json(groups))
}

async fn get_stats(
    State(state): State<ApiState>,
) -> Result<Json<crate::repository::AccountAggregates>, ApiError> {
    let agg = state
        .store
        .account_store
        .get_aggregates()
        .map_err(|e| ApiError::Internal(format!("聚合统计失败：{}", e)))?;
    Ok(Json(agg))
}
