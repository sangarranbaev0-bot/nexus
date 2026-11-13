#!/usr/bin/env bash
# Установка NexusChat на чистый Ubuntu/Debian VPS одной командой:
#   sudo bash deploy/install.sh chat.example.com
# Повторный запуск безопасен: пересобирает и перезапускает, секреты не трогает.
# Обновить версию: git pull && sudo bash deploy/install.sh
set -euo pipefail

DOMAIN="${1:-}"
cd "$(dirname "$0")"

if [[ ! -f .env ]]; then
  [[ -n "$DOMAIN" ]] || { echo "Укажите домен: sudo bash deploy/install.sh chat.example.com"; exit 1; }
  secret() { head -c 48 /dev/urandom | base64 | tr -d '/+=\n' | head -c "$1"; }
  cat > .env <<EOF
DOMAIN=$DOMAIN
POSTGRES_PASSWORD=$(secret 32)
JWT_SECRET=$(secret 48)
MINIO_PASSWORD=$(secret 32)
EOF
  chmod 600 .env
  echo "Секреты созданы в deploy/.env — сохраните копию этого файла."
fi

if ! command -v docker >/dev/null; then
  echo "Устанавливаю Docker…"
  curl -fsSL https://get.docker.com | sh
fi

# Сборка Rust-сервера требует памяти: на маленьком VPS добавляем 2 ГБ подкачки.
if (( $(free -m | awk '/Mem:/{print $2}') < 3000 )) && ! swapon --show | grep -q swapfile; then
  echo "Добавляю 2 ГБ подкачки для сборки…"
  fallocate -l 2G /swapfile && chmod 600 /swapfile && mkswap /swapfile && swapon /swapfile
  echo '/swapfile none swap sw 0 0' >> /etc/fstab
fi

docker compose up -d --build
echo
echo "Готово. Через минуту откройте https://$(grep ^DOMAIN= .env | cut -d= -f2)"
echo "Логи: cd $(pwd) && docker compose logs -f app"
