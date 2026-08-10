# Configuration File

Location: `~/.config/knotra/config.toml`

```toml
locale = "En"               # "En" or "Ja"
dark_theme = true
refresh_interval_secs = 60  # 0 = manual only
max_concurrent_reads = 8
max_log_entries = 200

# Optional — omit the key to disable
# external_editor = "/usr/bin/nvim"
# external_merge_tool = "/usr/bin/meld"

# Filesystem watch (default: disabled)
fs_watch_enabled = false
fs_debounce_secs = 2
```

Workspace definitions: `~/.config/knotra/workspaces/<uuid>.toml`  
Operation history: `~/.local/share/knotra/history/<timestamp>_<op-id>.json`

A corrupt or missing config causes knotra to start with safe defaults and display an error in the status bar.

## How saving works

knotra rewrites `config.toml` whenever a setting changes — including changing the
dashboard's **Group** or **Sort**, or collapsing a section, not only when you press
**Save Settings**.

Each save is atomic. The new contents go to a temporary file beside the target, are
flushed to disk, and are then renamed into place. A crash, power loss, or full disk
partway through leaves either the old file or the new one intact — never a truncated
one. An existing file's permissions are preserved, so a mode you set deliberately is
not reset by a save.

The same applies to workspace files and operation history.

## Keeping `config.toml` in a dotfiles repository

Symlinking the config file is supported:

```sh
ln -s ~/dotfiles/knotra/config.toml ~/.config/knotra/config.toml
```

knotra writes **through** the link and leaves the link itself in place, so the
repository keeps receiving changes. This works even if the target does not exist
yet — knotra creates it, as long as its directory does. So you can create the link
first and let knotra populate it.

If the link cannot be followed to somewhere writable — its target's directory does
not exist, or it points at another broken link — knotra **refuses to save** rather
than replacing your link with a regular file. The link is left exactly as it was.
Your change still applies for the rest of the session; it is simply not written to
disk. Repair the link, then make the change again to persist it.

Where the reason appears depends on how you triggered the save:

| Trigger | What you see |
|---|---|
| **Save Settings** | The full reason, naming the link and the missing directory |
| Group / Sort / collapsing a section | A short "could not save" notice in the status bar; the full reason goes to the log |

In the second case the full reason is logged at warning level, which knotra prints
by default — so it is visible if you started knotra from a terminal, and lost if you
started it from a desktop launcher.

## When the config directory cannot be found

knotra resolves `~/.config` (or the platform equivalent) from your environment. If
that fails, it falls back to the directory knotra was started from and says so in
the status bar. Without that warning the fallback would be silent, and settings
would appear to come and go depending on where you launched knotra from.
