@echo off
title Amni-Code One-Click Setup
echo.
echo   ============================================
echo     Amni-Code - One-Click Install
echo   ============================================
echo.
echo   This will clone, build, and launch Amni-Code.
echo.
where git >nul 2>nul
if errorlevel 1 (
    echo   ERROR: Git not found. Install from https://git-scm.com
    pause
    exit /b 1
)
if not exist "Amni-Code" (
    echo   Cloning Amni-Code...
    git clone https://github.com/anmire/Amni-Code.git
    if errorlevel 1 (
        echo   ERROR: Clone failed. Check your internet connection.
        pause
        exit /b 1
    )
)
cd Amni-Code
echo.
echo   --- API Key Setup ---
echo.
echo   Amni-Code defaults to xAI Grok. Enter your API key below.
echo   Press Enter to skip any provider you don't use.
echo.
set "XAI_KEY="
set "OPENAI_KEY="
set "ANTHROPIC_KEY="
set /p XAI_KEY="  xAI API Key (xai-...): "
set /p OPENAI_KEY="  OpenAI API Key (sk-...): "
set /p ANTHROPIC_KEY="  Anthropic API Key (sk-ant-...): "
echo.
if not "%XAI_KEY%"=="" (
    echo XAI_API_KEY=%XAI_KEY%> .env
    echo   Saved xAI key.
)
if not "%OPENAI_KEY%"=="" (
    echo OPENAI_API_KEY=%OPENAI_KEY%>> .env
    echo   Saved OpenAI key.
)
if not "%ANTHROPIC_KEY%"=="" (
    echo ANTHROPIC_API_KEY=%ANTHROPIC_KEY%>> .env
    echo   Saved Anthropic key.
)
echo.
where cargo >nul 2>nul
if errorlevel 1 (
    echo   Rust not found. Installing via rustup...
    powershell -Command "Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile '%TEMP%\rustup-init.exe'"
    "%TEMP%\rustup-init.exe" -y --default-toolchain stable
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
    echo   Rust installed.
    echo.
)
echo   Building Amni-Code...
echo.
cargo build --release
if errorlevel 1 (
    echo.
    echo   Build failed. Check Rust installation.
    pause
    exit /b 1
)
echo.
echo   ============================================
echo     Build complete! Launching Amni-Code...
echo   ============================================
echo.
echo   Open http://localhost:3000 in your browser.
echo.
target\release\amni-code.exe
pause
