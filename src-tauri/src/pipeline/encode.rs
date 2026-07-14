//! Кодирование кадров сегмента в .mkv через ffmpeg + NVENC.

use regex::Regex;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use crate::config::{Codec, UpscaleSettings, NVENC_PRESET};
use crate::error::{AppError, Result};
use crate::process::run_sidecar;

/// Кодирует кадры из `{seg_dir}/{frames_dir_name}` в `{seg_dir}/out.mkv`.
/// `raw_out_width` — фактическая ширина после апскейла (до применения
/// возможного -vf scale); если она больше settings.target_width, добавляется
/// downscale лучшим фильтром (lanczos) до target_width (высота -2).
/// Прогресс парсится из строк ffmpeg вида "frame=  123 fps=...".
pub async fn encode_segment(
    app: &AppHandle,
    seg_dir: &Path,
    frames_dir_name: &str,
    out_fps: f32,
    raw_out_width: u32,
    settings: &UpscaleSettings,
    total_frames: u64,
    cancel: &CancellationToken,
    mut on_progress: impl FnMut(u64) + Send,
) -> Result<PathBuf> {
    let frames_dir = seg_dir.join(frames_dir_name);
    let pattern = frames_dir.join("frame_%08d.png");
    let out_file = seg_dir.join("out.mkv");

    let mut args = vec![
        "-v".to_string(),
        "error".to_string(),
        // См. decode.rs: без -stats ffmpeg не печатает "frame=..." в stderr,
        // когда вывод не TTY (проверено на реальном sidecar-бинарнике).
        "-stats".to_string(),
        "-y".to_string(),
        "-framerate".to_string(),
        format!("{out_fps}"),
        // Кадры во всех промежуточных папках (in/, up/, rife/) нумеруются с 1
        // (decode пишет с frame_00000001.png; realesrgan сохраняет исходные
        // имена входных файлов; rife-ncnn-vulkan с явным -f frame_%08d.png
        // также нумерует с 1 — проверено на реальных бинарниках). Без явного
        // -start_number ffmpeg находит первый файл через автоопределение
        // (start_number_range=5), что работает, но неявно и хрупко.
        "-start_number".to_string(),
        "1".to_string(),
        "-i".to_string(),
        pattern.to_string_lossy().to_string(),
    ];

    if raw_out_width > settings.target_width {
        args.push("-vf".to_string());
        args.push(format!("scale={}:-2:flags=lanczos", settings.target_width));
    }

    match settings.codec {
        Codec::Hevc => {
            args.extend(
                [
                    "-c:v",
                    "hevc_nvenc",
                    "-preset",
                    NVENC_PRESET,
                    "-tune",
                    "hq",
                    "-rc",
                    "vbr",
                ]
                .map(String::from),
            );
            args.push("-cq".to_string());
            args.push(settings.cq.to_string());
            args.extend(
                [
                    "-b:v",
                    "0",
                    "-pix_fmt",
                    "p010le",
                    "-profile:v",
                    "main10",
                    "-spatial_aq",
                    "1",
                ]
                .map(String::from),
            );
        }
        Codec::H264 => {
            args.extend(
                [
                    "-c:v",
                    "h264_nvenc",
                    "-preset",
                    NVENC_PRESET,
                    "-tune",
                    "hq",
                    "-rc",
                    "vbr",
                ]
                .map(String::from),
            );
            args.push("-cq".to_string());
            args.push(settings.cq.to_string());
            args.extend(["-b:v", "0", "-pix_fmt", "yuv420p", "-profile:v", "high"].map(String::from));
        }
        Codec::Av1 => {
            args.extend(
                ["-c:v", "av1_nvenc", "-preset", NVENC_PRESET, "-rc", "vbr"].map(String::from),
            );
            args.push("-cq".to_string());
            args.push(settings.cq.to_string());
            args.extend(["-b:v", "0", "-pix_fmt", "p010le"].map(String::from));
        }
    }

    args.push(out_file.to_string_lossy().to_string());

    // Не паникует при некорректном regex — шаблон статический и валиден по построению.
    let frame_re = Regex::new(r"frame=\s*(\d+)").expect("статический regex должен быть валиден");

    run_sidecar(app, "ffmpeg", &args, cancel, &mut |line| {
        if let Some(caps) = frame_re.captures(line) {
            if let Ok(n) = caps[1].parse::<u64>() {
                on_progress(n.min(total_frames));
            }
        }
    })
    .await?;

    if !out_file.exists() {
        return Err(AppError::Other(format!(
            "encode: не создан файл {}",
            out_file.display()
        )));
    }

    on_progress(total_frames);

    Ok(out_file)
}
