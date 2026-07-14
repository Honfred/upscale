#!/usr/bin/env bash
# Общие функции для scripts/fetch-sidecars.sh и scripts/fetch-models.sh.
# Предназначен для `source`, не выполняется самостоятельно.
#
# Ожидает, что вызывающий скрипт объявил (или нет — тогда берутся дефолты):
#   FORCE=0|1 — форсировать перекачивание/перезапись.

log() {
  echo "[$(basename "${0:-fetch}")] $*" >&2
}

# download <url> <dest-file>
# Скачивает файл в кэш. Идемпотентно: пропускает, если файл уже есть и FORCE!=1.
download() {
  local url="$1" dest="$2"
  if [ -f "$dest" ] && [ "${FORCE:-0}" -ne 1 ]; then
    log "кэш есть, пропуск скачивания: $(basename "$dest")"
    return 0
  fi
  log "скачивание: $url"
  mkdir -p "$(dirname "$dest")"
  curl -fL --retry 3 --retry-delay 2 -o "$dest.part" "$url"
  mv "$dest.part" "$dest"
}

# download_any <dest-dir> <base-url> <preferred-asset> <alt-asset>
# Если alt-asset уже лежит в кэше (например, его скачал соседний шаг для
# другой цели) — переиспользует его вместо повторного скачивания preferred.
# Печатает в stdout полный путь к итоговому файлу.
download_any() {
  local dest_dir="$1" base_url="$2" preferred="$3" alt="$4"
  if [ -f "$dest_dir/$alt" ] && [ "${FORCE:-0}" -ne 1 ]; then
    log "переиспользую уже скачанный кэш: $(basename "$dest_dir/$alt")"
    echo "$dest_dir/$alt"
    return 0
  fi
  local dest="$dest_dir/$preferred"
  download "$base_url/$preferred" "$dest"
  echo "$dest"
}

# extract_zip_members <archive.zip> <dest-dir> <member...>
# Достаёт из zip только перечисленные пути (сохраняя структуру папок внутри
# архива). Работает и без unzip (например, в Git Bash на Windows), пробуя по
# очереди unzip -> 7z -> tar (bsdtar в Windows умеет читать zip).
extract_zip_members() {
  local archive="$1" dest="$2"
  shift 2
  mkdir -p "$dest"
  if command -v unzip >/dev/null 2>&1; then
    unzip -q -o "$archive" "$@" -d "$dest"
  elif command -v 7z >/dev/null 2>&1; then
    ( cd "$dest" && 7z x -y "$archive" "$@" >/dev/null )
  elif command -v tar >/dev/null 2>&1; then
    ( cd "$dest" && tar -xf "$archive" "$@" )
  else
    log "ошибка: не найден ни unzip, ни 7z, ни tar для распаковки zip"
    return 1
  fi
}
