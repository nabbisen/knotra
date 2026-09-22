# Developer Handoff — RFC-060 Stages 1 & 2: follow-up, then the re-run

RFC: `rfcs/accepted/060-ci-covers-what-we-ship.md`. Carries the findings from
`.git-exclude/reviewed/191-rfc-060-stage-1-advisory-gate-review.md` (F1, F2) and
`.git-exclude/reviewed/192-rfc-060-stage-2-windows-defect-ruling.md` (§5), and the re-run
that finally measures Tasks 081–083 on Windows.

One document because the three steps are ordered: both workflows must be correct before the
run that judges them is worth anything.

Baseline: `ff9a77d`, **309** tests (229 / 31 / 49), all gates green — re-run by me.
**This handoff is immutable.**

**Two commits, one per stage** — keep them separate so each stage's review stays about one
stage. The runs come after both.

## 1. Stage 1, F1 — the scan step uses a toolchain the runner does not have

`.github/workflows/advisories.yaml` runs `cargo +1.91 deny …` but never installs 1.91, and
defines no `TOOLCHAIN`. GitHub's runners ship rustup with a stable default, and rustup does
not auto-install a toolchain named with `+`.

Add what every other workflow already does — a top-level `env: TOOLCHAIN: "1.91"` and, before
the scan:

```yaml
- name: Install Rust toolchain
  run: rustup toolchain install "$TOOLCHAIN" --profile minimal
```

Keep the `+1.91` invocation, so the graph is resolved by the same toolchain the gates use.
`cargo install cargo-deny` on the runner's default stable is fine as it stands.

## 2. Stage 1, F2 — run it once

A workflow that has never executed is unverified, and that is how F1 survived every local
probe. **That gap was mine**, not yours: Stage 1 §7 never asked for a run.

`workflow_dispatch` `advisories.yaml` on `ci/rfc-060` after §1. Report the run URL and
confirm **both** steps executed and passed — the key-presence check and the scan.

## 3. Stage 2 — measure every crate on Windows, in one run

`cargo test -p knotra` failed, so `knotra-ui` and `knotra-vcs` never ran on Windows at all.
Their behaviour there is still unmeasured, and a second red run would repeat that.

Add **`if: always()`** to the second and third test steps in the matrix job. Later steps then
run, and **the job still fails** — which is the difference from `continue-on-error`, and why
that remains forbidden. Nothing else in the job changes.

## 4. The re-run — what Windows must now report

`workflow_dispatch` `ci.yaml` on `ci/rfc-060` once §1–§3 are in.

| Platform | Expected |
|---|---|
| Linux | **309** — 229 / 31 / 49 |
| macOS | **309** — no test is gated on a Unix platform |
| Windows | **302** — knotra **222**, `knotra-ui` 31, `knotra-vcs` 49 |

Windows's 222 is 229 − **7** `#[cfg(unix)]` tests: the same seven you named in request `091`
(six in `atomic_write.rs`, one in `persistence.rs`). I re-counted them at this baseline; the
six tests added by Tasks 081–083 are ungated, so the gated set has not changed. **If the
gated count differs from seven, stop and report** — that means something new was gated
without being noticed.

Three results matter beyond the totals, and each must be named individually in your report:

- **`delete_workspace_failure_keeps_state_and_dialog` now passes on Windows.** It is the test
  that found the defect; its passing there is what closes Task 081.
- **`delete_workspace_file_blocked_by_a_plain_file_is_an_error`**,
  **`load_workspaces_blocked_by_a_plain_file_reports_an_error`**,
  **`load_recent_logs_reports_a_directory_blocked_by_a_plain_file`** and
  **`load_config_with_a_blocked_config_directory_reports_and_uses_defaults`** pass on Windows.
  On Linux these reach their assertions through the *pre-existing* error path; **Windows is
  the only place the new classification is actually exercised.**

**`knotra-ui` and `knotra-vcs` have never run on Windows.** A failure there is a **new
finding**: report it with its log and stop. Do not fix it in this handoff, do not gate a test
away, do not mark anything `continue-on-error`. If everything passes, say so — that is a
result worth recording, not a non-event.

## 5. Out of scope

- **`release.yaml`** — Stage 3. Your uncommitted draft stays uncommitted; **neither commit
  here may include it**.
- **Action version bumps** — Stage 3, and it now covers **four** workflow files, including
  `advisories.yaml`.
- **Any product fix** the re-run reveals; **any source file at all**.
- **Deleting `ci/rfc-060`** — it is still needed for Stage 3's own dispatched run.

## 6. Verification

Local gates unchanged: **309** (229 / 31 / 49), suppression map **five**, dead-code probe
**one** line, `git diff --check ff9a77d..HEAD` clean. Across both commits,
`git diff --stat` touches **`.github/workflows/advisories.yaml`** and
**`.github/workflows/ci.yaml`** only.

## 7. What to report back

One review request covering both commits:

- the `advisories.yaml` diff, and the dispatched advisories run URL with both steps' results;
- the `ci.yaml` diff (the `if: always()` change);
- the re-run URL and **per-platform counts**, with the Windows gated-test list;
- **each of the five named tests' Windows result**, individually (§4);
- whether `knotra-ui` and `knotra-vcs` passed on Windows — and if not, the failure and its log;
- confirmation `release.yaml` is in neither commit;
- gate output, gate five in the range form.
