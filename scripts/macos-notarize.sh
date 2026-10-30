#!/usr/bin/env bash
# Билеты нотаризации macOS-артефактов после tauri-action.
# Tauri прикрепляет билет к .app командой `xcrun stapler staple`, но не проверяет её код возврата: если билет ещё
# не разошёлся по серверам Apple, сбой проходит молча, и .app без билета попадает в .dmg, .pkg и архив updater —
# без сети Gatekeeper такое приложение не пустит. Сам .dmg Tauri подписывает, но не нотаризует.
#
# Использование (каждой команде — ровно один путь):
#   macos-notarize.sh staple <путь>   — прикрепить билет с повторами, если его нет, и проверить
#   macos-notarize.sh app <App.app>   — билет у .app (до сборки .pkg из него) и оценка Gatekeeper
#   macos-notarize.sh dmg <file.dmg>  — билет у .app в образе, новая подпись, нотаризация и билет у .dmg
#   macos-notarize.sh pkg <file.pkg>  — проверка билета у .app внутри .pkg (сам .pkg не подписан — см. release.yml)
# Окружение для dmg: APPLE_CERTIFICATE, APPLE_CERTIFICATE_PASSWORD, APPLE_SIGNING_IDENTITY, APPLE_ID, APPLE_PASSWORD,
# APPLE_TEAM_ID. Повторы прикрепления: STAPLE_ATTEMPTS (по умолчанию 10) и STAPLE_DELAY (30 с).
set -euo pipefail

ATTEMPTS="${STAPLE_ATTEMPTS:-10}"
DELAY="${STAPLE_DELAY:-30}"
TMP="$(mktemp -d)"
KEYCHAIN=""

cleanup() {
  local m
  for m in "$TMP"/mnt-*; do
    [ -d "$m" ] && hdiutil detach "$m" -force -quiet 2>/dev/null || true
  done
  if [ -n "$KEYCHAIN" ]; then security delete-keychain "$KEYCHAIN" 2>/dev/null || true; fi
  # Сбой уборки не должен менять код выхода скрипта.
  rm -rf "$TMP" 2>/dev/null || true
}
trap cleanup EXIT

die() {
  echo "::error::$*" >&2
  exit 1
}

usage() {
  echo "использование: macos-notarize.sh staple|app|dmg|pkg <путь>" >&2
  exit 2
}

stapled() { xcrun stapler validate "$1" >/dev/null 2>&1; }

# Доказательство — только `stapler validate`: код возврата `stapler staple` Tauri и так не смотрит, а нам он не нужен.
staple() {
  local path="$1" i
  if stapled "$path"; then
    echo "билет уже прикреплён: $path"
    return 0
  fi
  for ((i = 1; i <= ATTEMPTS; i++)); do
    xcrun stapler staple "$path" || true
    if stapled "$path"; then
      echo "билет прикреплён (попытка $i): $path"
      return 0
    fi
    if ((i < ATTEMPTS)); then
      echo "билета ещё нет, повтор через $DELAY с ($i из $ATTEMPTS): $path"
      sleep "$DELAY"
    fi
  done
  die "не удалось прикрепить билет нотаризации за $ATTEMPTS попыток: $path"
}

# Единственный .app среди аргументов — результата glob.
one_app() {
  if [ "$#" -ne 1 ] || [ ! -d "$1" ]; then die "ожидался один .app, найдено: $*"; fi
  echo "$1"
}

mount_image() { # <образ> <имя точки монтирования> [параметры hdiutil]
  local image="$1" point="$TMP/mnt-$2"
  shift 2
  mkdir -p "$point"
  hdiutil attach "$image" -nobrowse -noautoopen -mountpoint "$point" "$@" >/dev/null
  echo "$point"
}

# Новая подпись образа — во временной связке ключей, как у Tauri (его связка к этому шагу уже удалена).
sign() {
  : "${APPLE_CERTIFICATE:?нет APPLE_CERTIFICATE}" "${APPLE_CERTIFICATE_PASSWORD:?нет APPLE_CERTIFICATE_PASSWORD}"
  : "${APPLE_SIGNING_IDENTITY:?нет APPLE_SIGNING_IDENTITY}"
  local password line i
  local keychains=()
  password="$(uuidgen)"
  while IFS= read -r line; do
    line="${line//\"/}"
    line="${line#"${line%%[![:space:]]*}"}"
    if [ -n "$line" ]; then keychains+=("$line"); fi
  done < <(security list-keychains -d user)
  KEYCHAIN="$TMP/sign.keychain-db"
  security create-keychain -p "$password" "$KEYCHAIN"
  security unlock-keychain -p "$password" "$KEYCHAIN"
  security set-keychain-settings -t 3600 -u "$KEYCHAIN"
  printf '%s' "$APPLE_CERTIFICATE" | base64 --decode >"$TMP/cert.p12"
  security import "$TMP/cert.p12" -f pkcs12 -t agg -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign -k "$KEYCHAIN" >/dev/null
  rm -f "$TMP/cert.p12"
  security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$password" "$KEYCHAIN" >/dev/null
  # Как у Tauri: связка — ещё и в списке поиска пользователя (delete-keychain при выходе её оттуда уберёт).
  security list-keychains -d user -s ${keychains[@]+"${keychains[@]}"} "$KEYCHAIN"
  # Сервер меток времени Apple иногда не отвечает («A timestamp was expected but was not found»).
  for ((i = 1; i <= 3; i++)); do
    if codesign --force --timestamp --sign "$APPLE_SIGNING_IDENTITY" --keychain "$KEYCHAIN" "$1"; then break; fi
    if ((i == 3)); then die "не удалось подписать $1"; fi
    sleep 10
  done
  codesign --verify --strict -vv "$1"
}

notarize() {
  : "${APPLE_ID:?нет APPLE_ID}" "${APPLE_PASSWORD:?нет APPLE_PASSWORD}" "${APPLE_TEAM_ID:?нет APPLE_TEAM_ID}"
  local auth=(--apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID")
  local out status id
  out="$(xcrun notarytool submit "$1" "${auth[@]}" --wait --timeout 30m --output-format json)" || true
  status="$(printf '%s' "$out" | plutil -extract status raw -o - - 2>/dev/null)" || status=""
  id="$(printf '%s' "$out" | plutil -extract id raw -o - - 2>/dev/null)" || id=""
  echo "нотаризация $(basename "$1"): ${status:-нет ответа} (${id:-без id})"
  if [ "$status" != "Accepted" ]; then
    printf '%s\n' "$out" >&2
    if [ -n "$id" ]; then xcrun notarytool log "$id" "${auth[@]}" >&2 || true; fi
    die "нотаризация $(basename "$1") не принята"
  fi
}

cmd_app() {
  if ! stapled "$1"; then
    # .dmg получит билет в шаге dmg, .pkg собирается из этого .app; архив updater остаётся как есть: новая упаковка
    # потребовала бы новой подписи updater.
    echo "::warning::Tauri не прикрепил билет к $(basename "$1"); в архиве updater приложение без билета"
  fi
  staple "$1"
  spctl --assess --type exec -vv "$1"
}

# Билет прикрепляется к .app прямо в образе Tauri — раскладка окна, фон и значок тома сохраняются; образ сжимается
# в прежний формат и подписывается заново. Так при каждом прогоне, даже если билет уже был: один путь для обоих
# случаев, и его проверяет каждый пробный прогон, а не только редкий сбой прикрепления.
cmd_dmg() {
  local dmg="$1" format mnt app
  # Имя прежнее: из него codesign берёт идентификатор подписи образа.
  local rw="$TMP/rw.dmg" out
  local key=()
  out="$TMP/out/$(basename "$dmg")"
  mkdir -p "$TMP/out"
  format="$(hdiutil imageinfo -format "$dmg")"
  # Сжатие как у Tauri (bundle_dmg.sh), иначе образ вырастает.
  if [ "$format" = UDZO ]; then key=(-imagekey zlib-level=9); fi
  hdiutil convert "$dmg" -format UDRW -o "$rw" -quiet
  mnt="$(mount_image "$rw" rw -readwrite)"
  app="$(one_app "$mnt"/*.app)"
  staple "$app"
  hdiutil detach "$mnt" -quiet
  hdiutil convert "$rw" -format "$format" ${key[@]+"${key[@]}"} -o "$out" -quiet
  sign "$out"
  notarize "$out"
  staple "$out"
  spctl --assess --type open --context context:primary-signature -vv "$out"
  mv -f "$out" "$dmg"
  # Итог — по готовому файлу: у .app в образе есть билет, Gatekeeper его пускает.
  mnt="$(mount_image "$dmg" final -readonly)"
  app="$(one_app "$mnt"/*.app)"
  stapled "$app" || die "в $(basename "$dmg") приложение без билета нотаризации"
  spctl --assess --type exec -vv "$app"
  hdiutil detach "$mnt" -quiet
}

cmd_pkg() {
  local app
  pkgutil --expand-full "$1" "$TMP/pkg" >/dev/null
  app="$(one_app "$TMP"/pkg/*.pkg/Payload/*.app)"
  stapled "$app" || die "в $(basename "$1") приложение без билета нотаризации"
  echo "билет у приложения в $(basename "$1") на месте"
}

case "${1:-}" in
  staple | app | dmg | pkg) ;;
  *) usage ;;
esac
cmd="$1"
shift
# Шаблон без совпадений приходит как есть, с несколькими — несколькими путями.
if [ "$#" -ne 1 ] || [ ! -e "$1" ]; then die "$cmd: ожидался один существующий путь, получено: $*"; fi
case "$cmd" in
  staple) staple "$1" ;;
  app) cmd_app "$1" ;;
  dmg) cmd_dmg "$1" ;;
  pkg) cmd_pkg "$1" ;;
esac
