# AnimeUpscale

Десктопное приложение для апскейла аниме-видео до 4K/60fps: `ffmpeg` → **Real-ESRGAN**
(ncnn-vulkan) → **RIFE** (ncnn-vulkan) → `ffmpeg` (NVENC). Построено на Tauri 2 + Svelte 5.

> _Скриншот приложения появится здесь после первого релиза._

## Требования

- **GPU с поддержкой Vulkan** — для апскейла (Real-ESRGAN) и интерполяции кадров (RIFE).
- **NVIDIA GPU** — для аппаратного энкодинга через NVENC (без него кодирование в текущей сборке
  ffmpeg недоступно, только декод/апскейл/интерполяция).
- Windows 10/11 x64 или Linux x64 (deb/AppImage).

## Установка

Готовые сборки — на странице [Releases](../../releases):

- **Windows** — `.msi` или `.exe` (NSIS)
- **Linux** — `.deb` или `.AppImage`

## Сборка из исходников

Понадобятся: Node.js 22+, Rust (stable, таргет вашей платформы), системные зависимости Tauri
(на Linux — `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`).

```bash
npm install

# Скачивает realesrgan-ncnn-vulkan / rife-ncnn-vulkan / ffmpeg / ffprobe
# в src-tauri/binaries/ (пиновые версии — см. scripts/versions.sh)
scripts/fetch-sidecars.sh          # Linux; на Windows — pwsh scripts/fetch-sidecars.ps1

# Скачивает модели (Real-ESRGAN anime-video-v3, RIFE v4.6)
# в src-tauri/resources/models-*/
scripts/fetch-models.sh

npm run tauri dev     # разработка
npm run tauri build   # прод-сборка (deb/appimage/msi/nsis — в зависимости от ОС)
```

Оба fetch-скрипта идемпотентны: повторный запуск ничего не перекачивает, если файлы уже на
месте. Флаг `--force` (`fetch-sidecars.sh`/`fetch-models.sh`) или `-Force`
(`fetch-sidecars.ps1`) принудительно перекачивает и перезаписывает.

Сторонние бинарники распространяются под собственными лицензиями (Real-ESRGAN/RIFE ncnn-vulkan,
ffmpeg-сборка BtbN — GPL); они не входят в исходники репозитория и не покрываются лицензией
ниже.

## Как выпустить релиз

Релиз собирается и публикуется автоматически в GitHub Actions (`.github/workflows/release.yml`)
по тегу вида `v*`:

```bash
git tag v0.1.0
git push --tags
```

Workflow соберёт и опубликует установщики для Linux (`deb`, `appimage`) и Windows (`msi`, `nsis`)
в виде GitHub Release.

## Архитектура

Кратко: Tauri 2 (Rust) + Svelte 5, видео обрабатывается чанками (сегментами) через пайплайн
`ffmpeg → Real-ESRGAN → RIFE → ffmpeg (NVENC)`, все тяжёлые инструменты запускаются как
sidecar-процессы. Подробнее — [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Лицензия

MIT, см. [LICENSE](LICENSE). © honfred, 2026.
