use std::path::{ Path, PathBuf };

use crate::error::AppError;

pub const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "m4a", "wav", "flac", "ogg", "aac", "opus"];

pub fn load_audio_files(folder: &Path) -> Result<Vec<PathBuf>, AppError> {
    if !folder.is_dir() {
        return Err(AppError::Message(format!("Not a folder: {}", folder.display())));
    }

    let mut files: Vec<PathBuf> = std::fs
        ::read_dir(folder)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            !is_hidden(path) &&
                path.is_file() &&
                path
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
        return Err(AppError::Message(format!("Not a folder: {}", folder.display())));
    }

    let query = query.to_lowercase();
    if !query.is_empty() {
        return load_matching_audio_files(folder, &query);
    }

    let mut entries: Vec<PathBuf> = std::fs
        ::read_dir(folder)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            if is_hidden(path) {
                return false;
            }

            let matches_query = path.to_string_lossy().to_lowercase().contains(&query);

            (path.is_dir() || is_audio_file(path)) && matches_query
        })
        .collect();

    entries.sort_by_key(|path| {
        ((!path.is_dir(),), path.file_name().map(|name| name.to_os_string()))
    });
    Ok(entries)
}

fn load_matching_audio_files(folder: &Path, query: &str) -> Result<Vec<PathBuf>, AppError> {
    let mut entries = Vec::new();

    let Ok(read_dir) = std::fs::read_dir(folder) else {
        return Ok(entries);
    };

    for entry in read_dir.filter_map(Result::ok) {
        let path = entry.path();
        if is_hidden(&path) || entry.file_type().map(|kind| kind.is_symlink()).unwrap_or(true) {
            continue;
        }

        if path.is_dir() {
            entries.extend(load_matching_audio_files(&path, query)?);
        } else if is_audio_file(&path) && path.to_string_lossy().to_lowercase().contains(query) {
            entries.push(path);
        }
    }

    entries.sort_by_key(|path| path.to_string_lossy().to_lowercase());
    Ok(entries)
}

fn is_audio_file(path: &Path) -> bool {
    path.is_file() &&
        path
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
