# Local Development

## Prerequisites

- Rust 2024 edition (rustc ≥ 1.85; tested with 1.91)
- Linux: `libwayland-dev`, `libxkbcommon-dev`, `libgl-dev`, `pkg-config`
- `git` in PATH; `jj` in PATH (optional, for jj tests)

## Build & test

```sh
cargo build
cargo test
cargo build --release -p knotra-app && ./target/release/knotra
```

## Adding i18n strings

Add key-value pairs to both `en_strings()` and `ja_strings()` in `crates/knotra-ui/src/i18n.rs`, then use `state.t("my.key")` in views. Never hardcode user-visible strings outside i18n.

## Release

Bump `version` in the workspace `Cargo.toml`, update `CHANGELOG.md`, then:

```sh
cargo test
tar --exclude='./target' -czf knotra-v<X.Y.Z>.tar.gz -C /path/to/knotra .
```

The `-C <dir> .` form extracts files to the root of the archive with no intermediate parent directory, matching the release spec. Verify with:

```sh
tar tzf knotra-v<X.Y.Z>.tar.gz | head -5
# should start with ./Cargo.toml, ./crates/, etc. — not knotra/Cargo.toml
```
