@echo off
echo Starting Windsurf Account Manager Auto Build Monitor...
echo.

:: 检查 PowerShell 是否可用
powershell -Command "Get-Host" >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo PowerShell is not available. Please install PowerShell.
    pause
    exit /b 1
)

:: 设置执行策略（如果需要）
powershell -Command "Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force" >nul 2>&1

echo Choose an option:
echo 1. Start file watcher (auto build on changes)
echo 2. Build now (admin version)
echo 3. Build web version only
echo 4. Start development server
echo 5. Exit
echo.

set /p choice="Enter your choice (1-5): "

if "%choice%"=="1" (
    echo Starting file watcher...
    powershell -File "auto-build.ps1" -Watch
) else if "%choice%"=="2" (
    echo Building admin version...
    powershell -File "auto-build.ps1" -Build -Target admin
) else if "%choice%"=="3" (
    echo Building web version...
    powershell -File "auto-build.ps1" -Build -Target web
) else if "%choice%"=="4" (
    echo Starting development server...
    powershell -File "auto-build.ps1" -Dev
) else if "%choice%"=="5" (
    echo Goodbye!
    exit /b 0
) else (
    echo Invalid choice. Please try again.
    pause
    goto :eof
)

pause
