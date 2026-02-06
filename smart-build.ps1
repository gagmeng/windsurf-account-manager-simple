# 智能自动构建系统
# 支持 Git 集成、智能触发和构建优化

param(
    [switch]$Init,
    [switch]$Start,
    [switch]$Stop,
    [switch]$Status,
    [string]$Config = "auto-build-config.json"
)

# 全局变量
$script:BuildProcess = $null
$script:WatcherJob = $null
$script:ConfigData = $null

# 加载配置
function Load-Config {
    param([string]$ConfigPath)
    
    if (-not (Test-Path $ConfigPath)) {
        Write-Warning "配置文件不存在: $ConfigPath"
        return $null
    }
    
    try {
        $config = Get-Content $ConfigPath -Raw | ConvertFrom-Json
        Write-Host "✅ 配置加载成功" -ForegroundColor Green
        return $config
    } catch {
        Write-Error "配置文件格式错误: $_"
        return $null
    }
}

# 检查 Git 状态
function Get-GitStatus {
    try {
        $gitStatus = git status --porcelain 2>$null
        $currentBranch = git branch --show-current 2>$null
        $lastCommit = git log -1 --format="%h %s" 2>$null
        
        return @{
            HasChanges = $gitStatus.Length -gt 0
            Branch = $currentBranch
            LastCommit = $lastCommit
            Changes = $gitStatus
        }
    } catch {
        return @{
            HasChanges = $false
            Branch = "unknown"
            LastCommit = "unknown"
            Changes = @()
        }
    }
}

# 智能构建决策
function Should-TriggerBuild {
    param(
        [string]$ChangedFile,
        [object]$Config
    )
    
    # 检查文件是否在监控路径中
    $shouldWatch = $false
    foreach ($pattern in $Config.autoTrigger.watchPaths) {
        $globPattern = $pattern -replace '\*\*', '*' -replace '/', '\'
        if ($ChangedFile -like $globPattern) {
            $shouldWatch = $true
            break
        }
    }
    
    if (-not $shouldWatch) { return $false }
    
    # 检查文件是否在忽略列表中
    foreach ($pattern in $Config.autoTrigger.ignorePaths) {
        $globPattern = $pattern -replace '\*\*', '*' -replace '/', '\'
        if ($ChangedFile -like $globPattern) {
            return $false
        }
    }
    
    # 检查文件扩展名
    $extension = [System.IO.Path]::GetExtension($ChangedFile)
    if ($Config.autoTrigger.fileExtensions -contains $extension) {
        return $true
    }
    
    return $false
}

# 执行构建
function Start-SmartBuild {
    param(
        [string]$Target,
        [object]$Config
    )
    
    $buildConfig = $Config.buildTargets.$Target
    if (-not $buildConfig) {
        Write-Error "未知的构建目标: $Target"
        return $false
    }
    
    Write-Host "🔨 开始构建: $($buildConfig.description)" -ForegroundColor Cyan
    
    $startTime = Get-Date
    
    try {
        # 执行构建命令
        $command = $buildConfig.command
        if ($command.StartsWith('./') -or $command.StartsWith('.\')) {
            # 批处理文件
            $result = & cmd /c $command
        } else {
            # PowerShell 命令
            $result = Invoke-Expression $command
        }
        
        $endTime = Get-Date
        $duration = $endTime - $startTime
        
        Write-Host "✅ 构建完成! 耗时: $($duration.ToString('mm\:ss'))" -ForegroundColor Green
        
        # 发送通知
        if ($Config.notifications.enabled -and $Config.notifications.showBuildComplete) {
            Show-Notification "构建完成" "目标: $Target, 耗时: $($duration.ToString('mm\:ss'))"
        }
        
        # Git 自动提交
        if ($Config.github.autoCommit) {
            $gitStatus = Get-GitStatus
            if ($gitStatus.HasChanges) {
                $commitMessage = $Config.github.commitMessage -replace '\{timestamp\}', (Get-Date -Format "yyyy-MM-dd HH:mm:ss")
                git add .
                git commit -m $commitMessage
                Write-Host "📝 自动提交: $commitMessage" -ForegroundColor Yellow
                
                if ($Config.github.autoPush) {
                    git push
                    Write-Host "🚀 自动推送到远程仓库" -ForegroundColor Yellow
                }
            }
        }
        
        return $true
    } catch {
        Write-Error "构建失败: $_"
        
        # 发送错误通知
        if ($Config.notifications.enabled -and $Config.notifications.showBuildError) {
            Show-Notification "构建失败" "目标: $Target, 错误: $_"
        }
        
        return $false
    }
}

# 显示系统通知
function Show-Notification {
    param(
        [string]$Title,
        [string]$Message
    )
    
    try {
        # 使用 Windows 10/11 通知系统
        Add-Type -AssemblyName System.Windows.Forms
        $notification = New-Object System.Windows.Forms.NotifyIcon
        $notification.Icon = [System.Drawing.SystemIcons]::Information
        $notification.BalloonTipTitle = $Title
        $notification.BalloonTipText = $Message
        $notification.Visible = $true
        $notification.ShowBalloonTip(3000)
        
        # 清理
        Start-Sleep -Seconds 3
        $notification.Dispose()
    } catch {
        # 如果通知失败，至少在控制台显示
        Write-Host "🔔 $Title : $Message" -ForegroundColor Magenta
    }
}

# 启动文件监控
function Start-FileMonitor {
    param([object]$Config)
    
    Write-Host "👀 启动智能文件监控..." -ForegroundColor Cyan
    Write-Host "监控目标: $($Config.autoTrigger.buildTarget)" -ForegroundColor Gray
    Write-Host "冷却时间: $($Config.autoTrigger.buildCooldown) 秒" -ForegroundColor Gray
    
    $script:WatcherJob = Start-Job -ScriptBlock {
        param($ProjectPath, $ConfigData)
        
        Set-Location $ProjectPath
        
        # 创建文件监控器
        $watcher = New-Object System.IO.FileSystemWatcher
        $watcher.Path = $ProjectPath
        $watcher.IncludeSubdirectories = $true
        $watcher.EnableRaisingEvents = $true
        
        $lastBuildTime = Get-Date
        $buildCooldown = $ConfigData.autoTrigger.buildCooldown
        
        # 事件处理
        $action = {
            $path = $Event.SourceEventArgs.FullPath
            $name = $Event.SourceEventArgs.Name
            $relativePath = $path.Replace($ProjectPath, "").TrimStart('\')
            
            # 智能构建决策
            $shouldBuild = $false
            
            # 检查监控路径
            foreach ($pattern in $ConfigData.autoTrigger.watchPaths) {
                $globPattern = $pattern -replace '\*\*', '*' -replace '/', '\'
                if ($relativePath -like $globPattern) {
                    $shouldBuild = $true
                    break
                }
            }
            
            if (-not $shouldBuild) { return }
            
            # 检查忽略路径
            foreach ($pattern in $ConfigData.autoTrigger.ignorePaths) {
                $globPattern = $pattern -replace '\*\*', '*' -replace '/', '\'
                if ($relativePath -like $globPattern) {
                    return
                }
            }
            
            # 检查文件扩展名
            $extension = [System.IO.Path]::GetExtension($name)
            if ($ConfigData.autoTrigger.fileExtensions -notcontains $extension) {
                return
            }
            
            # 防抖动
            $currentTime = Get-Date
            if (($currentTime - $script:lastBuildTime).TotalSeconds -lt $buildCooldown) {
                return
            }
            
            $script:lastBuildTime = $currentTime
            
            # 输出变化信息
            Write-Output "📁 文件变化: $relativePath"
            Write-Output "🔨 触发构建: $($ConfigData.autoTrigger.buildTarget)"
            
            # 触发构建
            & "$ProjectPath\smart-build.ps1" -Build -Target $ConfigData.autoTrigger.buildTarget
        }
        
        # 注册事件
        Register-ObjectEvent -InputObject $watcher -EventName "Changed" -Action $action
        Register-ObjectEvent -InputObject $watcher -EventName "Created" -Action $action
        
        # 保持运行
        try {
            while ($true) {
                Start-Sleep -Seconds 1
            }
        } finally {
            $watcher.EnableRaisingEvents = $false
            $watcher.Dispose()
            Get-EventSubscriber | Unregister-Event
        }
    } -ArgumentList $PWD, $Config
    
    Write-Host "✅ 文件监控已启动 (Job ID: $($script:WatcherJob.Id))" -ForegroundColor Green
    Write-Host "按 Ctrl+C 或运行 'smart-build.ps1 -Stop' 来停止监控" -ForegroundColor Yellow
}

# 停止监控
function Stop-FileMonitor {
    if ($script:WatcherJob) {
        Stop-Job $script:WatcherJob
        Remove-Job $script:WatcherJob
        Write-Host "🛑 文件监控已停止" -ForegroundColor Red
        $script:WatcherJob = $null
    } else {
        Write-Host "没有运行中的文件监控" -ForegroundColor Yellow
    }
}

# 显示状态
function Show-Status {
    param([object]$Config)
    
    Write-Host "📊 智能构建系统状态" -ForegroundColor Cyan
    Write-Host "=====================" -ForegroundColor Cyan
    
    # Git 状态
    $gitStatus = Get-GitStatus
    Write-Host "Git 分支: $($gitStatus.Branch)" -ForegroundColor White
    Write-Host "最后提交: $($gitStatus.LastCommit)" -ForegroundColor White
    Write-Host "Has Changes: $(if($gitStatus.HasChanges) { 'Yes' } else { 'No' })" -ForegroundColor White
    
    # 配置状态
    Write-Host "Auto Trigger: $(if($Config.autoTrigger.enabled) { 'Enabled' } else { 'Disabled' })" -ForegroundColor White
    Write-Host "Build Target: $($Config.autoTrigger.buildTarget)" -ForegroundColor White
    Write-Host "Monitor Status: $(if($script:WatcherJob) { 'Running' } else { 'Stopped' })" -ForegroundColor White
    
    # 监控路径
    Write-Host "监控路径:" -ForegroundColor White
    foreach ($path in $Config.autoTrigger.watchPaths) {
        Write-Host "  - $path" -ForegroundColor Gray
    }
}

# 初始化项目
function Initialize-Project {
    Write-Host "🚀 初始化智能构建系统..." -ForegroundColor Cyan
    
    # 检查必要文件
    $requiredFiles = @("package.json", "src-tauri")
    foreach ($file in $requiredFiles) {
        if (-not (Test-Path $file)) {
            Write-Error "缺少必要文件/目录: $file"
            return $false
        }
    }
    
    # 创建默认配置（如果不存在）
    if (-not (Test-Path $Config)) {
        Write-Host "📝 创建默认配置文件..." -ForegroundColor Yellow
        # 配置文件已经在前面创建了
    }
    
    # 检查 Git
    if (-not (Test-Path ".git")) {
        Write-Warning "项目未初始化 Git 仓库"
        $initGit = Read-Host "是否初始化 Git 仓库? (y/N)"
        if ($initGit -eq 'y' -or $initGit -eq 'Y') {
            git init
            git add .
            git commit -m "Initial commit"
            Write-Host "✅ Git 仓库初始化完成" -ForegroundColor Green
        }
    }
    
    Write-Host "✅ 智能构建系统初始化完成!" -ForegroundColor Green
    return $true
}

# 主函数
function Main {
    Write-Host "🤖 Windsurf Account Manager - 智能构建系统" -ForegroundColor Magenta
    Write-Host "=============================================" -ForegroundColor Magenta
    
    # 加载配置
    $script:ConfigData = Load-Config $Config
    if (-not $script:ConfigData) {
        Write-Error "无法加载配置文件"
        exit 1
    }
    
    # 根据参数执行操作
    switch ($true) {
        $Init {
            Initialize-Project
        }
        $Start {
            if ($script:ConfigData.autoTrigger.enabled) {
                Start-FileMonitor $script:ConfigData
                
                # 保持脚本运行
                try {
                    while ($script:WatcherJob -and $script:WatcherJob.State -eq "Running") {
                        Start-Sleep -Seconds 1
                    }
                } catch {
                    Write-Host "监控被中断" -ForegroundColor Yellow
                } finally {
                    Stop-FileMonitor
                }
            } else {
                Write-Warning "自动触发已禁用，请检查配置文件"
            }
        }
        $Stop {
            Stop-FileMonitor
        }
        $Status {
            Show-Status $script:ConfigData
        }
        default {
            Write-Host "用法:" -ForegroundColor White
            Write-Host "  .\smart-build.ps1 -Init                    # 初始化项目" -ForegroundColor Gray
            Write-Host "  .\smart-build.ps1 -Start                   # 启动智能监控" -ForegroundColor Gray
            Write-Host "  .\smart-build.ps1 -Stop                    # 停止监控" -ForegroundColor Gray
            Write-Host "  .\smart-build.ps1 -Status                  # 显示状态" -ForegroundColor Gray
            Write-Host "  .\smart-build.ps1 -Config [path]           # 指定配置文件" -ForegroundColor Gray
        }
    }
}

# 执行主函数
Main
