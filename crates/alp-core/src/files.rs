//! Atomic file replacement helpers.
//!
//! Project saves, peak caches and rendered audio all write to a temporary file
//! in the destination directory and rename it into place, so a crash can never
//! leave a half-written file under the real name.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::CoreError;

/// Create a uniquely named temporary file next to `target`.
pub fn create_temp_sibling(target: &Path) -> std::io::Result<(PathBuf, File)> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("alp");
    for attempt in 0..32u32 {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let temp = parent.join(format!(
            ".{name}.{}.{}.{attempt}.tmp",
            std::process::id(),
            stamp
        ));
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => return Ok((temp, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a temporary file name",
    ))
}

/// Rename a finished temporary file over its destination.
pub fn commit_temp(temp: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(temp, target)
}

/// Write `bytes` to `path` atomically.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    let (temp, mut file) = create_temp_sibling(path)?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        commit_temp(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map_err(CoreError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_contents() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("file.txt");
        write_atomic(&path, b"first").expect("write");
        write_atomic(&path, b"second").expect("write");
        assert_eq!(fs::read_to_string(&path).expect("read"), "second");
        assert!(fs::read_dir(dir.path()).expect("entries").all(|entry| {
            !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")
        }));
    }
}
