#!/usr/bin/env bash
# Пиновые версии/URL сторонних sidecar-бинарников и моделей для AnimeUpscale.
# Источник правды для scripts/fetch-sidecars.sh и scripts/fetch-models.sh.
#
# ВАЖНО: scripts/fetch-sidecars.ps1 (Windows) не может source-ить bash-файл —
# те же значения продублированы там в шапке скрипта. При обновлении версий
# здесь — обнови и .ps1.
#
# Этот файл только объявляет переменные, не выполняет побочных действий.
# Не является исполняемым самостоятельно (предназначен для `source`).

# --- Real-ESRGAN ncnn-vulkan (xinntao/Real-ESRGAN) ---
# https://github.com/xinntao/Real-ESRGAN/releases/tag/v0.2.5.0
REALESRGAN_TAG="v0.2.5.0"
REALESRGAN_LINUX_ASSET="realesrgan-ncnn-vulkan-20220424-ubuntu.zip"
REALESRGAN_WINDOWS_ASSET="realesrgan-ncnn-vulkan-20220424-windows.zip"
REALESRGAN_BASE_URL="https://github.com/xinntao/Real-ESRGAN/releases/download/${REALESRGAN_TAG}"
# Оба архива плоские (без общей корневой папки): бинарник и models/ лежат в корне zip.

# --- RIFE ncnn-vulkan (nihui/rife-ncnn-vulkan) ---
# https://github.com/nihui/rife-ncnn-vulkan/releases/tag/20221029
RIFE_TAG="20221029"
RIFE_LINUX_ASSET="rife-ncnn-vulkan-20221029-ubuntu.zip"
RIFE_WINDOWS_ASSET="rife-ncnn-vulkan-20221029-windows.zip"
RIFE_BASE_URL="https://github.com/nihui/rife-ncnn-vulkan/releases/download/${RIFE_TAG}"
# У этих архивов есть общая корневая папка внутри zip:
RIFE_LINUX_EXTRACT_DIR="rife-ncnn-vulkan-20221029-ubuntu"
RIFE_WINDOWS_EXTRACT_DIR="rife-ncnn-vulkan-20221029-windows"

# --- ffmpeg/ffprobe (статическая сборка с NVENC, BtbN/FFmpeg-Builds) ---
# Пиновый (не "latest"!) перманентный тег автосборки:
# https://github.com/BtbN/FFmpeg-Builds/releases/tag/autobuild-2026-07-13-14-11
# Взята версионированная сборка ffmpeg n7.1.5 (ветка release/7.1, GPL, с NVENC).
FFMPEG_TAG="autobuild-2026-07-13-14-11"
FFMPEG_LINUX_ASSET="ffmpeg-n7.1.5-2-g998de74adf-linux64-gpl-7.1.tar.xz"
FFMPEG_WINDOWS_ASSET="ffmpeg-n7.1.5-2-g998de74adf-win64-gpl-7.1.zip"
FFMPEG_BASE_URL="https://github.com/BtbN/FFmpeg-Builds/releases/download/${FFMPEG_TAG}"
# Корневая папка внутри архива (без расширения):
FFMPEG_LINUX_EXTRACT_DIR="ffmpeg-n7.1.5-2-g998de74adf-linux64-gpl-7.1"
FFMPEG_WINDOWS_EXTRACT_DIR="ffmpeg-n7.1.5-2-g998de74adf-win64-gpl-7.1"
