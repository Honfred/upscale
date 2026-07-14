//! Публичный API для фронтенда. КОНТРАКТ с src/lib/api.ts.

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

use crate::config::{Codec, UpscaleSettings};
use crate::error::{AppError, Result};
use crate::estimate::{self, DiskEstimate};
use crate::events::{self, JobError};
use crate::pipeline::{self, PipelineCtx};
use crate::probe::{self, SourceInfo};
use crate::state::{AppState, JobState, JobStatus};

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
    probe::probe(&app, &path).await
}

#[tauri::command]
pub async fn estimate_job(
    app: AppHandle,
    source: SourceInfo,
    settings: UpscaleSettings,
) -> Result<DiskEstimate> {
    let temp_root = match &settings.temp_dir {
        Some(dir) => dir.clone(),
        None => app
            .path()
            .app_cache_dir()
            .map_err(|e| AppError::Config(e.to_string()))?,
    };
    estimate::estimate(&source, &settings, &temp_root)
}

/// Запускает джобу в tokio::spawn, регистрирует в AppState, возвращает job_id.
/// Терминальные события job://done / job://error эмитит обёртка вокруг
/// pipeline::run (сам пайплайн эмитит только промежуточные Stage/Started/...).
/// Отклоняет запуск при активной джобе (JobAlreadyRunning).
#[tauri::command]
pub async fn start_job(
    app: AppHandle,
    source: SourceInfo,
    settings: UpscaleSettings,
    state: State<'_, AppState>,
) -> Result<String> {
    {
        let jobs = state.jobs.lock().unwrap();
        if jobs.any_running() {
            return Err(AppError::JobAlreadyRunning);
        }
    }

    let job_id = uuid::Uuid::new_v4().to_string();
    let cancel = tokio_util::sync::CancellationToken::new();

    {
        let mut jobs = state.jobs.lock().unwrap();
        jobs.insert(job_id.clone(), cancel.clone());
    }

    let app_task = app.clone();
    let job_id_task = job_id.clone();

    tokio::spawn(async move {
        let ctx = PipelineCtx {
            app: app_task.clone(),
            job_id: job_id_task.clone(),
            source,
            settings,
            cancel,
        };

        let result = pipeline::run(ctx).await;
        let app_state = app_task.state::<AppState>();

        match result {
            Ok(done) => {
                {
                    let mut jobs = app_state.jobs.lock().unwrap();
                    jobs.set_state(&job_id_task, JobState::Done);
                    jobs.set_progress(&job_id_task, 1.0);
                }
                events::emit_done(&app_task, &done);
            }
            Err(AppError::Cancelled) => {
                {
                    let mut jobs = app_state.jobs.lock().unwrap();
                    jobs.set_state(&job_id_task, JobState::Cancelled);
                }
                // Решение: помимо смены статуса в реестре (для get_job_status /
                // восстановления UI после ремаунта) явно эмитим job://error с
                // recoverable=true. Это не строго обязательно (фронт, вызвавший
                // cancel_job, и так знает об отмене), но делает поведение
                // консистентным для любого слушателя job://* и переживает
                // пересоздание webview.
                events::emit_error(
                    &app_task,
                    &JobError {
                        job_id: job_id_task.clone(),
                        stage: None,
                        message: "Отменено пользователем".to_string(),
                        recoverable: true,
                    },
                );
            }
            Err(e) => {
                let recoverable = matches!(e, AppError::DiskSpace { .. } | AppError::Gpu(_));
                let message = e.to_string();
                {
                    let mut jobs = app_state.jobs.lock().unwrap();
                    jobs.set_state(&job_id_task, JobState::Error);
                }
                events::emit_error(
                    &app_task,
                    &JobError {
                        job_id: job_id_task.clone(),
                        stage: None,
                        message,
                        recoverable,
                    },
                );
            }
        }
    });

    Ok(job_id)
}

#[tauri::command]
pub async fn cancel_job(job_id: String, state: State<'_, AppState>) -> Result<()> {
    let cancel = {
        let jobs = state.jobs.lock().unwrap();
        jobs.cancel_token(&job_id)
    };
    match cancel {
        // CancellationToken::cancel() идемпотентен: повторный вызов на уже
        // отменённом/завершённом job_id не является ошибкой.
        Some(token) => {
            token.cancel();
            Ok(())
        }
        None => Err(AppError::JobNotFound(job_id)),
    }
}

#[tauri::command]
pub fn get_job_status(job_id: String, state: State<'_, AppState>) -> Result<JobStatus> {
    let jobs = state.jobs.lock().unwrap();
    jobs.status(&job_id).ok_or(AppError::JobNotFound(job_id))
}

const SIDECAR_CHECK_TIMEOUT: Duration = Duration::from_secs(10);

/// Пытается запустить sidecar с безвредными аргументами (`-version`/`-h`) и
/// дождаться его завершения с таймаутом. При таймауте убивает процесс, чтобы
/// не оставлять висящих дочерних процессов (например, если ncnn-бинарник
/// ждёт ввод при отсутствии Vulkan-устройства). Возвращает (успех, stdout).
async fn probe_sidecar(app: &AppHandle, name: &str, args: &[&str]) -> (bool, String) {
    let cmd = match app.shell().sidecar(name) {
        Ok(c) => c,
        Err(_) => return (false, String::new()),
    };
    let (mut rx, child) = match cmd.args(args).spawn() {
        Ok(v) => v,
        Err(_) => return (false, String::new()),
    };

    let wait = async move {
        let mut stdout = Vec::new();
        let mut terminated = false;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(bytes) => {
                    stdout.extend(bytes);
                    stdout.push(b'\n');
                }
                CommandEvent::Terminated(_) => {
                    terminated = true;
                    break;
                }
                CommandEvent::Error(_) => break,
                _ => {}
            }
        }
        (terminated, stdout)
    };

    match tokio::time::timeout(SIDECAR_CHECK_TIMEOUT, wait).await {
        Ok((ok, stdout)) => (ok, String::from_utf8_lossy(&stdout).to_string()),
        Err(_) => {
            let _ = child.kill();
            (false, String::new())
        }
    }
}

fn parse_nvenc_codecs(ffmpeg_encoders_stdout: &str) -> Vec<Codec> {
    let mut codecs = Vec::new();
    if ffmpeg_encoders_stdout.contains("h264_nvenc") {
        codecs.push(Codec::H264);
    }
    if ffmpeg_encoders_stdout.contains("hevc_nvenc") {
        codecs.push(Codec::Hevc);
    }
    if ffmpeg_encoders_stdout.contains("av1_nvenc") {
        codecs.push(Codec::Av1);
    }
    codecs
}

/// Проверка sidecar-бинарников, Vulkan/GPU и доступных NVENC-кодеков.
/// Ни одна из проверок не приводит к ошибке команды — при сбое поле просто
/// становится false/пустым.
#[tauri::command]
pub async fn system_check(app: AppHandle) -> Result<SystemInfo> {
    let (ffmpeg_ok, _) = probe_sidecar(&app, "ffmpeg", &["-version"]).await;
    let (ffprobe_ok, _) = probe_sidecar(&app, "ffprobe", &["-version"]).await;
    let (realesrgan_ok, _) = probe_sidecar(&app, "realesrgan-ncnn-vulkan", &["-h"]).await;
    let (rife_ok, _) = probe_sidecar(&app, "rife-ncnn-vulkan", &["-h"]).await;

    let nvenc_codecs = if ffmpeg_ok {
        let (_, stdout) = probe_sidecar(&app, "ffmpeg", &["-hide_banner", "-encoders"]).await;
        parse_nvenc_codecs(&stdout)
    } else {
        Vec::new()
    };

    Ok(SystemInfo {
        // realesrgan-ncnn-vulkan/rife-ncnn-vulkan не стартуют без рабочего
        // Vulkan-устройства, поэтому их совместный успешный запуск и есть
        // эвристическая проверка Vulkan. gpu_name пока не определяем (v1).
        vulkan_ok: realesrgan_ok && rife_ok,
        gpu_name: None,
        // ffmpeg_ok в SystemInfo объединяет ffmpeg и ffprobe: пайплайну и
        // probe::probe нужны оба инструмента одновременно.
        ffmpeg_ok: ffmpeg_ok && ffprobe_ok,
        realesrgan_ok,
        rife_ok,
        nvenc_codecs,
    })
}

/// Показать файл в файловом менеджере.
#[tauri::command]
pub async fn reveal_output(app: AppHandle, path: String) -> Result<()> {
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| AppError::Other(e.to_string()))
}
