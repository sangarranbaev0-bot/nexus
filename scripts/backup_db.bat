@echo off
chcp 65001 >nul
REM Резервная копия базы NexusChat через Docker (pg_dump на Windows не нужен).
REM Хранит 14 последних копий в папке backups рядом с проектом.
cd /d "%~dp0.."
set BACKUP_DIR=%~dp0..\backups
mkdir "%BACKUP_DIR%" 2>nul
for /f %%i in ('powershell -NoProfile -Command "Get-Date -Format yyyy-MM-dd_HH-mm"') do set TS=%%i
docker compose exec -T postgres pg_dump -U nexus -d nexuschat_v032 --format=custom > "%BACKUP_DIR%\backup_%TS%.dump"
if errorlevel 1 (
  echo ОШИБКА: не удалось сделать копию. Запущен ли Docker и NexusChat?
  del "%BACKUP_DIR%\backup_%TS%.dump" 2>nul
  exit /b 1
)
echo OK: backups\backup_%TS%.dump
powershell -NoProfile -Command "Get-ChildItem '%BACKUP_DIR%\backup_*.dump' | Sort-Object LastWriteTime -Descending | Select-Object -Skip 14 | Remove-Item"
