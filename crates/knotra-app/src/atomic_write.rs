//! Atomic file writes: write-to-temp, `sync_all`, rename-over (Handoff 033
//! Task A).
//!
//! `std::fs::write` truncates the target before writing, so a crash, power
//! loss, or full disk mid-write leaves a truncated file. `config.rs`'s
//! `save_config` is called on every dashboard Group/Sort change and section
//! collapse, not only an explicit Save — a routine one-click action, not a
//! rare one — so the exposure is real rather than theoretical (`033` §1).
//!
//! Depends on nothing but `std`, so both `config.rs` and `persistence.rs`
//! (which already imports `crate::config::AppPaths`) can import this without
//! creating a module cycle.
//!
//! **No directory fsync after the rename.** `rename` itself is atomic on
//! both Unix and Windows (`std::fs::rename` uses `MOVEFILE_REPLACE_EXISTING`
//! on Windows, so an existing destination is replaced there too) — the risk
//! a directory fsync would additionally close is the sub-millisecond window
//! between the rename returning and the OS's own periodic flush of the
//! directory's metadata to disk, which would only matter if the machine
//! crashes in that exact window. For a single-user local desktop config file
//! that already degrades safely to defaults-plus-warning on any corruption
//! (`load_config`'s documented contract), that marginal durability is not
//! worth the added complexity — and unlike syncing the temp file's own
//! contents (portable via `File::sync_all`), syncing a directory's metadata
//! has no equivalent cross-platform API in `std`: `File::open` on a
//! directory to get a syncable handle works on Unix but fails outright on
//! Windows, which would mean `#[cfg(unix)]`-gating a guarantee this module
//! otherwise makes unconditionally. Recorded here as a considered choice,
//! not an oversight (`033` §2.5).

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Writes `contents` to `path` atomically. `path`'s parent directory must
/// already exist (callers already call `create_dir_all` before persisting;
/// this module does not duplicate that).
///
/// A temp file is written in the **same directory** as `path` (rename
/// across filesystems is not atomic and fails outright), `sync_all`'d
/// before the rename so the atomic swap can never land on a still-empty or
/// partial file, then renamed over `path`. The temp file is removed on
/// every failure path so `.tmp` litter never accumulates. If `path` already
/// exists, its permission bits are preserved across the swap — `File::create`
/// applies the process umask to the temp file, and a bare rename-over would
/// otherwise silently reset a mode the user deliberately set.
pub fn write(path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "path has no parent directory")
    })?;
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let temp_path = dir.join(format!("{}.tmp", file_name.to_string_lossy()));

    if let Err(e) = write_and_sync(&temp_path, contents.as_ref()) {
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }

    if let Err(e) = preserve_mode(path, &temp_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }

    if let Err(e) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }

    Ok(())
}

fn write_and_sync(temp_path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = File::create(temp_path)?;
    file.write_all(contents)?;
    file.sync_all()
}

#[cfg(unix)]
fn preserve_mode(existing_path: &Path, temp_path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    match fs::metadata(existing_path) {
        Ok(meta) => fs::set_permissions(
            temp_path,
            fs::Permissions::from_mode(meta.permissions().mode()),
        ),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(unix))]
fn preserve_mode(_existing_path: &Path, _temp_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_read_round_trips() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("config.toml");

        write(&path, "hello = 1").expect("write");

        assert_eq!(fs::read_to_string(&path).expect("read"), "hello = 1");
    }

    #[test]
    fn overwrite_replaces_contents_and_leaves_no_tmp_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("config.toml");

        write(&path, "first").expect("first write");
        write(&path, "second").expect("second write");

        assert_eq!(fs::read_to_string(&path).expect("read"), "second");
        assert!(
            !tmp.path().join("config.toml.tmp").exists(),
            "no .tmp litter should survive a successful write"
        );
    }

    /// Forces failure at the temp-write step (not the rename) by
    /// pre-occupying the exact temp path `write` will use with a directory,
    /// so `File::create` on it fails. `path` itself is never touched by
    /// `write` until the final rename, which this scenario never reaches —
    /// the old contents surviving is therefore proof the ordering is what
    /// this module claims, not an accident of the test.
    #[test]
    fn existing_contents_survive_a_failed_temp_write() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("config.toml");
        fs::write(&path, "original").expect("seed original contents");
        fs::create_dir(tmp.path().join("config.toml.tmp")).expect("block the temp path");

        let result = write(&path, "new contents");

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path).expect("read"), "original");
    }

    /// Forces failure at the rename step specifically — the temp file is
    /// written and synced successfully, then the rename target is an
    /// existing non-empty directory, which `fs::rename` cannot replace with
    /// a file. Exercises the cleanup-on-rename-failure path, not the
    /// cleanup-on-write-failure path the previous test covers.
    #[test]
    fn no_tmp_file_survives_a_failed_rename() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("blocked");
        fs::create_dir(&path).expect("target is a directory");
        fs::write(path.join("occupant"), "keeps the directory non-empty").expect("seed occupant");

        let result = write(&path, "new contents");

        assert!(result.is_err());
        assert!(
            !tmp.path().join("blocked.tmp").exists(),
            "the temp file written before the failed rename must be cleaned up"
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_existing_files_mode_survives_a_successful_write() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("config.toml");
        fs::write(&path, "original").expect("seed original contents");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("chmod");

        write(&path, "new contents").expect("write");

        let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "mode must survive the atomic swap");
    }
}
