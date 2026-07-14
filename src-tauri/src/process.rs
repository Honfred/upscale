//! Унифицированный запуск дочерних процессов (все инструменты — sidecar:
//! ffmpeg, ffprobe, realesrgan-ncnn-vulkan, rife-ncnn-vulkan).
//! Стриминг stderr для парсинга прогресса + кооперативная отмена.
//! Реализация — задача A.

use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, Result};

/// Колбэк построчного stderr (для парсинга прогресса). `+ Send`, т.к. вся
/// джоба (pipeline::run) выполняется внутри tokio::spawn на верхнем уровне
/// (см. commands::start_job) — future должен быть Send целиком.
pub type LineHandler<'a> = &'a mut (dyn FnMut(&str) + Send);

/// Максимальный размер хранимого хвоста stderr (используется для сообщений об ошибке).
const STDERR_TAIL_LIMIT: usize = 4096;

/// Добавляет строку в кольцевой (по факту — усекаемый спереди) буфер хвоста stderr.
fn push_tail(buf: &mut String, line: &str) {
    buf.push_str(line);
    buf.push('\n');
    if buf.len() > STDERR_TAIL_LIMIT {
        let excess = buf.len() - STDERR_TAIL_LIMIT;
        let mut cut = excess;
        while cut < buf.len() && !buf.is_char_boundary(cut) {
            cut += 1;
        }
        buf.drain(..cut);
    }
}

fn missing_sidecar_err(tool: &str, e: impl std::fmt::Display) -> AppError {
    AppError::SidecarMissing(format!("{tool}: {e}"))
}

/// Запускает sidecar `tool` с аргументами; читает stderr построчно в `on_line`;
/// при отмене токена убивает процесс и возвращает AppError::Cancelled.
/// При ненулевом коде выхода возвращает AppError::Process с хвостом stderr (~4КБ).
pub async fn run_sidecar(
    app: &AppHandle,
    tool: &str,
    args: &[String],
    cancel: &CancellationToken,
    on_line: LineHandler<'_>,
) -> Result<()> {
    let command = app
        .shell()
        .sidecar(tool)
        .map_err(|e| missing_sidecar_err(tool, e))?
        .args(args);

    let (mut rx, child) = command.spawn().map_err(|e| missing_sidecar_err(tool, e))?;

    let mut stderr_tail = String::new();
    let mut exit_code: Option<i32> = None;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = child.kill();
                return Err(AppError::Cancelled);
            }
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(CommandEvent::Stderr(bytes)) => {
                        let line = String::from_utf8_lossy(&bytes);
                        let line = line.trim_end_matches(['\r', '\n']);
                        push_tail(&mut stderr_tail, line);
                        on_line(line);
                    }
                    Some(CommandEvent::Stdout(_)) => {}
                    Some(CommandEvent::Error(err)) => {
                        push_tail(&mut stderr_tail, &err);
                    }
                    Some(CommandEvent::Terminated(payload)) => {
                        exit_code = payload.code;
                    }
                    // CommandEvent помечен #[non_exhaustive].
                    Some(_) => {}
                    None => break,
                }
            }
        }
    }

    match exit_code {
        Some(0) => Ok(()),
        Some(code) => Err(AppError::Process {
            tool: tool.to_string(),
            code,
            stderr: stderr_tail,
        }),
        None => Err(AppError::Process {
            tool: tool.to_string(),
            code: -1,
            stderr: stderr_tail,
        }),
    }
}

/// Как run_sidecar, но возвращает stdout целиком (для ffprobe -of json).
pub async fn run_sidecar_capture(app: &AppHandle, tool: &str, args: &[String]) -> Result<String> {
    let command = app
        .shell()
        .sidecar(tool)
        .map_err(|e| missing_sidecar_err(tool, e))?
        .args(args);

    let output = command
        .output()
        .await
        .map_err(|e| missing_sidecar_err(tool, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let mut tail = String::new();
        push_tail(&mut tail, stderr.trim_end_matches(['\r', '\n']));
        return Err(AppError::Process {
            tool: tool.to_string(),
            code: output.status.code().unwrap_or(-1),
            stderr: tail,
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_tail_truncates_to_limit() {
        let mut buf = String::new();
        for i in 0..2000 {
            push_tail(&mut buf, &format!("line {i}"));
        }
        assert!(buf.len() <= STDERR_TAIL_LIMIT + 32);
        // Хвост должен содержать самые свежие строки.
        assert!(buf.contains("line 1999"));
        assert!(!buf.contains("line 0\n"));
    }
}
