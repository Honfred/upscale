//! Разрешение путей: модели (bundled resources), temp-директория джобы,
//! выходной файл. Реализация — задача A.

use std::path::PathBuf;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use crate::config::{Container, UpscaleSettings, RIFE_MODEL_DIR};
use crate::error::{AppError, Result};
use crate::probe::SourceInfo;

/// Папка моделей Real-ESRGAN (resource "models-realesrgan").
pub fn esrgan_models_dir(app: &AppHandle) -> Result<PathBuf> {
    app.path()
        .resolve("models-realesrgan", BaseDirectory::Resource)
        .map_err(|e| AppError::Config(format!("не удалось найти папку моделей Real-ESRGAN: {e}")))
}

/// Папка модели RIFE (resource "models-rife/<RIFE_MODEL_DIR>").
pub fn rife_model_dir(app: &AppHandle) -> Result<PathBuf> {
    app.path()
        .resolve(
            format!("models-rife/{RIFE_MODEL_DIR}"),
            BaseDirectory::Resource,
        )
        .map_err(|e| AppError::Config(format!("не удалось найти папку модели RIFE: {e}")))
}

/// Temp-корень джобы: {settings.temp_dir | app_cache_dir}/jobs/{job_id}.
pub fn job_temp_dir(app: &AppHandle, settings: &UpscaleSettings, job_id: &str) -> Result<PathBuf> {
    let root = match &settings.temp_dir {
        Some(dir) => dir.clone(),
        None => app
            .path()
            .app_cache_dir()
            .map_err(|e| AppError::Config(format!("не удалось определить кэш-директорию: {e}")))?,
    };
    Ok(root.join("jobs").join(job_id))
}

/// Путь выходного файла: {output_dir | рядом с исходником}/{stem}_4k60.{mkv|mp4},
/// с дедупликацией имени при коллизии.
pub fn output_path(source: &SourceInfo, settings: &UpscaleSettings) -> Result<PathBuf> {
    let dir = match &settings.output_dir {
        Some(d) => d.clone(),
        None => source
            .path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".")),
    };

    let stem = source
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let ext = match settings.container {
        Container::Mkv => "mkv",
        Container::Mp4 => "mp4",
    };

    let mut candidate = dir.join(format!("{stem}_4k60.{ext}"));
    let mut n = 1u32;
    while candidate.exists() {
        candidate = dir.join(format!("{stem}_4k60_{n}.{ext}"));
        n += 1;
    }
    Ok(candidate)
}
