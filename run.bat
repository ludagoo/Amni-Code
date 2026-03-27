@echo off
title Amni-Code
echo.
echo   Amni-Code - AI Coding Agent
echo   ============================
echo.
where cargo >nul 2>nul
if errorlevel 1 (
    echo   Rust not found. Installing via rustup...
    echo.
    powershell -Command "Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile '%TEMP%\rustup-init.exe'"
    "%TEMP%\rustup-init.exe" -y --default-toolchain stable
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
    echo   Rust installed. Continuing...
    echo.
)
if not exist "target\release\amni.exe" (
    echo   Building Amni-Code - first run...
    echo.
    cargo build --release
    if errorlevel 1 (
        echo   Build failed. Check errors above.
        pause
        exit /b 1
    )
    echo.
    echo   Build complete!
    echo.
)
echo   Starting Amni-Code...
echo.
target\release\amni.exe