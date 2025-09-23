@echo off
echo Building Command-Sidekick...

echo.
echo [1/3] Building Rust Core Service...
cargo build --release
if %ERRORLEVEL% neq 0 (
    echo Failed to build Rust core service
    exit /b 1
)

echo.
echo [2/3] Building .NET Windows Probe...
cd windows-probe
dotnet publish -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true
if %ERRORLEVEL% neq 0 (
    echo Failed to build Windows probe
    exit /b 1
)
cd ..

echo.
echo [3/3] Building WPF Spinner Plugin...
cd spinner-plugin
dotnet publish -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true
if %ERRORLEVEL% neq 0 (
    echo Failed to build spinner plugin
    exit /b 1
)
cd ..

echo.
echo Creating output directory...
if not exist "dist" mkdir dist
if not exist "dist\bin" mkdir dist\bin

echo.
echo Copying binaries...
copy "target\release\sidekick-core.exe" "dist\bin\" > nul
copy "windows-probe\bin\Release\net8.0-windows\win-x64\publish\WindowsProbe.exe" "dist\bin\" > nul
copy "spinner-plugin\bin\Release\net8.0-windows\win-x64\publish\SpinnerPlugin.exe" "dist\bin\" > nul

echo.
echo Build completed successfully!
echo Binaries are available in the 'dist\bin' directory.
echo.
echo To install, run: install.bat