//! Апскейл кадров сегмента через realesrgan-ncnn-vulkan.

use std::path::Path;
use std::time::Duration;
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use crate::config::{ESRGAN_MODEL, NCNN_TILE};
use crate::error::{AppError, Result};
use crate::process::run_sidecar;

use super::progress::{count_pngs_now, spawn_frame_counter};

/// Апскейлит `{seg_dir}/in` -> `{seg_dir}/up` моделью ESRGAN_MODEL с масштабом
/// `scale`. Модель без суффикса -x{scale} передаётся через -n, сам масштаб —
/// через -s; при этом реальный файл модели `{ESRGAN_MODEL}-x{scale}.param`
/// должен существовать в `models_dir`, иначе SidecarMissing.
///
/// Прогресс сообщается через `on_progress(frames_done)`: т.к. stderr
/// ncnn-vulkan имеет нестабильный формат по кадру, реальный источник —
/// фоновый подсчёт PNG в выходной папке раз в 500мс.
pub async fn upscale_segment(
    app: &AppHandle,
    seg_dir: &Path,
    models_dir: &Path,
    scale: u32,
    total_frames: u64,
    cancel: &CancellationToken,
    mut on_progress: impl FnMut(u64) + Send + 'static,
) -> Result<()> {
    let in_dir = seg_dir.join("in");
    let out_dir = seg_dir.join("up");
    std::fs::create_dir_all(&out_dir)?;

    let model_file = models_dir.join(format!("{ESRGAN_MODEL}-x{scale}.param"));
    if !model_file.exists() {
        return Err(AppError::SidecarMissing(format!(
            "модель Real-ESRGAN не найдена: {}",
            model_file.display()
        )));
    }

    let args = vec![
        "-i".to_string(),
        in_dir.to_string_lossy().to_string(),
        "-o".to_string(),
        out_dir.to_string_lossy().to_string(),
        "-n".to_string(),
        ESRGAN_MODEL.to_string(),
        "-s".to_string(),
        scale.to_string(),
        "-m".to_string(),
        models_dir.to_string_lossy().to_string(),
        "-f".to_string(),
        "png".to_string(),
        "-t".to_string(),
        NCNN_TILE.to_string(),
        "-g".to_string(),
        "0".to_string(),
        "-j".to_string(),
        "2:2:2".to_string(),
    ];

    let (handle, stop_tx) =
        spawn_frame_counter(out_dir.clone(), Duration::from_millis(500), move |count| {
            on_progress(count);
        });

    let result = run_sidecar(app, "realesrgan-ncnn-vulkan", &args, cancel, &mut |_line| {}).await;

    let _ = stop_tx.send(());
    let _ = handle.await;

    result?;

    let actual = count_pngs_now(&out_dir);
    if actual != total_frames {
        return Err(AppError::Other(format!(
            "upscale: ожидалось {total_frames} кадров, получено {actual}"
        )));
    }

    Ok(())
}
