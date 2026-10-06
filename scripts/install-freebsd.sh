#!/usr/bin/env sh
# Установка бинарника wrklog как rc.d-сервиса (запуск от root: doas).

set -e

SERVICE_NAME="wrklog"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARY_SRC="$REPO_ROOT/target/release/$SERVICE_NAME"
BINARY_DEST="/usr/local/bin/$SERVICE_NAME"
RCD_SCRIPT_SRC="$SCRIPT_DIR/$SERVICE_NAME"
RCD_SCRIPT_DEST="/usr/local/etc/rc.d/$SERVICE_NAME"
ENV_DEST="/usr/local/etc/${SERVICE_NAME}.env"
REPO_ENV="$REPO_ROOT/.env"
INSTALL_ENV_TMP="/tmp/wrklog-install.env"
RC_CONF="/etc/rc.conf"
RC_CONF_LINE="${SERVICE_NAME}_enable=\"YES\""

if [ "$(id -u)" -ne 0 ]; then
  echo "Запустите скрипт с правами root: doas $0"
  exit 1
fi

# Приоритет: env из /tmp (создан task'ом через cfgy), иначе cfgy от имени пользователя, иначе .env из репо
if [ -f "$INSTALL_ENV_TMP" ]; then
  cp "$INSTALL_ENV_TMP" "$ENV_DEST"
  chmod 600 "$ENV_DEST"
  rm -f "$INSTALL_ENV_TMP"
  echo "Файл окружения установлен из $INSTALL_ENV_TMP в $ENV_DEST"
elif [ -n "${SUDO_USER:-${DOAS_USER}}" ] && command -v cfgy >/dev/null 2>&1; then
  echo "Генерация .env через cfgy (от пользователя ${SUDO_USER:-${DOAS_USER}})..."
  if su -m "${SUDO_USER:-${DOAS_USER}}" -c "cd '$REPO_ROOT' && cfgy env -o .env"; then
    echo ".env создан/обновлён."
  else
    echo "Предупреждение: cfgy завершился с ошибкой, используется существующий .env при наличии."
  fi
fi

if [ ! -f "$BINARY_SRC" ]; then
  echo "Ошибка: бинарник не найден: $BINARY_SRC"
  echo "Соберите проект: cargo build --release"
  exit 1
fi

cp "$BINARY_SRC" "$BINARY_DEST"
chmod 755 "$BINARY_DEST"
cp "$RCD_SCRIPT_SRC" "$RCD_SCRIPT_DEST"
chmod 555 "$RCD_SCRIPT_DEST"

if [ ! -f "$ENV_DEST" ]; then
  if [ -f "$REPO_ENV" ]; then
    cp "$REPO_ENV" "$ENV_DEST"
    chmod 600 "$ENV_DEST"
    echo "Файл окружения скопирован из $REPO_ENV в $ENV_DEST"
  else
    touch "$ENV_DEST"
    chmod 600 "$ENV_DEST"
    echo "Создан пустой $ENV_DEST — заполните переменные окружения при необходимости."
  fi
fi

if ! grep -q "^${SERVICE_NAME}_enable=" "$RC_CONF" 2>/dev/null; then
  echo "$RC_CONF_LINE" >> "$RC_CONF"
  echo "В $RC_CONF добавлено: $RC_CONF_LINE"
fi

echo ""
echo "Установка завершена."
echo ""
echo "Управление сервисом:"
echo "  service $SERVICE_NAME start"
echo "  service $SERVICE_NAME stop"
echo "  service $SERVICE_NAME status"
echo ""
echo "Файл окружения: $ENV_DEST"
