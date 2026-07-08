# Quick Start

## 1. Build

```sh
cargo build --release -p knotra-app
```

## 2. Run

```sh
./target/release/knotra
```

## 3. Add repositories

On first run the Dashboard shows an empty state with an **Add Project** button. Click it and provide the path to a local Git or jj repository. Repeat for each repository you want to monitor.

To add more projects to an existing workspace, use the **Add Project** button that appears in the empty-state view, or open the command palette with `⌘/Ctrl+K`.

## 4. Refresh

Press **⌘/Ctrl+R** or click **Refresh** on the Dashboard to load the current status of all projects.

## 5. Explore

- **Dashboard** — card-grid overview of all repositories, grouped by attention tier.
- **Sync & Pull** — bulk fetch and Smart Pull (selection bar or `f` / `p` keys).
- **Context Switch** — switch branch or change-set across repositories (`b` key or **Switch…** button).
- **Freezer** — create release tags/bookmarks atomically (`t` key or **Tag…** button).
- **History** — browse and copy operation logs.
