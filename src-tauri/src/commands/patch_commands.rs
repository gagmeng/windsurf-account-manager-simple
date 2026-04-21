use tauri::command;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use regex::Regex;
use chrono::Local;
use std::sync::Arc;
use tauri::State;
use crate::repository::DataStore;

// ==================== 无感换号补丁常量（单一真源） ====================
//
// 这两条正则同时被 `apply_seamless_patch_internal` 与 `check_patch_status` 消费，
// 用于判定 extension.js 是否仍保留未打补丁时的原始结构。抽成模块级常量是为了
// 避免历史上两处判定逻辑漂移（检测用"补丁标识字符串"、打补丁用"pattern 匹配"，
// 导致 UI 显示"未安装"但 apply 却返回"已打补丁"的不一致）。
//
// 维护守则：修改 pattern 时必须同步确认 apply 分支的 replacement 仍能命中新结构，
// 并在 `CURRENT_VERSION_MARKER` 登记新版本注入代码的特征字符串。
const PATTERN_URI_HANDLER_ORIGINAL: &str = r#"this\._uriHandler\.event\((\w+)=>\{"/refresh-authentication-session"===(\w+)\.path&&\(0,(\w+)\.refreshAuthenticationSession\)\(\)\}\)"#;
const PATTERN_TIMEOUT_ORIGINAL: &str = r#",new Promise\(\((\w+),(\w+)\)=>setTimeout\(\(\)=>\{(\w+)\(new (\w+)\)\},18e4\)\)"#;
/// 第 3 条：新版 Windsurf - Next 的 `maybeHandleUriWithToken` 在切号时弹出的
/// "Are you sure you want to log in using a different account?" 模态 prompt。
/// 捕获 `s.window.showWarningMessage(...)` 的 s 变量名（bundle 每次构建可能不同）。
/// 替换策略：把整个 `if("Yes"===await ...showWarningMessage(...))` 条件换成 `if(true)`，
/// 原条件的 if-body 是单语句 `try{...}catch{...}`（无大括号包裹），因此仅改 condition 即可
/// 完全旁路 prompt、语法保持合法、无需改动 body 结构。
const PATTERN_DIFF_ACCOUNT_PROMPT_ORIGINAL: &str = r#"if\("Yes"===await (\w+)\.window\.showWarningMessage\("Are you sure you want to log in using a different account\?",\{modal:!0\},"Yes"\)\)"#;
/// 当前版本注入代码独有的特征字符串，用于辅助判别"已安装的是当前版本还是历史/第三方版本"
const CURRENT_VERSION_MARKER: &str = "Failed to handle OAuth callback";

/// 客户端配置信息
struct ClientConfig {
    /// 进程名（不含 .exe）
    process_name: &'static str,
    /// Windows 开始菜单文件夹名
    start_menu_folder: &'static str,
    /// Windows 常见安装目录名
    install_dir_name: &'static str,
    /// macOS .app 名称
    #[allow(dead_code)]
    macos_app_name: &'static str,
}

fn get_client_config(client_type: &str) -> ClientConfig {
    match client_type {
        "windsurf-next" => ClientConfig {
            process_name: "Windsurf - Next",
            start_menu_folder: "Windsurf - Next",
            install_dir_name: "Windsurf - Next",
            macos_app_name: "Windsurf - Next",
        },
        _ => ClientConfig {
            process_name: "Windsurf",
            start_menu_folder: "Windsurf",
            install_dir_name: "Windsurf",
            macos_app_name: "Windsurf",
        },
    }
}

/// 获取 extension.js 相对路径（跨平台）
fn get_extension_js_relative_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        // macOS: Windsurf.app/Contents/Resources/app/extensions/windsurf/dist/extension.js
        PathBuf::from("Contents")
            .join("Resources")
            .join("app")
            .join("extensions")
            .join("windsurf")
            .join("dist")
            .join("extension.js")
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Windows/Linux: resources/app/extensions/windsurf/dist/extension.js
        PathBuf::from("resources")
            .join("app")
            .join("extensions")
            .join("windsurf")
            .join("dist")
            .join("extension.js")
    }
}

/// 检测客户端安装路径（内部函数，可跨模块调用）
pub fn detect_windsurf_path_internal(client_type: &str) -> Result<String, String> {
    let config = get_client_config(client_type);
    detect_windsurf_path_by_config(&config)
}

fn detect_windsurf_path_by_config(config: &ClientConfig) -> Result<String, String> {
    
    #[cfg(target_os = "windows")]
    {
        // Windows: 首先尝试从开始菜单快捷方式获取
        let start_menu_path = std::env::var("APPDATA")
            .map(|p| PathBuf::from(p).join(format!("Microsoft\\Windows\\Start Menu\\Programs\\{}", config.start_menu_folder)))
            .ok();
        
        if let Some(start_menu) = start_menu_path {
            if let Ok(entries) = fs::read_dir(&start_menu) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("lnk") {
                        if let Ok(target) = resolve_shortcut(&path) {
                            if let Some(parent) = target.parent() {
                                let windsurf_root = parent.to_path_buf();
                                let extension_file = windsurf_root.join(get_extension_js_relative_path());
                                
                                if extension_file.exists() {
                                    return Ok(windsurf_root.to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Windows: 尝试常见的安装位置
        let dir_name = config.install_dir_name;
        let possible_locations = vec![
            std::env::var("LOCALAPPDATA").ok().map(|p| PathBuf::from(p).join(format!("Programs\\{}", dir_name))),
            Some(PathBuf::from(format!("C:\\Program Files\\{}", dir_name))),
            Some(PathBuf::from(format!("C:\\Program Files (x86)\\{}", dir_name))),
            Some(PathBuf::from(format!("D:\\Program\\{}", dir_name))),
        ];
        
        for location in possible_locations.into_iter().flatten() {
            let extension_file = location.join(get_extension_js_relative_path());
            if extension_file.exists() {
                return Ok(location.to_string_lossy().to_string());
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        let app_name = format!("{}.app", config.macos_app_name);
        let possible_locations = vec![
            PathBuf::from(format!("/Applications/{}", app_name)),
            std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(format!("Applications/{}", app_name))).unwrap_or_default(),
        ];
        
        for location in possible_locations {
            let extension_file = location.join(get_extension_js_relative_path());
            if extension_file.exists() {
                return Ok(location.to_string_lossy().to_string());
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        let dir_name = config.install_dir_name;
        let possible_locations = vec![
            PathBuf::from(format!("/opt/{}", dir_name)),
            PathBuf::from(format!("/usr/share/{}", dir_name.to_lowercase())),
            std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(format!(".local/share/{}", dir_name))).unwrap_or_default(),
        ];
        
        for location in possible_locations {
            let extension_file = location.join(get_extension_js_relative_path());
            if extension_file.exists() {
                return Ok(location.to_string_lossy().to_string());
            }
        }
    }
    
    Err(format!("未找到 {} 安装路径", config.process_name))
}

/// 获取客户端的安装路径（Tauri 命令）
#[command]
pub async fn get_windsurf_path(client_type: Option<String>) -> Result<String, String> {
    let ct = client_type.as_deref().unwrap_or("windsurf");
    detect_windsurf_path_internal(ct)
}

/// 解析Windows快捷方式
#[cfg(target_os = "windows")]
fn resolve_shortcut(lnk_path: &Path) -> Result<PathBuf, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    
    let output = Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args(&[
            "-NoProfile",
            "-Command",
            &format!(
                "$sh = New-Object -ComObject WScript.Shell; $sh.CreateShortcut('{}').TargetPath",
                lnk_path.display()
            )
        ])
        .output()
        .map_err(|e| e.to_string())?;
    
    if output.status.success() {
        let target = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        
        if !target.is_empty() {
            Ok(PathBuf::from(target))
        } else {
            Err("快捷方式目标为空".to_string())
        }
    } else {
        Err("解析快捷方式失败".to_string())
    }
}

#[cfg(not(target_os = "windows"))]
fn resolve_shortcut(_lnk_path: &Path) -> Result<PathBuf, String> {
    Err("不支持的操作系统".to_string())
}

/// 应用无感换号补丁（内部函数，可跨模块调用）
pub async fn apply_seamless_patch_internal(
    windsurf_path: &str,
    data_store: &Arc<DataStore>,
) -> Result<serde_json::Value, String> {
    let extension_file = PathBuf::from(windsurf_path).join(get_extension_js_relative_path());
    
    if !extension_file.exists() {
        return Err(format!("extension.js 文件不存在: {:?}", extension_file));
    }
    
    let parent_dir = extension_file.parent()
        .ok_or("无法获取父目录")?
        .to_path_buf();
    
    // 1. 读取现有 extension.js 内容
    let content = fs::read_to_string(&extension_file)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    
    // 2. 编译与 check_patch_status 共享的 pattern（单一真源，避免判定漂移）
    let pattern1 = Regex::new(PATTERN_URI_HANDLER_ORIGINAL)
        .map_err(|e| format!("正则表达式错误: {}", e))?;
    let pattern2 = Regex::new(PATTERN_TIMEOUT_ORIGINAL)
        .map_err(|e| format!("正则表达式错误2: {}", e))?;
    let pattern3 = Regex::new(PATTERN_DIFF_ACCOUNT_PROMPT_ORIGINAL)
        .map_err(|e| format!("正则表达式错误3: {}", e))?;
    
    // 3. Dry-run 判定：三条原始 pattern 都匹配不上 = 已被当前版本补丁完整改写过。
    //    此时跳过备份创建（避免每次点"启用"都往目录堆一份"含补丁"的废备份），
    //    仅把 settings 同步到磁盘实际状态，让前端 UI 一致后返回。
    //    注：只打过 1+2 但缺 3 的历史补丁会命中 apply 分支，走"增量升级"补上 p3。
    let pattern1_still_original = pattern1.is_match(&content);
    let pattern2_still_original = pattern2.is_match(&content);
    let pattern3_still_original = pattern3.is_match(&content);
    if !pattern1_still_original && !pattern2_still_original && !pattern3_still_original {
        let mut settings = data_store.get_settings().await.map_err(|e| e.to_string())?;
        settings.seamless_switch_enabled = true;
        settings.windsurf_path = Some(windsurf_path.to_string());
        // 备份路径兜底：当前未记录或已失效时，从目录挑最新的现有备份回填，
        // 让后续"还原补丁"操作有一个可用 fallback。
        let backup_ok = settings.patch_backup_path
            .as_deref()
            .map(|p| PathBuf::from(p).exists())
            .unwrap_or(false);
        if !backup_ok {
            if let Ok(latest) = find_latest_backup(&parent_dir, &None) {
                settings.patch_backup_path = Some(latest.to_string_lossy().to_string());
            }
        }
        data_store.update_settings(settings).await.map_err(|e| e.to_string())?;
        
        return Ok(serde_json::json!({
            "success": true,
            "already_patched": true,
            "message": "补丁已经应用过了"
        }));
    }
    
    // 4. 未打补丁：先做备份轮转（最多保留 3 份）
    let mut backup_files: Vec<PathBuf> = fs::read_dir(&parent_dir)
        .map_err(|e| format!("读取目录失败: {}", e))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("extension.js.backup."))
                .unwrap_or(false)
        })
        .collect();
    
    // 按修改时间排序（最早的在前）
    backup_files.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    });
    
    while backup_files.len() >= 3 {
        if let Some(oldest) = backup_files.first() {
            fs::remove_file(oldest)
                .map_err(|e| format!("删除旧备份失败: {}", e))?;
            println!("删除旧备份文件: {:?}", oldest);
            backup_files.remove(0);
        } else {
            break;
        }
    }
    
    // 5. 新建备份文件（仅在确认需要修改后才创建，避免无意义 I/O 与垃圾备份）
    let backup_file = extension_file.with_extension(&format!(
        "js.backup.{}",
        Local::now().format("%Y%m%d_%H%M%S")
    ));
    fs::copy(&extension_file, &backup_file)
        .map_err(|e| format!("备份失败: {}", e))?;
    
    // 6. 应用修改
    let mut modified_content = content.clone();
    let mut modifications: Vec<&str> = vec![];
    
    // 6.1 添加全局 OAuth 回调处理器
    if let Some(captures) = pattern1.captures(&modified_content) {
        let var_name1 = &captures[1];
        let var_name2 = &captures[2];
        let module_name = &captures[3];
        
        if var_name1 == var_name2 {
            // 注入策略：不依赖任何 bundle 标识符（u / class 名），内联 URLSearchParams 解析
            // fragment 提取 access_token，直接转交给实例方法 handleAuthToken。
            // 兼容 Windsurf 1.110+ 把 handleUri 重构为 class static 方法的情形。
            let replacement = format!(
                r#"this._uriHandler.event(async {}=>{{if("/refresh-authentication-session"==={}.path){{(0,{}.refreshAuthenticationSession)()}}else{{try{{const t=new URLSearchParams({}.fragment).get("access_token");if(!t)throw new Error("No access_token in URI fragment");await this.handleAuthToken(t)}}catch(e){{console.error("[Windsurf] Failed to handle OAuth callback:",e)}}}}}})"#,
                var_name1, var_name1, module_name, var_name1
            );
            
            let full_match = captures.get(0).unwrap().as_str();
            modified_content = modified_content.replace(full_match, &replacement);
            modifications.push("OAuth回调处理器");
        }
    }
    
    // 6.2 移除 180 秒超时限制
    if let Some(captures) = pattern2.captures(&modified_content) {
        let reject_var1 = &captures[2];
        let reject_var2 = &captures[3];
        
        if reject_var1 == reject_var2 {
            let full_match = captures.get(0).unwrap().as_str();
            modified_content = modified_content.replace(full_match, "");
            modifications.push("移除超时限制");
        }
    }
    
    // 6.3 跳过切号确认对话框（"Are you sure you want to log in using a different account?"）
    //     将 if 条件整体替换为 `if(true)`，body 仍是原来的 try{...}catch{...} 单语句，语法合法。
    if let Some(captures) = pattern3.captures(&modified_content) {
        let full_match = captures.get(0).unwrap().as_str();
        let replacement = "if(true)";
        modified_content = modified_content.replace(full_match, replacement);
        modifications.push("跳过切号确认对话框");
    }
    
    // 7. 二次校验：dry-run 判定需要改但 replacement 实际没改到任何东西，
    //    说明 pattern 与 replacement 之间存在版本漂移（例如 var_name 校验未通过）。
    //    回滚刚创建的备份，Fail-Fast 报错以便用户/开发者感知。
    if modified_content == content {
        let _ = fs::remove_file(&backup_file);
        return Err("pattern 可匹配但替换后无变化，疑似 pattern/replacement 版本不匹配，请升级工具".to_string());
    }
    
    // 8. 写入修改后的文件
    fs::write(&extension_file, &modified_content)
        .map_err(|e| format!("写入文件失败: {}", e))?;
    
    // 7. 保存补丁状态到设置
    let mut settings = data_store.get_settings().await.map_err(|e| e.to_string())?;
    settings.seamless_switch_enabled = true;
    settings.windsurf_path = Some(windsurf_path.to_string());
    settings.patch_backup_path = Some(backup_file.to_string_lossy().to_string());
    let client_type = settings.windsurf_client_type.clone();
    data_store.update_settings(settings).await.map_err(|e| e.to_string())?;
    
    // 8. 重启Windsurf（仅在进程运行中时才重启）
    let restarted = restart_windsurf(&client_type).await?;
    let message = if restarted {
        "补丁应用成功，客户端正在重启"
    } else {
        "补丁应用成功（客户端未运行，无需重启）"
    };
    
    Ok(serde_json::json!({
        "success": true,
        "modifications": modifications,
        "backup_file": backup_file.to_string_lossy().to_string(),
        "restarted": restarted,
        "message": message
    }))
}

/// 应用无感换号补丁（Tauri 命令）
#[command]
pub async fn apply_seamless_patch(
    windsurf_path: String,
    data_store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    apply_seamless_patch_internal(&windsurf_path, &data_store).await
}

/// 还原无感换号补丁
#[command]
pub async fn restore_seamless_patch(
    data_store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let settings = data_store.get_settings().await.map_err(|e| e.to_string())?;
    
    let windsurf_path = settings.windsurf_path
        .ok_or_else(|| "未找到Windsurf路径".to_string())?;
    
    let extension_file = PathBuf::from(&windsurf_path).join(get_extension_js_relative_path());
    let extension_dir = extension_file.parent()
        .ok_or("无法获取扩展目录")?
        .to_path_buf();
    
    // 尝试找到可用的备份文件
    let backup_path = find_latest_backup(&extension_dir, &settings.patch_backup_path)?;
    
    println!("使用备份文件还原: {:?}", backup_path);
    
    // 还原备份文件
    fs::copy(&backup_path, &extension_file)
        .map_err(|e| format!("还原失败: {} (备份文件: {:?})", e, backup_path))?;
    
    // 更新设置
    let mut settings = data_store.get_settings().await.map_err(|e| e.to_string())?;
    let client_type = settings.windsurf_client_type.clone();
    settings.seamless_switch_enabled = false;
    data_store.update_settings(settings).await.map_err(|e| e.to_string())?;
    
    // 重启Windsurf（仅在进程运行中时才重启）
    let restarted = restart_windsurf(&client_type).await?;
    let message = if restarted {
        "补丁已还原，客户端正在重启"
    } else {
        "补丁已还原（客户端未运行，无需重启）"
    };
    
    Ok(serde_json::json!({
        "success": true,
        "restarted": restarted,
        "message": message,
        "backup_used": backup_path.to_string_lossy().to_string()
    }))
}

/// 查找最新的可用备份文件
fn find_latest_backup(extension_dir: &Path, saved_backup_path: &Option<String>) -> Result<PathBuf, String> {
    // 1. 首先尝试使用设置中保存的备份路径
    if let Some(ref saved_path) = saved_backup_path {
        let saved = PathBuf::from(saved_path);
        if saved.exists() {
            return Ok(saved);
        }
        println!("设置中保存的备份文件不存在: {:?}", saved);
    }
    
    // 2. 查找目录中所有备份文件，按时间排序使用最新的
    let mut backup_files: Vec<PathBuf> = fs::read_dir(extension_dir)
        .map_err(|e| format!("读取目录失败: {}", e))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("extension.js.backup."))
                .unwrap_or(false)
        })
        .collect();
    
    if backup_files.is_empty() {
        return Err("未找到任何备份文件，无法还原。请手动重新安装 Windsurf 或从官方下载 extension.js 文件".to_string());
    }
    
    // 按修改时间排序（最新的在前）
    backup_files.sort_by(|a, b| {
        let time_a = fs::metadata(a).and_then(|m| m.modified()).ok();
        let time_b = fs::metadata(b).and_then(|m| m.modified()).ok();
        time_b.cmp(&time_a)
    });
    
    // 返回最新的备份文件
    Ok(backup_files.remove(0))
}

/// 检查补丁状态
///
/// 判定口径与 `apply_seamless_patch_internal` 保持一致：以"原始 pattern 是否还能匹配"
/// 为主要依据，避免历史上两处使用不同标识字符串导致 UI 与磁盘实际状态脱节。
/// 兼容第三方 / 旧版本工具打过的补丁：只要原始结构已被替换掉就视为"已安装"。
#[command]
pub async fn check_patch_status(
    windsurf_path: String,
) -> Result<serde_json::Value, String> {
    let extension_file = PathBuf::from(&windsurf_path).join(get_extension_js_relative_path());
    
    if !extension_file.exists() {
        return Ok(serde_json::json!({
            "installed": false,
            "error": "extension.js文件不存在"
        }));
    }
    
    let content = fs::read_to_string(&extension_file)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    
    let pattern1 = Regex::new(PATTERN_URI_HANDLER_ORIGINAL)
        .map_err(|e| format!("正则表达式错误: {}", e))?;
    let pattern2 = Regex::new(PATTERN_TIMEOUT_ORIGINAL)
        .map_err(|e| format!("正则表达式错误2: {}", e))?;
    let pattern3 = Regex::new(PATTERN_DIFF_ACCOUNT_PROMPT_ORIGINAL)
        .map_err(|e| format!("正则表达式错误3: {}", e))?;
    
    // 原始 pattern 仍能匹配 = 对应结构原封未动；匹配不上 = 已被补丁改写
    let pattern1_original_present = pattern1.is_match(&content);
    let pattern2_original_present = pattern2.is_match(&content);
    let pattern3_original_present = pattern3.is_match(&content);
    
    // 三条原始 pattern 都不再匹配 = 完整打过当前版本补丁
    // 注：只打过 1+2 的旧补丁 installed=false，让前端同步开关并提示用户重新"启用"升级到完整补丁
    let installed = !pattern1_original_present
        && !pattern2_original_present
        && !pattern3_original_present;
    // 辅助标识：文件里是否含有当前版本注入代码的特征字符串，便于前端区分版本
    let has_oauth_handler = content.contains(CURRENT_VERSION_MARKER);
    let has_timeout_removed = !pattern2_original_present;
    let prompt_bypass_applied = !pattern3_original_present;
    
    Ok(serde_json::json!({
        "installed": installed,
        "current_version": has_oauth_handler,
        "oauth_handler": has_oauth_handler,
        "timeout_removed": has_timeout_removed,
        "prompt_bypass_applied": prompt_bypass_applied
    }))
}

/// 检测指定进程是否正在运行
fn is_process_running(process_name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        
        let exe_name = format!("{}.exe", process_name);
        if let Ok(output) = Command::new("tasklist")
            .creation_flags(CREATE_NO_WINDOW)
            .args(&["/FI", &format!("IMAGENAME eq {}", exe_name), "/NH"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains(&exe_name);
        }
        false
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = Command::new("pgrep")
            .args(&["-f", process_name])
            .output()
        {
            return output.status.success();
        }
        false
    }
}

/// 重启Windsurf（仅在进程运行中时才执行）
/// 返回 Ok(true) 表示执行了重启，Ok(false) 表示进程未运行跳过重启
async fn restart_windsurf(client_type: &str) -> Result<bool, String> {
    let config = get_client_config(client_type);
    
    // 先检测进程是否在运行
    if !is_process_running(config.process_name) {
        println!("[restart] {} 未运行，跳过重启", config.process_name);
        return Ok(false);
    }
    
    println!("[restart] {} 正在运行，执行重启...", config.process_name);
    
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        
        let exe_name = format!("{}.exe", config.process_name);
        
        // 1. 关闭进程
        Command::new("taskkill")
            .creation_flags(CREATE_NO_WINDOW)
            .args(&["/F", "/IM", &exe_name])
            .output()
            .map_err(|e| format!("关闭 {} 失败: {}", config.process_name, e))?;
        
        // 等待进程完全结束
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        // 2. 尝试从开始菜单启动
        let start_menu = std::env::var("APPDATA")
            .map(|p| PathBuf::from(p).join(format!("Microsoft\\Windows\\Start Menu\\Programs\\{}", config.start_menu_folder)))
            .map_err(|e| format!("获取开始菜单路径失败: {}", e))?;
        
        if let Ok(entries) = fs::read_dir(&start_menu) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.file_name().and_then(|n| n.to_str()).map(|n| n.contains(config.process_name)).unwrap_or(false) 
                   && path.extension().and_then(|s| s.to_str()) == Some("lnk") {
                    
                    Command::new("cmd")
                        .creation_flags(CREATE_NO_WINDOW)
                        .args(&["/C", "start", "", &path.to_string_lossy()])
                        .spawn()
                        .map_err(|e| format!("启动 {} 失败: {}", config.process_name, e))?;
                    
                    return Ok(true);
                }
            }
        }
        
        return Err(format!("未找到 {} 快捷方式", config.process_name));
    }
    
    #[cfg(target_os = "macos")]
    {
        // 1. 关闭进程
        Command::new("pkill")
            .args(&["-f", config.process_name])
            .output()
            .map_err(|e| format!("关闭 {} 失败: {}", config.process_name, e))?;
        
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        // 2. 启动
        Command::new("open")
            .args(&["-a", config.macos_app_name])
            .spawn()
            .map_err(|e| format!("启动 {} 失败: {}", config.process_name, e))?;
        
        return Ok(true);
    }
    
    #[cfg(target_os = "linux")]
    {
        // 1. 关闭进程
        Command::new("pkill")
            .args(&["-f", &config.process_name.to_lowercase()])
            .output()
            .map_err(|e| format!("关闭 {} 失败: {}", config.process_name, e))?;
        
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        // 2. 启动
        Command::new(&config.process_name.to_lowercase())
            .spawn()
            .map_err(|e| format!("启动 {} 失败: {}", config.process_name, e))?;
        
        return Ok(true);
    }
    
    #[allow(unreachable_code)]
    Err("不支持的操作系统".to_string())
}
