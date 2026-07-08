# RFC-016 — Keyboard Shortcut System and Cheat-Sheet Overlay

| Field          | Value                                                                |
|----------------|----------------------------------------------------------------------|
| Status         | Proposed                                                             |
| Priority       | Medium — completes the keyboard-first interaction model              |
| Effort         | Medium — central key-binding table, overlay, conflict resolution     |
| Target version | v0.13                                                                |
| Related        | RFC-009 (selection keys), RFC-012 (palette key), RFC-015 (tabs)   |

## Summary

Define a complete keyboard shortcut scheme for knotra, organised around
the redesigned UI (selection-driven actions, modals, command palette).
Implement a `?` overlay that displays the scheme as a cheat sheet,
context-aware to what the user is currently doing.  Centralise key
bindings in a single table so changes don't require editing scattered
subscription code.

## Background

knotra v0.11.0 has keyboard shortcuts (`Ctrl+R`, `Ctrl+K`, `Ctrl+T`,
`Ctrl+/`, `Esc`) but they are:

- **Not documented in-app.**  Users discover them only through external
  docs or guesswork.
- **Scattered**: defined in various subscription handlers and `view/`
  modules.
- **Not consistent with the redesigned UI.**  The redesign assumes
  keyboard navigation through cards, selection toggles, action buttons,
  modal triggers — none of this exists yet.

This RFC establishes the **complete** keyboard scheme as one design
document, the place where future shortcut changes are discussed, and an
in-app cheat sheet that makes the scheme learnable.

## Requirements

| #   | Requirement |
|-----|-------------|
| R1  | All shortcuts defined in one central table (Rust constant) |
| R2  | The `?` key opens a cheat-sheet overlay |
| R3  | The cheat sheet is context-aware: shows shortcuts relevant to the current state (e.g., selection bar shortcuts only when selection is active) |
| R4  | The cheat sheet itself is dismissable with `?` or `Esc` |
| R5  | Each shortcut is i18n-aware: the binding stays the same, but the description is translated |
| R6  | Conflicting bindings (e.g., `Ctrl+W` for window close vs. modal close) are resolved by context |
| R7  | macOS uses `⌘` where Linux/Windows use `Ctrl` for parity bindings |
| R8  | Two-key sequences (vim-style `g h`) are supported for navigation |
| R9  | A leader key timeout (1 second) elapses between key 1 and key 2 |

## External Design

### Full keyboard scheme

#### Global (work in any state unless modal-focused)

| Shortcut             | Action                                  | Notes                                |
|----------------------|-----------------------------------------|--------------------------------------|
| `⌘K` / `Ctrl+K`      | Open command palette                    | RFC-012                             |
| `?`                  | Open cheat-sheet overlay                | This RFC                             |
| `⌘,` / `Ctrl+,`      | Open Settings                           |                                      |
| `⌘R` / `Ctrl+R`      | Refresh active workspace                | Existing                             |
| `⌘1` … `⌘9`          | Switch to workspace N                   | RFC-015                             |
| `⌘Shift+]`           | Next workspace                          | RFC-015                             |
| `⌘Shift+[`           | Previous workspace                      | RFC-015                             |
| `⌘N` / `Ctrl+N`      | New workspace                           | (was `Ctrl+T`)                       |
| `⌘W` / `Ctrl+W`      | Close current modal (or no-op if none)  | RFC-013                             |
| `Esc`                | Close palette → modal → selection → focus | Cascading priority (see below)     |

#### Dashboard navigation

| Shortcut             | Action                                                 |
|----------------------|--------------------------------------------------------|
| `↑` / `↓` / `j` / `k`| Move focus to previous / next card                     |
| `g g`                | Jump to top of list (vim style)                        |
| `G`                  | Jump to bottom of list                                 |
| `Enter`              | Open detail panel for focused card                     |
| `/`                  | Focus the search input                                 |
| `g h`                | Go to History (open activity popover)                  |
| `g s`                | Go to Settings                                         |
| `g w`                | Open workspace switcher (focus tab strip)              |

#### Selection (when ≥1 selected or focus on a card)

| Shortcut             | Action                                                       |
|----------------------|--------------------------------------------------------------|
| `Space`              | Toggle selection on focused card                             |
| `Shift+Space`        | Range-select to focused card                                 |
| `⌘+Space`            | Add focused card to selection (without affecting others)     |
| `⌘A` / `Ctrl+A`      | Select all in active workspace                               |
| `Esc`                | Clear selection (if any)                                     |

#### Selection bar actions (when ≥1 selected)

| Shortcut             | Action                                |
|----------------------|---------------------------------------|
| `f`                  | Fetch selected                        |
| `p`                  | Smart pull selected                   |
| `t`                  | Tag selected (opens Tag modal)        |
| `b`                  | Switch branch (opens Switch modal)    |
| `c`                  | Generate changelog (opens modal)      |

#### Modal-specific

| Shortcut             | Action                                |
|----------------------|---------------------------------------|
| `Esc`                | Cancel modal                          |
| `Enter`              | Trigger primary action button         |
| `Tab` / `Shift+Tab`  | Move between inputs                   |

#### Cheat sheet overlay

| Shortcut             | Action                                |
|----------------------|---------------------------------------|
| `?`                  | Open/close cheat sheet                |
| `Esc`                | Close cheat sheet                     |
| (typing in search)   | Filter shortcuts by name              |

### Cheat sheet overlay

```
┌─────────────────────────────────────────────────────────────────┐
│   Keyboard shortcuts                                       [✕]   │
│   ─────────────────────────────────────────────────────────────  │
│   [Type to filter…]                                              │
│                                                                  │
│   GLOBAL                                                         │
│     ⌘K                Open command palette                       │
│     ?                 Open this cheat sheet                      │
│     ⌘R                Refresh workspace                          │
│     ⌘N                New workspace                              │
│                                                                  │
│   NAVIGATE                                                       │
│     ↓ / j             Next card                                  │
│     ↑ / k             Previous card                              │
│     Enter             Open detail                                │
│     /                 Focus search                               │
│     g g               Top of list                                │
│     g h               History                                    │
│                                                                  │
│   SELECT                                                         │
│     Space             Toggle selection                           │
│     Shift+Space       Range select                               │
│     ⌘A                Select all                                 │
│     Esc               Clear selection                            │
│                                                                  │
│   ACTIONS (selection ≥ 1)                                        │
│     f                 Fetch                                      │
│     p                 Smart pull                                 │
│     t                 Tag…                                       │
│     b                 Switch branch…                             │
│     c                 Generate changelog…                        │
│                                                                  │
│   WORKSPACES                                                     │
│     ⌘1 … ⌘9          Switch to workspace 1-9                     │
│     ⌘Shift+]         Next workspace                              │
│     ⌘Shift+[         Previous workspace                          │
└─────────────────────────────────────────────────────────────────┘
```

The overlay is filterable: typing `tag` highlights only the rows
containing "tag".

### Esc cascading priority

A single `Esc` key has multiple possible interpretations.  Order of
priority (first match wins):

1. **Palette open?**  Close palette.
2. **Modal open?**  Cancel modal.
3. **Resolve panel open?**  Close panel.
4. **Detail panel open?**  Close panel.
5. **Cheat sheet open?**  Close cheat sheet.
6. **Selection non-empty?**  Clear selection.
7. **Search field focused?**  Clear search.
8. **Otherwise:** no-op.

This is mostly the existing iced behaviour for input forms; modal/palette
cases need explicit handling.

### Two-key sequences (leader keys)

Sequences like `g h` (go to history) are detected via a small state
machine:

1. Press `g` — knotra enters "g-leader" mode for 1 second.
2. Press `h` within 1 second — execute "Go to History."
3. Otherwise, after 1 second timeout, drop the leader state silently.

During the 1-second window, a small indicator at the bottom-right shows
`g…` so the user sees they're in a leader state.

This pattern is established (vim, helix, github).  Avoids consuming many
top-level single-letter keys for navigation.

### Modifier conventions

| Platform  | Primary modifier | Secondary | Naming in docs    |
|-----------|------------------|-----------|-------------------|
| macOS     | `⌘` (Command)    | `⌃` (Control) | `⌘K`           |
| Linux     | `Ctrl`           | `Alt`         | `Ctrl+K`        |
| Windows   | `Ctrl`           | `Alt`         | `Ctrl+K`        |

The cheat sheet and tooltips render the correct symbol per platform.

## Internal Design

### Central key-binding table

```rust
// shortcuts.rs
pub struct KeyBinding {
    pub id:          &'static str,        // "global.palette_open"
    pub i18n_key:    &'static str,        // "shortcut.palette_open"
    pub modifiers:   ModifierSet,         // e.g. Modifiers::COMMAND
    pub key:         BindingKey,
    pub leader:      Option<BindingKey>,  // Some(g) for "g h"
    pub context:     Context,             // when is this binding active
    pub dispatch:    fn(&AppState) -> Option<Message>,
}

pub enum BindingKey {
    Named(iced::keyboard::key::Named),
    Char(&'static str),    // "k", "?"
}

pub enum Context {
    Global,
    Dashboard,
    SelectionActive,
    ModalOpen,
    PaletteOpen,
    InputFocused,
}

pub static BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        id: "global.palette_open",
        i18n_key: "shortcut.palette_open",
        modifiers: Modifiers::COMMAND,
        key: BindingKey::Char("k"),
        leader: None,
        context: Context::Global,
        dispatch: |_| Some(Message::Palette(PaletteMessage::OpenRequested)),
    },
    KeyBinding {
        id: "selection.fetch",
        i18n_key: "shortcut.fetch",
        modifiers: Modifiers::NONE,
        key: BindingKey::Char("f"),
        leader: None,
        context: Context::SelectionActive,
        dispatch: |state| {
            if state.selection.is_empty() { return None; }
            Some(Message::Selection(SelectionMessage::ApplyAction(BulkAction::Fetch)))
        },
    },
    KeyBinding {
        id: "navigate.history",
        i18n_key: "shortcut.go_history",
        modifiers: Modifiers::NONE,
        key: BindingKey::Char("h"),
        leader: Some(BindingKey::Char("g")),
        context: Context::Dashboard,
        dispatch: |_| Some(Message::Activity(ActivityMessage::PopoverToggled)),
    },
    // ... ~50 bindings total
];
```

### Leader-key state

```rust
// state/mod.rs
pub struct AppState {
    // ... existing ...

    pub leader_key: Option<LeaderKeyState>,
}

pub struct LeaderKeyState {
    pub key:        BindingKey,           // the leader that was pressed
    pub started_at: std::time::Instant,
}
```

A subscription ticks every 200 ms while a leader is active; if more than
1 second has elapsed, clears `leader_key`.

### Subscription

A single global key subscription that walks `BINDINGS`:

```rust
fn handle_key_press(state: &AppState, key: BindingKey, mods: ModifierSet) -> Option<Message> {
    let active_context = active_context(state);

    for binding in BINDINGS {
        if !binding.context.is_active_in(active_context) { continue; }
        if binding.modifiers != mods { continue; }

        // Leader handling.
        match (&binding.leader, &state.leader_key) {
            (Some(leader), Some(active)) if leader == &active.key => {
                if binding.key == key {
                    return (binding.dispatch)(state);
                }
            }
            (None, _) => {
                if binding.key == key {
                    return (binding.dispatch)(state);
                }
            }
            (Some(_), None) => continue, // binding needs leader, none active
            _ => continue,
        }
    }

    // No binding matched; if key is a leader candidate, start leader state.
    if mods == ModifierSet::NONE && is_leader_candidate(&key) {
        return Some(Message::Shortcut(ShortcutMessage::LeaderStarted(key)));
    }
    None
}
```

### Cheat sheet view

```rust
// view/shortcuts.rs
pub fn overlay(state: &AppState) -> Option<Element<Message>> {
    if !state.cheat_sheet_open { return None; }

    let groups = group_bindings_by_section(state);
    let mut col = column![];
    for (section_title, bindings) in groups {
        col = col.push(text(state.t(&section_title)).size(14).bold());
        for b in bindings {
            col = col.push(binding_row(state, b));
        }
    }
    Some(modal_overlay(state, col))
}

fn binding_row(state: &AppState, b: &KeyBinding) -> Element<Message> {
    let shortcut_display = format_shortcut(b);  // "⌘K" on mac, "Ctrl+K" elsewhere
    let label = state.t(b.i18n_key);
    row![text(shortcut_display).font(MONO).width(150), text(label)].spacing(20).into()
}
```

### Context-awareness

```rust
fn active_context(state: &AppState) -> Context {
    if state.palette.open                       { return Context::PaletteOpen; }
    if !matches!(state.modal, Modal::None)      { return Context::ModalOpen; }
    if state.input_focused                       { return Context::InputFocused; }
    if !state.selection.is_empty()              { return Context::SelectionActive; }
    Context::Dashboard
}

impl Context {
    fn is_active_in(self, current: Context) -> bool {
        match (self, current) {
            (Context::Global, _) => true,
            (Context::Dashboard, Context::Dashboard) => true,
            (Context::Dashboard, Context::SelectionActive) => true,
            (Context::SelectionActive, Context::SelectionActive) => true,
            (Context::ModalOpen, Context::ModalOpen) => true,
            (Context::PaletteOpen, Context::PaletteOpen) => true,
            (Context::InputFocused, Context::InputFocused) => true,
            _ => false,
        }
    }
}
```

## Migration Plan

| Phase | Version | Scope |
|-------|---------|-------|
| 1     | v0.13   | Central BINDINGS table; existing shortcuts migrated; cheat sheet basics |
| 2     | v0.13   | New navigation shortcuts (j/k, g h, g s); leader-key state machine |
| 3     | v0.14   | Selection-bar shortcuts (f, p, t, b, c) |
| 4     | v0.15   | Workspace tab shortcuts (⌘1..⌘9) |
| 5     | v0.16   | Cheat sheet filter; first-launch banner suggesting "?" |

## Test Plan

### Unit tests

1. **`binding_resolves_to_message`** — known input (Ctrl+K) → palette open
   message.
2. **`context_filters_bindings`** — when selection is empty, `f` does
   nothing; when selection ≥ 1, `f` triggers fetch.
3. **`leader_sequence_completes`** — fire `g` then `h` within 500 ms →
   history toggle message.
4. **`leader_sequence_times_out`** — fire `g`, wait > 1 s, fire `h`.
   State: g consumed, h becomes a standalone (currently no `h` binding;
   silently dropped).
5. **`esc_priority_palette_first`** — palette open AND modal open;
   Esc closes palette only.
6. **`platform_modifier_rendering`** — `format_shortcut` on macOS returns
   "⌘K"; on Linux returns "Ctrl+K".

### Manual

1. `?` → cheat sheet opens.
2. Type "tag" in cheat sheet search → shortcut rows filter.
3. `g h` quickly → activity popover opens.
4. `g` then wait 2 s, then `h` → activity popover does NOT open.
5. Open Pull modal → `Esc` closes modal but selection persists.
6. With selection active, press `f` → bulk fetch starts.

## Open Questions

### Q1 — Conflict with letter-keys in input fields

When a text field has focus, `f` should not trigger fetch.  Resolution:
the `Context::InputFocused` check is mandatory and pre-empts all
selection-bar shortcuts.  Implementation note: track input focus via iced
`focus_next` / `focus_previous` semantics.

### Q2 — Leader chord vs. learning curve

Two-key sequences like `g h` are powerful but unfamiliar to non-vim users.
Mitigation: every leader-key action has a direct alternative (e.g., the
command palette).  Users who don't learn leaders can use ⌘K.

### Q3 — Customisable bindings

Should users be able to redefine shortcuts?  **Tentative answer**: not in
v0.13–v0.16.  The fixed scheme is documented and discoverable; remap
support is a future RFC.

### Q4 — Conflicts with iced built-ins

iced has built-in handling for some shortcuts (e.g., text input `Ctrl+A`
for select-all-text inside a field).  Our `Ctrl+A` for "select all
projects" must not fire when a text input is focused.  Handled by
`Context::InputFocused`.

## Security Considerations

None.  Shortcuts dispatch existing actions; they do not bypass any
permissions.
