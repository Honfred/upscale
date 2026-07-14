//! Оценка требований к диску и авто-подбор segment_seconds.
//! КРИТИЧНО для 16GB RAM / ограниченного диска. Реализация — задача A.

use serde::Serialize;

use crate::config::UpscaleSettings;
use crate::error::Result;
use crate::probe::SourceInfo;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskEstimate {
    pub temp_peak_bytes: u64,
    pub temp_total_written: u64,
    pub output_bytes_est: u64,
    pub free_bytes: u64,
    pub sufficient: bool,
    /// Фактический сегмент (сек), который будет использован.
    pub segment_seconds: u32,
    /// Фактический масштаб модели (2/3/4).
    pub scale: u32,
    pub out_width: u32,
    pub out_height: u32,
}

/// Оценивает пик temp-диска (кадры in/up/rife одного сегмента) и подбирает
/// segment_seconds так, чтобы пик был < 60% свободного места (диапазон 6..20 с),
/// если пользователь не задал его явно.
pub fn estimate(source: &SourceInfo, settings: &UpscaleSettings, temp_root: &std::path::Path) -> Result<DiskEstimate> {
    let _ = (source, settings, temp_root);
    todo!("задача A: PNG 4K ≈ 10МБ/кадр, см. docs/PIPELINE.md")
}
