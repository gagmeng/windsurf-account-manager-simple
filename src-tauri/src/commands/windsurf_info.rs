use crate::utils::errors::AppError;
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct WindsurfCurrentInfo {
    pub email: Option<String>,
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub plan_name: Option<String>,
    pub team_id: Option<String>,
    pub version: Option<String>,
    pub is_active: bool,
    /// 活跃客户端类型："windsurf" | "windsurf-next"
    pub client_type: String,
    /// 客户端展示名："Windsurf" | "Windsurf - Next"
    pub client_display_name: String,
    /// 活跃客户端进程是否正在运行
    pub is_running: bool,
}

// ==================== 活跃客户端判定（进程优先 + state.vscdb mtime fallback） ====================

/// 客户端候选（Windsurf / Windsurf - Next）
struct ClientCandidate {
    client_type: &'static str,
    display_name: &'static str,
    base_dir_name: &'static str,
    process_name: &'static str,
}

const CANDIDATES: &[ClientCandidate] = &[
    ClientCandidate {
        client_type: "windsurf",
        display_name: "Windsurf",
        base_dir_name: "Windsurf",
        process_name: "Windsurf.exe",
    },
    ClientCandidate {
        client_type: "windsurf-next",
        display_name: "Windsurf - Next",
        base_dir_name: "Windsurf - Next",
        process_name: "Windsurf - Next.exe",
    },
];

fn get_state_db_path(appdata: &Path, candidate: &ClientCandidate) -> PathBuf {
    appdata
        .join(candidate.base_dir_name)
        .join("User")
        .join("globalStorage")
        .join("state.vscdb")
}

fn get_state_db_mtime(db_path: &Path) -> Option<SystemTime> {
    db_path.metadata().ok().and_then(|m| m.modified().ok())
}

/// 检测 Windsurf / Windsurf - Next 进程是否在运行
///
/// Windows 使用 `tasklist /FI "IMAGENAME eq XXX.exe"`；
/// Unix 使用 `pgrep -f` 匹配（去除 .exe 后缀）
#[cfg(target_os = "windows")]
fn is_process_running(process_name: &str) -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = Command::new("tasklist")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "/FI",
            &format!("IMAGENAME eq {}", process_name),
            "/NH",
            "/FO",
            "CSV",
        ])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // tasklist 找不到进程时输出 "INFO: No tasks are running..."，找到时行含进程名
            stdout.contains(process_name)
        }
        Err(_) => false,
    }
}

#[cfg(not(target_os = "windows"))]
fn is_process_running(process_name: &str) -> bool {
    use std::process::Command;
    let clean_name = process_name.trim_end_matches(".exe");
    match Command::new("pgrep").args(["-f", clean_name]).output() {
        Ok(out) => !out.stdout.is_empty(),
        Err(_) => false,
    }
}

/// 活跃客户端快照
struct CandidateSnapshot {
    candidate: &'static ClientCandidate,
    db_path: PathBuf,
    db_exists: bool,
    is_running: bool,
    db_mtime: Option<SystemTime>,
}

/// 活跃客户端判定（进程优先 + state.vscdb mtime fallback）
///
/// 优先级：
/// 1. 仅一个进程在运行 → 选它
/// 2. 两个进程都在运行 → state.vscdb 修改时间更近的
/// 3. 无进程运行 → state.vscdb 修改时间更近的（"最后打开"语义）
/// 4. 只有一个安装 → 选它
/// 5. 都没有 state.vscdb → None
fn resolve_active_client(appdata: &Path) -> Option<CandidateSnapshot> {
    let snapshots: Vec<CandidateSnapshot> = CANDIDATES
        .iter()
        .map(|c| {
            let db_path = get_state_db_path(appdata, c);
            let db_exists = db_path.exists();
            CandidateSnapshot {
                candidate: c,
                db_exists,
                is_running: is_process_running(c.process_name),
                db_mtime: if db_exists {
                    get_state_db_mtime(&db_path)
                } else {
                    None
                },
                db_path,
            }
        })
        .collect();

    let installed_indices: Vec<usize> = (0..snapshots.len())
        .filter(|&i| snapshots[i].db_exists)
        .collect();
    if installed_indices.is_empty() {
        return None;
    }

    let running_indices: Vec<usize> = installed_indices
        .iter()
        .copied()
        .filter(|&i| snapshots[i].is_running)
        .collect();

    let chosen_idx = if running_indices.len() == 1 {
        // 规则 1：仅一个进程在运行
        running_indices[0]
    } else {
        // 规则 2/3/4：0 个或 ≥2 个进程在运行 → 按 mtime 选更近的
        let pool: &[usize] = if !running_indices.is_empty() {
            &running_indices
        } else {
            &installed_indices
        };
        *pool
            .iter()
            .max_by_key(|&&i| snapshots[i].db_mtime)
            .unwrap()
    };

    snapshots.into_iter().nth(chosen_idx)
}

// ==================== Protobuf 字段提取 ====================

/// 从 Protobuf 二进制数据中提取邮箱地址
fn extract_email_from_protobuf(data: &[u8]) -> Option<String> {
    // 邮箱在 Protobuf 中以明文存储，格式为: field_tag + length + email_string
    // 我们可以通过搜索 @ 符号来定位邮箱
    let data_str = String::from_utf8_lossy(data);
    
    // 使用正则表达式匹配邮箱格式
    let email_pattern = regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").ok()?;
    
    if let Some(mat) = email_pattern.find(&data_str) {
        let mut email = mat.as_str().to_string();
        // 修剪顶级域名，检测常见的顶级域名并截断多余字符
        if let Some(last_dot) = email.rfind('.') {
            let tld = &email[last_dot + 1..];
            // 检查是否以常见顶级域名开头，如果是则截断到正确长度
            let common_tlds = ["com", "net", "org", "edu", "gov", "info", "io", "co", "cn", "uk", "de", "fr", "jp", "kr", "ru", "br", "au", "in", "us", "ca", "es", "it", "nl", "se", "no", "fi", "dk", "pl", "cz", "at", "ch", "be", "pt", "gr", "hu", "ro", "bg", "sk", "hr", "si", "rs", "ua", "by", "kz", "tr", "il", "ae", "sa", "eg", "za", "ng", "ke", "gh", "tz", "ug", "rw", "et", "ma", "dz", "tn", "ly", "sd", "ao", "mz", "zm", "zw", "bw", "na", "sz", "ls", "mg", "mu", "sc", "re", "yt", "km", "dj", "so", "er", "ss", "cf", "td", "ne", "ml", "bf", "ci", "gh", "tg", "bj", "gn", "sl", "lr", "mr", "sn", "gm", "gw", "cv", "st", "gq", "ga", "cg", "cd", "rw", "bi", "ug", "ke", "tz", "mw", "zm", "zw", "bw", "sz", "ls", "na", "ao", "mz"];
            for common_tld in common_tlds {
                if tld.to_lowercase().starts_with(common_tld) && tld.len() >= common_tld.len() {
                    // 截断到常见顶级域名的长度
                    email = email[..last_dot + 1 + common_tld.len()].to_string();
                    break;
                }
            }
        }
        return Some(email);
    }
    
    None
}

/// 从 Protobuf 二进制数据中提取用户名
fn extract_name_from_protobuf(data: &[u8]) -> Option<String> {
    // 用户名通常在数据开头附近，格式为: 0x1a + length + name
    // 查找 field 3 (0x1a) 后的字符串
    for i in 0..data.len().saturating_sub(2) {
        if data[i] == 0x1a {
            let len = data[i + 1] as usize;
            if i + 2 + len <= data.len() {
                if let Ok(name) = std::str::from_utf8(&data[i + 2..i + 2 + len]) {
                    // 过滤掉明显不是名字的内容
                    if !name.contains('@') && !name.contains('/') && name.len() > 1 && name.len() < 50 {
                        // 检查是否是有效的可打印字符
                        if name.chars().all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_') {
                            return Some(name.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

// ==================== 客户端信息读取 ====================

/// 从指定客户端候选的 state.vscdb 读取完整账号信息
///
/// 保留原有的 protobuf 双格式解析（新版 `userStatusProtoBinaryBase64` / 旧版顶级 JSON 字段）
/// 与 `apiKey` 双路径兼容；新增 `client_type` / `client_display_name` / `is_running` 字段。
fn read_client_info(
    candidate: &ClientCandidate,
    db_path: &Path,
    is_running: bool,
) -> WindsurfCurrentInfo {
    let mut info = WindsurfCurrentInfo {
        email: None,
        name: None,
        api_key: None,
        plan_name: None,
        team_id: None,
        version: None,
        is_active: false,
        client_type: candidate.client_type.to_string(),
        client_display_name: candidate.display_name.to_string(),
        is_running,
    };

    if !db_path.exists() {
        return info;
    }

    // 只读模式，避免与 Windsurf 客户端写入冲突
    let connection = match rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to open {}: {}", db_path.display(), e);
            return info;
        }
    };

    let auth_status: Option<String> = connection
        .query_row(
            "SELECT value FROM ItemTable WHERE key = ?",
            ["windsurfAuthStatus"],
            |row| row.get(0),
        )
        .ok();

    let version: Option<String> = connection
        .query_row(
            "SELECT value FROM ItemTable WHERE key = ?",
            ["windsurfChangelog/lastVersion"],
            |row| row.get(0),
        )
        .ok();
    info.version = version;

    if let Some(auth_json) = auth_status {
        log::debug!(
            "[{}] auth_json length: {}",
            candidate.client_type,
            auth_json.len()
        );
        if let Ok(json_value) = serde_json::from_str::<Value>(&auth_json) {
            // apiKey 两种格式共用字段
            if let Some(api_key) = json_value.get("apiKey").and_then(|v| v.as_str()) {
                info.api_key = Some(api_key.to_string());
                info.is_active = true;
            }

            // 新格式：从 userStatusProtoBinaryBase64 提取用户信息
            if let Some(user_status_base64) = json_value
                .get("userStatusProtoBinaryBase64")
                .and_then(|v| v.as_str())
            {
                match general_purpose::STANDARD.decode(user_status_base64) {
                    Ok(decoded) => {
                        if let Some(email) = extract_email_from_protobuf(&decoded) {
                            info.email = Some(email);
                        }
                        if let Some(name) = extract_name_from_protobuf(&decoded) {
                            info.name = Some(name);
                        }
                    }
                    Err(e) => log::error!("Failed to decode base64: {}", e),
                }
            }

            // 旧格式：直接顶级 JSON 字段（向后兼容）
            if info.email.is_none() {
                if let Some(email) = json_value.get("email").and_then(|v| v.as_str()) {
                    info.email = Some(email.to_string());
                }
            }
            if info.name.is_none() {
                if let Some(name) = json_value.get("name").and_then(|v| v.as_str()) {
                    info.name = Some(name.to_string());
                }
            }
            if let Some(team_id) = json_value.get("teamId").and_then(|v| v.as_str()) {
                info.team_id = Some(team_id.to_string());
            }
            if let Some(plan_name) = json_value.get("planName").and_then(|v| v.as_str()) {
                info.plan_name = Some(plan_name.to_string());
            }
        }
    }

    info
}

// ==================== Tauri 命令入口 ====================

/// 获取当前活跃 Windsurf / Windsurf - Next 账号信息
///
/// 按「进程优先 + state.vscdb 修改时间 fallback」策略自动选择活跃客户端，
/// 返回结构额外包含 `client_type` / `client_display_name` / `is_running` 三个字段，
/// 前端可根据 `client_display_name` 动态切换「Windsurf 版本」↔「Windsurf - Next 版本」文案。
#[tauri::command]
pub fn get_current_windsurf_info() -> Result<WindsurfCurrentInfo, AppError> {
    let appdata = std::env::var("APPDATA")
        .map_err(|e| AppError::Config(format!("Failed to get APPDATA: {}", e)))?;
    let appdata_path = PathBuf::from(appdata);

    match resolve_active_client(&appdata_path) {
        Some(snapshot) => Ok(read_client_info(
            snapshot.candidate,
            &snapshot.db_path,
            snapshot.is_running,
        )),
        None => {
            // 两个客户端都未安装 → 返回空结构，默认标记为 windsurf
            Ok(WindsurfCurrentInfo {
                email: None,
                name: None,
                api_key: None,
                plan_name: None,
                team_id: None,
                version: None,
                is_active: false,
                client_type: "windsurf".to_string(),
                client_display_name: "Windsurf".to_string(),
                is_running: false,
            })
        }
    }
}
