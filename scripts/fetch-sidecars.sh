#!/usr/bin/env bash
# Скачивает и раскладывает sidecar-бинарники (realesrgan-ncnn-vulkan,
# rife-ncnn-vulkan, ffmpeg, ffprobe) в src-tauri/binaries/ с именами
# <name>-<target-triple>[.exe] — как того требует Tauri externalBin
# (см. src-tauri/tauri.conf.json -> bundle.externalBin).
#
# Пиновые версии/URL см. в scripts/versions.sh.
#
# Использование:
#   scripts/fetch-sidecars.sh [target-triple] [--force]
#
# target-triple по умолчанию берётся из `rustc -Vv`.
# Поддерживаются: x86_64-unknown-linux-gnu, x86_64-pc-windows-msvc.
# --force перекачивает архивы и перезаписывает уже существующие бинарники.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=./lib.sh
source "$SCRIPT_DIR/lib.sh"
# shellcheck source=./versions.sh
source "$SCRIPT_DIR/versions.sh"

FORCE=0
TARGET_TRIPLE=""
for arg in "$@"; do
  case "$arg" in
    --force) FORCE=1 ;;
    *) TARGET_TRIPLE="$arg" ;;
  esac
done

if [ -z "$TARGET_TRIPLE" ]; then
  if command -v rustc >/dev/null 2>&1; then
    TARGET_TRIPLE="$(rustc -Vv | sed -n 's/host: //p')"
  fi
fi
if [ -z "$TARGET_TRIPLE" ]; then
  echo "Не удалось определить target-triple (rustc не найден)." >&2
  echo "Передайте его первым аргументом, например:" >&2
  echo "  scripts/fetch-sidecars.sh x86_64-unknown-linux-gnu" >&2
  exit 1
fi

case "$TARGET_TRIPLE" in
  x86_64-unknown-linux-gnu) PLATFORM="linux"; EXE_EXT="" ;;
  x86_64-pc-windows-msvc)  PLATFORM="windows"; EXE_EXT=".exe" ;;
  *)
    echo "Неподдерживаемый target-triple: $TARGET_TRIPLE" >&2
    echo "Поддерживаются: x86_64-unknown-linux-gnu, x86_64-pc-windows-msvc" >&2
    exit 1
    ;;
esac

CACHE_DIR="$ROOT_DIR/.cache"
DOWNLOAD_DIR="$CACHE_DIR/downloads"
EXTRACT_DIR="$CACHE_DIR/sidecars-extract"
BIN_DIR="$ROOT_DIR/src-tauri/binaries"
mkdir -p "$DOWNLOAD_DIR" "$EXTRACT_DIR" "$BIN_DIR"

# place_binary <src-file> <sidecar-base-name (без триплета/.exe)>
place_binary() {
  local src="$1" name="$2"
  local dest="$BIN_DIR/${name}-${TARGET_TRIPLE}${EXE_EXT}"
  if [ -f "$dest" ] && [ "$FORCE" -ne 1 ]; then
    log "бинарник уже на месте, пропуск: $(basename "$dest")"
    return 0
  fi
  cp "$src" "$dest"
  if [ "$PLATFORM" = "linux" ]; then
    chmod +x "$dest"
  fi
  log "готово: $(basename "$dest")"
}

### 1. Real-ESRGAN ncnn-vulkan ###
if [ "$PLATFORM" = "linux" ]; then
  RE_ASSET="$REALESRGAN_LINUX_ASSET"
  RE_BIN_NAME="realesrgan-ncnn-vulkan"
else
  RE_ASSET="$REALESRGAN_WINDOWS_ASSET"
  RE_BIN_NAME="realesrgan-ncnn-vulkan.exe"
fi
RE_ZIP="$DOWNLOAD_DIR/$RE_ASSET"
download "$REALESRGAN_BASE_URL/$RE_ASSET" "$RE_ZIP"

RE_EXTRACT="$EXTRACT_DIR/realesrgan-$PLATFORM"
if [ "$FORCE" -eq 1 ] || [ ! -f "$RE_EXTRACT/$RE_BIN_NAME" ]; then
  rm -rf "$RE_EXTRACT"
  mkdir -p "$RE_EXTRACT"
  unzip -q -o "$RE_ZIP" "$RE_BIN_NAME" -d "$RE_EXTRACT"
fi
place_binary "$RE_EXTRACT/$RE_BIN_NAME" "animeupscale-realesrgan"

### 2. RIFE ncnn-vulkan ###
if [ "$PLATFORM" = "linux" ]; then
  RIFE_ASSET="$RIFE_LINUX_ASSET"
  RIFE_TOPDIR="$RIFE_LINUX_EXTRACT_DIR"
  RIFE_BIN_NAME="rife-ncnn-vulkan"
else
  RIFE_ASSET="$RIFE_WINDOWS_ASSET"
  RIFE_TOPDIR="$RIFE_WINDOWS_EXTRACT_DIR"
  RIFE_BIN_NAME="rife-ncnn-vulkan.exe"
fi
RIFE_ZIP="$DOWNLOAD_DIR/$RIFE_ASSET"
download "$RIFE_BASE_URL/$RIFE_ASSET" "$RIFE_ZIP"

RIFE_EXTRACT="$EXTRACT_DIR/rife-$PLATFORM"
if [ "$FORCE" -eq 1 ] || [ ! -f "$RIFE_EXTRACT/$RIFE_TOPDIR/$RIFE_BIN_NAME" ]; then
  rm -rf "$RIFE_EXTRACT"
  mkdir -p "$RIFE_EXTRACT"
  unzip -q -o "$RIFE_ZIP" "$RIFE_TOPDIR/$RIFE_BIN_NAME" -d "$RIFE_EXTRACT"
fi
place_binary "$RIFE_EXTRACT/$RIFE_TOPDIR/$RIFE_BIN_NAME" "animeupscale-rife"

### 3. ffmpeg + ffprobe ###
if [ "$PLATFORM" = "linux" ]; then
  FF_ASSET="$FFMPEG_LINUX_ASSET"
  FF_TOPDIR="$FFMPEG_LINUX_EXTRACT_DIR"
  FF_EXE_EXT=""
else
  FF_ASSET="$FFMPEG_WINDOWS_ASSET"
  FF_TOPDIR="$FFMPEG_WINDOWS_EXTRACT_DIR"
  FF_EXE_EXT=".exe"
fi
FF_ARCHIVE="$DOWNLOAD_DIR/$FF_ASSET"
download "$FFMPEG_BASE_URL/$FF_ASSET" "$FF_ARCHIVE"

FF_EXTRACT="$EXTRACT_DIR/ffmpeg-$PLATFORM"
if [ "$FORCE" -eq 1 ] || [ ! -f "$FF_EXTRACT/$FF_TOPDIR/bin/ffmpeg$FF_EXE_EXT" ]; then
  rm -rf "$FF_EXTRACT"
  mkdir -p "$FF_EXTRACT"
  if [ "$PLATFORM" = "linux" ]; then
    tar -xJf "$FF_ARCHIVE" -C "$FF_EXTRACT" \
      "$FF_TOPDIR/bin/ffmpeg" "$FF_TOPDIR/bin/ffprobe"
  else
    unzip -q -o "$FF_ARCHIVE" \
      "$FF_TOPDIR/bin/ffmpeg.exe" "$FF_TOPDIR/bin/ffprobe.exe" -d "$FF_EXTRACT"
  fi
fi
place_binary "$FF_EXTRACT/$FF_TOPDIR/bin/ffmpeg$FF_EXE_EXT" "animeupscale-ffmpeg"
place_binary "$FF_EXTRACT/$FF_TOPDIR/bin/ffprobe$FF_EXE_EXT" "animeupscale-ffprobe"

log "Готово. target-triple=$TARGET_TRIPLE"
ls -la "$BIN_DIR" >&2
