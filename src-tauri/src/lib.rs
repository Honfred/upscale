//! Bootstrap приложения. Финализация (плагины, state, obработка ошибок
//! запуска) — задача B.

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::default())
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
