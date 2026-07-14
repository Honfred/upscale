//! Интерполяция кадров сегмента (изменение fps) через rife-ncnn-vulkan.

use std::path::Path;
use std::time::Duration;
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, Result};
use crate::process::run_sidecar;

use super::progress::{count_pngs_now, spawn_frame_counter};
use super::segment::Segment;

/// true, если стадия интерполяции должна выполняться: target_fps задан и
/// строго больше fps исходника. Иначе стадия пропускается, а encode берёт
/// кадры из `up/`.
pub fn should_interpolate(source_fps: f32, target_fps: Option<f32>) -> bool {
    target_fps.map(|t| t > source_fps).unwrap_or(false)
}

/// Число выходных (после интерполяции) кадров для сегмента `seg`, посчитанное
/// покадрово по накопительной формуле:
/// frames_out(seg) = round(cum(end_frame)) - round(cum(start_frame)),
/// где cum(x) = x / fps * target_fps — кумулятивное число выходных кадров
/// на позиции x исходных кадров. Так суммарно по всем сегментам джобы
/// результат телескопически сходится к round(total_frames / fps * target_fps),
/// без накопления ошибки округления.
pub fn target_frames_for_segment(seg: &Segment, fps: f32, target_fps: f32) -> u64 {
    let cum = |source_frame: u64| -> f64 { source_frame as f64 / fps as f64 * target_fps as f64 };

    let start_cum = cum(seg.start_frame).round();
    let end_cum = cum(seg.end_frame()).round();

    (end_cum - start_cum).max(0.0) as u64
}

/// Интерполирует `{seg_dir}/{frames_dir_name}` -> `{seg_dir}/rife`, добиваясь
/// `target_frames` выходных кадров (rife-ncnn-vulkan поддерживает
/// произвольное -n). `frames_dir_name` — обычно "up" (после апскейла), но
/// "in" (прямо после decode), если стадия апскейла была пропущена (source
/// уже не уже target_width, см. config::scale_for).
/// Прогресс — фоновый подсчёт PNG в `rife/` раз в 500мс (см. progress.rs).
pub async fn interpolate_segment(
    app: &AppHandle,
    seg_dir: &Path,
    frames_dir_name: &str,
    rife_model_dir: &Path,
    target_frames: u64,
    cancel: &CancellationToken,
    mut on_progress: impl FnMut(u64) + Send + 'static,
) -> Result<()> {
    let in_dir = seg_dir.join(frames_dir_name);
    let out_dir = seg_dir.join("rife");
    std::fs::create_dir_all(&out_dir)?;

    if !rife_model_dir.exists() {
        return Err(AppError::SidecarMissing(format!(
            "модель RIFE не найдена: {}",
            rife_model_dir.display()
        )));
    }

    let args = vec![
        "-i".to_string(),
        in_dir.to_string_lossy().to_string(),
        "-o".to_string(),
        out_dir.to_string_lossy().to_string(),
        "-m".to_string(),
        rife_model_dir.to_string_lossy().to_string(),
        "-n".to_string(),
        target_frames.to_string(),
        "-g".to_string(),
        "0".to_string(),
        "-j".to_string(),
        "2:2:2".to_string(),
        "-f".to_string(),
        "frame_%08d.png".to_string(),
    ];

    let (handle, stop_tx) =
        spawn_frame_counter(out_dir.clone(), Duration::from_millis(500), move |count| {
            on_progress(count);
        });

    let result = run_sidecar(app, "rife-ncnn-vulkan", &args, cancel, &mut |_line| {}).await;

    let _ = stop_tx.send(());
    let _ = handle.await;

    result?;

    let actual = count_pngs_now(&out_dir);
    if actual != target_frames {
        return Err(AppError::Other(format!(
            "interpolate: ожидалось {target_frames} кадров, получено {actual}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_interpolate_true_when_target_higher() {
        assert!(should_interpolate(24.0, Some(60.0)));
    }

    #[test]
    fn should_interpolate_false_when_none() {
        assert!(!should_interpolate(24.0, None));
    }

    #[test]
    fn should_interpolate_false_when_target_lower_or_equal() {
        assert!(!should_interpolate(60.0, Some(60.0)));
        assert!(!should_interpolate(60.0, Some(24.0)));
    }

    #[test]
    fn rife_distribution_sums_to_expected_total() {
        // 543 кадра @ 24fps -> target 60fps, сегменты по 240 кадров (10с).
        let segments = super::super::segment::compute_segments(543, 24.0, 10);
        let fps = 24.0;
        let target_fps = 60.0;

        let total_out: u64 = segments
            .iter()
            .map(|s| target_frames_for_segment(s, fps, target_fps))
            .sum();

        let expected = (543.0 / fps * target_fps).round() as u64;
        assert_eq!(total_out, expected);
    }

    #[test]
    fn rife_distribution_matches_manual_calc_for_simple_case() {
        // 2 сегмента по 240 кадров @24fps -> 60fps: каждый сегмент 10с * 60 = 600 кадров.
        let segments = super::super::segment::compute_segments(480, 24.0, 10);
        assert_eq!(segments.len(), 2);
        for seg in &segments {
            assert_eq!(target_frames_for_segment(seg, 24.0, 60.0), 600);
        }
    }

    #[test]
    fn rife_distribution_handles_fractional_fps_without_drift() {
        // 23.976 fps -> 60 fps, множество сегментов, сумма должна точно сойтись.
        let fps = 23.976;
        let target_fps = 60.0;
        let total_frames = 10_007u64;
        let segments = super::super::segment::compute_segments(total_frames, fps, 8);

        let total_out: u64 = segments
            .iter()
            .map(|s| target_frames_for_segment(s, fps, target_fps))
            .sum();

        let expected = (total_frames as f64 / fps as f64 * target_fps as f64).round() as u64;
        assert_eq!(total_out, expected);
    }
}
