//! Унифицированный запуск дочерних процессов (все инструменты — sidecar:
//! ffmpeg, ffprobe, realesrgan-ncnn-vulkan, rife-ncnn-vulkan).
//! Стриминг stderr для парсинга прогресса + кооперативная отмена.
//! Реализация — задача A.

use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use crate::error::Result;

/// Колбэк построчного stderr (для парсинга прогресса).
pub type LineHandler<'a> = &'a mut dyn FnMut(&str);

/// Запускает sidecar `tool` с аргументами; читает stderr построчно в `on_line`;
/// при отмене токена убивает процесс и возвращает AppError::Cancelled.
/// При ненулевом коде выхода возвращает AppError::Process с хвостом stderr (~4КБ).
pub async fn run_sidecar(
    app: &AppHandle,
    tool: &str,
    args: &[String],
    cancel: &CancellationToken,
    on_line: LineHandler<'_>,
) -> Result<()> {
    let _ = (app, tool, args, cancel, on_line);
    todo!("задача A: tauri_plugin_shell sidecar + tokio::select! с cancel")
}

/// Как run_sidecar, но возвращает stdout целиком (для ffprobe -of json).
pub async fn run_sidecar_capture(app: &AppHandle, tool: &str, args: &[String]) -> Result<String> {
    let _ = (app, tool, args);
    todo!("задача A")
}
