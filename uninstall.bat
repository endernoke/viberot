@echo off
echo Command-Sidekick Uninstaller
echo ============================

REM Check if running as administrator
net session >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo This script requires administrator privileges.
    echo Please run as administrator.
    pause
    exit /b 1
)

echo.
echo Stopping services...
sc stop CommandSidekickProbe > nul 2>&1
sc stop CommandSidekickCore > nul 2>&1

echo.
echo Removing services...
sc delete CommandSidekickProbe > nul 2>&1
sc delete CommandSidekickCore > nul 2>&1

set INSTALL_DIR=%ProgramFiles%\Command-Sidekick

echo.
echo Removing installation directory: %INSTALL_DIR%
if exist "%INSTALL_DIR%" (
    rmdir /s /q "%INSTALL_DIR%"
    echo Installation directory removed.
) else (
    echo Installation directory not found.
)

echo.
echo Uninstallation completed!
echo.
echo Note: Configuration files in %USERPROFILE%\.config\command-sidekick were preserved.
echo You can manually delete them if desired.
pause