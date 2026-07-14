//! Общий механизм прогресса для ncnn-vulkan бинарников (realesrgan, rife):
//! их stderr-прогресс имеет нестабильный построчный формат по кадру, поэтому
//! надёжнее фоновым тасктом раз в N мс считать число PNG в выходной папке.

use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::interval;

fn count_pngs(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext.eq_ignore_ascii_case("png"))
                        .unwrap_or(false)
                })
                .count() as u64
        })
        .unwrap_or(0)
}

/// Запускает фоновый таск, который каждые `poll` считает PNG-файлы в `dir` и
/// передаёт число в `on_count`. Останавливается, когда придёт сигнал в
/// возвращённый stop-канал (см. `stop`).
pub fn spawn_frame_counter<F>(dir: PathBuf, poll: Duration, mut on_count: F) -> (JoinHandle<()>, oneshot::Sender<()>)
where
    F: FnMut(u64) + Send + 'static,
{
    let (stop_tx, mut stop_rx) = oneshot::channel();

    let handle = tokio::spawn(async move {
        let mut ticker = interval(poll);
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let count = count_pngs(&dir);
                    on_count(count);
                }
                _ = &mut stop_rx => break,
            }
        }
    });

    (handle, stop_tx)
}

/// Синхронный подсчёт PNG-файлов в директории (для финальной валидации после
/// завершения процесса).
pub fn count_pngs_now(dir: &Path) -> u64 {
    count_pngs(dir)
}
