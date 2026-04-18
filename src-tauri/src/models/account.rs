use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 带颜色的标签
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagWithColor {
    pub name: String,
    pub color: String, // RGBA格式，如 "rgba(255, 100, 100, 1)"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub email: String,
    pub password: String, // 加密后的密码
    pub nickname: String,
    pub tags: Vec<String>,
    #[serde(default, rename = "tagColors")]
    pub tag_colors: Vec<TagWithColor>, // 带颜色的标签
    pub group: Option<String>,
    pub token: Option<String>, // 加密后的Token
    pub refresh_token: Option<String>, // 加密后的Refresh Token
    pub token_expires_at: Option<DateTime<Utc>>,
    pub last_seat_count: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub status: AccountStatus,
    // 配额和套餐信息
    pub plan_name: Option<String>,
    pub used_quota: Option<i32>,
    pub total_quota: Option<i32>,
    pub last_quota_update: Option<DateTime<Utc>>,
    // 订阅到期时间
    pub subscription_expires_at: Option<DateTime<Utc>>,
    // 订阅是否激活 (从 GetCurrentUser API 的 subscription.subscription_active 获取)
    #[serde(default)]
    pub subscription_active: Option<bool>,
    // Windsurf API Key (用户的 UUID，从 GetCurrentUser API 获取)
    pub windsurf_api_key: Option<String>,
    // 账户是否被禁用 (从 GetCurrentUser API 的 user.disable_codeium 获取)
    #[serde(default)]
    pub is_disabled: Option<bool>,
    // 是否为团队所有者（Admin角色，有团队成员的主账号）
    #[serde(default)]
    pub is_team_owner: Option<bool>,
    // 计费策略 (0=UNSPECIFIED, 1=CREDITS, 2=QUOTA, 3=ACU)
    #[serde(default)]
    pub billing_strategy: Option<i32>,
    // 日配额剩余百分比 (0-100，仅 billing_strategy=2(QUOTA) 时有效)
    #[serde(default)]
    pub daily_quota_remaining_percent: Option<i32>,
    // 周配额剩余百分比 (0-100，仅 billing_strategy=2(QUOTA) 时有效)
    #[serde(default)]
    pub weekly_quota_remaining_percent: Option<i32>,
    // 日配额重置时间 (Unix时间戳秒)
    #[serde(default)]
    pub daily_quota_reset_at_unix: Option<i64>,
    // 周配额重置时间 (Unix时间戳秒)
    #[serde(default)]
    pub weekly_quota_reset_at_unix: Option<i64>,
    // 额外使用余额 (微美元，除以1e6得到美元)
    #[serde(default)]
    pub overage_balance_micros: Option<i64>,
    // 自定义排序顺序（用于拖拽排序）
    #[serde(default, rename = "sortOrder")]
    pub sort_order: i32,

    // ==================== Devin Session 认证字段 ====================
    // 当此账号通过 Devin 账密登录时，以下字段会被填充；
    // 此时 `token` 字段将存放 Devin session_token（与旧 Firebase id_token 共用 token 字段，实现下游 API 透明使用）

    /// Devin Auth1 Token（一级认证令牌，可用于再次换取 session_token）
    #[serde(default)]
    pub devin_auth1_token: Option<String>,
    /// Devin 账号 ID（格式：account-<32 字符十六进制>）
    #[serde(default)]
    pub devin_account_id: Option<String>,
    /// Devin 主组织 ID
    #[serde(default)]
    pub devin_primary_org_id: Option<String>,
    /// 认证提供方："firebase"（默认旧体系）或 "devin"（Devin Session 新体系）
    #[serde(default)]
    pub auth_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Active,
    Inactive,
    Error(String),
}

impl Account {
    pub fn new(email: String, password: String, nickname: String, tags: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            email,
            password,
            nickname,
            tags,
            tag_colors: Vec::new(),
            group: None,
            token: None,
            refresh_token: None,
            token_expires_at: None,
            last_seat_count: None,
            created_at: Utc::now(),
            last_login_at: None,
            status: AccountStatus::Inactive,
            plan_name: None,
            used_quota: None,
            total_quota: None,
            last_quota_update: None,
            subscription_expires_at: None,
            subscription_active: None,
            windsurf_api_key: None,
            is_disabled: None,
            is_team_owner: None,
            billing_strategy: None,
            daily_quota_remaining_percent: None,
            weekly_quota_remaining_percent: None,
            daily_quota_reset_at_unix: None,
            weekly_quota_reset_at_unix: None,
            overage_balance_micros: None,
            sort_order: 0,
            devin_auth1_token: None,
            devin_account_id: None,
            devin_primary_org_id: None,
            auth_provider: None,
        }
    }

    /// 判断账号是否通过 Devin Session 认证
    pub fn is_devin_account(&self) -> bool {
        matches!(self.auth_provider.as_deref(), Some("devin"))
    }

    pub fn is_token_valid(&self) -> bool {
        if let Some(expires_at) = self.token_expires_at {
            expires_at > Utc::now()
        } else {
            false
        }
    }
}
