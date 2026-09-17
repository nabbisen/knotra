# Developer Handoff supplement — RFC-059 A1: retarget to snora 0.50

**Read with** `implementation-handoff.md` in this directory. This supplement changes **only
the target version**. Every instruction, stop condition, evidence requirement and scope
rule in the original stands.

Authority: `rfcs/accepted/059-upgrade-snora-from-0.38-to-0.49.md` § Amendments, A1
(2026-09-17). Measured at `d2727cf` before issue.

**This supplement is immutable.** Corrections after issue go in a new document.

## 1. What to substitute

| In the original | Read as |
|---|---|
| §3 manifest line `version = "0.49"` | `version = "0.50"` |
| every `0.49` / `0.49.0` — §1, §2's evidence table, §3's lockfile list, §4, §5 | `0.50` / `0.50.0` |
| §2: "each at 0.38 and at 0.49" | each at 0.38 and at **0.50** |
| §5: re-cite `Sheet` and the `body_small` claim "at 0.49.0" | at **0.50.0** |

## 2. Expected lockfile movement at 0.50.0

Measured with exactly the original's command. **Nothing else may move:**

```
Updating snora         v0.38.0 -> v0.50.0
Updating snora-core    v0.38.0 -> v0.50.0
Updating snora-design  v0.38.0 -> v0.50.0
Updating snora-style   v0.38.0 -> v0.50.0
Updating snora-widgets v0.38.0 -> v0.50.0
Removing float_next_after v1.0.0
Removing lyon v1.0.19
Removing lyon_algorithms v1.0.20
Removing lyon_geom v1.0.19
Removing lyon_path v1.0.19
Removing lyon_tessellation v1.0.20
```

Also measured at 0.50.0: compile and clippy clean, **303** tests pass, the `windows-gnu`
check passes, no `windows` entry moves.

## 3. The two re-verified claims still hold at 0.50.0

Between 0.49.0 and 0.50.0, `snora-core`, `snora-design` and `snora-style` are
byte-identical. So at 0.50.0: `Sheet` still exposes only `new`/`at`/`with_size`, its panel
border is still `background.weak`, and `body_small` is still read only by `snora-style`'s
own definitions and test module. Verify them yourself as §5 requires — do not copy this.

## 4. If you had already started at 0.49

Nothing is lost. Change the manifest line to `0.50`, run `cargo +1.91 update -p snora`
again, and compare against §2. **If the movement differs, stop and report.** Evidence
captured at 0.49 does not count for the upgraded side: E1–E3 are re-captured at 0.50.

## 5. What 0.50.0 changed, so a surprise is recognisable

Only snora's sidebar rail — button padding, snora RFC-099. knotra renders no sidebar. **Any
visible difference between 0.49 and 0.50 in knotra is unexpected: stop and report it.**
