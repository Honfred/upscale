//! Оркестратор джобы: последовательная обработка сегментов
//! decode → upscale → interpolate → encode, затем concat + очистка.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use crate::config::UpscaleSettings;
use crate::error::{AppError, Result};
use crate::estimate;
use crate::events::{self, JobDone, JobEvent, Stage};
use crate::paths;
use crate::probe::SourceInfo;

mod cleanup;
mod concat;
mod decode;
mod encode;
mod interpolate;
mod progress;
mod segment;
mod upscale;

use segment::Segment;

pub struct PipelineCtx {
    pub app: AppHandle,
    pub job_id: String,
    pub source: SourceInfo,
    pub settings: UpscaleSettings,
    pub cancel: CancellationToken,
}

/// Троттлинг эмита Stage-событий.
const STAGE_EMIT_INTERVAL: Duration = Duration::from_millis(100);
/// ETA/fps не показываются первые ~3с джобы (нет ещё стабильной скорости).
const WARMUP: Duration = Duration::from_secs(3);
/// Коэффициент сглаживания EMA скорости обработки (кадры/с).
const EMA_ALPHA: f32 = 0.2;

/// Веса стадий одного сегмента (Decode/Upscale/Interpolate/Encode).
/// Их сумма по умолчанию 0.99 (см. events::stage_weight), оставшийся 0.01
/// приходится на Concat, который считается ОДИН РАЗ на всю джобу (не за
/// сегмент). Если интерполяция пропущена, её вес перераспределяется между
/// остальными тремя стадиями пропорционально — сумма (0.99) не меняется,
/// поэтому формула overall_progress ниже остаётся верной независимо от
/// того, выполняется интерполяция или нет.
#[derive(Clone, Copy)]
struct StageWeights {
    decode: f32,
    upscale: f32,
    interpolate: f32,
    encode: f32,
}

impl StageWeights {
    fn new(interpolating: bool) -> Self {
        let decode = events::stage_weight(Stage::Decode);
        let upscale = events::stage_weight(Stage::Upscale);
        let interpolate = events::stage_weight(Stage::Interpolate);
        let encode = events::stage_weight(Stage::Encode);

        if interpolating {
            Self {
                decode,
                upscale,
                interpolate,
                encode,
            }
        } else {
            let total = decode + upscale + interpolate + encode;
            let remaining = decode + upscale + encode;
            let factor = total / remaining;
            Self {
                decode: decode * factor,
                upscale: upscale * factor,
                interpolate: 0.0,
                encode: encode * factor,
            }
        }
    }

    /// Сумма весов стадий, идущих строго до `stage` (в порядке
    /// decode -> upscale -> interpolate -> encode).
    fn completed_before(&self, stage: Stage) -> f32 {
        match stage {
            Stage::Decode => 0.0,
            Stage::Upscale => self.decode,
            Stage::Interpolate => self.decode + self.upscale,
            Stage::Encode => self.decode + self.upscale + self.interpolate,
            Stage::Concat => self.decode + self.upscale + self.interpolate + self.encode,
        }
    }

    fn weight(&self, stage: Stage) -> f32 {
        match stage {
            Stage::Decode => self.decode,
            Stage::Upscale => self.upscale,
            Stage::Interpolate => self.interpolate,
            Stage::Encode => self.encode,
            Stage::Concat => events::stage_weight(Stage::Concat),
        }
    }

    /// Сумма весов Decode+Upscale+Interpolate+Encode (всегда ~0.99):
    /// "единица" одного полностью пройденного сегмента в overall_progress.
    fn segment_unit(&self) -> f32 {
        self.decode + self.upscale + self.interpolate + self.encode
    }
}

/// Общее состояние для расчёта fps_now/eta_seconds (EMA по джобе) и
/// троттлинга эмита Stage-событий. Обёрнуто в Arc<Mutex<..>>, т.к. колбэки
/// прогресса upscale/interpolate вызываются из отдельного tokio-таска
/// (см. pipeline/progress.rs), поэтому не могут просто занимать `&mut` из
/// текущего стека вызовов.
struct ProgressState {
    job_start: Instant,
    last_emit: Instant,
    ema_fps: Option<f32>,
    last_sample_at: Instant,
    last_sample_frames: u64,
    /// Кадры, "накопленные" завершёнными стадиями (в единицах Started.total_frames).
    frames_done_before_stage: u64,
    total_frames_job: u64,
}

type SharedProgress = Arc<Mutex<ProgressState>>;

fn new_shared_progress(total_frames_job: u64) -> SharedProgress {
    let now = Instant::now();
    Arc::new(Mutex::new(ProgressState {
        job_start: now,
        // Позволяем первому событию проэмититься сразу.
        last_emit: now - STAGE_EMIT_INTERVAL,
        ema_fps: None,
        last_sample_at: now,
        last_sample_frames: 0,
        frames_done_before_stage: 0,
        total_frames_job,
    }))
}

/// Формирует и (при необходимости, с учётом троттлинга) эмитит
/// JobEvent::Stage. `force` игнорирует троттлинг (используется на 100%
/// каждой стадии, чтобы гарантированно показать её завершение).
#[allow(clippy::too_many_arguments)]
fn report_stage(
    app: &AppHandle,
    job_id: &str,
    shared: &SharedProgress,
    weights: &StageWeights,
    segment_index: u32,
    total_segments: u32,
    stage: Stage,
    frames_done_stage: u64,
    frames_total_stage: u64,
    force: bool,
) {
    let mut state = match shared.lock() {
        Ok(s) => s,
        Err(_) => return,
    };

    let frames_done_stage = frames_done_stage.min(frames_total_stage.max(frames_done_stage));
    let frames_done_global = state.frames_done_before_stage + frames_done_stage;

    let now = Instant::now();
    let dt = now.duration_since(state.last_sample_at).as_secs_f32();
    if dt >= 0.05 {
        let df = frames_done_global.saturating_sub(state.last_sample_frames) as f32;
        let inst_fps = if dt > 0.0 { df / dt } else { 0.0 };
        state.ema_fps = Some(match state.ema_fps {
            Some(prev) => EMA_ALPHA * inst_fps + (1.0 - EMA_ALPHA) * prev,
            None => inst_fps,
        });
        state.last_sample_at = now;
        state.last_sample_frames = frames_done_global;
    }

    let is_final_of_stage = frames_done_stage >= frames_total_stage;
    if !force && !is_final_of_stage && now.duration_since(state.last_emit) < STAGE_EMIT_INTERVAL {
        return;
    }
    state.last_emit = now;

    let stage_progress = if frames_total_stage == 0 {
        1.0
    } else {
        (frames_done_stage as f32 / frames_total_stage as f32).clamp(0.0, 1.0)
    };

    let completed_before = weights.completed_before(stage);
    let stage_weight = weights.weight(stage);
    let segment_unit = weights.segment_unit();

    // Concat — не по-сегментный шаг (выполняется один раз на всю джобу),
    // поэтому его вклад НЕ делится на total_segments: он просто достраивает
    // прогресс от уже гарантированных 0.99 (segment_unit) до 1.0.
    let overall_progress = if matches!(stage, Stage::Concat) {
        segment_unit + stage_weight * stage_progress
    } else {
        ((segment_index as f32) * segment_unit + completed_before + stage_weight * stage_progress)
            / (total_segments as f32).max(1.0)
    };
    let overall_progress = overall_progress.clamp(0.0, 1.0);

    let (fps_now, eta_seconds) = if state.job_start.elapsed() < WARMUP {
        (None, None)
    } else {
        match state.ema_fps {
            Some(fps) if fps > 0.01 => {
                let remaining = state
                    .total_frames_job
                    .saturating_sub(frames_done_global) as f32;
                (Some(fps), Some((remaining / fps).max(0.0) as u64))
            }
            _ => (None, None),
        }
    };

    events::emit_progress(
        app,
        &JobEvent::Stage {
            job_id: job_id.to_string(),
            stage,
            segment_index,
            total_segments,
            stage_progress,
            overall_progress,
            fps_now,
            eta_seconds,
            frames_done: frames_done_stage,
            frames_total: frames_total_stage,
        },
    );
}

fn mark_stage_done(shared: &SharedProgress, frames_total_stage: u64) {
    if let Ok(mut state) = shared.lock() {
        state.frames_done_before_stage += frames_total_stage;
    }
}

/// Выходное число кадров сегмента, которое реально "потребляет" encode:
/// после интерполяции (если она выполняется) либо после апскейла.
fn encode_input_frames(seg: &Segment, interpolating: bool, fps: f32, target_fps: Option<f32>) -> u64 {
    if interpolating {
        interpolate::target_frames_for_segment(seg, fps, target_fps.unwrap())
    } else {
        seg.frame_count
    }
}

/// Суммарное число "выходных" кадров всех стадий всех сегментов. Используется
/// как знаменатель для fps/ETA и как поле Started.total_frames.
///
/// Для сегмента это: decode_out (=seg.frame_count) + upscale_out
/// (=seg.frame_count, апскейл не меняет число кадров) + interpolate_out
/// (0, если стадия пропущена, иначе результат target_frames_for_segment) +
/// encode_out (столько же кадров, сколько encode реально кодирует —
/// после интерполяции либо после апскейла).
fn total_job_frames(segments: &[Segment], interpolating: bool, fps: f32, target_fps: Option<f32>) -> u64 {
    segments
        .iter()
        .map(|seg| {
            let decode_out = seg.frame_count;
            let upscale_out = seg.frame_count;
            let interpolate_out = if interpolating {
                interpolate::target_frames_for_segment(seg, fps, target_fps.unwrap())
            } else {
                0
            };
            let encode_out = encode_input_frames(seg, interpolating, fps, target_fps);
            decode_out + upscale_out + interpolate_out + encode_out
        })
        .sum()
}

/// Выполняет полный пайплайн. Эмитит события через crate::events
/// (Started, Stage с троттлингом ~10Гц, SegmentDone). Терминальные
/// job://done / job://error эмитит ВЫЗЫВАЮЩИЙ (state.rs), не пайплайн.
/// Проверяет cancel после каждого шага; чистит temp согласно keep_intermediate.
/// На отмену/ошибку сам pipeline::run отвечает за полную очистку temp-корня
/// джобы (если !keep_intermediate) перед тем как вернуть ошибку вызывающему.
pub async fn run(ctx: PipelineCtx) -> Result<JobDone> {
    let temp_root = paths::job_temp_dir(&ctx.app, &ctx.settings, &ctx.job_id)?;
    std::fs::create_dir_all(&temp_root)?;

    let result = run_inner(&ctx, &temp_root).await;

    if result.is_err() {
        let _ = cleanup::remove_job_temp(&temp_root, ctx.settings.keep_intermediate);
    }

    result
}

async fn run_inner(ctx: &PipelineCtx, temp_root: &std::path::Path) -> Result<JobDone> {
    let PipelineCtx {
        app,
        job_id,
        source,
        settings,
        cancel,
    } = ctx;

    let job_start = Instant::now();

    let est = estimate::estimate(source, settings, temp_root)?;
    let scale = est.scale;
    // Кадры up/rife лежат на диске в СЫРОМ (не capped) разрешении апскейла —
    // downscale до target_width применяется только внутри encode. См.
    // комментарий в estimate::estimate.
    let raw_width = source.width * scale;
    let raw_height = source.height * scale;

    let segments = segment::compute_segments(source.frame_count, source.fps, est.segment_seconds);
    let total_segments = segments.len() as u32;
    if total_segments == 0 {
        return Err(AppError::Other(
            "в исходном видео не найдено кадров для обработки".to_string(),
        ));
    }

    let interpolating = interpolate::should_interpolate(source.fps, settings.target_fps);
    let weights = StageWeights::new(interpolating);

    let total_frames_job = total_job_frames(&segments, interpolating, source.fps, settings.target_fps);

    events::emit_progress(
        app,
        &JobEvent::Started {
            job_id: job_id.clone(),
            total_segments,
            total_frames: total_frames_job,
        },
    );

    let esrgan_models_dir = paths::esrgan_models_dir(app)?;
    let rife_dir = if interpolating {
        Some(paths::rife_model_dir(app)?)
    } else {
        None
    };

    let shared_progress = new_shared_progress(total_frames_job);

    let out_fps = if interpolating {
        settings.target_fps.unwrap()
    } else {
        source.fps
    };

    let mut segment_outputs: Vec<PathBuf> = Vec::with_capacity(segments.len());

    for seg in &segments {
        if cancel.is_cancelled() {
            return Err(AppError::Cancelled);
        }

        let seg_dir = temp_root.join(format!("seg{:04}", seg.index));
        std::fs::create_dir_all(&seg_dir)?;

        // Проверка свободного места непосредственно перед сегментом (диск
        // мог заполниться другими процессами/предыдущими сегментами).
        let free_now = estimate::available_space(temp_root)?;
        let peak_now =
            estimate::segment_peak_bytes(source, settings, raw_width, raw_height, seg.frame_count);
        if peak_now as f64 >= free_now as f64 {
            return Err(AppError::DiskSpace {
                needed: peak_now,
                free: free_now,
            });
        }

        // --- decode ---
        // Колбэк decode вызывается синхронно внутри run_sidecar (не в
        // спавненном таске), поэтому можно просто занимать ссылки на стек.
        if cancel.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        decode::decode_segment(
            app,
            &source.path,
            source.fps,
            seg,
            &seg_dir,
            cancel,
            |done| {
                report_stage(
                    app,
                    job_id,
                    &shared_progress,
                    &weights,
                    seg.index,
                    total_segments,
                    Stage::Decode,
                    done,
                    seg.frame_count,
                    false,
                );
            },
        )
        .await?;
        report_stage(
            app,
            job_id,
            &shared_progress,
            &weights,
            seg.index,
            total_segments,
            Stage::Decode,
            seg.frame_count,
            seg.frame_count,
            true,
        );
        mark_stage_done(&shared_progress, seg.frame_count);

        // --- upscale ---
        // Колбэк upscale вызывается из фонового tokio-таска (см.
        // pipeline/progress.rs), поэтому должен быть Send + 'static —
        // клонируем всё нужное состояние в замыкание.
        if cancel.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        {
            let app_c = app.clone();
            let job_id_c = job_id.clone();
            let shared_c = shared_progress.clone();
            let weights_c = weights;
            let seg_index = seg.index;
            let seg_total = seg.frame_count;
            upscale::upscale_segment(
                app,
                &seg_dir,
                &esrgan_models_dir,
                scale,
                seg.frame_count,
                cancel,
                move |done| {
                    report_stage(
                        &app_c,
                        &job_id_c,
                        &shared_c,
                        &weights_c,
                        seg_index,
                        total_segments,
                        Stage::Upscale,
                        done,
                        seg_total,
                        false,
                    );
                },
            )
            .await?;
        }
        report_stage(
            app,
            job_id,
            &shared_progress,
            &weights,
            seg.index,
            total_segments,
            Stage::Upscale,
            seg.frame_count,
            seg.frame_count,
            true,
        );
        mark_stage_done(&shared_progress, seg.frame_count);

        // Если интерполяция пропущена, encode будет читать из up/, поэтому
        // in/ можно освободить уже сейчас (раньше, чем при обычном тайминге
        // "после interpolate") — см. cleanup.rs.
        if !interpolating {
            cleanup::remove_in_dir(&seg_dir, settings.keep_intermediate)?;
        }

        // --- interpolate (опционально) ---
        let (encode_frames_dir_name, encode_input_total) = if interpolating {
            if cancel.is_cancelled() {
                return Err(AppError::Cancelled);
            }
            let target_frames = interpolate::target_frames_for_segment(
                seg,
                source.fps,
                settings.target_fps.unwrap(),
            );
            {
                let app_c = app.clone();
                let job_id_c = job_id.clone();
                let shared_c = shared_progress.clone();
                let weights_c = weights;
                let seg_index = seg.index;
                interpolate::interpolate_segment(
                    app,
                    &seg_dir,
                    rife_dir.as_ref().expect("rife_dir должен быть Some при interpolating=true"),
                    target_frames,
                    cancel,
                    move |done| {
                        report_stage(
                            &app_c,
                            &job_id_c,
                            &shared_c,
                            &weights_c,
                            seg_index,
                            total_segments,
                            Stage::Interpolate,
                            done,
                            target_frames,
                            false,
                        );
                    },
                )
                .await?;
            }
            report_stage(
                app,
                job_id,
                &shared_progress,
                &weights,
                seg.index,
                total_segments,
                Stage::Interpolate,
                target_frames,
                target_frames,
                true,
            );
            mark_stage_done(&shared_progress, target_frames);

            // in/ и up/ полностью потреблены интерполяцией.
            cleanup::remove_in_dir(&seg_dir, settings.keep_intermediate)?;
            cleanup::remove_up_dir(&seg_dir, settings.keep_intermediate)?;

            ("rife", target_frames)
        } else {
            ("up", seg.frame_count)
        };

        // --- encode ---
        // Синхронный колбэк (парсинг stderr внутри run_sidecar), Send не нужен.
        if cancel.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        encode::encode_segment(
            app,
            &seg_dir,
            encode_frames_dir_name,
            out_fps,
            raw_width,
            settings,
            encode_input_total,
            cancel,
            |done| {
                report_stage(
                    app,
                    job_id,
                    &shared_progress,
                    &weights,
                    seg.index,
                    total_segments,
                    Stage::Encode,
                    done,
                    encode_input_total,
                    false,
                );
            },
        )
        .await?;
        report_stage(
            app,
            job_id,
            &shared_progress,
            &weights,
            seg.index,
            total_segments,
            Stage::Encode,
            encode_input_total,
            encode_input_total,
            true,
        );
        mark_stage_done(&shared_progress, encode_input_total);

        // Освобождаем последнюю оставшуюся директорию кадров сегмента.
        if interpolating {
            cleanup::remove_rife_dir(&seg_dir, settings.keep_intermediate)?;
        } else {
            cleanup::remove_up_dir(&seg_dir, settings.keep_intermediate)?;
        }

        segment_outputs.push(seg_dir.join("out.mkv"));

        events::emit_progress(
            app,
            &JobEvent::SegmentDone {
                job_id: job_id.clone(),
                segment_index: seg.index,
            },
        );
    }

    if cancel.is_cancelled() {
        return Err(AppError::Cancelled);
    }

    report_stage(
        app,
        job_id,
        &shared_progress,
        &weights,
        total_segments,
        total_segments,
        Stage::Concat,
        0,
        1,
        true,
    );

    let output_path = paths::output_path(source, settings)?;
    let warnings = concat::concat_segments(
        app,
        &segment_outputs,
        temp_root,
        source,
        settings,
        &output_path,
        cancel,
    )
    .await?;

    report_stage(
        app,
        job_id,
        &shared_progress,
        &weights,
        total_segments,
        total_segments,
        Stage::Concat,
        1,
        1,
        true,
    );

    for w in warnings {
        events::emit_progress(
            app,
            &JobEvent::Warning {
                job_id: job_id.clone(),
                message: w,
            },
        );
    }

    let output_bytes = std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0);

    cleanup::remove_job_temp(temp_root, settings.keep_intermediate)?;

    Ok(JobDone {
        job_id: job_id.clone(),
        output_path: output_path.to_string_lossy().to_string(),
        elapsed_sec: job_start.elapsed().as_secs(),
        output_bytes,
    })
}
