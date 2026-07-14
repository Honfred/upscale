//! Оркестратор джобы: последовательная обработка сегментов
//! decode → upscale → interpolate → encode, затем concat + очистка.
//! Реализация — задача A (submodules: segment, decode, upscale, interpolate,
//! encode, concat, cleanup — создать по мере необходимости).

use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use crate::config::UpscaleSettings;
use crate::error::Result;
use crate::events::JobDone;
use crate::probe::SourceInfo;

pub struct PipelineCtx {
    pub app: AppHandle,
    pub job_id: String,
    pub source: SourceInfo,
    pub settings: UpscaleSettings,
    pub cancel: CancellationToken,
}

/// Выполняет полный пайплайн. Эмитит события через crate::events
/// (Started, Stage с троттлингом ~10Гц, SegmentDone). Терминальные
/// job://done / job://error эмитит ВЫЗЫВАЮЩИЙ (state.rs), не пайплайн.
/// Проверяет cancel после каждого шага; чистит temp согласно keep_intermediate.
pub async fn run(ctx: PipelineCtx) -> Result<JobDone> {
    let _ = ctx;
    todo!("задача A")
}
