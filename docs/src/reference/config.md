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
