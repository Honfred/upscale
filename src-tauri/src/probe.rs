//! ffprobe: параметры исходного видео. Реализация — задача A.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;

use crate::error::{AppError, Result};
use crate::process::run_sidecar_capture;

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
    /// true, если r_frame_rate заметно (>1%) отличается от avg_frame_rate —
    /// признак переменного кадра (VFR). Сегментация по фиксированному fps в
    /// этом случае может давать неточные границы (см. pipeline::run).
    pub is_vfr: bool,
    pub has_audio: bool,
    pub subtitle_streams: Vec<u32>,
    pub codec_name: String,
    pub pix_fmt: String,
}

#[derive(Deserialize)]
struct VideoStream {
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    nb_frames: Option<String>,
    pix_fmt: Option<String>,
    codec_name: Option<String>,
}

#[derive(Deserialize)]
struct FormatInfo {
    duration: Option<String>,
}

#[derive(Deserialize)]
struct FfprobeVideoOutput {
    #[serde(default)]
    streams: Vec<VideoStream>,
    format: Option<FormatInfo>,
}

#[derive(Deserialize)]
struct StreamMeta {
    index: u32,
    codec_type: String,
}

#[derive(Deserialize)]
struct FfprobeStreamsOutput {
    #[serde(default)]
    streams: Vec<StreamMeta>,
}

/// Разбирает строку вида "24000/1001" в f64. Возвращает None при делении на 0
/// или некорректном формате.
fn parse_ratio(s: &str) -> Option<f64> {
    let mut parts = s.splitn(2, '/');
    let num: f64 = parts.next()?.trim().parse().ok()?;
    let den: f64 = match parts.next() {
        Some(d) => d.trim().parse().ok()?,
        None => 1.0,
    };
    if den == 0.0 {
        None
    } else {
        Some(num / den)
    }
}

/// Выбирает fps из r_frame_rate/avg_frame_rate. Для VFR-контента (когда
/// avg_frame_rate заметно отличается от r_frame_rate) предпочитает
/// avg_frame_rate, т.к. r_frame_rate для VFR обычно завышен/неточен.
/// Возвращает (fps, is_vfr).
fn choose_fps(r_frame_rate: Option<&str>, avg_frame_rate: Option<&str>) -> Option<(f64, bool)> {
    let r = r_frame_rate.and_then(parse_ratio).filter(|v| *v > 0.0);
    let avg = avg_frame_rate.and_then(parse_ratio).filter(|v| *v > 0.0);
    match (r, avg) {
        (Some(r), Some(avg)) => {
            let is_vfr = ((r - avg).abs() / r) > 0.01;
            if is_vfr {
                Some((avg, true))
            } else {
                Some((r, false))
            }
        }
        (Some(r), None) => Some((r, false)),
        (None, Some(avg)) => Some((avg, false)),
        (None, None) => None,
    }
}

/// Запускает ffprobe (sidecar) и парсит JSON-вывод.
pub async fn probe(app: &AppHandle, path: &str) -> Result<SourceInfo> {
    let video_json = run_sidecar_capture(
        app,
        crate::config::BIN_FFPROBE,
        &[
            "-v".to_string(),
            "error".to_string(),
            "-select_streams".to_string(),
            "v:0".to_string(),
            "-show_entries".to_string(),
            "stream=width,height,r_frame_rate,avg_frame_rate,nb_frames,pix_fmt,codec_name"
                .to_string(),
            "-show_entries".to_string(),
            "format=duration".to_string(),
            "-of".to_string(),
            "json".to_string(),
            path.to_string(),
        ],
    )
    .await?;

    let parsed: FfprobeVideoOutput = serde_json::from_str(&video_json)
        .map_err(|e| AppError::Probe(format!("не удалось разобрать JSON ffprobe: {e}")))?;

    let stream = parsed
        .streams
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Probe("в файле не найден видеопоток".to_string()))?;

    let width = stream
        .width
        .ok_or_else(|| AppError::Probe("не удалось определить ширину видео".to_string()))?;
    let height = stream
        .height
        .ok_or_else(|| AppError::Probe("не удалось определить высоту видео".to_string()))?;

    let (fps, is_vfr) = choose_fps(stream.r_frame_rate.as_deref(), stream.avg_frame_rate.as_deref())
        .ok_or_else(|| AppError::Probe("не удалось определить fps".to_string()))?;
    let fps = fps as f32;

    let duration_sec: f64 = parsed
        .format
        .and_then(|f| f.duration)
        .and_then(|d| d.trim().parse().ok())
        .ok_or_else(|| AppError::Probe("не удалось определить длительность".to_string()))?;

    let frame_count = stream
        .nb_frames
        .as_deref()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .filter(|&n| n > 0)
        .unwrap_or_else(|| (fps as f64 * duration_sec).round() as u64);

    let codec_name = stream.codec_name.unwrap_or_default();
    let pix_fmt = stream.pix_fmt.unwrap_or_default();

    let streams_json = run_sidecar_capture(
        app,
        crate::config::BIN_FFPROBE,
        &[
            "-v".to_string(),
            "error".to_string(),
            "-show_entries".to_string(),
            "stream=index,codec_type".to_string(),
            "-of".to_string(),
            "json".to_string(),
            path.to_string(),
        ],
    )
    .await?;

    let streams_parsed: FfprobeStreamsOutput = serde_json::from_str(&streams_json)
        .map_err(|e| AppError::Probe(format!("не удалось разобрать JSON потоков: {e}")))?;

    let has_audio = streams_parsed
        .streams
        .iter()
        .any(|s| s.codec_type == "audio");
    let subtitle_streams = streams_parsed
        .streams
        .iter()
        .filter(|s| s.codec_type == "subtitle")
        .map(|s| s.index)
        .collect();

    Ok(SourceInfo {
        path: PathBuf::from(path),
        width,
        height,
        fps,
        duration_sec,
        frame_count,
        is_vfr,
        has_audio,
        subtitle_streams,
        codec_name,
        pix_fmt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ratio_ntsc() {
        assert!((parse_ratio("24000/1001").unwrap() - 23.976_023_976).abs() < 1e-6);
    }

    #[test]
    fn parse_ratio_integer() {
        assert_eq!(parse_ratio("25/1").unwrap(), 25.0);
    }

    #[test]
    fn parse_ratio_no_denominator() {
        assert_eq!(parse_ratio("30").unwrap(), 30.0);
    }

    #[test]
    fn parse_ratio_zero_denominator() {
        assert!(parse_ratio("0/0").is_none());
    }

    #[test]
    fn choose_fps_prefers_r_when_close() {
        let (fps, is_vfr) = choose_fps(Some("24000/1001"), Some("23976/1000")).unwrap();
        assert!((fps - 23.976_023_976).abs() < 1e-3);
        assert!(!is_vfr);
    }

    #[test]
    fn choose_fps_prefers_avg_for_vfr() {
        // r_frame_rate завышен (типично для VFR), avg_frame_rate ближе к реальности.
        let (fps, is_vfr) = choose_fps(Some("60/1"), Some("24/1")).unwrap();
        assert_eq!(fps, 24.0);
        assert!(is_vfr);
    }

    #[test]
    fn choose_fps_falls_back_to_only_available() {
        assert_eq!(choose_fps(None, Some("30/1")), Some((30.0, false)));
        assert_eq!(choose_fps(Some("30/1"), None), Some((30.0, false)));
        assert_eq!(choose_fps(None, None), None);
    }
}
