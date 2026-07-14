//! Разрешение путей: модели (bundled resources), temp-директория джобы,
//! выходной файл. Реализация — задача A.

use std::path::PathBuf;
use tauri::AppHandle;

use crate::config::UpscaleSettings;
use crate::error::Result;
use crate::probe::SourceInfo;

/// Папка моделей Real-ESRGAN (resource "models-realesrgan").
pub fn esrgan_models_dir(app: &AppHandle) -> Result<PathBuf> {
    let _ = app;
    todo!("задача A: app.path().resolve(.., BaseDirectory::Resource)")
}

/// Папка модели RIFE (resource "models-rife/<RIFE_MODEL_DIR>").
pub fn rife_model_dir(app: &AppHandle) -> Result<PathBuf> {
    let _ = app;
    todo!("задача A")
}

/// Temp-корень джобы: {settings.temp_dir | app_cache_dir}/jobs/{job_id}.
pub fn job_temp_dir(app: &AppHandle, settings: &UpscaleSettings, job_id: &str) -> Result<PathBuf> {
    let _ = (app, settings, job_id);
    todo!("задача A")
}

/// Путь выходного файла: {output_dir | рядом с исходником}/{stem}_4k60.{mkv|mp4},
/// с дедупликацией имени при коллизии.
pub fn output_path(source: &SourceInfo, settings: &UpscaleSettings) -> Result<PathBuf> {
    let _ = (source, settings);
    todo!("задача A")
}
