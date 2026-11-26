# Quick Start

## 1. Build

```sh
cargo build --release -p knotra
```

## 2. Run

```sh
./target/release/knotra
```

## 3. Add repositories

On first run the Dashboard shows an empty state with an **Add project folder** button. Click it and provide the path to a local Git or jj repository. Repeat for each repository you want to monitor.

To add more projects to an existing workspace, open the **Command palette** from the header and choose "Add project folder" there.

## 4. Refresh

Press **Ctrl+R** / **⌘R** or click **Refresh** on the Dashboard to load the current status of all projects.

## 5. Explore

- **Dashboard** — card-grid overview of all repositories, grouped by how much attention they need.
- **Get latest safely** — select projects on the Dashboard, then click **Check for updates** or **Get latest safely** on the selection bar.
- **Change work area** — switch branch or change-set across repositories (`Ctrl+K` or the **Change work area** button).
- **Save release point** — create release tags/bookmarks atomically (`Ctrl+T` or the **Save release point** button).
- **History** — browse and copy operation logs, with each one shown in your own time zone.
