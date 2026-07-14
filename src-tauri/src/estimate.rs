//! Оценка требований к диску и авто-подбор segment_seconds.
//! КРИТИЧНО для 16GB RAM / ограниченного диска. Реализация — задача A.

use serde::Serialize;
use std::path::Path;

use crate::config::UpscaleSettings;
use crate::error::{AppError, Result};
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

/// Доля свободного места, которую не должен превышать пиковый расход temp.
const PEAK_FRACTION: f64 = 0.6;
/// Диапазон авто-подбора segment_seconds.
const SEGMENT_MIN: u32 = 6;
const SEGMENT_MAX: u32 = 20;
/// PNG (аниме, плоские цвета) весит примерно 0.35 от несжатого RGB24-кадра.
const PNG_COMPRESSION_FACTOR: f64 = 0.35;
/// Грубая оценка выходного битрейта HEVC 4K60 (бит/с).
const OUTPUT_BITRATE_BPS: f64 = 15_000_000.0;

fn bytes_per_png(width: u32, height: u32) -> f64 {
    width as f64 * height as f64 * 3.0 * PNG_COMPRESSION_FACTOR
}

/// Итоговое разрешение после апскейла с учётом ограничения по target_width
/// (соответствует -vf scale=target_width:-2 в encode.rs: высота подгоняется
/// под чётное значение, сохраняя пропорции).
pub(crate) fn capped_out_resolution(
    source: &SourceInfo,
    settings: &UpscaleSettings,
    scale: u32,
) -> (u32, u32) {
    let raw_w = source.width * scale;
    let raw_h = source.height * scale;
    if raw_w > settings.target_width && raw_w > 0 {
        let target_w = settings.target_width;
        let mut h = ((raw_h as f64 * target_w as f64 / raw_w as f64) / 2.0).round() as u32 * 2;
        if h == 0 {
            h = 2;
        }
        (target_w, h)
    } else {
        (raw_w, raw_h)
    }
}

/// true, если интерполяция кадров будет реально выполняться (не пропущена).
fn will_interpolate(source_fps: f32, target_fps: Option<f32>) -> bool {
    target_fps.map(|t| t > source_fps).unwrap_or(false)
}

/// round(segment_seconds * fps) — число кадров в "полном" сегменте.
pub(crate) fn frames_for_seconds(segment_seconds: u32, fps: f32) -> u64 {
    (segment_seconds as f64 * fps as f64).round().max(1.0) as u64
}

/// Пиковый объём данных на диске для сегмента длиной `seg_frames` исходных
/// кадров: {seg}/in (source-разрешение) + {seg}/up (out-разрешение) +
/// {seg}/rife (out-разрешение, если интерполяция включена) + {seg}/out.mkv
/// (грубая оценка по битрейту). Принимает число кадров напрямую (а не
/// секунды), чтобы одной и той же функцией можно было точно оценить как
/// "усреднённый" сегмент при авто-подборе segment_seconds, так и фактический
/// (в т.ч. последний, неполный) сегмент в pipeline::run.
pub(crate) fn segment_peak_bytes(
    source: &SourceInfo,
    settings: &UpscaleSettings,
    out_width: u32,
    out_height: u32,
    seg_frames: u64,
) -> u64 {
    let seg_frames_f = (seg_frames.max(1)) as f64;
    let duration = seg_frames_f / source.fps as f64;

    let bytes_in = seg_frames_f * bytes_per_png(source.width, source.height);
    let bytes_up = seg_frames_f * bytes_per_png(out_width, out_height);

    let bytes_rife = if will_interpolate(source.fps, settings.target_fps) {
        let target_fps = settings.target_fps.unwrap() as f64;
        let out_frames = (duration * target_fps).round().max(1.0);
        out_frames * bytes_per_png(out_width, out_height)
    } else {
        0.0
    };

    let bytes_mkv = duration * OUTPUT_BITRATE_BPS / 8.0;

    (bytes_in + bytes_up + bytes_rife + bytes_mkv).round() as u64
}

/// Свободное место на устройстве, содержащем `path`. Если `path` ещё не
/// существует (temp-директория джобы создаётся позже), поднимается по
/// дереву до первого существующего предка.
pub(crate) fn available_space(path: &Path) -> Result<u64> {
    let mut p = path.to_path_buf();
    loop {
        if p.as_os_str().is_empty() {
            return fs2::available_space(Path::new(".")).map_err(AppError::Io);
        }
        if p.exists() {
            return fs2::available_space(&p).map_err(AppError::Io);
        }
        if !p.pop() {
            return fs2::available_space(Path::new(".")).map_err(AppError::Io);
        }
    }
}

/// Оценивает пик temp-диска (кадры in/up/rife одного сегмента) и подбирает
/// segment_seconds так, чтобы пик был < 60% свободного места (диапазон 6..20 с),
/// если пользователь не задал его явно.
pub fn estimate(
    source: &SourceInfo,
    settings: &UpscaleSettings,
    temp_root: &Path,
) -> Result<DiskEstimate> {
    let scale = settings.scale_for(source.width);
    // out_width/out_height — ФИНАЛЬНОЕ разрешение видео (для UI/отчёта), с
    // учётом возможного downscale в encode. Промежуточные PNG в up/ и rife/
    // при этом лежат на диске в СЫРОМ (не capped) разрешении апскейла —
    // downscale до target_width применяется только внутри ffmpeg на стадии
    // encode (-vf scale=...), а не заранее. Поэтому для оценки пикового
    // расхода диска используется raw_width/raw_height, а не capped.
    let (out_width, out_height) = capped_out_resolution(source, settings, scale);
    let raw_width = source.width * scale;
    let raw_height = source.height * scale;

    let free_bytes = available_space(temp_root)?;

    let (segment_seconds, temp_peak_bytes, sufficient) = match settings.segment_seconds {
        Some(explicit) => {
            let frames = frames_for_seconds(explicit, source.fps);
            let peak = segment_peak_bytes(source, settings, raw_width, raw_height, frames);
            let sufficient = (peak as f64) < PEAK_FRACTION * free_bytes as f64;
            (explicit, peak, sufficient)
        }
        None => {
            let mut chosen: Option<(u32, u64)> = None;
            for candidate in (SEGMENT_MIN..=SEGMENT_MAX).rev() {
                let frames = frames_for_seconds(candidate, source.fps);
                let peak = segment_peak_bytes(source, settings, raw_width, raw_height, frames);
                if (peak as f64) < PEAK_FRACTION * free_bytes as f64 {
                    chosen = Some((candidate, peak));
                    break;
                }
            }
            match chosen {
                Some((seconds, peak)) => (seconds, peak, true),
                None => {
                    let frames = frames_for_seconds(SEGMENT_MIN, source.fps);
                    let peak =
                        segment_peak_bytes(source, settings, raw_width, raw_height, frames);
                    (SEGMENT_MIN, peak, false)
                }
            }
        }
    };

    let seg_frames = frames_for_seconds(segment_seconds, source.fps) as f64;
    let num_segments = (source.frame_count as f64 / seg_frames).ceil().max(1.0);
    let temp_total_written = (temp_peak_bytes as f64 * num_segments).round() as u64;

    let output_bytes_est = (source.duration_sec * OUTPUT_BITRATE_BPS / 8.0).round() as u64;

    Ok(DiskEstimate {
        temp_peak_bytes,
        temp_total_written,
        output_bytes_est,
        free_bytes,
        sufficient,
        segment_seconds,
        scale,
        out_width,
        out_height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Codec, Container};

    fn source(width: u32, height: u32, fps: f32, duration_sec: f64) -> SourceInfo {
        SourceInfo {
            path: "/tmp/in.mkv".into(),
            width,
            height,
            fps,
            duration_sec,
            frame_count: (fps as f64 * duration_sec).round() as u64,
            has_audio: true,
            subtitle_streams: vec![],
            codec_name: "h264".into(),
            pix_fmt: "yuv420p".into(),
        }
    }

    fn settings(target_width: u32, target_fps: Option<f32>) -> UpscaleSettings {
        UpscaleSettings {
            target_width,
            target_fps,
            codec: Codec::Hevc,
            cq: 19,
            container: Container::Mkv,
            segment_seconds: None,
            keep_intermediate: false,
            output_dir: None,
            temp_dir: None,
        }
    }

    #[test]
    fn capped_resolution_caps_to_target_width() {
        let src = source(1920, 1080, 24.0, 60.0);
        let settings = settings(3840, Some(60.0));
        let scale = settings.scale_for(src.width); // x2
        assert_eq!(scale, 2);
        let (w, h) = capped_out_resolution(&src, &settings, scale);
        assert_eq!(w, 3840);
        assert_eq!(h, 2160);
    }

    #[test]
    fn capped_resolution_keeps_raw_when_under_target() {
        // source 640x480 x scale 4 = 2560x1920, меньше target_width 3840.
        let src = source(640, 480, 24.0, 60.0);
        let settings = settings(3840, None);
        let scale = settings.scale_for(src.width);
        assert_eq!(scale, 4);
        let (w, h) = capped_out_resolution(&src, &settings, scale);
        assert_eq!((w, h), (2560, 1920));
    }

    #[test]
    fn segment_peak_includes_rife_only_when_interpolating() {
        let src = source(1920, 1080, 24.0, 60.0);
        let with_interp = settings(3840, Some(60.0));
        let without_interp = settings(3840, None);
        let frames = frames_for_seconds(10, src.fps);
        let peak_with = segment_peak_bytes(&src, &with_interp, 3840, 2160, frames);
        let peak_without = segment_peak_bytes(&src, &without_interp, 3840, 2160, frames);
        assert!(peak_with > peak_without);
    }

    #[test]
    fn estimate_picks_largest_segment_within_budget() {
        let src = source(1920, 1080, 24.0, 600.0);
        let settings = settings(3840, Some(60.0));
        let temp_root = std::env::temp_dir();
        let est = estimate(&src, &settings, &temp_root).unwrap();
        assert!(est.segment_seconds >= SEGMENT_MIN && est.segment_seconds <= SEGMENT_MAX);
        assert_eq!(est.scale, 2);
        assert_eq!((est.out_width, est.out_height), (3840, 2160));
    }

    #[test]
    fn estimate_respects_explicit_segment_seconds() {
        let src = source(1920, 1080, 24.0, 600.0);
        let mut settings = settings(3840, Some(60.0));
        settings.segment_seconds = Some(12);
        let temp_root = std::env::temp_dir();
        let est = estimate(&src, &settings, &temp_root).unwrap();
        assert_eq!(est.segment_seconds, 12);
    }
}
