@echo off
setlocal
start "" "%~dp0ironwood-companion.exe" --start-if-needed %*
exit /b %errorlevel%
