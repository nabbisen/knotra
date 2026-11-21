# RFC-073 handoff — Amendment A7: stage 4's loose ends, with stage 5

Issued 2026-11-21. Reasons:

- `.git-exclude/reviewed/287-rfc-073-a5-a6-stage-4-review.md` §4;
- `.git-exclude/reviewed/288-task-114-review.md` §2.

**Done in stage 5, as their own commit before stage 5's own.** The evidence rules are A4's, Task
108's session and Task 114's: fixtures shown under `/tmp/knotra-demo`, and every path, name and
time on screen read.

## 1. The detail panel's actions never cut a label (Review 288 §2)

Stage 4 renamed "Fetch" to "Check for updates".

- In the detail panel, "Refresh" and "Check for updates" share one row of fixed-height
  (`BUTTON_HEIGHT`) buttons (`detail_panel.rs`, `--- Actions ---`).
- At 1280 × 900, English, zoom 100 %, the label wraps onto two lines inside its button, and
  "updates" is cut off at the bottom. See `.git-exclude/evidence/task-114/detail-panel-ahead-behind-en-after.png`.

**The fix:**

- **The actions row wraps** (`row![…].wrap()`, as the chip row does since Task 110). A button that
  does not fit goes to the next line.
- **A button's label never wraps inside the button:** `Wrapping::None` on the label text, so the
  button takes the label's width.
  - Apply it in `centered_button_label`, so every button gets it, if the label guard shows no
    other control breaking. Otherwise apply it in the detail panel only, and **report** each
    control that broke.
- **The label guard (Task 110) also checks the detail panel's three buttons:**
  - Refresh, Check for updates, Remove from workspace;
  - both languages, zoom 100 % and 150 %;
  - each window width at which the panel is drawn.
  - **Seen to fail first,** on today's code.

## 2. Four texts that still say "check" (Review 287 §4)

**The owner approved all four on 2026-11-21, exactly as below. Apply them.**

| Key | Text |
|---|---|
| `plain.get_latest.note_unsaved` | "Has unsaved work — only checks for updates by default." / 「未保存の作業あり — 既定では更新の確認のみ」 |
| `plain.empty.welcome_body` | English: "Add your first project folder. knotra will read it and show what needs your attention." Japanese unchanged |
| `plain.release.blocker_unreadable` | English: "Couldn't read this project. Make sure its folder still exists." Japanese unchanged |
| `plain.switch.dirty_hint` | English: "This project has unsaved work. Review it before changing work area." Japanese unchanged |

**Apply these only once the owner has approved them,** exactly as approved.

- Then remove the four exemptions from `check_means_only_checking_for_updates`.
- If the owner has not decided when you reach this section, do §1, and report §2 as waiting.
- Stage 4's CHANGELOG line ("'Check' now always means checking for updates") is not true until
  §2 is done. If the owner declines, I will reword that line.

## 3. Evidence plan

| Question | Level | How |
|---|---|---|
| No detail-panel label is cut, at any zoom, width or language | 2 | the guard, failing first |
| The wrapped actions read well | 3 | **one** capture: the detail panel at zoom 150 %, the wider language; one baseline under the same conditions |
| §2's texts, if made | 1 | the catalog diff, and the test with its exemptions gone |

## 4. CHANGELOG

§1 adds no entry: it is part of stage 4's renaming. §2 adds none either: stage 4's entry
covers it.
