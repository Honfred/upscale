//! Bootstrap приложения.

mod commands;
mod config;
mod error;
mod estimate;
mod events;
mod paths;
mod pipeline;
mod probe;
mod process;
mod state;

use tauri::{Listener, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::default())
        .setup(|app| {
            let app_handle = app.handle().clone();
            // Зеркалим overall_progress из событий job://progress в
            // AppState.jobs, чтобы get_job_status отдавал актуальный прогресс
            // даже если webview был пересоздан (и полностью потерял состояние
            // JS-подписчика событий).
            app.listen(events::EV_PROGRESS, move |event| {
                let Ok(value) = serde_json::from_str::<serde_json::Value>(event.payload())
                else {
                    return;
                };
                if value.get("kind").and_then(|k| k.as_str()) != Some("stage") {
                    return;
                }
                let job_id = value.get("jobId").and_then(|v| v.as_str());
                let progress = value.get("overallProgress").and_then(|v| v.as_f64());
                if let (Some(job_id), Some(progress)) = (job_id, progress) {
                    let state = app_handle.state::<state::AppState>();
                    let mut jobs = state.jobs.lock().unwrap();
                    jobs.set_progress(job_id, progress as f32);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::probe_source,
            commands::estimate_job,
            commands::start_job,
            commands::cancel_job,
            commands::get_job_status,
            commands::system_check,
            commands::reveal_output,
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска AnimeUpscale");
}
