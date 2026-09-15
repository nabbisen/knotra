# Developer Handoff — RFC-059: upgrade snora from 0.38 to 0.49

Issued per `.git-exclude/roles/high-capability-model-operating-instructions.md` §5.
RFC: `rfcs/accepted/059-upgrade-snora-from-0.38-to-0.49.md`,
**accepted 2026-09-15 (project owner)**.

Baseline: `feac8cf`, **303** tests, all gates green — re-run by me at `feac8cf` today.

**Everything factual below verified at `feac8cf`, and against snora 0.49.0 in a scratch
copy of the tree.** Your working tree has not been touched.

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. What you are shipping, and what you are proving

Code-wise this is small: one manifest line, the lockfile, two comments, one CHANGELOG
entry. **The substance is proof.** The upgrade fixes two pointer defects that no test in
this repository can observe, so the fix counts only once you have *seen* both defects at
0.38 and seen them gone at 0.49.

**Order matters: capture the 0.38 evidence before you bump anything.** Once the lockfile
moves, the defect is gone from your build.

## 1. The two defects — what the source says you will see

knotra wires `on_close_modals` (`view.rs`, `app_view`), so clicking the dim closes a
dialog. At 0.38.0 snora's dialog content is not `opaque` and the dim has no scroll
handler. At 0.49.0 both are fixed (`center(opaque(content))`;
`mouse_area(dim).on_press(..).on_scroll(..)`; the dim wrapped in `opaque`).

Predicted at **0.38**:

1. clicking a **non-interactive area inside a dialog** (padding, plain text) **closes it**,
   losing anything typed;
2. **scrolling over an open dialog scrolls the screen behind it**.

Predicted at **0.49**: neither happens. Clicking the dim *outside* the dialog still closes
it at both versions.

**This is a prediction from source, not an observation.** If 0.38 does **not** reproduce
either defect, **stop and report** — it would mean my reading of snora's rendering is
wrong, and that matters more than the upgrade.

This is **not** a click-through-to-a-guarded-control bug for knotra. That only occurs
without a close handler on the dim, and knotra has one. Do not report it as such.

## 2. Evidence (RFC-059 D2 / R4)

Follow `.git-exclude/reference/002-keyboard-evidence-runbook.md`. The sections you need, by
name:

- **"Authorisation (owner, 2026-08-02)"** — the standing allowance, and the isolated
  `XDG_CONFIG_HOME` requirement. This is the owner's live desktop.
- **"Pointer input has the same `--window` problem as keyboard" → "The fix"** — bare,
  absolute-coordinate `xdotool mousemove` / `click`. No `--window`.
- **"MANDATORY: use niri's own IPC to focus and to capture"** — focus and screenshots
  through the compositor.
- **"Capture timing: a screenshot is not proof of application state"**.
- **"Troubleshooting: `wgpu` Validation Error / Invalid surface at startup"** if the
  binary will not start.

Three observations, **each at 0.38 and at 0.49** — six captures:

| # | Action | 0.38 expected | 0.49 expected |
|---|---|---|---|
| E1 | open a dialog, type into it, click a non-interactive area **inside** it | dialog closes, text lost | stays open, text intact |
| E2 | open a dialog over scrollable content, scroll the wheel over the dialog | content behind scrolls | nothing behind moves |
| E3 | click the dim **outside** the dialog | dialog closes | dialog closes |

Choose any of the five `Dialog` surfaces; one with a text field makes E1's consequence
visible. **Name the dialog you used.** E2 needs enough content behind the dialog to scroll
— set up the isolated workspace so it does, and say what you scrolled.

**Synthetic scroll is not documented in the runbook.** `xdotool click 4` / `click 5` is the
usual wheel event, but nobody has verified it reaches knotra in this environment. If it
does not, **report the tooling gap** and capture E1 and E3 anyway. Do not substitute a
different observation and call it E2.

Store captures under `.git-exclude/evidence/rfc-059/`, alongside the existing per-RFC
evidence folders. They are not committed.

## 3. The bump (D1 / R1)

`Cargo.toml` line 29:

```toml
snora                       = { version = "0.49", features = ["design", "lucide-icons"] }
```

then:

```
cargo +1.91 update -p snora
```

I ran exactly this against a copy of `feac8cf`. Expected movement, and **nothing else**:

```
Updating snora         v0.38.0 -> v0.49.0
Updating snora-core    v0.38.0 -> v0.49.0
Updating snora-design  v0.38.0 -> v0.49.0
Updating snora-style   v0.38.0 -> v0.49.0
Updating snora-widgets v0.38.0 -> v0.49.0
Removing float_next_after v1.0.0
Removing lyon v1.0.19
Removing lyon_algorithms v1.0.20
Removing lyon_geom v1.0.19
Removing lyon_path v1.0.19
Removing lyon_tessellation v1.0.20
```

The six removals are graphics packages from iced's `canvas` path, which snora stopped
enabling in 0.42.0. knotra never used `canvas`.

**If cargo moves any package not listed, stop and report before committing.** Read the
result back from `Cargo.lock`, not from cargo's output — Task 079 showed the lockfile can
change in ways the output does not print. A dropped dependency edge inside a listed
package is a consequence of the version change, not a surprise; a package added, removed,
or re-versioned outside the list is.

**No source change should be needed to compile.** I checked: zero errors, zero clippy
warnings, 303 tests passing. If your build needs a code change, **stop and report** (R2).

## 4. Windows (D5 / R5)

We ship Windows and CI does not build it. Two checks:

1. **No `windows` package version decreases in `Cargo.lock`.** At baseline there are two
   entries, `0.58.0` and `0.62.2`; `gpu-allocator` is `0.27.0`, `wgpu-hal` `27.0.4`. I saw
   none of them move.
2. `cargo +1.91 check --workspace --target x86_64-pc-windows-gnu` passes. The target is
   installed. I ran it at baseline and at 0.49; both passed. The release builds
   `x86_64-pc-windows-msvc`, which is not installed here — `gnu` compiles the same Rust in
   the `windows` crates, which is where the known break lives.

## 5. Two comments pinned to 0.38.0 (D3 / R6)

Both are **current claims**, so both must be re-verified at 0.49.0 and re-cited **by
symbol and version, with no line numbers** — line numbers are what made them stale.

**`crates/knotra-app/src/view.rs`, the `ActiveModal::Resolve` arm.** Cites
`snora-core-0.38.0/src/overlay.rs:206-227` for `Sheet` having only
`new`/`at`/`with_size`. At 0.49.0 that is still true — no style hook — but at lines
214–232. Re-verify and re-cite. The 1.29–1.35:1 edge figure in the same comment stays:
`sheet.rs`'s panel style is unchanged, and so are the presets and theme derivation it
reads.

**`crates/knotra-ui/src/theme.rs`, `with_knotra_typography`'s doc comment.** Says the
`body_small` override was "verified against the 0.38.0 source of **all four** snora
crates". **snora has five crates** — `snora`, `snora-core`, `snora-design`, `snora-style`,
`snora-widgets` — at both 0.38.0 and 0.49.0. Re-verify across all five at 0.49.0 that
snora's own rendering never reads `body_small`, and state what you checked. I found
readers only in `snora-style`'s `text.rs`: its own definitions, and its
`#[cfg(test)] mod tests`. Confirm rather than copy that.

**If a re-verified claim does not hold at 0.49, stop and report** rather than rewording
the comment to match.

**Leave historical records alone**: `notice.rs`'s "re-measured after the snora 0.38 bump"
and `theme.rs`'s RFC-056 Stage 1 figures describe what happened then, and are still true
as history.

## 6. CHANGELOG (D4 / R7)

One entry under `## [Unreleased]`, in user terms, in the file's existing style — a
`### Fixed — ...` heading and short prose, like 0.28.0's entries. It says: clicking inside
a dialog no longer closes it, and scrolling over a dialog no longer scrolls the screen
behind. Wording is yours.

Do not mention snora version numbers as the headline. Users experience the fix, not the
dependency. One closing sentence naming the upgrade is fine.

## 7. Out of scope

- **Any test.** Nothing in the suite can observe pointer routing between overlay layers.
  A test that asserts snora's version, or that `opaque` is called, would pass without
  proving the fix. The evidence is the proof. Test count stays **303**.
- **Overlay keyboard focus, F6, themes, CI.** Each has its own RFC.
- **Any visual change.** None is expected. If you see one at 0.49 outside the three
  observations, **report it** — that contradicts the source diff in RFC-059.
- **iced or `lucide-icons` upgrades**; `rust-version` stays **1.88** (R8).
- **knotra's own version or a release.** The owner's call.
- **`crates/knotra-vcs`** — zero lines.
- **The `lru` advisory.** It comes through iced, not snora.

Files you should touch: `Cargo.toml`, `Cargo.lock`, `crates/knotra-app/src/view.rs`
(comment), `crates/knotra-ui/src/theme.rs` (comment), `CHANGELOG.md`. Nothing else.

## 8. Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check feac8cf..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
cargo +1.91 check --workspace --target x86_64-pc-windows-gnu
```

Expect **303** exactly (223 / 31 / 49), suppression map **five**, dead-code probe **one**
line, Windows check passing.

## 9. What to report back

A review request, paths relative to the project root, stating:

- **the lockfile movement**, read back from `Cargo.lock`, against §3's list;
- **E1–E3 at both versions**: the dialog used, what you scrolled, and where the captures
  are — or, for E2, the tooling gap if synthetic scroll did not work;
- whether 0.38 reproduced both defects as predicted (§1's stop condition);
- the Windows lockfile check and the `windows-gnu` result;
- **both re-verified comments, quoted**, and what you checked across the five crates;
- the CHANGELOG entry, quoted;
- confirmation that no file outside §7's list changed;
- gate output, gate five in the range form.
