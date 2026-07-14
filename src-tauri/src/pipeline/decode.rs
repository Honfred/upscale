//! Декодирование сегмента исходного видео в PNG-кадры (rgb24) через ffmpeg.

use regex::Regex;
use std::path::Path;
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, Result};
use crate::process::run_sidecar;

use super::segment::{format_timestamp, seek_timestamp, Segment};

/// Декодирует кадры сегмента `seg` из `source_path` в `{seg_dir}/in/frame_%08d.png`.
/// Использует input-seek (-ss перед -i) для скорости и -frames:v для точного
/// числа кадров. Для VFR-источников критичны -fps_mode cfr -r {fps}.
/// Прогресс парсится из периодических stats-строк ffmpeg ("frame=  123 ..."),
/// которые печатаются в stderr независимо от -v error (управляются -stats/-nostats).
pub async fn decode_segment(
    app: &AppHandle,
    source_path: &Path,
    fps: f32,
    seg: &Segment,
    seg_dir: &Path,
    cancel: &CancellationToken,
    mut on_progress: impl FnMut(u64) + Send,
) -> Result<()> {
    let in_dir = seg_dir.join("in");
    std::fs::create_dir_all(&in_dir)?;

    let pattern = in_dir.join("frame_%08d.png");

    let mut args = vec![
        "-v".to_string(),
        "error".to_string(),
        // Без -stats периодическая строка "frame=..." не печатается вовсе,
        // когда stderr не TTY (а это всегда так для sidecar-процесса) —
        // проверено на реальном ffmpeg n7.1: без -stats stderr пуст.
        "-stats".to_string(),
        "-y".to_string(),
    ];

    // Сик на границе сегмента (start_frame > 0): используем таймкод,
    // смещённый на пол-кадра НАЗАД (см. seek_timestamp), чтобы первым
    // декодированным кадром гарантированно оказался ровно start_frame даже
    // при дробном fps (23.976 и т.п.), независимо от float-округления.
    // Для start_frame == 0 -ss не нужен вовсе.
    if let Some(ts) = seek_timestamp(seg.start_frame, fps) {
        args.push("-ss".to_string());
        args.push(format_timestamp(ts));
    }

    args.extend([
        "-i".to_string(),
        source_path.to_string_lossy().to_string(),
        "-frames:v".to_string(),
        seg.frame_count.to_string(),
        "-fps_mode".to_string(),
        "cfr".to_string(),
        "-r".to_string(),
        fps.to_string(),
        "-pix_fmt".to_string(),
        "rgb24".to_string(),
        pattern.to_string_lossy().to_string(),
    ]);

    let frame_re = Regex::new(r"frame=\s*(\d+)").expect("статический regex должен быть валиден");

    run_sidecar(app, crate::config::BIN_FFMPEG, &args, cancel, &mut |line| {
        if let Some(caps) = frame_re.captures(line) {
            if let Ok(n) = caps[1].parse::<u64>() {
                on_progress(n.min(seg.frame_count));
            }
        }
    })
    .await?;

    let actual = count_pngs(&in_dir)?;
    if actual as u64 != seg.frame_count {
        return Err(AppError::Other(format!(
            "decode: сегмент {}: ожидалось {} кадров, получено {}",
            seg.index, seg.frame_count, actual
        )));
    }

    on_progress(seg.frame_count);

    Ok(())
}

fn count_pngs(dir: &Path) -> Result<usize> {
    Ok(std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
        })
        .count())
}
