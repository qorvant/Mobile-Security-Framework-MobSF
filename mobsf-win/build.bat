@echo off
setlocal
REM ---------------------------------------------------------------------------
REM build.bat — build the MobSF-Win single-executable desktop app (Tauri v2).
REM Usage:
REM   build.bat            -> release single .exe  (target\release\mobsf-win.exe)
REM   build.bat installer  -> release + NSIS installer (needs `cargo tauri` CLI)
REM ---------------------------------------------------------------------------
cd /d "%~dp0"

where node >nul 2>nul
if %errorlevel%==0 (
    echo [1/3] Generating Windows icon from source logo...
    node scripts/gen_icon.mjs
) else (
    echo [1/3] node.exe not found; skipping icon regen (app\icons\icon.ico unchanged)
)

if /I "%1"=="installer" (
    echo [2/3] Building release installer via `cargo tauri build`...
    cargo tauri build
    if %errorlevel% neq 0 (
        echo.
        echo Tauri build failed. Ensure `cargo tauri` CLI is installed:
        echo   cargo install tauri-cli --version "^2"
        exit /b 1
    )
    echo [3/3] Done. Installer under target\tauri\mobsf-win\.
    goto :eof
)

echo [2/3] Building release binary (first run downloads/compiles deps; may take minutes)...
REM --features custom-protocol is REQUIRED: without it tauri/build.rs sets
REM `dev = true`, the frontend is not embedded and the WebView loads devUrl
REM (http://localhost:5173) -> ERR_CONNECTION_REFUSED in the release exe.
cargo build --release -p mobsf-win --features custom-protocol
if %errorlevel% neq 0 (
    echo.
    echo Build failed. See errors above.
    exit /b 1
)

echo [3/3] Done. Executable:
for %%f in ("target\release\mobsf-win.exe") do (
    echo   %%~ff  (%%~zf bytes)
)

echo.
echo NOTE: the .exe needs Microsoft Edge WebView2 Runtime, preinstalled on Win10/11.
echo       For a packaged installer instead, run:  build.bat installer
endlocal
