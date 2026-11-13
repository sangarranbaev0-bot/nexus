@echo off
chcp 65001 >nul
cd /d "%~dp0"
title NexusChat
echo Сборка и запуск NexusChat. Первый раз 10-15 минут, не закрывайте это окно...
docker info >nul 2>&1
if errorlevel 1 (
  echo DOCKER_NOT_RUNNING> build.log
  echo Docker Desktop не запущен. Запустите его, дождитесь "Engine running" и снова откройте этот файл.
  pause
  exit /b 1
)
docker compose --progress plain --profile app up -d --build > build.log 2>&1
if errorlevel 1 (
  echo BUILD_FAILED>> build.log
  echo Ошибка сборки, подробности в build.log
  pause
  exit /b 1
)
echo Сервер собран, жду запуска...
for /l %%i in (1,1,60) do (
  curl -sf http://localhost:3001/api/health >nul 2>&1 && goto ready
  timeout /t 2 >nul
)
echo SERVER_NOT_RESPONDING>> build.log
docker compose --profile app logs --tail 80 app >> build.log 2>&1
echo Сервер не отвечает, подробности в build.log
pause
exit /b 1

:ready
echo READY>> build.log
echo Готово! Открываю http://localhost:3001
start "" http://localhost:3001
timeout /t 5 >nul
