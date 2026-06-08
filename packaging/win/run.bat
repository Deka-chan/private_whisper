@echo off
rem privatewhisper (lite) — does NOT set ORT_DYLIB_PATH, so on first run the app
rem downloads the GPU runtime itself into %LOCALAPPDATA%\privatewhisper\runtime.
setlocal
set "RUST_LOG=info"
"%~dp0privatewhisper.exe"
echo.
echo (privatewhisper exited — window kept open so you can read any messages)
pause
