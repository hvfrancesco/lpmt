//! Crash-recovery snapshots for unsaved project edits.
//!
//! A recovery file is a hidden sibling of the project file. It is written while
//! the project is dirty and removed after a successful save. On startup the app
//! compares a snapshot with the project on disk: an identical snapshot is
//! stale and can be discarded, while a different or orphaned snapshot is
//! offered to the user for restoration.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};
use lpmt_core::model::Project;

use crate::project_io;

#[derive(Debug, Clone)]
pub struct RecoveryCandidate {
    pub project_path: PathBuf,
    pub recovery_path: PathBuf,
    pub project: Project,
}

/// The recovery file is a hidden sibling, for example
/// `/shows/.show.lpmt.json.lpmt-recovery`.
pub fn path_for(project_path: &Path) -> PathBuf {
    let parent = project_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new(""));
    let file_name = project_path
        .file_name()
        .unwrap_or_else(|| OsStr::new("project.lpmt.json"));
    let mut name = OsString::from(".");
    name.push(file_name);
    name.push(".lpmt-recovery");
    parent.join(name)
}

pub fn write(project_path: &Path, project: &Project) -> Result<PathBuf> {
    let recovery_path = path_for(project_path);
    let json =
        serde_json::to_string_pretty(project).context("failed to serialize recovery snapshot")?;
    project_io::write_atomic(&recovery_path, json.as_bytes())?;
    Ok(recovery_path)
}

pub fn remove(project_path: &Path) -> Result<()> {
    let recovery_path = path_for(project_path);
    match fs::remove_file(&recovery_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to remove recovery snapshot {}",
                recovery_path.display()
            )
        }),
    }
}

/// Find a recovery snapshot that differs from the project currently on disk.
/// An identical snapshot is stale and is removed automatically.
pub fn detect(project_path: &Path) -> Result<Option<RecoveryCandidate>> {
    let recovery_path = path_for(project_path);
    let recovery_json = match fs::read_to_string(&recovery_path) {
        Ok(json) => json,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to read recovery snapshot {}",
                    recovery_path.display()
                )
            });
        }
    };
    let recovered: Project = serde_json::from_str(&recovery_json).with_context(|| {
        format!(
            "failed to parse recovery snapshot {}",
            recovery_path.display()
        )
    })?;
    ensure!(
        recovered.schema_version <= 2,
        "recovery snapshot schema version {} is newer than this build supports",
        recovered.schema_version
    );

    let original = fs::read_to_string(project_path)
        .ok()
        .and_then(|json| serde_json::from_str::<Project>(&json).ok());
    if original.as_ref() == Some(&recovered) {
        let _ = fs::remove_file(&recovery_path);
        return Ok(None);
    }

    Ok(Some(RecoveryCandidate {
        project_path: project_path.to_path_buf(),
        recovery_path,
        project: recovered,
    }))
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
        let directory = std::env::temp_dir().join(format!(
            "lpmt-recovery-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn write_project(path: &Path, project: &Project) {
        let json = serde_json::to_string_pretty(project).unwrap();
        fs::write(path, json).unwrap();
    }

    #[test]
    fn recovery_path_is_a_hidden_sibling() {
        assert_eq!(
            path_for(Path::new("/shows/show.lpmt.json")),
            PathBuf::from("/shows/.show.lpmt.json.lpmt-recovery")
        );
        assert_eq!(
            path_for(Path::new("project.lpmt.json")),
            PathBuf::from(".project.lpmt.json.lpmt-recovery")
        );
    }

    #[test]
    fn identical_snapshot_is_stale_and_removed() {
        let directory = scratch_directory();
        let project_path = directory.join("show.json");
        let project = Project::demo();
        write_project(&project_path, &project);
        let recovery_path = write(&project_path, &project).unwrap();

        assert!(detect(&project_path).unwrap().is_none());
        assert!(!recovery_path.exists());

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_different_snapshot_is_detected() {
        let directory = scratch_directory();
        let project_path = directory.join("show.json");
        let project = Project::demo();
        write_project(&project_path, &project);

        let mut recovered = Project::demo();
        recovered.name = "Recovered unsaved show".to_owned();
        let recovery_path = write(&project_path, &recovered).unwrap();

        let candidate = detect(&project_path).unwrap().expect("recovery candidate");
        assert_eq!(candidate.project_path, project_path);
        assert_eq!(candidate.recovery_path, recovery_path);
        assert_eq!(candidate.project.name, "Recovered unsaved show");

        remove(&project_path).unwrap();
        assert!(!recovery_path.exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn removing_a_missing_snapshot_is_not_an_error() {
        let directory = scratch_directory();
        remove(&directory.join("missing.json")).unwrap();
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn an_orphaned_snapshot_is_detected() {
        let directory = scratch_directory();
        let project_path = directory.join("show.json");
        let mut recovered = Project::demo();
        recovered.name = "Orphaned recovery".to_owned();
        write(&project_path, &recovered).unwrap();

        let candidate = detect(&project_path).unwrap().expect("recovery candidate");
        assert_eq!(candidate.project.name, "Orphaned recovery");

        remove(&project_path).unwrap();
        fs::remove_dir_all(directory).unwrap();
    }
}
