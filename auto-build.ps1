# 自动构建监控脚本
# 监控源代码文件变化，自动触发构建

param(
    [switch]$Watch,
    [switch]$Build,
    [switch]$Dev,
    [string]$Target = "all"
)

# 颜色输出函数
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

function Write-Info($message) {
    Write-ColorOutput Cyan "ℹ️ $message"
}

function Write-Success($message) {
    Write-ColorOutput Green "✅ $message"
}

function Write-Warning($message) {
    Write-ColorOutput Yellow "⚠️ $message"
}

function Write-Error($message) {
    Write-ColorOutput Red "❌ $message"
}

# 检查依赖
function Test-Dependencies {
    Write-Info "检查构建依赖..."
    
    # 检查 Node.js
    try {
        $nodeVersion = node --version
        Write-Success "Node.js: $nodeVersion"
    } catch {
        Write-Error "Node.js 未安装或不在 PATH 中"
        return $false
    }
    
    # 检查 npm
    try {
        $npmVersion = npm --version
        Write-Success "npm: $npmVersion"
    } catch {
        Write-Error "npm 未安装或不在 PATH 中"
        return $false
    }
    
    # 检查 Rust
    try {
        $rustVersion = rustc --version
        Write-Success "Rust: $rustVersion"
    } catch {
        Write-Error "Rust 未安装或不在 PATH 中"
        return $false
    }
    
    # 检查 Tauri CLI
    try {
        $tauriVersion = npm list -g @tauri-apps/cli --depth=0 2>$null
        if ($tauriVersion -match "@tauri-apps/cli") {
            Write-Success "Tauri CLI: 已安装"
        } else {
            Write-Warning "Tauri CLI 未全局安装，尝试本地安装..."
        }
    } catch {
        Write-Warning "无法检查 Tauri CLI 状态"
    }
    
    return $true
}

# 安装依赖
function Install-Dependencies {
    Write-Info "安装项目依赖..."
    
    try {
        npm install
        Write-Success "依赖安装完成"
        return $true
    } catch {
        Write-Error "依赖安装失败: $_"
        return $false
    }
}

# 执行构建
function Start-Build {
    param([string]$BuildTarget = "all")
    
    Write-Info "开始构建 (目标: $BuildTarget)..."
    
    $startTime = Get-Date
    
    try {
        switch ($BuildTarget.ToLower()) {
            "dev" {
                Write-Info "启动开发服务器..."
                npm run dev
            }
            "web" {
                Write-Info "构建 Web 版本..."
                npm run build
            }
            "tauri" {
                Write-Info "构建 Tauri 应用..."
                npm run tauri build
            }
            "admin" {
                Write-Info "构建带管理员权限的应用..."
                & ".\build_with_admin.bat"
            }
            default {
                Write-Info "构建完整应用..."
                & ".\build_with_admin.bat"
            }
        }
        
        $endTime = Get-Date
        $duration = $endTime - $startTime
        Write-Success "构建完成! 耗时: $($duration.ToString('mm\:ss'))"
        
        # 显示输出文件
        if (Test-Path "src-tauri\target\release") {
            Write-Info "构建输出:"
            Get-ChildItem "src-tauri\target\release" -Recurse -Include "*.exe", "*.msi" | ForEach-Object {
                Write-Output "  📦 $($_.FullName)"
            }
        }
        
        return $true
    } catch {
        Write-Error "构建失败: $_"
        return $false
    }
}

# 文件监控
function Start-FileWatcher {
    Write-Info "启动文件监控..."
    Write-Info "监控目录: src/, src-tauri/, package.json, vite.config.ts"
    Write-Info "按 Ctrl+C 停止监控"
    
    # 创建文件系统监控器
    $watcher = New-Object System.IO.FileSystemWatcher
    $watcher.Path = $PWD
    $watcher.IncludeSubdirectories = $true
    $watcher.EnableRaisingEvents = $true
    
    # 监控的文件类型
    $watchExtensions = @("*.ts", "*.js", "*.vue", "*.rs", "*.json", "*.toml", "*.html", "*.css", "*.scss")
    
    # 忽略的目录
    $ignorePatterns = @(
        "node_modules",
        "target",
        "dist",
        ".git",
        ".vscode"
    )
    
    $lastBuildTime = Get-Date
    $buildCooldown = 5 # 秒
    
    # 事件处理
    $action = {
        $path = $Event.SourceEventArgs.FullPath
        $changeType = $Event.SourceEventArgs.ChangeType
        $name = $Event.SourceEventArgs.Name
        
        # 检查是否应该忽略此文件
        $shouldIgnore = $false
        foreach ($pattern in $ignorePatterns) {
            if ($path -like "*$pattern*") {
                $shouldIgnore = $true
                break
            }
        }
        
        if ($shouldIgnore) { return }
        
        # 检查文件扩展名
        $shouldWatch = $false
        foreach ($ext in $watchExtensions) {
            if ($name -like $ext) {
                $shouldWatch = $true
                break
            }
        }
        
        if (-not $shouldWatch) { return }
        
        # 防抖动 - 避免频繁构建
        $currentTime = Get-Date
        if (($currentTime - $script:lastBuildTime).TotalSeconds -lt $buildCooldown) {
            return
        }
        
        Write-Info "检测到文件变化: $name ($changeType)"
        Write-Info "触发自动构建..."
        
        $script:lastBuildTime = $currentTime
        
        # 异步执行构建
        Start-Job -ScriptBlock {
            param($projectPath, $target)
            Set-Location $projectPath
            & ".\auto-build.ps1" -Build -Target $target
        } -ArgumentList $PWD, $Target | Out-Null
    }
    
    # 注册事件
    Register-ObjectEvent -InputObject $watcher -EventName "Changed" -Action $action
    Register-ObjectEvent -InputObject $watcher -EventName "Created" -Action $action
    Register-ObjectEvent -InputObject $watcher -EventName "Deleted" -Action $action
    
    try {
        # 保持脚本运行
        while ($true) {
            Start-Sleep -Seconds 1
        }
    } finally {
        # 清理
        $watcher.EnableRaisingEvents = $false
        $watcher.Dispose()
        Get-EventSubscriber | Unregister-Event
        Get-Job | Remove-Job -Force
        Write-Info "文件监控已停止"
    }
}

# 主逻辑
function Main {
    Write-Info "Windsurf Account Manager - 自动构建工具"
    Write-Info "========================================"
    
    # 检查是否在项目根目录
    if (-not (Test-Path "package.json") -or -not (Test-Path "src-tauri")) {
        Write-Error "请在项目根目录运行此脚本"
        exit 1
    }
    
    # 检查依赖
    if (-not (Test-Dependencies)) {
        Write-Error "依赖检查失败，请安装必要的工具"
        exit 1
    }
    
    # 根据参数执行相应操作
    if ($Build) {
        # 安装依赖
        if (-not (Install-Dependencies)) {
            exit 1
        }
        
        # 执行构建
        if (-not (Start-Build -BuildTarget $Target)) {
            exit 1
        }
    } elseif ($Dev) {
        # 安装依赖
        if (-not (Install-Dependencies)) {
            exit 1
        }
        
        # 启动开发服务器
        Start-Build -BuildTarget "dev"
    } elseif ($Watch) {
        # 启动文件监控
        Start-FileWatcher
    } else {
        # 显示帮助信息
        Write-Info "用法:"
        Write-Info "  .\auto-build.ps1 -Build [-Target <target>]  # 执行构建"
        Write-Info "  .\auto-build.ps1 -Dev                       # 启动开发服务器"
        Write-Info "  .\auto-build.ps1 -Watch [-Target <target>]  # 启动文件监控"
        Write-Info ""
        Write-Info "构建目标:"
        Write-Info "  all     - 完整构建 (默认)"
        Write-Info "  web     - 仅构建 Web 版本"
        Write-Info "  tauri   - 仅构建 Tauri 应用"
        Write-Info "  admin   - 构建带管理员权限的应用"
        Write-Info "  dev     - 启动开发服务器"
        Write-Info ""
        Write-Info "示例:"
        Write-Info "  .\auto-build.ps1 -Build -Target admin"
        Write-Info "  .\auto-build.ps1 -Watch -Target tauri"
    }
}

# 执行主函数
Main
