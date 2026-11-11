# RFC-073 handoff — Amendment A4: evidence under the owner's updated GUI rules

Issued 2026-11-11. Reason: `.git-exclude/reviewed/273-gui-test-rules-update-assessment.md`.

**It applies from now on, to stage 3 and every later stage.**

- It replaces the handoff's "use fictional repositories under `/tmp`" and A3 §4's "under `/tmp`
  or `/var/tmp`". Everything else in A3 §4 stands.
- Evidence already taken under the old wording stays acceptable. Name which files those are.

**Read first:** the "GUI Evidence and Automation Rules" and "Test Optimization Rules" in
`.git-exclude/rules/project-instructions-gui-common.md`.

## 1. Where fixtures and scripts work

- **Fixtures:** `.git-exclude/fixtures/`, built by Task 108 and recorded with checksums.
- **A session's own files** (its XDG folders, its copy of the fixtures): under
  `.git-exclude/tmp/session-<timestamp>/`.
- **Captures:** `.git-exclude/evidence/rfc-073/stage-N/`, by absolute path.

Every script keeps A3 §4's rules:

- `set -euo pipefail`;
- one absolute root, now one of the two folders above;
- a `pwd -P` check before every write;
- no relative `cd`;
- knotra's `git status --short` and `reflog -3` in the report.

**Two more git guards,** because a fixture repository now sits inside knotra's working tree:

- `export GIT_CEILING_DIRECTORIES="$ROOT"`, before any git command;
- before the first write in each fixture repository, stop unless
  `[ "$(git -C "$REPO" rev-parse --show-toplevel)" = "$REPO" ]`.

## 2. Stage 3's captures wait for Task 108

**Take stage 3's captures with Task 108's shared session script, not with a launch helper of
your own.** Stage 3's code and tests can continue before then.

- **Do not reuse** `evidence/rfc-066/run.sh` or `evidence/task-097/run.sh`. They float and resize
  a running window, which the rules forbid.

## 3. Stage 3's evidence plan

The stage's question: **is Settings shorter, and are the choices readable?**

| Question | Level | How |
|---|---|---|
| D11: Advanced is closed on every visit, and opens and closes | 1 | state tests |
| D12: each of the four number settings loads a saved value off its list as the nearest choice, ties going downward | 1 | a test per setting, as D12 asks |
| D13: each setting that needs a restart carries `settings.needs_restart` | 1 | a test on the settings the view is built from |
| A3 §1: one status sign at the three sites | 1 | tests on the badge each site builds |
| A3 §2: every field goes through the height | 1 | the guard A3 §2 asks for, seen to fail first |
| A3 §2: a taller field breaks no form | 2, or 3 | with Task 108's harness if its probe shows it can read a widget's bounds (Task 108 §3); otherwise one capture of the form with the most fields, under the worst-case conditions below |
| A3 §3: multi-line output keeps its indent, on screen and in the copied log | 1 | its test, with a two-line stderr |
| Settings is shorter, and readable | 3 | the captures below |

**Captures, the worst case plus one baseline:**

- **The conditions:** zoom 150 %, the session's fixed window size (1280 × 720, which is 853 px
  of logical width at 150 %), and the language whose Settings labels are wider.
  - Find that language by measuring, not by assuming: name the widest Settings label in each
    language and its rendered width.
  - Use the default theme, Light.
- **Three captures after:** Settings closed, Settings with Advanced open, one select open.
- **One baseline:** Settings closed, from a build of `d246460` (before stage 3), under the same conditions, from the same
  script.

## 4. Report

- the plan above, with any change to it and the reason;
- for each capture: its file, the question it answers, and the conditions (size, language, zoom,
  theme, backend);
- for each script: the guards, and knotra's status and reflog afterwards.
