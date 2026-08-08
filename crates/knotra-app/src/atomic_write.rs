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
//! on Windows, so an existing destination is replaced there too). The window
//! a directory fsync would additionally close is not sub-millisecond — on
//! ext4 the relevant interval is the journal commit, whose mount default is
//! `commit=5`, up to roughly five seconds — but it still does not protect
//! against corruption: because contents are `sync_all`'d *before* the
//! rename, a crash at any point already yields either the old file wholly
//! intact or the new file wholly intact, never torn or truncated. A
//! directory fsync would only narrow which of those two already-safe states
//! a crash lands in, for a single-user local desktop config file that
//! already degrades safely to defaults-plus-warning on any corruption
//! (`load_config`'s documented contract) — not worth the added complexity,
//! and unlike syncing the temp file's own contents (portable via
//! `File::sync_all`), syncing a directory's metadata has no equivalent
//! cross-platform API in `std`: `File::open` on a directory to get a
//! syncable handle works on Unix but fails outright on Windows, which would
//! mean `#[cfg(unix)]`-gating a guarantee this module otherwise makes
//! unconditionally. Recorded here as a considered choice, not an oversight
//! (`033` §2.5, corrected by `117` §5/Handoff 034 §3).

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Writes `contents` to `path` atomically. `path`'s parent directory must
/// already exist (callers already call `create_dir_all` before persisting;
/// this module does not duplicate that).
///
/// **`path` is resolved with `fs::canonicalize` before anything is derived
/// from it** (Handoff 034 Item 1): when `path` is a symlink — mainstream for
/// a config file kept in a dotfiles repository — a bare rename onto `path`
/// would replace *the link itself* with a regular file, silently breaking
/// it and leaving the file it pointed at holding stale contents. Resolving
/// first puts the temp file beside the *real* file and renames onto the
/// real file, so the swap happens on the file the link points at and the
/// link itself is never touched.
///
/// **A dangling symlink (Handoff 035) is not treated as "nothing exists" —
/// its target's parent directory decides what happens, and the choice
/// between the two is a governing principle, not a convenience call: an
/// operation must never silently destroy something the user created.**
///
/// - If the target's parent directory **exists**, this is the standard way a
///   managed config gets set up (`ln -s ~/dotfiles/knotra/config.toml
///   ~/.config/knotra/config.toml`, then let the app populate it) — the
///   write goes through to that target, exactly as `std::fs::write` (what
///   this module replaced) already did via `open(O_CREAT)` following the
///   link. The symlink is left untouched; only the file it will point at is
///   created.
/// - If the target's parent directory is **missing**, there is nothing safe
///   to write through to, and self-healing by replacing the link with a
///   regular file would destroy something the user deliberately created,
///   irreversibly, with nothing shown. `write` refuses instead: the link is
///   left exactly as it was, and the returned `io::Error` names both the
///   link and the missing directory, so `save_config`'s
///   `format!("cannot write config.toml: {e}")` reaches the status bar with
///   something the user can act on. `load_config`'s existing
///   defaults-plus-warning contract means nothing is lost in the meantime —
///   only the session's in-memory choice, which the user just made again.
///
/// Symlink resolution here goes **exactly one level** past whatever
/// `canonicalize` alone can already resolve — deliberately bounded, not an
/// oversight. `canonicalize` itself already resolves any chain of *valid*
/// symlinks; the one extra level here only ever applies to a *dangling*
/// link, and if that one-level target turns out to be *another* symlink
/// (a dangling chain) this is treated the same as a missing parent
/// directory — refused, nothing touched — rather than chased further.
///
/// This resolution is not only a symlink accommodation — it also makes the
/// same-filesystem guarantee below *more* robust. A symlink may point across
/// a mount boundary; without resolving first, the temp file would land
/// beside the *link* while the rename target is the link itself, which is
/// fine, but if some future caller's `path` were resolved differently the
/// unresolved form risks a temp file and rename target on different
/// filesystems. Resolving first keeps both on the real file's filesystem by
/// construction.
///
/// **Windows note:** `fs::canonicalize` returns the extended-length
/// `\\?\C:\…` form there. This module never stores that path — callers keep
/// their own `AppPaths` — so the only reachable effect is that form
/// appearing inside an error string surfaced to the status bar on a write
/// failure. Cosmetic; not otherwise handled here (`118` §5).
///
/// A temp file is written in the **same directory** as the resolved path
/// (rename across filesystems is not atomic and fails outright), `sync_all`'d
/// before the rename so the atomic swap can never land on a still-empty or
/// partial file, then renamed over the resolved path. The temp file is
/// removed on every failure path so `.tmp` litter never accumulates. If the
/// resolved path already exists, its permission bits are preserved across
/// the swap — `File::create` applies the process umask to the temp file, and
/// a bare rename-over would otherwise silently reset a mode the user
/// deliberately set.
pub fn write(path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    let real_path = resolve_write_target(path)?;

    let dir = real_path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "path has no parent directory")
    })?;
    let file_name = real_path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let temp_path = dir.join(format!("{}.tmp", file_name.to_string_lossy()));

    if let Err(e) = write_and_sync(&temp_path, contents.as_ref()) {
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }

    if let Err(e) = preserve_mode(&real_path, &temp_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }

    if let Err(e) = fs::rename(&temp_path, &real_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }

    Ok(())
}

/// The path `write` should actually write to — resolved, never the symlink
/// itself (Handoffs 034/035). See `write`'s own doc comment for the full
/// reasoning; this function is the decision, kept separate so it can fail
/// (case (a), a dangling link with no safe target) without `write` having
/// touched any file yet.
fn resolve_write_target(path: &Path) -> io::Result<PathBuf> {
    if let Ok(real) = fs::canonicalize(path) {
        return Ok(real);
    }

    // `canonicalize` failed. Either nothing is at `path` at all (first-ever
    // write), or `path` is a symlink whose chain does not fully resolve.
    let Ok(link_meta) = fs::symlink_metadata(path) else {
        return Ok(path.to_path_buf());
    };
    if !link_meta.file_type().is_symlink() {
        return Ok(path.to_path_buf());
    }

    // Resolve exactly one level (the module doc comment's deliberate bound)
    // rather than chasing an unbounded chain. A relative target is joined
    // against the *link's* parent directory, never the process working
    // directory (Handoff 035 §3.1) — getting this wrong would reintroduce
    // the exact launch-directory-dependent bug Handoff 033 Task B removed.
    let raw_target = fs::read_link(path)?;
    let link_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let resolved_target = if raw_target.is_absolute() {
        raw_target
    } else {
        link_dir.join(raw_target)
    };

    // The one-level target is itself another symlink: a dangling chain.
    // `canonicalize` above already resolves any chain that terminates in a
    // real file; reaching here means it does not, so this is refused rather
    // than chased further (Handoff 035 §3.2).
    if let Ok(target_meta) = fs::symlink_metadata(&resolved_target)
        && target_meta.file_type().is_symlink()
    {
        return Err(io::Error::other(format!(
            "\"{}\" points to another symlink (\"{}\") that does not itself \
             resolve to a real file. This only follows one level of a \
             dangling link chain, to avoid an unbounded chase — nothing \
             was written and the symlink was left untouched. Fix the chain \
             manually before trying again.",
            path.display(),
            resolved_target.display(),
        )));
    }

    let target_parent = resolved_target.parent();
    let target_parent_exists =
        target_parent.is_some_and(|p| fs::metadata(p).is_ok_and(|m| m.is_dir()));

    if target_parent_exists {
        // Case (b): a dangling link whose target directory is ready and
        // waiting — write through, as `std::fs::write` already did.
        return Ok(resolved_target);
    }

    // Case (a): nothing safe to write through to. Refuse rather than
    // self-heal by replacing the link with a regular file — that would
    // destroy something the user deliberately created, irreversibly, with
    // nothing shown (owner direction, `119`).
    Err(io::Error::other(format!(
        "\"{}\" is a symlink pointing to a missing location (\"{}\"); the \
         directory it would need to be created in (\"{}\") does not exist. \
         Nothing was written and the symlink was left untouched.",
        path.display(),
        resolved_target.display(),
        target_parent
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| resolved_target.display().to_string()),
    )))
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

    /// Handoff 034 Item 1's direct regression guard: `std::fs::write` (what
    /// this module replaced) follows a symlink and updates its target;
    /// `atomic_write::write` must do the same rather than replacing the
    /// link itself with a regular file. Asserts both halves, the same shape
    /// `117`'s throwaway probe used to find the bug in the first place.
    #[cfg(unix)]
    #[test]
    fn writing_through_a_symlink_preserves_the_link_and_updates_its_target() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let real_target = tmp.path().join("real-config.toml");
        let link_path = tmp.path().join("config.toml");
        fs::write(&real_target, "original").expect("seed original contents");
        std::os::unix::fs::symlink(&real_target, &link_path).expect("create symlink");

        write(&link_path, "updated").expect("write");

        assert!(
            fs::symlink_metadata(&link_path)
                .expect("symlink_metadata")
                .file_type()
                .is_symlink(),
            "the link itself must survive — only its target's contents change"
        );
        assert_eq!(
            fs::read_to_string(&real_target).expect("read real target"),
            "updated",
            "the real file the link points at must hold the new contents"
        );
    }

    /// Handoff 035 case (b): a dangling link whose target's parent
    /// directory already exists — the standard way a managed config gets
    /// set up (link first, let the app populate it). Replaces the retired
    /// `writing_through_a_dangling_symlink_replaces_it_with_a_regular_file`,
    /// which asserted the opposite outcome (`119` superseded that behaviour
    /// before this handoff was ever issued — the retired test's fixture,
    /// `tmp.path().join("does-not-exist.toml")`, has an existing parent, so
    /// it was always this case, not case (a)).
    #[cfg(unix)]
    #[test]
    fn writing_through_a_dangling_symlink_with_an_existing_target_directory_writes_through() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let missing_target = tmp.path().join("does-not-exist.toml");
        let link_path = tmp.path().join("config.toml");
        std::os::unix::fs::symlink(&missing_target, &link_path).expect("create dangling symlink");

        write(&link_path, "content").expect("write");

        assert!(
            fs::symlink_metadata(&link_path)
                .expect("symlink_metadata")
                .file_type()
                .is_symlink(),
            "the link must survive — nothing about this case destroys it"
        );
        assert_eq!(
            fs::read_to_string(&missing_target).expect("read the now-created target"),
            "content"
        );
    }

    /// Case (b) with a relative link target — the case that would silently
    /// break if the resolved target were joined against the process working
    /// directory instead of the link's own parent (Handoff 035 §3.1).
    #[cfg(unix)]
    #[test]
    fn writing_through_a_dangling_symlink_with_a_relative_target_resolves_against_the_links_directory()
     {
        let tmp = tempfile::tempdir().expect("tempdir");
        let sub_dir = tmp.path().join("sub");
        fs::create_dir(&sub_dir).expect("create sub dir");
        let link_path = sub_dir.join("config.toml");
        // Relative to `sub_dir`, not the process's actual working directory.
        std::os::unix::fs::symlink("real-config.toml", &link_path)
            .expect("create dangling symlink with a relative target");

        write(&link_path, "content").expect("write");

        assert!(
            fs::symlink_metadata(&link_path)
                .expect("symlink_metadata")
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(sub_dir.join("real-config.toml"))
                .expect("read the target resolved relative to the link's own directory"),
            "content"
        );
    }

    /// Handoff 035 case (a): a dangling link whose target's parent
    /// directory does not exist either. Nothing safe to write through to —
    /// `write` must refuse rather than self-heal by replacing the link
    /// (owner direction, `119`). Asserts all three: the call fails, the
    /// link survives untouched, and the error names the missing directory.
    #[cfg(unix)]
    #[test]
    fn writing_through_a_dangling_symlink_with_a_missing_target_directory_is_refused() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let missing_target = tmp.path().join("no-such-dir").join("config.toml");
        let link_path = tmp.path().join("config.toml");
        std::os::unix::fs::symlink(&missing_target, &link_path).expect("create dangling symlink");

        let result = write(&link_path, "content");

        let err = result.expect_err("must refuse rather than self-heal");
        let message = err.to_string();
        assert!(
            message.contains("no-such-dir"),
            "error must name the missing directory: {message:?}"
        );
        assert!(
            fs::symlink_metadata(&link_path)
                .expect("symlink_metadata")
                .file_type()
                .is_symlink(),
            "the link must be left exactly as it was"
        );
    }

    /// Handoff 035 §3.2's deliberate one-level bound: a link pointing at
    /// *another* dangling link must land in case (a)'s safe branch rather
    /// than being chased further or treated as case (b) just because the
    /// one-level target's own parent directory happens to exist.
    #[cfg(unix)]
    #[test]
    fn writing_through_a_dangling_symlink_chain_is_refused_and_destroys_nothing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let link1 = tmp.path().join("link1.toml");
        let link2 = tmp.path().join("link2.toml");
        let never_exists = tmp.path().join("still-does-not-exist.toml");
        std::os::unix::fs::symlink(&never_exists, &link2).expect("create the dangling second link");
        std::os::unix::fs::symlink(&link2, &link1)
            .expect("create the first link, pointing at link2");

        let result = write(&link1, "content");

        assert!(result.is_err(), "a dangling chain must not resolve");
        assert!(
            fs::symlink_metadata(&link1)
                .expect("symlink_metadata link1")
                .file_type()
                .is_symlink(),
            "link1 must survive untouched"
        );
        assert!(
            fs::symlink_metadata(&link2)
                .expect("symlink_metadata link2")
                .file_type()
                .is_symlink(),
            "link2 must survive untouched too — nothing in the chain is destroyed"
        );
        assert!(!never_exists.exists());
    }
}
