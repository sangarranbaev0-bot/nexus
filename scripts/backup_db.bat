@echo off
REM NexusChat DB backup helper. Requires pg_dump on PATH (PostgreSQL +4bin).
set BACKUP_DIR=%~dp0..\backups
set PGUSER=nexus
set PGPASSWORD=nexus
mkdir "%BACKUP_DIR%" 2>nul
for /f "tokens=2-4 delims=/ " %%a in ('date /t') do set DT=%%c%%a%%b
for /f "tokens=1-2 delims=: " %%a in ('time /t') do set TM=%%a%%b
set TIMESTAMP=%DT%_%TM%
pg_dump -U %PGUSER% -h localhost nexuschat_v032 --format=custom --file="%BACKUP_DIR%\backup_%TIMESTAMP%.dump"
if %ERRORLEVEL% neq 0 (
    echo FAILED to dump database (exit code %ERRORLEVEL%)
    exit /b %ERRORLEVEL%
)
echo OK: backup/%TIMESTAMP%.dump
pushd "%BACKUP_DIR%"
dir /b /o-d backup_*.dump | findstr /n . > "%TEMP%\baklist.txt"
for /f "skip=14 tokens=1*" %%a in ('type "%TEMP%\baklist.txt"') do del "%%b"
del "%TEMP%\baklist.txt"
popd
