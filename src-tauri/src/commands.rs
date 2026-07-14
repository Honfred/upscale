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
        let jobs = state.jobs.lock().unwrap_or_else(|e| e.into_inner());
        if jobs.any_running() {
            return Err(AppError::JobAlreadyRunning);
        }
    }

    let job_id = uuid::Uuid::new_v4().to_string();
    let cancel = tokio_util::sync::CancellationToken::new();

    {
        let mut jobs = state.jobs.lock().unwrap_or_else(|e| e.into_inner());
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

        // pipeline::run выполняется в ОТДЕЛЬНОМ spawn, а не напрямую здесь:
        // JoinHandle::await возвращает Err(JoinError), если внутренняя задача
        // запаниковала, — это позволяет честно завершить джобу (state=Error,
        // job://error) вместо того, чтобы она навсегда осталась в Running
        // (и JobAlreadyRunning блокировал бы все последующие запуски вплоть
        // до перезапуска приложения).
        let inner = tokio::spawn(async move { pipeline::run(ctx).await });
        let result = match inner.await {
            Ok(r) => r,
            Err(join_err) => Err(AppError::Other(format!(
                "внутренняя ошибка пайплайна (паника): {join_err}"
            ))),
        };
        let app_state = app_task.state::<AppState>();

        match result {
            Ok(done) => {
                {
                    let mut jobs = app_state.jobs.lock().unwrap_or_else(|e| e.into_inner());
                    jobs.set_state(&job_id_task, JobState::Done);
                    jobs.set_progress(&job_id_task, 1.0);
                }
                events::emit_done(&app_task, &done);
            }
            Err(AppError::Cancelled) => {
                {
                    let mut jobs = app_state.jobs.lock().unwrap_or_else(|e| e.into_inner());
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
                    let mut jobs = app_state.jobs.lock().unwrap_or_else(|e| e.into_inner());
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
        let jobs = state.jobs.lock().unwrap_or_else(|e| e.into_inner());
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
    let jobs = state.jobs.lock().unwrap_or_else(|e| e.into_inner());
    jobs.status(&job_id).ok_or(AppError::JobNotFound(job_id))
}

const SIDECAR_CHECK_TIMEOUT: Duration = Duration::from_secs(10);
/// Таймаут микро-прогона realesrgan на крошечном (32x32) изображении для
/// честной проверки Vulkan/GPU (см. probe_vulkan). realesrgan/rife с `-h`
/// завершаются ДО инициализации Vulkan-устройства, поэтому их успешный
/// запуск с `-h` НЕ доказывает работоспособность Vulkan — нужен реальный
/// прогон с фактическим обращением к GPU.
const VULKAN_CHECK_TIMEOUT: Duration = Duration::from_secs(20);

/// Пытается запустить sidecar с безвредными аргументами (`-version`/`-h`) и
/// дождаться его завершения с таймаутом. При таймауте убивает процесс, чтобы
/// не оставлять висящих дочерних процессов (например, если ncnn-бинарник
/// ждёт ввод при отсутствии Vulkan-устройства). Возвращает (код_выхода, stdout);
/// код_выхода = None, если процесс не запустился, был убит по таймауту, либо
/// завершился аварийно (CommandEvent::Error) — во всех этих случаях он
/// однозначно НЕ считается успехом вызывающим кодом.
async fn probe_sidecar(app: &AppHandle, name: &str, args: &[&str]) -> (Option<i32>, String) {
    let cmd = match app.shell().sidecar(name) {
        Ok(c) => c,
        Err(_) => return (None, String::new()),
    };
    let (mut rx, child) = match cmd.args(args).spawn() {
        Ok(v) => v,
        Err(_) => return (None, String::new()),
    };

    let wait = async move {
        let mut stdout = Vec::new();
        let mut code = None;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(bytes) => {
                    stdout.extend(bytes);
                    stdout.push(b'\n');
                }
                CommandEvent::Terminated(payload) => {
                    code = payload.code;
                    break;
                }
                CommandEvent::Error(_) => break,
                _ => {}
            }
        }
        (code, stdout)
    };

    match tokio::time::timeout(SIDECAR_CHECK_TIMEOUT, wait).await {
        Ok((code, stdout)) => (code, String::from_utf8_lossy(&stdout).to_string()),
        Err(_) => {
            let _ = child.kill();
            (None, String::new())
        }
    }
}

/// Запускает sidecar `name` и ждёт завершения с произвольным таймаутом,
/// возвращая true только при чистом завершении с кодом 0. В отличие от
/// `probe_sidecar`, не собирает stdout (не нужен вызывающему) и не убивает
/// таймаутом бесконечно — используется для однократных микро-прогонов вроде
/// генерации тестового PNG или апскейла одного кадра.
async fn spawn_and_wait_success(
    app: &AppHandle,
    name: &str,
    args: Vec<String>,
    timeout: Duration,
) -> bool {
    let cmd = match app.shell().sidecar(name) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let (mut rx, child) = match cmd.args(args).spawn() {
        Ok(v) => v,
        Err(_) => return false,
    };

    let wait = async move {
        let mut code = None;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Terminated(payload) => {
                    code = payload.code;
                    break;
                }
                CommandEvent::Error(_) => break,
                _ => {}
            }
        }
        code
    };

    match tokio::time::timeout(timeout, wait).await {
        Ok(code) => code == Some(0),
        Err(_) => {
            let _ = child.kill();
            false
        }
    }
}

/// Честная проверка Vulkan/GPU: генерирует крошечный (32x32) PNG через
/// ffmpeg и реально прогоняет его через realesrgan-ncnn-vulkan (-s 2). Успех
/// = процесс завершился с кодом 0 И выходной файл действительно создан.
/// В отличие от простого запуска `realesrgan-ncnn-vulkan -h` (который
/// возвращает управление ДО инициализации Vulkan-устройства и потому не
/// доказывает его работоспособность), этот прогон реально обращается к GPU.
async fn probe_vulkan(app: &AppHandle) -> bool {
    let dir = std::env::temp_dir().join(format!("animeupscale-vkcheck-{}", uuid::Uuid::new_v4()));
    if std::fs::create_dir_all(&dir).is_err() {
        return false;
    }
    let ok = probe_vulkan_inner(app, &dir).await;
    let _ = std::fs::remove_dir_all(&dir);
    ok
}

async fn probe_vulkan_inner(app: &AppHandle, dir: &std::path::Path) -> bool {
    let img_path = dir.join("probe.png");
    let out_path = dir.join("probe_out.png");

    let gen_ok = spawn_and_wait_success(
        app,
        crate::config::BIN_FFMPEG,
        vec![
            "-v".to_string(),
            "error".to_string(),
            "-y".to_string(),
            "-f".to_string(),
            "lavfi".to_string(),
            "-i".to_string(),
            "color=c=red:s=32x32:d=0.1".to_string(),
            "-frames:v".to_string(),
            "1".to_string(),
            img_path.to_string_lossy().to_string(),
        ],
        VULKAN_CHECK_TIMEOUT,
    )
    .await;
    if !gen_ok || !img_path.exists() {
        return false;
    }

    let models_dir = match crate::paths::esrgan_models_dir(app) {
        Ok(d) => d,
        Err(_) => return false,
    };

    let run_ok = spawn_and_wait_success(
        app,
        crate::config::BIN_REALESRGAN,
        vec![
            "-i".to_string(),
            img_path.to_string_lossy().to_string(),
            "-o".to_string(),
            out_path.to_string_lossy().to_string(),
            "-s".to_string(),
            "2".to_string(),
            "-n".to_string(),
            crate::config::ESRGAN_MODEL.to_string(),
            "-m".to_string(),
            models_dir.to_string_lossy().to_string(),
            "-t".to_string(),
            "32".to_string(),
        ],
        VULKAN_CHECK_TIMEOUT,
    )
    .await;

    run_ok && out_path.exists()
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
    // ffmpeg/ffprobe -version корректно завершаются кодом 0 — это честный
    // критерий "работает".
    let (ffmpeg_code, _) = probe_sidecar(&app, crate::config::BIN_FFMPEG, &["-version"]).await;
    let (ffprobe_code, _) = probe_sidecar(&app, crate::config::BIN_FFPROBE, &["-version"]).await;
    let ffmpeg_ok = ffmpeg_code == Some(0);
    let ffprobe_ok = ffprobe_code == Some(0);

    // ВАЖНО: realesrgan-ncnn-vulkan/rife-ncnn-vulkan с `-h` по факту всегда
    // завершаются кодом 255 (проверено на реальных бинарниках) — это их
    // нормальное поведение при выводе справки, а не признак сбоя. Поэтому
    // критерий успеха здесь — просто "процесс запустился и корректно
    // завершился сам" (code.is_some(), т.е. не был убит по таймауту и не
    // упал с ошибкой запуска), а НЕ code == 0. Из этой же причины
    // realesrgan_ok/rife_ok НЕ являются проверкой Vulkan (см. probe_vulkan
    // ниже) — они лишь подтверждают, что сам бинарник присутствует и
    // исполняем.
    let (realesrgan_code, _) = probe_sidecar(&app, crate::config::BIN_REALESRGAN, &["-h"]).await;
    let (rife_code, _) = probe_sidecar(&app, crate::config::BIN_RIFE, &["-h"]).await;
    let realesrgan_ok = realesrgan_code.is_some();
    let rife_ok = rife_code.is_some();

    let nvenc_codecs = if ffmpeg_ok {
        let (_, stdout) = probe_sidecar(&app, crate::config::BIN_FFMPEG, &["-hide_banner", "-encoders"]).await;
        parse_nvenc_codecs(&stdout)
    } else {
        Vec::new()
    };

    // Честная проверка Vulkan: realesrgan/rife с `-h` завершаются ДО
    // инициализации Vulkan-устройства, поэтому их запуск НЕ доказывает
    // работоспособность GPU (см. комментарий выше) — нужен реальный
    // микро-прогон апскейла на крошечном изображении.
    let vulkan_ok = probe_vulkan(&app).await;

    Ok(SystemInfo {
        vulkan_ok,
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
