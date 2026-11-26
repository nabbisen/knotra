# FAQ

**Q: "Unknown" status on a project?**  
A: The repository path is unreachable. Check the path in Settings.

**Q: Why does Get latest safely run sequentially?**\
A: Sequential execution prevents stacked conflict states.

**Q: Save release point says "This release name is already in use" — how do I force it?**\
A: Delete the tag manually (`git tag -d <name>`) and choose the name again in Save release point. knotra never overwrites automatically.

**Q: Where are settings stored?**  
A: `~/.config/knotra/config.toml`. See [Configuration File](config.md).
