//! Склейка сегментных .mkv в финальный файл + мультиплексирование
//! аудио/субтитров исходника через concat demuxer.

use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use crate::config::{Container, UpscaleSettings};
use crate::error::{AppError, Result};
use crate::probe::SourceInfo;
use crate::process::run_sidecar;

/// Пишет concat.txt со строками `file '{abs_path}'`, экранируя одинарные
/// кавычки в путях (стандартный приём для POSIX-подобного парсера ffmpeg
/// concat demuxer).
fn write_concat_file(path: &Path, segment_outputs: &[PathBuf]) -> Result<()> {
    let mut content = String::new();
    for seg in segment_outputs {
        let abs = seg.canonicalize().unwrap_or_else(|_| seg.clone());
        let escaped = abs.to_string_lossy().replace('\'', "'\\''");
        content.push_str(&format!("file '{escaped}'\n"));
    }
    std::fs::write(path, content)?;
    Ok(())
}

async fn run_concat_ffmpeg(
    app: &AppHandle,
    concat_txt: &Path,
    source: &SourceInfo,
    settings: &UpscaleSettings,
    output_path: &Path,
    include_subs: bool,
    cancel: &CancellationToken,
) -> Result<()> {
    let mut args = vec![
        "-v".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-f".to_string(),
        "concat".to_string(),
        "-safe".to_string(),
        "0".to_string(),
        "-i".to_string(),
        concat_txt.to_string_lossy().to_string(),
        "-i".to_string(),
        source.path.to_string_lossy().to_string(),
        "-map".to_string(),
        "0:v:0".to_string(),
        "-map".to_string(),
        "1:a?".to_string(),
    ];

    if include_subs {
        args.push("-map".to_string());
        args.push("1:s?".to_string());
    }

    args.push("-c".to_string());
    args.push("copy".to_string());

    if include_subs && matches!(settings.container, Container::Mp4) {
        // mp4 не поддерживает большинство text-based субтитров (напр. ASS)
        // напрямую — конвертируем в mov_text.
        args.push("-c:s".to_string());
        args.push("mov_text".to_string());
    }

    // Пишем во временный файл {output}.part — ffmpeg не может вывести формат
    // из такого расширения, поэтому муксер задаётся явно.
    args.push("-f".to_string());
    args.push(
        match settings.container {
            Container::Mkv => "matroska",
            Container::Mp4 => "mp4",
        }
        .to_string(),
    );

    args.push(output_path.to_string_lossy().to_string());

    run_sidecar(app, crate::config::BIN_FFMPEG, &args, cancel, &mut |_line| {}).await
}

/// Имя временного файла для атомарной записи финального результата:
/// `{output_path}.part` в ТОЙ ЖЕ директории, что и `output_path` (условие
/// атомарности rename на одном устройстве).
fn part_path_for(output_path: &Path) -> PathBuf {
    let mut name = output_path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".part");
    output_path.with_file_name(name)
}

/// Склеивает сегментные .mkv (`segment_outputs`, в порядке индексов) в
/// `output_path`, домешивая аудио/субтитры из исходника. Если сборка с
/// субтитрами не удалась (напр. несовместимый формат вроде ASS в mp4),
/// повторяет попытку без них и возвращает предупреждение через `warnings`.
///
/// Пишет во временный файл `{output_path}.part` и переименовывает в
/// `output_path` только по успеху (rename в той же директории — атомарная
/// операция на большинстве ФС). При ошибке/отмене `.part` удаляется, чтобы
/// в пользовательской папке не оставался битый выходной файл, который
/// paths::output_path на следующем запуске принял бы за существующий и
/// начал дедуплицировать имя (_1, _2...).
pub async fn concat_segments(
    app: &AppHandle,
    segment_outputs: &[PathBuf],
    temp_root: &Path,
    source: &SourceInfo,
    settings: &UpscaleSettings,
    output_path: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<String>> {
    let concat_txt = temp_root.join("concat.txt");
    write_concat_file(&concat_txt, segment_outputs)?;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let part_path = part_path_for(output_path);
    // На случай, если от предыдущего неудачного/отменённого прогона остался
    // осиротевший .part (например, приложение было убито до cleanup).
    let _ = std::fs::remove_file(&part_path);

    let mut warnings = Vec::new();
    let has_subs = !source.subtitle_streams.is_empty();

    let result = run_concat_ffmpeg(
        app,
        &concat_txt,
        source,
        settings,
        &part_path,
        has_subs,
        cancel,
    )
    .await;

    let result = match result {
        Ok(()) => Ok(()),
        // Отмену и общие ошибки процесса не маскируем повторной попыткой.
        Err(AppError::Cancelled) => Err(AppError::Cancelled),
        Err(e) if has_subs => {
            warnings.push(format!(
                "не удалось смультиплексировать субтитры (несовместимый формат для контейнера): {e}; файл собран без субтитров"
            ));
            run_concat_ffmpeg(app, &concat_txt, source, settings, &part_path, false, cancel).await
        }
        Err(e) => Err(e),
    };

    match result {
        Ok(()) => {
            std::fs::rename(&part_path, output_path)?;
            Ok(warnings)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&part_path);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_path_is_in_same_directory_with_part_suffix() {
        let output = PathBuf::from("/videos/out/anime_4k60.mkv");
        let part = part_path_for(&output);
        assert_eq!(part, PathBuf::from("/videos/out/anime_4k60.mkv.part"));
        // Та же директория — обязательное условие атомарности rename.
        assert_eq!(part.parent(), output.parent());
    }

    #[test]
    fn part_path_preserves_mp4_extension_in_stem() {
        let output = PathBuf::from("anime_4k60.mp4");
        let part = part_path_for(&output);
        assert_eq!(part, PathBuf::from("anime_4k60.mp4.part"));
    }
}
