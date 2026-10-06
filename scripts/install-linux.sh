#!/usr/bin/env bash
# Установка бинарника wrklog как systemd-сервиса (запуск от root: sudo).

set -e

SERVICE_NAME="wrklog"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARY_SRC="$REPO_ROOT/target/release/$SERVICE_NAME"
BINARY_DEST="/usr/local/bin/$SERVICE_NAME"
UNIT_SRC="$SCRIPT_DIR/$SERVICE_NAME.service"
UNIT_DEST="/etc/systemd/system/$SERVICE_NAME.service"
ENV_DIR="/usr/local/etc/$SERVICE_NAME"
ENV_FILE="$ENV_DIR/env"
REPO_ENV="$REPO_ROOT/.env"
# Временный env от task install (cfgy вызывается до sudo, файл в /tmp)
INSTALL_ENV_TMP="/tmp/wrklog-install.env"

if [[ $EUID -ne 0 ]]; then
  echo "Запустите скрипт с правами root: sudo $0"
  exit 1
fi

# Приоритет: env из /tmp (создан task'ом через cfgy), иначе cfgy от имени пользователя, иначе .env из репо
if [[ -f "$INSTALL_ENV_TMP" ]]; then
  mkdir -p "$ENV_DIR"
  cp "$INSTALL_ENV_TMP" "$ENV_FILE"
  chmod 600 "$ENV_FILE"
  rm -f "$INSTALL_ENV_TMP"
  echo "Файл окружения установлен из $INSTALL_ENV_TMP в $ENV_FILE"
elif [[ -n "${SUDO_USER:-$DOAS_USER}" ]] && command -v cfgy &>/dev/null; then
  echo "Генерация .env через cfgy (от пользователя ${SUDO_USER:-$DOAS_USER})..."
  if sudo -u "${SUDO_USER:-$DOAS_USER}" bash -c "cd '$REPO_ROOT' && cfgy env -o .env"; then
    echo ".env создан/обновлён."
  else
    echo "Предупреждение: cfgy завершился с ошибкой, используется существующий .env при наличии."
  fi
fi

if [[ ! -f "$BINARY_SRC" ]]; then
  echo "Ошибка: бинарник не найден: $BINARY_SRC"
  echo "Соберите проект: cargo build --release"
  exit 1
fi

# Останавливаем сервис перед перезаписью бинарника (иначе cp: Text file busy)
WAS_RUNNING=false
if [[ -f "$BINARY_DEST" ]] && systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
  echo "Останавливаем сервис $SERVICE_NAME для обновления бинарника..."
  systemctl stop "$SERVICE_NAME"
  WAS_RUNNING=true
fi

mkdir -p "$ENV_DIR"
cp "$BINARY_SRC" "$BINARY_DEST"
chmod 755 "$BINARY_DEST"
cp "$UNIT_SRC" "$UNIT_DEST"

if [[ ! -f "$ENV_FILE" ]]; then
  if [[ -f "$REPO_ENV" ]]; then
    cp "$REPO_ENV" "$ENV_FILE"
    chmod 600 "$ENV_FILE"
    echo "Файл окружения скопирован из $REPO_ENV в $ENV_FILE"
  else
    touch "$ENV_FILE"
    chmod 600 "$ENV_FILE"
    echo "Создан пустой $ENV_FILE — заполните переменные окружения при необходимости."
  fi
fi

systemctl daemon-reload
systemctl enable "$SERVICE_NAME"

if [[ "$WAS_RUNNING" == true ]]; then
  echo "Запускаем обновлённый сервис $SERVICE_NAME..."
  systemctl start "$SERVICE_NAME"
fi

echo ""
echo "Установка завершена."
echo ""
echo "Управление сервисом:"
echo "  systemctl start $SERVICE_NAME"
echo "  systemctl stop $SERVICE_NAME"
echo "  systemctl status $SERVICE_NAME"
echo ""
echo "Просмотр логов:"
echo "  journalctl -u $SERVICE_NAME -f"
echo ""
echo "Файл окружения: $ENV_FILE"
