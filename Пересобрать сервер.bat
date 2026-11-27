@echo off
chcp 65001 >nul
cd /d "%~dp0"
title NexusChat - пересборка сервера
echo Собираю сервер на Rust (первый раз 10-20 минут, дальше быстрее)...
docker build -t nexuschat-rust -f Dockerfile.rust . > build-server.log 2>&1
if errorlevel 1 (
  echo SERVER_BUILD_FAILED>> build-server.log
  echo Ошибка сборки сервера, подробности в build-server.log
  pause
  exit /b 1
)
echo SERVER_BUILD_OK>> build-server.log
call "%~dp0Запустить NexusChat.bat"
