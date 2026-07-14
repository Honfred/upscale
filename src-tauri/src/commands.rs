//! Публичный API для фронтенда. КОНТРАКТ с src/lib/api.ts.
//! Реализация тел — задача B (делегация в probe/estimate/pipeline/state).

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::config::{Codec, UpscaleSettings};
use crate::error::{AppError, Result};
use crate::estimate::DiskEstimate;
use crate::probe::SourceInfo;
use crate::state::{AppState, JobStatus};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub vulkan_ok: bool,
    pub gpu_name: Option<String>,
    pub ffmpeg_ok: bool,
    pub realesrgan_ok: bool,
    pub rife_ok: bool,
    pub nvenc_codecs: Vec<Codec>,
}

#[tauri::command]
pub async fn probe_source(app: AppHandle, path: String) -> Result<SourceInfo> {
    let _ = (app, path);
    todo!("задача B: делегировать в probe::probe")
}

#[tauri::command]
pub async fn estimate_job(
    app: AppHandle,
    source: SourceInfo,
    settings: UpscaleSettings,
) -> Result<DiskEstimate> {
    let _ = (app, source, settings);
    todo!("задача B: делегировать в estimate::estimate")
}

/// Запускает джобу в tokio::spawn, регистрирует в AppState, возвращает job_id.
/// Терминальные события job://done / job://error эмитит обёртка вокруг
/// pipeline::run. Отклоняет запуск при активной джобе (JobAlreadyRunning).
#[tauri::command]
pub async fn start_job(
    app: AppHandle,
    source: SourceInfo,
    settings: UpscaleSettings,
    state: State<'_, AppState>,
) -> Result<String> {
    let _ = (app, source, settings, state);
    todo!("задача B")
}

#[tauri::command]
pub async fn cancel_job(job_id: String, state: State<'_, AppState>) -> Result<()> {
    let _ = (job_id, state);
    todo!("задача B")
}

#[tauri::command]
pub fn get_job_status(job_id: String, state: State<'_, AppState>) -> Result<JobStatus> {
    let _ = (job_id, state);
    todo!("задача B")
}

/// Проверка sidecar-бинарников, Vulkan/GPU и доступных NVENC-кодеков.
#[tauri::command]
pub async fn system_check(app: AppHandle) -> Result<SystemInfo> {
    let _ = app;
    todo!("задача B")
}

/// Показать файл в файловом менеджере (tauri_plugin_opener::reveal_item_in_dir).
#[tauri::command]
pub async fn reveal_output(app: AppHandle, path: String) -> Result<()> {
    let _ = (app, path);
    todo!("задача B")
}
