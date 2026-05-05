use crate::models::{Account, AppConfig, OperationLog};
use crate::repository::sqlite_account_store::SqliteAccountStore;
use crate::utils::{AppError, AppResult};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;
use tauri::{Manager, Emitter};
use chrono::Local;
use serde::Serialize;

/// Token 刷新事件负载
#[derive(Clone, Serialize)]
pub struct TokenRefreshedPayload {
    pub account_id: String,
    pub token: String,
    pub token_expires_at: String,
}

pub struct DataStore {
    pub config: Arc<RwLock<AppConfig>>,
    config_path: PathBuf,
    pub logs: Arc<RwLock<Vec<OperationLog>>>,
    logs_path: PathBuf,

    // ========== Save Coalescer（写盘合并器，v1.7.8，与主端同步） ==========
    //
    // 用于批量场景的写盘合并：多个并发 `request_save_coalesced()` 调用会被汇聚
    // 成 1~2 次实际 atomic_write，将批量导入/批量更新等场景的 IO 次数从 O(N) 降到 O(1~2)。
    //
    // 语义：
    // - `request_save_coalesced()`：非阻塞，登记脏标记；第一个调用者 spawn worker，
    //   后续调用直接返回（CAS 合并）。worker 抢到 lock 后清零脏标记并执行一次 save。
    // - `flush_pending_saves()`：阻塞，抢占所有 lock 并检查脏标记，有脏就立即同步落盘。
    //   仅用于 app exit hook，保证 crash 安全。
    // - `save()` / `save_logs()`：语义不变，仍然是立即同步落盘，供需要强一致性的路径使用。
    save_pending: Arc<AtomicBool>,
    save_logs_pending: Arc<AtomicBool>,
    save_coalesce_lock: Arc<Mutex<()>>,
    save_logs_coalesce_lock: Arc<Mutex<()>>,
    app_handle: tauri::AppHandle,

    /// **v1.7.8 方案 B SQLite 重构**：账号存储层。
    /// 替代原 `AppConfig.accounts: Vec<Account>` + JSON 全量落盘。
    pub account_store: Arc<SqliteAccountStore>,
}

impl DataStore {
    pub fn new(app_handle: &tauri::AppHandle) -> AppResult<Self> {
        let app_data_dir = app_handle.path().app_data_dir()
            .map_err(|e| AppError::Config(format!("Failed to get app data dir: {}", e)))?;
        
        // 确保目录存在
        fs::create_dir_all(&app_data_dir)?;
        
        let config_path = app_data_dir.join("accounts.json");
        let mut config = Self::load_config(&config_path)?;
        
        let logs_path = app_data_dir.join("logs.json");
        let mut logs = Self::load_logs(&logs_path)?;
        
        // 迁移旧的日志数据
        if !config.logs.is_empty() && logs.is_empty() {
            logs = config.logs.clone();
            config.logs.clear();
            
            // 保存迁移后的数据
            let logs_data = serde_json::to_string_pretty(&logs)?;
            fs::write(&logs_path, logs_data)?;
            
            let config_data = serde_json::to_string_pretty(&config)?;
            fs::write(&config_path, config_data)?;
        }
        
        // **v1.7.8 方案 B**：初始化 SQLite 账号存储
        let db_path = app_data_dir.join("accounts.db");
        let account_store = Arc::new(SqliteAccountStore::open(&db_path)?);

        // 自动迁移：accounts.json 中的账号 → SQLite（仅在 SQLite 为空且 JSON 有数据时执行）
        if !config.accounts.is_empty() && account_store.count().unwrap_or(0) == 0 {
            let migrated = config.accounts.len();
            match account_store.bulk_insert(&config.accounts) {
                Ok(count) => {
                    println!("[DataStore] 迁移 accounts.json → SQLite: {}/{} 条成功", count, migrated);
                    config.accounts.clear();
                    let config_data = serde_json::to_string_pretty(&config)?;
                    fs::write(&config_path, config_data)?;
                }
                Err(e) => {
                    eprintln!("[DataStore] 迁移失败（数据仍在 accounts.json，下次启动重试）: {}", e);
                }
            }
        }

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
            logs: Arc::new(RwLock::new(logs)),
            logs_path,
            save_pending: Arc::new(AtomicBool::new(false)),
            save_logs_pending: Arc::new(AtomicBool::new(false)),
            save_coalesce_lock: Arc::new(Mutex::new(())),
            save_logs_coalesce_lock: Arc::new(Mutex::new(())),
            app_handle: app_handle.clone(),
            account_store,
        })
    }

    fn load_config(path: &PathBuf) -> AppResult<AppConfig> {
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(data) => {
                    match serde_json::from_str(&data) {
                        Ok(config) => Ok(config),
                        Err(e) => {
                            // JSON 解析失败，尝试从备份恢复
                            println!("[DataStore] Config file corrupted: {}, trying backup...", e);
                            Self::recover_from_backup(path)
                        }
                    }
                }
                Err(e) => {
                    // 文件读取失败，尝试从备份恢复
                    println!("[DataStore] Failed to read config: {}, trying backup...", e);
                    Self::recover_from_backup(path)
                }
            }
        } else {
            Ok(AppConfig::default())
        }
    }
    
    /// 从备份文件恢复配置
    fn recover_from_backup(path: &PathBuf) -> AppResult<AppConfig> {
        let backup_path = path.with_extension("json.backup");
        
        if backup_path.exists() {
            println!("[DataStore] Found backup file, attempting recovery...");
            let data = fs::read_to_string(&backup_path)?;
            let config: AppConfig = serde_json::from_str(&data)?;
            
            // 恢复成功后，将备份复制回主文件
            fs::copy(&backup_path, path)?;
            println!("[DataStore] Successfully recovered from backup!");
            
            Ok(config)
        } else {
            println!("[DataStore] No backup found, using default config");
            Ok(AppConfig::default())
        }
    }
    
    fn load_logs(path: &PathBuf) -> AppResult<Vec<OperationLog>> {
        if path.exists() {
            let data = fs::read_to_string(path)?;
            let logs: Vec<OperationLog> = serde_json::from_str(&data)?;
            Ok(logs)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn save(&self) -> AppResult<()> {
        let config = self.config.read().await;
        let data = serde_json::to_string_pretty(&*config)?;
        let path = self.config_path.clone();
        drop(config); // 提前释放读锁
        
        // 使用 spawn_blocking 将同步文件写入移到阻塞线程池，避免阻塞 tokio 运行时
        tokio::task::spawn_blocking(move || {
            Self::atomic_write(&path, &data)
        }).await
            .map_err(|e| AppError::Config(format!("Task join error: {}", e)))?
            .map_err(AppError::from)?;
        
        Ok(())
    }
    
    /// 原子写入：先写临时文件，创建备份，再重命名
    fn atomic_write(path: &PathBuf, data: &str) -> std::io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // 使用时间戳+进程ID生成唯一临时文件名，避免并发冲突
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let pid = std::process::id();
        
        let file_stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data");
        let parent = path.parent().unwrap_or(path);
        
        let temp_path = parent.join(format!("{}.tmp.{}.{}", file_stem, pid, timestamp));
        let backup_path = path.with_extension("json.backup");
        
        // 1. 先写入临时文件
        fs::write(&temp_path, data)?;
        
        // 2. 验证临时文件可以正常解析
        let verify_data = fs::read_to_string(&temp_path)?;
        if serde_json::from_str::<serde_json::Value>(&verify_data).is_err() {
            let _ = fs::remove_file(&temp_path);
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Written data failed JSON validation"
            ));
        }
        
        // 3. 如果原文件存在，创建备份
        if path.exists() {
            // 复制到备份文件（覆盖旧备份）
            if let Err(e) = fs::copy(path, &backup_path) {
                let _ = fs::remove_file(&temp_path);
                return Err(e);
            }
        }
        
        // 4. 原子重命名临时文件为目标文件
        if let Err(e) = fs::rename(&temp_path, path) {
            // 重命名失败时清理临时文件
            let _ = fs::remove_file(&temp_path);
            return Err(e);
        }
        
        Ok(())
    }
    
    pub async fn save_logs(&self) -> AppResult<()> {
        let logs = self.logs.read().await;
        let data = serde_json::to_string_pretty(&*logs)?;
        let path = self.logs_path.clone();
        drop(logs); // 提前释放读锁
        
        // 使用 spawn_blocking 将同步文件写入移到阻塞线程池，避免阻塞 tokio 运行时
        tokio::task::spawn_blocking(move || {
            fs::write(&path, data)
        }).await
            .map_err(|e| AppError::Config(format!("Task join error: {}", e)))?
            .map_err(AppError::from)?;
        
        Ok(())
    }

    // ========== Save Coalescer 实现（v1.7.8，与主端同步） ==========

    /// 请求合并写盘（非阻塞）：多次并发调用会被 CAS 合并为最多 1~2 次实际 save。
    ///
    /// 批量场景（批量导入等）应调用此方法替代直接 `save()`。
    /// 应用退出前必须调用 [`Self::flush_pending_saves`] 强制同步落盘。
    pub fn request_save_coalesced(self: &Arc<Self>) {
        // CAS 合并：旧值为 true 说明已有 pending save 排队，直接返回
        if self.save_pending.swap(true, Ordering::AcqRel) {
            return;
        }

        let this = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(e) = this.do_coalesced_save_once().await {
                eprintln!("[DataStore] coalesced save failed: {}", e);
            }
        });
    }

    /// 请求合并写日志（非阻塞）：语义与 `request_save_coalesced` 一致，针对 logs.json。
    pub fn request_save_logs_coalesced(self: &Arc<Self>) {
        if self.save_logs_pending.swap(true, Ordering::AcqRel) {
            return;
        }

        let this = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(e) = this.do_coalesced_save_logs_once().await {
                eprintln!("[DataStore] coalesced save_logs failed: {}", e);
            }
        });
    }

    /// 原子执行一次合并 save：抢 lock → swap 清零脏标记 → 若脏则 save。
    async fn do_coalesced_save_once(&self) -> AppResult<()> {
        let _guard = self.save_coalesce_lock.lock().await;
        if self.save_pending.swap(false, Ordering::AcqRel) {
            self.save().await?;
        }
        Ok(())
    }

    async fn do_coalesced_save_logs_once(&self) -> AppResult<()> {
        let _guard = self.save_logs_coalesce_lock.lock().await;
        if self.save_logs_pending.swap(false, Ordering::AcqRel) {
            self.save_logs().await?;
        }
        Ok(())
    }

    /// 阻塞 flush 所有 pending 的 coalesced save（config + logs），仅在 app exit hook 中调用。
    pub async fn flush_pending_saves(&self) -> AppResult<()> {
        self.do_coalesced_save_once().await?;
        self.do_coalesced_save_logs_once().await?;
        Ok(())
    }

    // ==================== 账号管理方法（v1.7.8 方案 B：全部委托 SQLite） ====================

    pub async fn add_account(&self, email: String, password: String, nickname: String) -> AppResult<Account> {
        self.add_account_no_save(email, password, nickname).await
    }

    pub async fn add_account_no_save(&self, email: String, password: String, nickname: String) -> AppResult<Account> {
        if self.account_store.email_exists(&email)? {
            return Err(AppError::Config(format!("Account with email {} already exists", email)));
        }
        let account = Account::new(email, password, nickname, Vec::new());
        self.account_store.insert_account(&account)?;
        Ok(account)
    }

    pub async fn get_account(&self, id: Uuid) -> AppResult<Account> {
        self.account_store.get_account(&id)
    }

    pub async fn get_all_accounts(&self) -> AppResult<Vec<Account>> {
        self.account_store.get_all_accounts()
    }

    pub async fn update_account(&self, account: Account) -> AppResult<()> {
        self.update_account_internal(account, true).await
    }

    pub async fn update_account_no_save(&self, account: Account) -> AppResult<()> {
        self.update_account_internal(account, false).await
    }

    async fn update_account_internal(&self, mut account: Account, _save_immediately: bool) -> AppResult<()> {
        if let Ok(existing) = self.account_store.get_account(&account.id) {
            account.password = existing.password;
        }
        self.account_store.upsert_account(&account)?;
        Ok(())
    }

    pub async fn delete_account(&self, id: Uuid) -> AppResult<()> {
        if !self.account_store.delete_account(&id)? {
            return Err(AppError::AccountNotFound(id.to_string()));
        }
        let mut logs = self.logs.write().await;
        logs.retain(|log| log.account_id != Some(id));
        drop(logs);
        self.save_logs().await?;
        Ok(())
    }

    pub async fn delete_accounts_batch(
        &self,
        ids: &[Uuid],
    ) -> AppResult<(Vec<Uuid>, Vec<Uuid>)> {
        if ids.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let deleted_ids = self.account_store.delete_accounts_batch(ids)?;
        let deleted_set: std::collections::HashSet<Uuid> = deleted_ids.iter().copied().collect();
        let not_found_ids: Vec<Uuid> = ids.iter().filter(|id| !deleted_set.contains(id)).copied().collect();

        if !deleted_ids.is_empty() {
            let mut logs = self.logs.write().await;
            logs.retain(|log| log.account_id.map(|aid| !deleted_set.contains(&aid)).unwrap_or(true));
            drop(logs);
            self.save_logs().await?;
        }
        Ok((deleted_ids, not_found_ids))
    }

    pub async fn update_account_password(&self, id: Uuid, new_password: String) -> AppResult<()> {
        let mut account = self.account_store.get_account(&id)?;
        account.password = new_password;
        self.account_store.upsert_account(&account)?;
        Ok(())
    }

    pub async fn update_account_token(&self, id: Uuid, token: String, expires_at: chrono::DateTime<chrono::Utc>) -> AppResult<()> {
        let mut account = self.account_store.get_account(&id)?;
        account.token = Some(token);
        account.token_expires_at = Some(expires_at);
        account.last_login_at = Some(chrono::Utc::now());
        account.status = crate::models::AccountStatus::Active;
        self.account_store.upsert_account(&account)?;
        Ok(())
    }
    
    /// 更新账号 token，默认立即保存
    pub async fn update_account_tokens(&self, id: Uuid, token: String, refresh_token: String, expires_at: chrono::DateTime<chrono::Utc>) -> AppResult<()> {
        self.update_account_tokens_internal(id, token, refresh_token, expires_at, true).await
    }
    
    /// 更新账号 token，不立即保存（用于批量操作）
    pub async fn update_account_tokens_no_save(&self, id: Uuid, token: String, refresh_token: String, expires_at: chrono::DateTime<chrono::Utc>) -> AppResult<()> {
        self.update_account_tokens_internal(id, token, refresh_token, expires_at, false).await
    }
    
    /// 内部方法：更新账号 token（v1.7.8 方案 B：委托 SQLite）
    async fn update_account_tokens_internal(&self, id: Uuid, token: String, refresh_token: String, expires_at: chrono::DateTime<chrono::Utc>, _save_immediately: bool) -> AppResult<()> {
        let token_for_event = token.clone();

        let mut account = self.account_store.get_account(&id)?;
        account.token = Some(token);
        account.refresh_token = Some(refresh_token);
        account.token_expires_at = Some(expires_at);
        account.last_login_at = Some(chrono::Utc::now());
        account.status = crate::models::AccountStatus::Active;
        self.account_store.upsert_account(&account)?;

        let payload = TokenRefreshedPayload {
            account_id: id.to_string(),
            token: token_for_event,
            token_expires_at: expires_at.to_rfc3339(),
        };
        if let Err(e) = self.app_handle.emit("token-refreshed", payload) {
            println!("[DataStore] Failed to emit token-refreshed event: {}", e);
        }

        Ok(())
    }
    
    /// 手动触发保存（用于批量操作结束后）
    pub async fn flush(&self) -> AppResult<()> {
        self.save().await
    }

    pub async fn get_decrypted_password(&self, id: Uuid) -> AppResult<String> {
        let account = self.account_store.get_account(&id)?;
        Ok(account.password.clone())
    }

    pub async fn get_decrypted_token(&self, id: Uuid) -> AppResult<Option<String>> {
        let account = self.account_store.get_account(&id)?;
        Ok(account.token.clone())
    }

    // 分组管理
    pub async fn add_group(&self, name: String) -> AppResult<()> {
        let mut config = self.config.write().await;
        
        if !config.groups.contains(&name) {
            config.groups.push(name);
        }
        
        drop(config);
        self.save().await?;
        Ok(())
    }

    pub async fn delete_group(&self, name: String) -> AppResult<()> {
        let mut config = self.config.write().await;
        config.groups.retain(|g| g != &name);
        drop(config);
        self.save().await?;
        let _ = self.account_store.update_group_for_all(&name, None);
        Ok(())
    }

    pub async fn rename_group(&self, old_name: String, new_name: String) -> AppResult<()> {
        let mut config = self.config.write().await;
        if config.groups.contains(&new_name) {
            return Err(AppError::Config(format!("Group '{}' already exists", new_name)));
        }
        if let Some(index) = config.groups.iter().position(|g| g == &old_name) {
            config.groups[index] = new_name.clone();
        } else {
            return Err(AppError::Config(format!("Group '{}' not found", old_name)));
        }
        drop(config);
        self.save().await?;
        let _ = self.account_store.update_group_for_all(&old_name, Some(&new_name));
        Ok(())
    }

    pub async fn get_groups(&self) -> AppResult<Vec<String>> {
        let config = self.config.read().await;
        Ok(config.groups.clone())
    }

    // 标签管理
    pub async fn get_tags(&self) -> AppResult<Vec<crate::models::GlobalTag>> {
        let config = self.config.read().await;
        Ok(config.tags.clone())
    }

    pub async fn add_tag(&self, tag: crate::models::GlobalTag) -> AppResult<()> {
        let mut config = self.config.write().await;
        
        // 检查标签是否已存在
        if config.tags.iter().any(|t| t.name == tag.name) {
            return Err(AppError::Config(format!("Tag '{}' already exists", tag.name)));
        }
        
        config.tags.push(tag);
        drop(config);
        self.save().await?;
        Ok(())
    }

    pub async fn update_tag(&self, old_name: String, tag: crate::models::GlobalTag) -> AppResult<()> {
        let mut config = self.config.write().await;
        if old_name != tag.name && config.tags.iter().any(|t| t.name == tag.name) {
            return Err(AppError::Config(format!("Tag '{}' already exists", tag.name)));
        }
        if let Some(index) = config.tags.iter().position(|t| t.name == old_name) {
            config.tags[index] = tag.clone();
        } else {
            return Err(AppError::Config(format!("Tag '{}' not found", old_name)));
        }
        drop(config);
        self.save().await?;
        if old_name != tag.name {
            let _ = self.account_store.rename_tag_for_all(&old_name, &tag.name);
        }
        Ok(())
    }

    pub async fn delete_tag(&self, name: String) -> AppResult<()> {
        let mut config = self.config.write().await;
        config.tags.retain(|t| t.name != name);
        drop(config);
        self.save().await?;
        let _ = self.account_store.remove_tag_for_all(&name);
        Ok(())
    }

    pub async fn batch_update_account_tags(
        &self,
        account_ids: Vec<String>,
        add_tags: Vec<String>,
        remove_tags: Vec<String>,
    ) -> AppResult<(usize, usize)> {
        let config = self.config.read().await;
        let global_tags = config.tags.clone();
        drop(config);

        let mut success_count = 0;
        let mut failed_count = 0;

        for id in account_ids {
            if let Ok(uuid) = uuid::Uuid::parse_str(&id) {
                if let Ok(mut account) = self.account_store.get_account(&uuid) {
                    for tag_name in &add_tags {
                        if !account.tags.contains(tag_name) {
                            account.tags.push(tag_name.clone());
                            if let Some(global_tag) = global_tags.iter().find(|t| &t.name == tag_name) {
                                if !account.tag_colors.iter().any(|tc| &tc.name == tag_name) {
                                    account.tag_colors.push(crate::models::TagWithColor {
                                        name: tag_name.clone(),
                                        color: global_tag.color.clone(),
                                    });
                                }
                            }
                        }
                    }
                    for tag_name in &remove_tags {
                        account.tags.retain(|t| t != tag_name);
                        account.tag_colors.retain(|tc| &tc.name != tag_name);
                    }
                    if self.account_store.upsert_account(&account).is_ok() {
                        success_count += 1;
                    } else {
                        failed_count += 1;
                    }
                } else {
                    failed_count += 1;
                }
            } else {
                failed_count += 1;
            }
        }
        Ok((success_count, failed_count))
    }

    // 日志管理
    pub async fn add_log(&self, log: OperationLog) -> AppResult<()> {
        let mut logs = self.logs.write().await;
        
        logs.push(log);
        
        // 限制日志数量，保留最新的1000条
        if logs.len() > 1000 {
            let start = logs.len() - 1000;
            logs.drain(0..start);
        }
        
        drop(logs);
        self.save_logs().await?;
        Ok(())
    }

    pub async fn get_logs(&self, limit: Option<usize>) -> AppResult<Vec<OperationLog>> {
        let logs = self.logs.read().await;
        let logs_vec = logs.clone();
        
        if let Some(limit) = limit {
            let start = logs_vec.len().saturating_sub(limit);
            Ok(logs_vec[start..].to_vec())
        } else {
            Ok(logs_vec)
        }
    }

    pub async fn clear_logs(&self) -> AppResult<()> {
        let mut logs = self.logs.write().await;
        logs.clear();
        drop(logs);
        self.save_logs().await?;
        Ok(())
    }

    // 设置管理
    pub async fn get_settings(&self) -> AppResult<crate::models::Settings> {
        let config = self.config.read().await;
        Ok(config.settings.clone())
    }

    pub async fn update_settings(&self, settings: crate::models::Settings) -> AppResult<()> {
        let mut config = self.config.write().await;
        config.settings = settings;
        drop(config);
        self.save().await?;
        Ok(())
    }
    
    // ==================== 数据安全功能 ====================
    
    /// 创建带时间戳的备份（备份整个数据目录下的所有JSON文件）
    pub async fn create_timestamped_backup(&self) -> AppResult<PathBuf> {
        let config = self.config.read().await;
        let max_count = config.settings.backup_max_count as usize;
        drop(config);
        
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let data_dir = self.config_path.parent()
            .ok_or_else(|| AppError::Config("Invalid config path".to_string()))?;
        let backup_root = data_dir.join("backups");
        
        // 创建以时间戳命名的备份子目录
        let backup_subdir = backup_root.join(format!("backup_{}", timestamp));
        fs::create_dir_all(&backup_subdir)?;
        
        // 备份数据目录下的所有数据文件（v1.7.8 方案 B：增加 accounts.db SQLite 数据库）
        let data_files = ["accounts.json", "logs.json", "auto_reset_configs.json", "reset_records.json", "success_bins.json", "accounts.db"];
        let mut backed_up_count = 0;
        
        for file_name in &data_files {
            let src_path = data_dir.join(file_name);
            if src_path.exists() {
                let dest_path = backup_subdir.join(file_name);
                fs::copy(&src_path, &dest_path)?;
                backed_up_count += 1;
            }
        }
        
        // 清理旧备份目录，使用配置的最大备份数
        Self::cleanup_old_backup_dirs(&backup_root, max_count)?;
        
        println!("[Backup] 备份创建成功: {:?}, 共 {} 个文件, 最大保留 {} 份", 
            backup_subdir.file_name(), backed_up_count, max_count);
        
        Ok(backup_subdir)
    }
    
    /// 清理旧备份目录，只保留最近 N 个
    fn cleanup_old_backup_dirs(backup_root: &PathBuf, keep_count: usize) -> std::io::Result<()> {
        if !backup_root.exists() {
            return Ok(());
        }
        
        let mut backup_dirs: Vec<_> = fs::read_dir(backup_root)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_dir() && path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("backup_"))
                    .unwrap_or(false)
            })
            .collect();
        
        // 按修改时间排序（最新的在前）
        backup_dirs.sort_by(|a, b| {
            let time_a = fs::metadata(a).and_then(|m| m.modified()).ok();
            let time_b = fs::metadata(b).and_then(|m| m.modified()).ok();
            time_b.cmp(&time_a)
        });
        
        // 删除超出数量的旧备份目录
        for old_backup in backup_dirs.iter().skip(keep_count) {
            let _ = fs::remove_dir_all(old_backup);
            println!("[Backup] 已删除旧备份: {:?}", old_backup.file_name());
        }
        
        Ok(())
    }
    
    /// 导出数据到指定路径
    pub async fn export_data(&self, export_path: &PathBuf) -> AppResult<()> {
        let accounts = self.account_store.get_all_accounts()?;
        let config = self.config.read().await;
        let export_data = serde_json::json!({
            "version": "1.0",
            "exported_at": Local::now().to_rfc3339(),
            "accounts": accounts,
            "groups": config.groups,
            "settings": config.settings
        });
        drop(config);
        
        let data = serde_json::to_string_pretty(&export_data)?;
        fs::write(export_path, data)?;
        
        Ok(())
    }
    
    /// 从指定路径导入数据
    pub async fn import_data(&self, import_path: &PathBuf, merge: bool) -> AppResult<ImportResult> {
        let data = fs::read_to_string(import_path)?;
        let import_data: serde_json::Value = serde_json::from_str(&data)?;
        
        self.create_timestamped_backup().await?;
        
        let mut result = ImportResult::default();
        
        // v1.7.8 方案 B：导入账号到 SQLite
        if let Some(accounts) = import_data.get("accounts") {
            let imported_accounts: Vec<Account> = serde_json::from_value(accounts.clone())?;
            
            if merge {
                for account in imported_accounts {
                    if !self.account_store.email_exists(&account.email).unwrap_or(true) {
                        if self.account_store.insert_account(&account).is_ok() {
                            result.accounts_added += 1;
                        } else {
                            result.accounts_skipped += 1;
                        }
                    } else {
                        result.accounts_skipped += 1;
                    }
                }
            } else {
                if let Ok(existing) = self.account_store.get_all_accounts() {
                    let ids: Vec<Uuid> = existing.iter().map(|a| a.id).collect();
                    let _ = self.account_store.delete_accounts_batch(&ids);
                }
                result.accounts_added = imported_accounts.len();
                let _ = self.account_store.bulk_insert(&imported_accounts);
            }
        }
        
        if let Some(groups) = import_data.get("groups") {
            let mut config = self.config.write().await;
            let imported_groups: Vec<String> = serde_json::from_value(groups.clone())?;
            for group in imported_groups {
                if !config.groups.contains(&group) {
                    config.groups.push(group);
                    result.groups_added += 1;
                }
            }
            drop(config);
            self.save().await?;
        }
        
        Ok(result)
    }
    
    /// 获取备份列表（返回备份目录列表）
    pub async fn list_backups(&self) -> AppResult<Vec<BackupInfo>> {
        let backup_root = self.config_path.parent()
            .ok_or_else(|| AppError::Config("Invalid config path".to_string()))?
            .join("backups");
        
        if !backup_root.exists() {
            return Ok(Vec::new());
        }
        
        let mut backups: Vec<BackupInfo> = fs::read_dir(&backup_root)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                let name = path.file_name()?.to_str()?.to_string();
                // 匹配备份目录 backup_YYYYMMDD_HHMMSS
                if path.is_dir() && name.starts_with("backup_") {
                    let metadata = fs::metadata(&path).ok()?;
                    // 计算目录总大小
                    let total_size = Self::calculate_dir_size(&path);
                    Some(BackupInfo {
                        name,
                        path: path.to_string_lossy().to_string(),
                        size: total_size,
                        created_at: metadata.modified().ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs() as i64),
                    })
                } else {
                    None
                }
            })
            .collect();
        
        // 按创建时间降序排列
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(backups)
    }
    
    /// 计算目录大小
    fn calculate_dir_size(path: &PathBuf) -> u64 {
        fs::read_dir(path)
            .map(|entries| {
                entries.filter_map(|e| e.ok())
                    .filter_map(|e| fs::metadata(e.path()).ok())
                    .map(|m| m.len())
                    .sum()
            })
            .unwrap_or(0)
    }
    
    /// 从备份目录恢复所有数据
    pub async fn restore_from_backup(&self, backup_path: &PathBuf) -> AppResult<()> {
        let data_dir = self.config_path.parent()
            .ok_or_else(|| AppError::Config("Invalid config path".to_string()))?;
        
        // 先备份当前数据
        self.create_timestamped_backup().await?;
        
        // 从备份目录恢复所有文件（v1.7.8 方案 B：增加 accounts.db SQLite 数据库）
        let data_files = ["accounts.json", "logs.json", "auto_reset_configs.json", "reset_records.json", "success_bins.json", "accounts.db"];
        
        for file_name in &data_files {
            let src_path = backup_path.join(file_name);
            let dest_path = data_dir.join(file_name);
            if src_path.exists() {
                fs::copy(&src_path, &dest_path)?;
                println!("[Backup] 已恢复: {}", file_name);
            }
        }
        
        // 重新加载 accounts.json 到内存
        let accounts_path = data_dir.join("accounts.json");
        if accounts_path.exists() {
            let data = fs::read_to_string(&accounts_path)?;
            let config: AppConfig = serde_json::from_str(&data)?;
            let mut current_config = self.config.write().await;
            *current_config = config;
            drop(current_config);
        }
        
        Ok(())
    }
    
    /// 删除指定备份
    pub async fn delete_backup(&self, backup_name: &str) -> AppResult<()> {
        let backup_root = self.config_path.parent()
            .ok_or_else(|| AppError::Config("Invalid config path".to_string()))?
            .join("backups");
        
        let backup_path = backup_root.join(backup_name);
        
        // 安全检查：确保备份路径在备份目录下
        if !backup_path.starts_with(&backup_root) {
            return Err(AppError::Config("Invalid backup path".to_string()));
        }
        
        // 确保是目录且以backup_开头
        if !backup_path.is_dir() || !backup_name.starts_with("backup_") {
            return Err(AppError::Config("Invalid backup".to_string()));
        }
        
        // 删除备份目录
        fs::remove_dir_all(&backup_path)?;
        println!("[Backup] 已删除备份: {}", backup_name);
        
        Ok(())
    }
    
    /// 获取数据目录路径
    pub fn get_data_dir(&self) -> PathBuf {
        self.config_path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default()
    }
    
    /// 更新账户排序顺序（用于拖拽排序）
    pub async fn update_accounts_order(&self, account_ids: Vec<String>) -> AppResult<()> {
        for (index, id_str) in account_ids.iter().enumerate() {
            if let Ok(uuid) = Uuid::parse_str(id_str) {
                if let Ok(mut account) = self.account_store.get_account(&uuid) {
                    account.sort_order = index as i32;
                    let _ = self.account_store.upsert_account(&account);
                }
            }
        }
        Ok(())
    }
    
    /// 获取排序后的账户列表
    pub async fn get_sorted_accounts(&self, sort_field: &crate::models::SortField, sort_direction: &crate::models::SortDirection) -> AppResult<Vec<Account>> {
        use crate::models::{SortField, SortDirection};
        
        let mut accounts = self.account_store.get_all_accounts()?;
        
        // 根据排序字段排序
        match sort_field {
            SortField::Email => {
                accounts.sort_by(|a, b| a.email.to_lowercase().cmp(&b.email.to_lowercase()));
            }
            SortField::CreatedAt => {
                accounts.sort_by_key(|a| a.created_at);
            }
            SortField::UsedQuota => {
                accounts.sort_by_key(|a| a.used_quota.unwrap_or(0));
            }
            SortField::RemainingQuota => {
                accounts.sort_by_key(|a| {
                    let total = a.total_quota.unwrap_or(0);
                    let used = a.used_quota.unwrap_or(0);
                    total - used
                });
            }
            SortField::TokenExpiresAt => {
                accounts.sort_by(|a, b| {
                    match (&a.token_expires_at, &b.token_expires_at) {
                        (Some(a_exp), Some(b_exp)) => a_exp.cmp(b_exp),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
            }
            SortField::SubscriptionExpiresAt => {
                accounts.sort_by(|a, b| {
                    match (&a.subscription_expires_at, &b.subscription_expires_at) {
                        (Some(a_exp), Some(b_exp)) => a_exp.cmp(b_exp),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
            }
            SortField::PlanName => {
                // 定义套餐优先级: Enterprise > Teams > Pro > Trial > Free > None
                let plan_priority = |plan: &Option<String>| -> i32 {
                    match plan.as_ref().map(|s| s.to_lowercase()).as_deref() {
                        Some("enterprise") => 5,
                        Some("teams") => 4,
                        Some("pro") => 3,
                        Some("trial") => 2,
                        Some("free") => 1,
                        _ => 0,
                    }
                };
                accounts.sort_by(|a, b| plan_priority(&b.plan_name).cmp(&plan_priority(&a.plan_name)));
            }
            // 日配额剩余百分比：Some 靠前（升序小→大），None 靠后；与 TokenExpiresAt 同模式
            SortField::DailyQuotaRemaining => {
                accounts.sort_by(|a, b| {
                    match (&a.daily_quota_remaining_percent, &b.daily_quota_remaining_percent) {
                        (Some(x), Some(y)) => x.cmp(y),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
            }
            // 周配额剩余百分比：同上
            SortField::WeeklyQuotaRemaining => {
                accounts.sort_by(|a, b| {
                    match (&a.weekly_quota_remaining_percent, &b.weekly_quota_remaining_percent) {
                        (Some(x), Some(y)) => x.cmp(y),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
            }
        }
        
        // 根据排序方向反转
        if *sort_direction == SortDirection::Desc && *sort_field != SortField::PlanName {
            accounts.reverse();
        } else if *sort_direction == SortDirection::Asc && *sort_field == SortField::PlanName {
            accounts.reverse();
        }
        
        Ok(accounts)
    }
}

/// 导入结果
#[derive(Debug, Default, serde::Serialize)]
pub struct ImportResult {
    pub accounts_added: usize,
    pub accounts_skipped: usize,
    pub groups_added: usize,
}

/// 备份信息
#[derive(Debug, serde::Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub created_at: Option<i64>,
}
