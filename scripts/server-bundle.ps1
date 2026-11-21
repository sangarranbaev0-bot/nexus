# Упаковка сервера NexusChat для распространения как ZIP с .exe
# Запуск: pwsh scripts/server-bundle.ps1 [output_dir]
param([string]$OutDir = "D:\nc\nexuschat\releases")

$ErrorActionPreference = "Stop"

$ServerBin = Join-Path $PSScriptRoot "..\server\target\release\nexus-server.exe"
if (-not (Test-Path $ServerBin)) {
    Write-Host "Сервер не собран. Запускаю cargo build --release..."
    Push-Location (Join-Path $PSScriptRoot "..\server")
    cargo build --release
    Pop-Location
}

Write-Host "Собираю артефакт..."

# Создаём структуру бандла
$BundleDir = Join-Path $OutDir "nexus-server-$($env:TAG_VERSION ?? 'v0.6.0')"
New-Item -ItemType Directory -Force -Path $BundleDir | Out-Null
Copy-Item $ServerBin (Join-Path $BundleDir "nexus-server.exe")

# Копируем скрипт миграций и стартовый батник
$MigDir = Join-Path $PSScriptRoot "..\server\migrations"
if (Test-Path $MigDir) {
    Copy-Item -Recurse $MigDir (Join-Path $BundleDir "migrations") -Force
}

# BAT для быстрого старта
$BatContent = @'
@echo off
chcp 65001 >nul
echo NexusChat Server
echo =================
echo Перед запуском настройте переменные окружения или создайте .env файл.
echo Пример: copy .env.example .env
echo.

if not exist ".env" (
    echo ОШИБКА: Не найден .env файл.
    echo Создайте его из .env.example и заполните POSTGRES_URL, JWT_SECRET и другие поля.
    pause
    exit /b 1
)

if not exist "nexus-server.exe" (
    echo ОШИБКА: nexus-server.exe не найден.
    pause
    exit /b 1
)

echo Запуск сервера...
nexus-server
pause
'@
Set-Content -Path (Join-Path $BundleDir "run.bat") -Value $BatContent -Encoding UTF8

# README внутри архива
$ReadmeContent = @'
# NexusChat Server — Windows

## Требования
- Windows 10/11 (x64)
- PostgreSQL 16+ (установите отдельно или используйте Docker)

## Быстрый старт

```cmd
copy .env.example .env
# Отредактируйте .env: POSTGRES_URL, JWT_SECRET, DOMAIN
run.bat
```

## Переменные окружения

| Переменная      | Описание                                  | По умолчанию              |
|----------------|-------------------------------------------|--------------------------|
| PORT           | Порт HTTP/WSS                             | 3001                     |
| DOMAIN         | Домен приложения                          | http://localhost:3001    |
| POSTGRES_URL   | DSN PostgreSQL                            | postgresql://nexus:nexus@localhost/nexus |
| JWT_SECRET     | Секрет подписи JWT (мин 32 символа)       | —                        |
| CLIENT_DIST    | Путь к dist клиента (статика)             | ./client                 |
| MINIO_ENDPOINT | S3-совместимое хранилище                  | localhost:9000           |
| MINIO_ACCESS_KEY | Ключ MinIO                             | minioadmin               |
| MINIO_SECRET_KEY | Секрет MinIO                           | minioadmin               |
| UPLOADS_DIR    | Папка для вложений                         | ./uploads                |

## Обновления

Повторно запустите `run.bat` после замены `nexus-server.exe`. База и конфиг не затрагиваются.
'@
Set-Content -Path (Join-Path $BundleDir "README.txt") -Value $ReadmeContent -Encoding UTF8

# Упаковываем в ZIP
$ZipName = "nexus-server-windows-x64.zip"
$ZipPath = Join-Path $OutDir $ZipName
if (Test-Path $ZipPath) { Remove-Item $ZipPath -Force }
Compress-Archive -Path "$BundleDir\*" -DestinationPath $ZipPath -Force

$SizeMB = [math]::Round((Get-Item $ZipPath).Length / 1MB, 2)
Write-Host "Готово: $ZipPath  ($SizeMB MB)"
Write-Host "Артефакт: $BundleDir"
