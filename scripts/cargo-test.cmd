@echo off
setlocal

set "VSDEVCMD=C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\Tools\VsDevCmd.bat"
if not exist "%VSDEVCMD%" set "VSDEVCMD=C:\Program Files\Microsoft Visual Studio\18\Professional\Common7\Tools\VsDevCmd.bat"

if not exist "%VSDEVCMD%" (
  echo VsDevCmd.bat not found. Install Visual Studio C++ build tools or run cargo from a Developer PowerShell.
  exit /b 1
)

call "%VSDEVCMD%" -arch=x64 -host_arch=x64 >nul
pushd "%~dp0..\src-tauri"
cargo test %*
set "EXIT_CODE=%ERRORLEVEL%"
popd
exit /b %EXIT_CODE%
