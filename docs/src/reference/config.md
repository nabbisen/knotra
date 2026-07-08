# Configuration File

Location: `~/.config/knotra/config.toml`

```toml
locale = "En"              # "En" or "Ja"
dark_theme = true
refresh_interval_secs = 60 # 0 = manual only
max_concurrent_reads = 8
external_editor = ""       # empty string = disabled
external_merge_tool = ""
max_log_entries = 200
```

Workspaces: `~/.config/knotra/workspaces/<uuid>.toml`  
History: `~/.local/share/knotra/history/<timestamp>_<op-id>.json`

A corrupt or missing config causes knotra to start with safe defaults and display an error in the status bar.
