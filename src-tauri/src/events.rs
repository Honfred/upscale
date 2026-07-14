//! События прогресса. КОНТРАКТ с src/lib/events.ts и types.ts.
//! Каналы: job://progress (JobEvent), job://done (JobDone), job://error (JobError).

use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const EV_PROGRESS: &str = "job://progress";
pub const EV_DONE: &str = "job://done";
pub const EV_ERROR: &str = "job://error";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Decode,
    Upscale,
    Interpolate,
    Encode,
    Concat,
}

/// Веса стадий для overall_progress (сумма = 1.0).
pub fn stage_weight(stage: Stage) -> f32 {
    match stage {
        Stage::Decode => 0.05,
        Stage::Upscale => 0.65,
        Stage::Interpolate => 0.20,
        Stage::Encode => 0.09,
        Stage::Concat => 0.01,
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum JobEvent {
    Started {
        job_id: String,
        total_segments: u32,
        total_frames: u64,
    },
    Stage {
        job_id: String,
        stage: Stage,
        segment_index: u32,
        total_segments: u32,
        /// 0.0..1.0 внутри текущей стадии текущего сегмента.
        stage_progress: f32,
        /// 0.0..1.0 по всей джобе (взвешенно по stage_weight).
        overall_progress: f32,
        fps_now: Option<f32>,
        eta_seconds: Option<u64>,
        frames_done: u64,
        frames_total: u64,
    },
    SegmentDone {
        job_id: String,
        segment_index: u32,
    },
    Warning {
        job_id: String,
        message: String,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDone {
    pub job_id: String,
    pub output_path: String,
    pub elapsed_sec: u64,
    pub output_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobError {
    pub job_id: String,
    pub stage: Option<Stage>,
    pub message: String,
    pub recoverable: bool,
}

pub fn emit_progress(app: &AppHandle, ev: &JobEvent) {
    let _ = app.emit(EV_PROGRESS, ev);
}

pub fn emit_done(app: &AppHandle, ev: &JobDone) {
    let _ = app.emit(EV_DONE, ev);
}

pub fn emit_error(app: &AppHandle, ev: &JobError) {
    let _ = app.emit(EV_ERROR, ev);
}
