use std::fs;
use std::path::{Path, PathBuf};

fn sibling(final_path: &Path, tag: &str) -> PathBuf {
    let name = final_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    final_path.with_file_name(format!("{name}.{tag}-{}", uuid::Uuid::new_v4()))
}

/// A fresh sibling directory to write the new package into.
pub fn stage_dir(final_path: &Path) -> std::io::Result<PathBuf> {
    let parent = final_path.parent().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent folder"))?;
    if !parent.is_dir() { return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "the destination folder does not exist")); }
    let stage = sibling(final_path, "tmp");
    fs::create_dir(&stage)?;
    Ok(stage)
}

/// Swaps the staged directory into place. Either the old package or the new one survives a failure.
pub fn commit(stage: &Path, final_path: &Path) -> std::io::Result<()> {
    let backup = if final_path.exists() {
        let b = sibling(final_path, "bak");
        fs::rename(final_path, &b)?;
        Some(b)
    } else { None };
    if let Err(e) = fs::rename(stage, final_path) {
        let _ = fs::remove_dir_all(stage);
        if let Some(b) = &backup {
            if let Err(restore_err) = fs::rename(b, final_path) {
                // Neither the new package nor the restore made it; say exactly where the
                // old package still lives so it isn't mistaken for lost data.
                return Err(std::io::Error::other(format!(
                    "The previous project could not be restored; it is intact at {}. The new version was not saved. ({e}; restore failed: {restore_err})",
                    b.display()
                )));
            }
        }
        return Err(e);
    }
    // The new package is already in place; a failure to clean up the backup is not
    // this call's problem to report, so make it best-effort.
    if let Some(b) = backup { let _ = fs::remove_dir_all(&b); }
    Ok(())
}

pub fn abort(stage: &Path) { let _ = fs::remove_dir_all(stage); }

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("compositor-atomic-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn commit_replaces_existing_package_and_removes_backup() {
        let root = temp();
        let final_path = root.join("A.comp");
        fs::create_dir_all(final_path.join("images")).unwrap();
        fs::write(final_path.join("manifest.json"), b"old").unwrap();
        fs::write(final_path.join("images").join("x.png"), b"x").unwrap();
        let stage = stage_dir(&final_path).unwrap();
        fs::create_dir_all(stage.join("images")).unwrap();
        fs::write(stage.join("manifest.json"), b"new").unwrap();
        commit(&stage, &final_path).unwrap();
        assert_eq!(fs::read(final_path.join("manifest.json")).unwrap(), b"new");
        assert!(!final_path.join("images").join("x.png").exists(), "removed assets are dropped");
        let leftovers: Vec<_> = fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect();
        assert_eq!(leftovers, vec!["A.comp".to_string()]);
    }

    #[test]
    fn commit_into_a_file_path_fails_and_keeps_the_original() {
        let root = temp();
        let blocker = root.join("not-a-directory");
        fs::write(&blocker, b"1").unwrap();
        let final_path = blocker.join("CannotSave.comp");
        assert!(stage_dir(&final_path).is_err());
        assert_eq!(fs::read(&blocker).unwrap(), b"1");
    }

    // A `commit_succeeds_even_if_backup_cleanup_is_blocked` test (deliberately locking a
    // file inside the backup so its `remove_dir_all` fails) was tried and dropped: the
    // only deterministic way found to make a file undeletable on Windows is an exclusive,
    // no-sharing handle (`OpenOptions::share_mode(0)`), but that also blocks renaming the
    // *directory* that contains it -- which happens earlier, when the old package is
    // renamed to its backup path, well before the cleanup step this test wants to exercise.
    // That made the test fail for the wrong reason (the rename itself, not the cleanup) and
    // therefore unable to isolate the code path in question. Rust's own `fs::File::open`
    // does not block deletion on Windows (it shares `FILE_SHARE_DELETE` by default), so a
    // simple open handle does not reproduce the failure either. Absent a deterministic way
    // to fail only the cleanup step, the best-effort behavior is instead exercised
    // structurally below (no backup means nothing to clean up, exercising the `None` arm
    // of `if let Some(b) = backup`) and covered by code review of the `let _ =` in `commit`.

    #[test]
    fn commit_without_existing_package_has_no_backup() {
        let root = temp();
        let final_path = root.join("A.comp");
        let stage = stage_dir(&final_path).unwrap();
        fs::create_dir_all(stage.join("images")).unwrap();
        fs::write(stage.join("manifest.json"), b"new").unwrap();
        commit(&stage, &final_path).unwrap();
        assert_eq!(fs::read(final_path.join("manifest.json")).unwrap(), b"new");
        let leftovers: Vec<_> = fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect();
        assert_eq!(leftovers, vec!["A.comp".to_string()], "no backup directory should be left behind when there was nothing to back up");
    }
}
