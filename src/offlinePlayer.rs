use std::path::{Path, PathBuf};

use crate::error::AppError;

pub const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "m4a", "wav", "flac", "ogg", "aac", "opus"];

pub fn load_audio_files(folder: &Path) -> Result<Vec<PathBuf>, AppError> {
    if !folder.is_dir() {
        return Err(AppError::Message(format!(
            "Not a folder: {}",
            folder.display()
        )));
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(folder)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            !is_hidden(path)
                && path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        SUPPORTED_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
                    })
        })
        .collect();

    files.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));
    Ok(files)
}

pub fn load_entries(folder: &Path, query: &str) -> Result<Vec<PathBuf>, AppError> {
    if !folder.is_dir() {
        return Err(AppError::Message(format!(
            "Not a folder: {}",
            folder.display()
        )));
    }

    let query = query.to_ascii_lowercase();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(folder)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            if is_hidden(path) {
                return false;
            }

            let matches_query = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_ascii_lowercase().contains(&query))
                .unwrap_or(false);

            (path.is_dir() || is_audio_file(path)) && matches_query
        })
        .collect();

    entries.sort_by_key(|path| {
        (
            (!path.is_dir(),),
            path.file_name().map(|name| name.to_os_string()),
        )
    });
    Ok(entries)
}

fn is_audio_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                SUPPORTED_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
            })
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}
