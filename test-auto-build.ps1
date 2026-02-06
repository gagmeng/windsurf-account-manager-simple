# 简单的自动构建测试脚本
param(
    [switch]$Test,
    [switch]$Watch
)

function Test-BuildSystem {
    Write-Host "Testing Auto Build System..." -ForegroundColor Cyan
    
    # 检查必要文件
    $files = @("package.json", "auto-build-config.json", "build_with_admin.bat")
    foreach ($file in $files) {
        if (Test-Path $file) {
            Write-Host "✅ Found: $file" -ForegroundColor Green
        } else {
            Write-Host "❌ Missing: $file" -ForegroundColor Red
        }
    }
    
    # 检查 Node.js
    try {
        $nodeVersion = node --version
        Write-Host "✅ Node.js: $nodeVersion" -ForegroundColor Green
    } catch {
        Write-Host "❌ Node.js not found" -ForegroundColor Red
    }
    
    # 检查 npm
    try {
        $npmVersion = npm --version
        Write-Host "✅ npm: $npmVersion" -ForegroundColor Green
    } catch {
        Write-Host "❌ npm not found" -ForegroundColor Red
    }
    
    # 测试配置文件
    try {
        $config = Get-Content "auto-build-config.json" -Raw | ConvertFrom-Json
        Write-Host "✅ Config loaded successfully" -ForegroundColor Green
        Write-Host "   Auto trigger: $($config.autoTrigger.enabled)" -ForegroundColor Gray
        Write-Host "   Build target: $($config.autoTrigger.buildTarget)" -ForegroundColor Gray
    } catch {
        Write-Host "❌ Config file error: $_" -ForegroundColor Red
    }
}

function Start-SimpleWatch {
    Write-Host "Starting simple file watcher..." -ForegroundColor Cyan
    Write-Host "Watching for changes in src/ and src-tauri/" -ForegroundColor Gray
    Write-Host "Press Ctrl+C to stop" -ForegroundColor Yellow
    
    $watcher = New-Object System.IO.FileSystemWatcher
    $watcher.Path = $PWD
    $watcher.IncludeSubdirectories = $true
    $watcher.EnableRaisingEvents = $true
    
    $action = {
        $path = $Event.SourceEventArgs.FullPath
        $name = $Event.SourceEventArgs.Name
        $changeType = $Event.SourceEventArgs.ChangeType
        
        # 只监控特定文件类型
        $extensions = @(".ts", ".js", ".vue", ".rs", ".json")
        $extension = [System.IO.Path]::GetExtension($name)
        
        if ($extensions -contains $extension) {
            $relativePath = $path.Replace($PWD, "").TrimStart('\')
            Write-Host "📁 File changed: $relativePath ($changeType)" -ForegroundColor Yellow
            Write-Host "🔨 Would trigger build here..." -ForegroundColor Cyan
        }
    }
    
    Register-ObjectEvent -InputObject $watcher -EventName "Changed" -Action $action
    Register-ObjectEvent -InputObject $watcher -EventName "Created" -Action $action
    
    try {
        while ($true) {
            Start-Sleep -Seconds 1
        }
    } finally {
        $watcher.EnableRaisingEvents = $false
        $watcher.Dispose()
        Get-EventSubscriber | Unregister-Event
        Write-Host "File watcher stopped" -ForegroundColor Red
    }
}

if ($Test) {
    Test-BuildSystem
} elseif ($Watch) {
    Start-SimpleWatch
} else {
    Write-Host "Usage:" -ForegroundColor White
    Write-Host "  .\test-auto-build.ps1 -Test    # Test the build system" -ForegroundColor Gray
    Write-Host "  .\test-auto-build.ps1 -Watch   # Start simple file watcher" -ForegroundColor Gray
}
