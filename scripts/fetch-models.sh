#!/usr/bin/env bash
# Извлекает модели Real-ESRGAN (anime video v3, x2/x3/x4) и RIFE (v4.6) из тех
# же релизных архивов, что и scripts/fetch-sidecars.sh, и раскладывает их в
# src-tauri/resources/{models-realesrgan,models-rife}/ — как того требует
# секция bundle.resources в src-tauri/tauri.conf.json.
#
# ВАЖНО: этот скрипт запускается в CI на Windows тоже через Git Bash
# (шаг с shell: bash) — используем только POSIX-утилиты + curl + unzip
# (с фолбэком на 7z/tar, см. scripts/lib.sh), никакого PowerShell.
# Модели платформо-независимы, поэтому архив нужен только один (linux или
# windows — неважно, содержимое models/ и rife-v4.6/ идентично); скрипт
# переиспользует то, что уже скачал fetch-sidecars.* в общий кэш .cache/.
#
# Пиновые версии/URL см. в scripts/versions.sh.
#
# Использование:
#   scripts/fetch-models.sh [--force]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=./lib.sh
source "$SCRIPT_DIR/lib.sh"
# shellcheck source=./versions.sh
source "$SCRIPT_DIR/versions.sh"

FORCE=0
for arg in "$@"; do
  case "$arg" in
    --force) FORCE=1 ;;
  esac
done

CACHE_DIR="$ROOT_DIR/.cache"
DOWNLOAD_DIR="$CACHE_DIR/downloads"
EXTRACT_DIR="$CACHE_DIR/models-extract"
RESOURCES_DIR="$ROOT_DIR/src-tauri/resources"
REALESRGAN_MODELS_DIR="$RESOURCES_DIR/models-realesrgan"
RIFE_MODELS_DIR="$RESOURCES_DIR/models-rife"
mkdir -p "$DOWNLOAD_DIR" "$EXTRACT_DIR" "$REALESRGAN_MODELS_DIR" "$RIFE_MODELS_DIR"

### 1. Real-ESRGAN: models/realesr-animevideov3-x{2,3,4}.{param,bin} ###
RE_NEED=1
if [ "$FORCE" -ne 1 ]; then
  RE_NEED=0
  for scale in x2 x3 x4; do
    for ext in param bin; do
      if [ ! -f "$REALESRGAN_MODELS_DIR/realesr-animevideov3-$scale.$ext" ]; then
        RE_NEED=1
      fi
    done
  done
fi

if [ "$RE_NEED" -eq 1 ]; then
  RE_ZIP="$(download_any "$DOWNLOAD_DIR" "$REALESRGAN_BASE_URL" \
    "$REALESRGAN_LINUX_ASSET" "$REALESRGAN_WINDOWS_ASSET")"
  RE_EXTRACT="$EXTRACT_DIR/realesrgan"
  rm -rf "$RE_EXTRACT"
  extract_zip_members "$RE_ZIP" "$RE_EXTRACT" \
    "models/realesr-animevideov3-x2.param" "models/realesr-animevideov3-x2.bin" \
    "models/realesr-animevideov3-x3.param" "models/realesr-animevideov3-x3.bin" \
    "models/realesr-animevideov3-x4.param" "models/realesr-animevideov3-x4.bin"
  cp "$RE_EXTRACT"/models/realesr-animevideov3-*.param "$REALESRGAN_MODELS_DIR/"
  cp "$RE_EXTRACT"/models/realesr-animevideov3-*.bin "$REALESRGAN_MODELS_DIR/"
  log "модели Real-ESRGAN скопированы в $REALESRGAN_MODELS_DIR"
else
  log "модели Real-ESRGAN уже на месте, пропуск"
fi

### 2. RIFE: rife-v4.6/{flownet.param,flownet.bin} ###
RIFE_DEST="$RIFE_MODELS_DIR/rife-v4.6"
mkdir -p "$RIFE_DEST"

if [ "$FORCE" -eq 1 ] || [ ! -f "$RIFE_DEST/flownet.param" ] || [ ! -f "$RIFE_DEST/flownet.bin" ]; then
  RIFE_ZIP="$(download_any "$DOWNLOAD_DIR" "$RIFE_BASE_URL" \
    "$RIFE_LINUX_ASSET" "$RIFE_WINDOWS_ASSET")"
  case "$(basename "$RIFE_ZIP")" in
    "$RIFE_LINUX_ASSET") RIFE_TOPDIR="$RIFE_LINUX_EXTRACT_DIR" ;;
    "$RIFE_WINDOWS_ASSET") RIFE_TOPDIR="$RIFE_WINDOWS_EXTRACT_DIR" ;;
    *)
      log "неизвестный архив RIFE: $RIFE_ZIP"
      exit 1
      ;;
  esac
  RIFE_EXTRACT="$EXTRACT_DIR/rife"
  rm -rf "$RIFE_EXTRACT"
  extract_zip_members "$RIFE_ZIP" "$RIFE_EXTRACT" \
    "$RIFE_TOPDIR/rife-v4.6/flownet.param" \
    "$RIFE_TOPDIR/rife-v4.6/flownet.bin"
  cp "$RIFE_EXTRACT/$RIFE_TOPDIR/rife-v4.6/flownet.param" "$RIFE_DEST/"
  cp "$RIFE_EXTRACT/$RIFE_TOPDIR/rife-v4.6/flownet.bin" "$RIFE_DEST/"
  log "модель RIFE v4.6 скопирована в $RIFE_DEST"
else
  log "модель RIFE v4.6 уже на месте, пропуск"
fi

log "Готово."
