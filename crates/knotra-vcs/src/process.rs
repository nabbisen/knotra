//! RFC-068 D1/D2: inside a Flatpak sandbox, knotra has no `git`/`jj` of its
//! own and its `XDG_*_HOME` variables point at the sandbox's own folders,
//! not the user's. This module makes both transparent to every caller:
//! `host_command` always runs the named program on the user's own system,
//! and `restore_host_xdg_dirs` puts the four `XDG_*_HOME` variables back to
//! the values the user's own Git already reads.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Whether knotra is running inside a Flatpak sandbox: the file
/// `/.flatpak-info` exists in every sandbox Flatpak creates, and in no
/// other environment. Detected once and cached — the answer cannot change
/// while knotra runs.
fn in_flatpak() -> bool {
    static DETECTED: OnceLock<bool> = OnceLock::new();
    *DETECTED.get_or_init(|| Path::new("/.flatpak-info").exists())
}

/// The pure half of [`host_command`]: given whether knotra is sandboxed,
/// returns the program to run, the leading arguments to run it with, and
/// the working folder to set on the `Command` (if any) — everything
/// [`host_command`] needs to build the real `std::process::Command`,
/// without touching the filesystem or the environment itself. This is what
/// the tests exercise directly, for both branches and every `cwd` shape.
fn command_line(
    in_flatpak: bool,
    program: &str,
    cwd: Option<&Path>,
) -> (String, Vec<String>, Option<PathBuf>) {
    if in_flatpak {
        let mut leading = vec!["--host".to_owned(), "--watch-bus".to_owned()];
        if let Some(cwd) = cwd {
            leading.push(format!("--directory={}", cwd.display()));
        }
        leading.push(program.to_owned());
        ("flatpak-spawn".to_owned(), leading, None)
    } else {
        (program.to_owned(), Vec::new(), cwd.map(Path::to_path_buf))
    }
}

/// Builds the command that runs `program` on the user's own system, in `cwd`.
/// Inside a Flatpak it goes through `flatpak-spawn --host`, so the user's own
/// installation, settings and credentials apply; elsewhere it is the program itself.
pub fn host_command(program: &str, cwd: Option<&Path>) -> Command {
    let (program, leading_args, working_dir) = command_line(in_flatpak(), program, cwd);
    // RFC-068 D1: the one allowed direct construction — every other call
    // site in the workspace goes through this function instead (enforced
    // by `clippy.toml`'s `disallowed-methods`).
    #[allow(clippy::disallowed_methods)]
    let mut cmd = Command::new(program);
    cmd.args(leading_args);
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    cmd
}

/// The pure half of [`restore_host_xdg_dirs`]: the folder `var` (one of
/// `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_STATE_HOME`)
/// should point at — `host_value` (Flatpak's own `HOST_XDG_*` passthrough)
/// if it is set to an absolute path, else the XDG default under `home`. A
/// relative `host_value` is treated as unset, per the XDG Base Directory
/// specification, which requires every one of these to be absolute.
fn host_xdg_dir(var: &str, host_value: Option<&OsStr>, home: &Path) -> PathBuf {
    if let Some(value) = host_value {
        let path = Path::new(value);
        if path.is_absolute() {
            return path.to_path_buf();
        }
    }
    let default_suffix = match var {
        "XDG_CONFIG_HOME" => ".config",
        "XDG_DATA_HOME" => ".local/share",
        "XDG_CACHE_HOME" => ".cache",
        "XDG_STATE_HOME" => ".local/state",
        other => unreachable!("host_xdg_dir called with an unknown XDG variable: {other}"),
    };
    home.join(default_suffix)
}

/// RFC-068 D2: puts `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` and
/// `XDG_STATE_HOME` back to the values the user's own Git and Jujutsu read
/// outside the sandbox, so in-process reads (gix) see the same
/// `~/.config/git/config` and `~/.config/git/ignore` the user's own `git`
/// does, and knotra's own settings live in the same place as the tar.gz
/// build's. Does nothing outside a Flatpak.
///
/// Must be called before any other thread exists — it is the first
/// statement of `main`, and the `unsafe` block below is sound only because
/// of that: `std::env::set_var` is unsound if another thread could read or
/// write the environment concurrently, and at this point in startup none
/// does.
pub fn restore_host_xdg_dirs() {
    if !in_flatpak() {
        return;
    }
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let home = PathBuf::from(home);

    let targets: Vec<(&str, PathBuf)> = [
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "XDG_STATE_HOME",
    ]
    .into_iter()
    .map(|var| {
        let host_value = std::env::var_os(format!("HOST_{var}"));
        (var, host_xdg_dir(var, host_value.as_deref(), &home))
    })
    .collect();

    // Sound: this runs as the first statement of `main`, before any other
    // thread exists to read or write the environment concurrently.
    unsafe {
        for (var, value) in targets {
            std::env::set_var(var, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_outside_a_flatpak_is_todays_behaviour() {
        let (program, args, cwd) = command_line(false, "git", Some(Path::new("/tmp/repo")));
        assert_eq!(program, "git");
        assert!(args.is_empty());
        assert_eq!(cwd, Some(PathBuf::from("/tmp/repo")));
    }

    #[test]
    fn command_line_outside_a_flatpak_with_no_cwd() {
        let (program, args, cwd) = command_line(false, "jj", None);
        assert_eq!(program, "jj");
        assert!(args.is_empty());
        assert_eq!(cwd, None);
    }

    #[test]
    fn command_line_inside_a_flatpak_with_a_cwd() {
        let (program, args, cwd) = command_line(true, "git", Some(Path::new("/tmp/repo")));
        assert_eq!(program, "flatpak-spawn");
        assert_eq!(
            args,
            vec!["--host", "--watch-bus", "--directory=/tmp/repo", "git"]
        );
        assert_eq!(cwd, None);
    }

    #[test]
    fn command_line_inside_a_flatpak_with_no_cwd() {
        let (program, args, cwd) = command_line(true, "jj", None);
        assert_eq!(program, "flatpak-spawn");
        assert_eq!(args, vec!["--host", "--watch-bus", "jj"]);
        assert_eq!(cwd, None);
    }

    #[test]
    fn host_xdg_dir_uses_an_absolute_host_value() {
        let got = host_xdg_dir(
            "XDG_CONFIG_HOME",
            Some(OsStr::new("/home/user/.config")),
            Path::new("/home/user"),
        );
        assert_eq!(got, PathBuf::from("/home/user/.config"));
    }

    #[test]
    fn host_xdg_dir_falls_back_when_unset() {
        let home = Path::new("/home/user");
        assert_eq!(
            host_xdg_dir("XDG_CONFIG_HOME", None, home),
            home.join(".config")
        );
        assert_eq!(
            host_xdg_dir("XDG_DATA_HOME", None, home),
            home.join(".local/share")
        );
        assert_eq!(
            host_xdg_dir("XDG_CACHE_HOME", None, home),
            home.join(".cache")
        );
        assert_eq!(
            host_xdg_dir("XDG_STATE_HOME", None, home),
            home.join(".local/state")
        );
    }

    #[test]
    fn host_xdg_dir_treats_a_relative_host_value_as_unset() {
        let home = Path::new("/home/user");
        let got = host_xdg_dir("XDG_DATA_HOME", Some(OsStr::new("relative/path")), home);
        assert_eq!(got, home.join(".local/share"));
    }
}
