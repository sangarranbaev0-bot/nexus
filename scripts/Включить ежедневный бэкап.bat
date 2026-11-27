@echo off
chcp 65001 >nul
REM Включает ежедневную резервную копию базы NexusChat в 03:00 (Планировщик заданий Windows).
schtasks /Create /F /SC DAILY /ST 03:00 /TN "NexusChat backup" /TR "\"%~dp0backup_db.bat\"" 
if errorlevel 1 ( echo Не получилось создать задание. & pause & exit /b 1 )
echo Готово: копия базы будет делаться каждый день в 03:00 в папку backups.
echo Отключить: schtasks /Delete /TN "NexusChat backup" /F
pause
