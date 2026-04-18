use tauri::command;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use regex::Regex;
use chrono::Local;
use std::sync::Arc;
use tauri::State;
use crate::repository::DataStore;

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
    
    // 1. 管理备份文件（最多保留3份）
    let parent_dir = extension_file.parent()
        .ok_or("无法获取父目录")?;
    
    // 查找所有现有备份文件
    let mut backup_files: Vec<PathBuf> = fs::read_dir(parent_dir)
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
    
    // 如果备份文件数量达到3个或更多，删除最早的备份
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
    
    // 创建新的备份文件
    let backup_file = extension_file.with_extension(&format!(
        "js.backup.{}",
        Local::now().format("%Y%m%d_%H%M%S")
    ));
    
    fs::copy(&extension_file, &backup_file)
        .map_err(|e| format!("备份失败: {}", e))?;
    
    // 2. 读取文件内容
    let content = fs::read_to_string(&extension_file)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    
    let mut modified_content = content.clone();
    let mut modifications = vec![];
    
    // 3. 应用修改1: 添加全局 OAuth 回调处理器
    let pattern1_str = r#"this\._uriHandler\.event\((\w+)=>\{"/refresh-authentication-session"===(\w+)\.path&&\(0,(\w+)\.refreshAuthenticationSession\)\(\)\}\)"#;
    let pattern1 = Regex::new(pattern1_str)
        .map_err(|e| format!("正则表达式错误: {}", e))?;
    
    if let Some(captures) = pattern1.captures(&modified_content) {
        let var_name1 = &captures[1];
        let var_name2 = &captures[2];
        let module_name = &captures[3];
        
        // 检查两个变量名是否相同
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
    
    // 4. 应用修改2: 移除180秒超时限制
    let pattern2_str = r#",new Promise\(\((\w+),(\w+)\)=>setTimeout\(\(\)=>\{(\w+)\(new (\w+)\)\},18e4\)\)"#;
    let pattern2 = Regex::new(pattern2_str)
        .map_err(|e| format!("正则表达式错误2: {}", e))?;
    
    if let Some(captures) = pattern2.captures(&modified_content) {
        let reject_var1 = &captures[2];  // 第二个参数
        let reject_var2 = &captures[3];  // setTimeout中的变量
        
        // 检查是否是同一个reject变量
        if reject_var1 == reject_var2 {
            let full_match = captures.get(0).unwrap().as_str();
            modified_content = modified_content.replace(full_match, "");
            modifications.push("移除超时限制");
        }
    }
    
    // 5. 验证修改
    if modified_content == content {
        // 如果内容没有变化，说明已经打过补丁
        return Ok(serde_json::json!({
            "success": true,
            "already_patched": true,
            "message": "补丁已经应用过了"
        }));
    }
    
    // 6. 写入修改后的文件
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
    
    // 检查是否包含补丁标识
    let has_oauth_handler = content.contains("Failed to handle OAuth callback");
    let has_timeout_removed = !content.contains("18e4");
    
    Ok(serde_json::json!({
        "installed": has_oauth_handler,
        "oauth_handler": has_oauth_handler,
        "timeout_removed": has_timeout_removed
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
