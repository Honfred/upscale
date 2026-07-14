//! ffprobe: параметры исходного видео. Реализация — задача A.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;

use crate::error::Result;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    /// Вычисленный из r_frame_rate (напр. 24000/1001 = 23.976).
    pub fps: f32,
    pub duration_sec: f64,
    /// nb_frames либо round(fps * duration).
    pub frame_count: u64,
    pub has_audio: bool,
    pub subtitle_streams: Vec<u32>,
    pub codec_name: String,
    pub pix_fmt: String,
}

/// Запускает ffprobe (sidecar) и парсит JSON-вывод.
pub async fn probe(app: &AppHandle, path: &str) -> Result<SourceInfo> {
    let _ = (app, path);
    todo!("задача A: ffprobe -show_entries stream/format -of json")
}
