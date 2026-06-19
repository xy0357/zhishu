use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

use uuid::Uuid;

pub fn build_object_key(bucket: &str, original_name: &str) -> String {
    let file_name = sanitize_file_name(original_name);
    format!("{bucket}/documents/{}-{}", Uuid::new_v4(), file_name)
}

pub fn build_object_path(storage_dir: &str, object_key: &str) -> PathBuf {
    let mut path = PathBuf::from(storage_dir);
    for part in object_key.split('/') {
        path.push(part);
    }
    path
}

pub fn store_bytes(storage_dir: &str, object_key: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let object_path = build_object_path(storage_dir, object_key);
    if let Some(parent) = object_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&object_path, bytes)?;
    Ok(object_path)
}

pub fn read_bytes(storage_dir: &str, object_key: &str) -> io::Result<Vec<u8>> {
    fs::read(build_object_path(storage_dir, object_key))
}

fn sanitize_file_name(value: &str) -> String {
    let extension = Path::new(value)
        .extension()
        .and_then(|item| item.to_str())
        .filter(|item| !item.is_empty())
        .map(|item| format!(".{}", item))
        .unwrap_or_default();
    let normalized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .replace("--", "-");
    let trimmed = normalized.trim_matches('-');
    let has_meaningful_stem = trimmed
        .trim_end_matches(&extension)
        .chars()
        .any(|ch| ch.is_ascii_alphanumeric());
    if trimmed.is_empty() || !has_meaningful_stem {
        format!("uploaded-file{}", extension)
    } else {
        trimmed.to_string()
    }
}
