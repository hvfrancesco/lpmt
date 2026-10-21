//! Project file writes.
//!
//! Saves go to a temporary file in the destination directory, are flushed and
//! then renamed over the target. On POSIX this replaces the previous project
//! atomically, so an interrupted save cannot leave a half-written show file.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    let file_name = path
        .file_name()
        .with_context(|| format!("invalid project path {}", path.display()))?;

    let mut last_collision = None;
    for attempt in 0..100 {
        let temporary = temporary_path(parent, file_name, attempt);
        let file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                last_collision = Some(error);
                continue;
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to create {}", temporary.display()));
            }
        };

        let result = write_and_replace(file, &temporary, path, parent, contents);
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        return result;
    }

    match last_collision {
        Some(error) => Err(error).with_context(|| {
            format!(
                "failed to create a temporary file next to {}",
                path.display()
            )
        }),
        None => bail!(
            "failed to create a temporary file next to {}",
            path.display()
        ),
    }
}

fn write_and_replace(
    mut file: File,
    temporary: &Path,
    path: &Path,
    parent: &Path,
    contents: &[u8],
) -> Result<()> {
    file.write_all(contents)
        .with_context(|| format!("failed to write {}", temporary.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to flush {}", temporary.display()))?;
    drop(file);

    replace_file(temporary, path)?;

    // Persist the directory entry where the platform supports it. Failure to
    // sync the directory should not turn a successful save into an error.
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, path: &Path) -> Result<()> {
    fs::rename(temporary, path).with_context(|| format!("failed to replace {}", path.display()))?;
    Ok(())
}

#[cfg(windows)]
fn replace_file(temporary: &Path, path: &Path) -> Result<()> {
    match fs::rename(temporary, path) {
        Ok(()) => Ok(()),
        Err(_error) if path.exists() => {
            // std::fs::rename does not replace an existing file on Windows.
            // Move the old file aside and restore it if the replacement fails.
            let backup = path.with_extension("lpmt-save-backup");
            let _ = fs::remove_file(&backup);
            fs::rename(path, &backup)
                .with_context(|| format!("failed to move {} aside", path.display()))?;
            match fs::rename(temporary, path) {
                Ok(()) => {
                    let _ = fs::remove_file(&backup);
                    Ok(())
                }
                Err(replace_error) => {
                    let _ = fs::rename(&backup, path);
                    Err(replace_error)
                        .with_context(|| format!("failed to replace {}", path.display()))
                }
            }
        }
        Err(error) => Err(error).with_context(|| format!("failed to replace {}", path.display())),
    }
}

fn temporary_path(parent: &Path, file_name: &std::ffi::OsStr, attempt: u32) -> PathBuf {
    let mut name = OsString::from(".");
    name.push(file_name);
    name.push(format!(".tmp-{}-{attempt}", std::process::id()));
    parent.join(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_directory() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("lpmt-project-io-{}-{unique}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn replaces_an_existing_project_without_leaving_temporary_files() {
        let directory = scratch_directory();
        let path = directory.join("show.lpmt.json");

        write_atomic(&path, br#"{"version":1}"#).unwrap();
        write_atomic(&path, br#"{"version":2}"#).unwrap();

        assert_eq!(fs::read(&path).unwrap(), br#"{"version":2}"#);
        let entries: Vec<_> = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![OsString::from("show.lpmt.json")]);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn failed_replacement_removes_the_temporary_file() {
        let directory = scratch_directory();
        let path = directory.join("show.lpmt.json");
        fs::create_dir(&path).unwrap();

        assert!(write_atomic(&path, b"project").is_err());
        let entries: Vec<_> = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![OsString::from("show.lpmt.json")]);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn creates_missing_parent_directories() {
        let directory = scratch_directory();
        let path = directory.join("nested").join("show.lpmt.json");

        write_atomic(&path, b"project").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"project");
        fs::remove_dir_all(directory).unwrap();
    }
}
