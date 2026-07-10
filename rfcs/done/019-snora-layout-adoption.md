# RFC-0019 — Adopt snora layout framework (v0.18.0)

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| Status         | Implemented (v0.16.0)                                                              |
| Priority       | Medium                                                                |
| Effort         | Medium                                                                |
| Target version | v0.16 (or later, pending RFC-0017)                                    |
| Related        | RFC-0017 (screen removal), RFC-0018 (crate migration)                 |

## Summary

snora 0.18.0 is available and it is a **confirmed fit** for knotra. This
RFC defines what to adopt, what to keep, how the two layers divide
responsibility, and what the adoption entails in concrete code terms. It
is the "snora layout-framework adoption" deferred from RFC-0018 and now
more precisely specified thanks to the upstream author's clarification
(recorded in RFC-0018's resolved open question #5).

## What snora 0.18.0 provides

snora is an iced 0.14 GUI skeleton framework. Its three published crates:

- **`snora-core`** — pure vocabulary, no iced dependency: `AppLayout`,
  `LayoutDirection`, `Edge`, `Toast`, `ToastIntent`, `ToastLifetime`,
  `ToastPosition`, `Dialog`, `Sheet`, `SheetEdge`, `SheetSize`, `Menu`,
  `MenuItem`, `MenuAction`, `SideBar`, `SideBarItem`, `Tab`, `TabBar`,
  `TabAction`, `Crumb`, `BreadcrumbAction`, `Icon`.
- **`snora`** — the iced engine: `render(AppLayout<Element, Message>) ->
  Element` (the single entry point), `toast::subscription` +
  `toast::sweep_expired` (framework-managed TTL), `keyboard::dismiss_on_escape`,
  plus re-exports of all `snora-core` vocabulary.
- **`snora-widgets`** (optional, on by default) — prefab `iced::Element`
  builders: `app_header`, `app_side_bar`, `app_footer`, `app_tab_bar`,
  `app_breadcrumb`, `render_menu`, `icon_element`.

The framework's design contract (confirmed authoritative from the author):

- **Theme-delegating, not theme-owning.** Prefab widgets pull chrome colors
  from the active iced `Theme`. knotra defines the iced theme; snora
  picks it up automatically.
- **i18n catalog-agnostic.** knotra owns its message catalog. snora owns
  only layout direction (ABDD): `Edge::Start/End`, `LayoutDirection::Ltr/Rtl`,
  automatic RTL mirroring of sidebars, toasts, sheet anchors, and separator
  glyphs.
- **`#[non_exhaustive]` `AppLayout`.** Builder-style construction
  (`AppLayout::new(body).header(h)…`) is the stable API; future overlay
  surfaces can be added as additive minor releases.
- **Skeleton, not styling.** Slot content, typography, and card visuals
  are the application's domain. snora positions and stacks.

snora 0.18.0 uses iced 0.14, matching knotra's dependency exactly —
no version conflict.

## What knotra currently hand-rolls that snora covers

### Layer composition / z-stack (`view/mod.rs::app_view`)

knotra's `app_view` hand-builds a `stack!` with five conditional layers
(base, add-project modal, bulk-action modal, command palette, shortcuts
overlay). It also manages the dim backdrop and click-outside semantics
for each modal manually (`container(m).center(Length::Fill)`). snora's
`render(AppLayout)` implements this layer composition with correct z-order
(skeleton → menu backdrop → header/context menus → modal dim → dialog →
sheet → toasts) plus graceful degradation for absent close sinks.

| knotra hand-roll | snora equivalent |
|---|---|
| `stack!([base, ..layers])` | `render(AppLayout::new(body)…)` |
| Manual modal dim + click-outside | `AppLayout::on_close_modals(msg)` |
| Manual menu backdrop | `AppLayout::on_close_menus(msg)` |
| `container(m).center(Length::Fill)` for dialogs | `AppLayout::dialog(Dialog::new(el))` |
| `container(m).align_x(End)` for sheets | `AppLayout::sheet(Sheet::new(el).at(SheetEdge::End))` |
| `stack!` with toast Vec | `AppLayout::toasts(vec)` + `toast::sweep_expired` |
| `keyboard::Key::Named(Named::Escape)` match in `handle_key_event` | `snora::keyboard::dismiss_on_escape` |

### Workspace tabs (`view/workspace_tabs.rs`)

knotra already has a hand-rolled workspace tab strip (RFC-0015). This is
a peer-level view switcher exactly matching snora's `TabBar<TabId>` /
`app_tab_bar` model. The `TabBar` vocabulary is direction-aware; the
hand-rolled version is not.

### `nav_menu` in `knotra-ui`

`knotra-ui::nav_menu` (the now-unused `nav_bar` function) is made
redundant by snora's `app_header` + `render_menu` + `app_side_bar`. It
can be removed.

## What knotra correctly keeps in `knotra-ui`

Per the authoritative theme/i18n split from RFC-0018's resolved open
question #5:

- **`KnotraTheme` + `StatusColor`** — knotra's iced theme/palette. snora
  consumes this by reading `theme.extended_palette()`. No change needed;
  the theme slot is already correct.
- **i18n `Catalog` / `Locale`** — knotra's message catalog. snora has no
  equivalent; the app owns it.
- **`widget::CARD_GAP` / `CARD_RADIUS` / `CARD_PADDING`** — knotra-specific
  card layout tokens. snora does not provide card-level tokens.
- **`widget::SIDEBAR_WIDTH` / `CARD_MIN_WIDTH`** — likewise knotra-specific.

`knotra-ui` retains all of these unchanged.

## What is NOT in scope for this RFC

- **snora `SideBar` icon rail.** `knotra-ui::nav_menu` is currently dead
  code (knotra-app does not render a sidebar rail). Adding one is a
  deliberate UI decision that belongs to a future feature RFC, not here.
- **`app_breadcrumb`.** No breadcrumb concept exists in knotra today.
- **`Icon` / `lucide-icons`.** knotra uses text/emoji labels today. Icon
  migration is a separate visual decision.
- **RTL.** knotra is currently LTR-only. Enabling RTL is a separate RFC.
  This adoption wires `LayoutDirection::Ltr` correctly so RTL is additive.

## Proposed adoption plan

### Phase 1 — `snora` engine dependency + overlay re-layer

Add `snora = "0.18"` to `knotra-app/Cargo.toml`. Rewrite `app_view` in
`view/mod.rs`:

```
// Before (simplified):
let mut layers = vec![base];
if let Some(m) = modal_layer { layers.push(dim + m); }
if let Some(p) = palette_layer { layers.push(p); }
...
stack(layers)

// After:
let mut layout = AppLayout::new(base)
    .direction(LayoutDirection::Ltr)
    .on_close_modals(Message::CloseModals)
    .on_close_menus(Message::CloseMenus);

if let ActiveModal::Pull = state.active_modal {
    layout = layout.dialog(Dialog::new(pull_modal_el));
}
// knotra's command palette, shortcuts overlay, and add-project modal
// are NOT standard modal overlays (they have their own state channels);
// they remain as additional stack layers pushed after render(layout).

render(layout)
```

**Note on knotra's non-standard overlays.** knotra has four overlay types
that do not map 1:1 to snora's two modal slots (dialog / sheet):

- `ActiveModal` (Pull, Tag, Switch, Resolve, Changelog) — centred modals
  or right-docked panel. These map to `Dialog` (centred) / `Sheet` (docked).
- Command palette — a distinct overlay with its own open/close state.
  Keep as a stack layer above `render(layout)` output.
- Shortcuts overlay — likewise.
- Add-project modal — likewise.

The correct approach is to use `render(layout)` as layer 0 and continue
pushing knotra-specific overlay layers above it, unchanged.

### Phase 2 — Workspace tab bar via `app_tab_bar`

Replace `view/workspace_tabs.rs` with `app_tab_bar(TabBar { tabs, active },
&Message::SelectWorkspace, LayoutDirection::Ltr)`. This makes the tab
strip direction-aware at no additional complexity.

### Phase 3 — Toast lifecycle via `snora::toast`

knotra currently has no toast / notification display. If future work adds
transient status notifications, use `Toast<Message>` + `toast::subscription`
+ `toast::sweep_expired` via the snora framework rather than hand-rolling.
This phase has no code change today — it is a design commitment for when
toasts are added.

### Phase 4 — Remove `knotra-ui::nav_menu`

Delete the dead `nav_menu` module. It is unused by `knotra-app` and is
superseded by snora's `app_header` / `render_menu` / `app_side_bar`.

## Dependency change

```toml
# crates/knotra-app/Cargo.toml
snora = "0.18"
```

No change to `knotra-ui`. No new transitive dependencies (snora shares iced
with the existing workspace).

## Non-goals and non-decisions

- **knotra-ui does not become a thin wrapper around snora.** knotra-ui
  retains `KnotraTheme`, `StatusColor`, the i18n catalog, and layout
  tokens. These are knotra-specific and snora has no equivalent.
- **`knotra-app` does not import snora types in its domain model.** snora
  types (`Toast`, `Dialog`, etc.) appear only in view functions.
- **No hollow/unused dependency.** Every adopted surface replaces
  hand-written code. Do not add snora if only the vocabulary types would
  be used without the engine or widgets.

## Open questions

None. The snora author's design stance is authoritatively documented in
RFC-0018's resolved open question #5. The adoption scope above follows
directly from that answer.

## Sequencing

This RFC may be ordered relative to RFC-0017 (screen removal):

- RFC-0017 first makes `app_view` simpler before re-layering it.
- RFC-0019 can proceed independently if RFC-0017 is deferred.

The implementation phases above can ship in one version or split across
two (Phase 1 + 4 first; Phase 2 + 3 when relevant).
