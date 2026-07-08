# FAQ

**Q: "Unknown" status on a project?**  
A: The repository path is unreachable. Check the path in Settings.

**Q: Why does Smart Pull run sequentially?**  
A: Sequential execution prevents stacked conflict states.

**Q: Freezer blocked "tag already exists" — how to force-overwrite?**  
A: Delete the tag manually (`git tag -d <name>`) and re-validate in the Tag modal. knotra never overwrites automatically.

**Q: Where are settings stored?**  
A: `~/.config/knotra/config.toml`. See [Configuration File](config.md).
