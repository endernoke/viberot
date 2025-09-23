@echo off
setlocal enabledelayedexpansion

echo Command-Sidekick Installer
echo =========================

REM Check if running as administrator
net session >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo This script requires administrator privileges.
    echo Please run as administrator.
    pause
    exit /b 1
)

echo.
echo Installing Command-Sidekick...

REM Define installation directory
set INSTALL_DIR=%ProgramFiles%\Command-Sidekick
set CONFIG_DIR=%USERPROFILE%\.config\command-sidekick

echo.
echo Creating installation directory: %INSTALL_DIR%
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"

echo.
echo Copying binaries...
if not exist "dist\bin\sidekick-core.exe" (
    echo Error: sidekick-core.exe not found. Please run build.bat first.
    pause
    exit /b 1
)

copy "dist\bin\sidekick-core.exe" "%INSTALL_DIR%\" > nul
copy "dist\bin\WindowsProbe.exe" "%INSTALL_DIR%\" > nul
copy "dist\bin\SpinnerPlugin.exe" "%INSTALL_DIR%\" > nul

echo.
echo Creating config directory: %CONFIG_DIR%
if not exist "%CONFIG_DIR%" mkdir "%CONFIG_DIR%"

echo.
echo Creating default configuration...
(
echo # Command-Sidekick Configuration
echo # Add rules to match commands and trigger actions
echo.
echo [[rule]]
echo command = "npm install*"
echo [rule.action]
echo type = "exec"
echo path = "%INSTALL_DIR%\SpinnerPlugin.exe"
echo.
echo [[rule]]
echo command = "npm ci*"
echo [rule.action]
echo type = "exec"
echo path = "%INSTALL_DIR%\SpinnerPlugin.exe"
echo.
echo [[rule]]
echo command = "docker build*"
echo [rule.action]
echo type = "exec"
echo path = "%INSTALL_DIR%\SpinnerPlugin.exe"
) > "%CONFIG_DIR%\config.toml"

echo.
echo Creating Windows services...

REM Create Core Service
sc create "CommandSidekickCore" binPath= "\"%INSTALL_DIR%\sidekick-core.exe\"" start= auto DisplayName= "Command Sidekick Core Service" > nul
if %ERRORLEVEL% neq 0 (
    echo Warning: Failed to create Core Service. It may already exist.
)

REM Create Windows Probe Service  
sc create "CommandSidekickProbe" binPath= "\"%INSTALL_DIR%\WindowsProbe.exe\"" start= auto DisplayName= "Command Sidekick Windows Probe" depend= CommandSidekickCore > nul
if %ERRORLEVEL% neq 0 (
    echo Warning: Failed to create Windows Probe Service. It may already exist.
)

echo.
echo Starting services...
sc start CommandSidekickCore > nul
timeout /t 2 /nobreak > nul
sc start CommandSidekickProbe > nul

echo.
echo Installation completed!
echo.
echo Services installed:
echo   - Command Sidekick Core Service
echo   - Command Sidekick Windows Probe
echo.
echo Configuration file: %CONFIG_DIR%\config.toml
echo.
echo You can now run 'npm install' or other configured commands to see the spinner!
echo.
echo To uninstall, run: uninstall.bat
pause