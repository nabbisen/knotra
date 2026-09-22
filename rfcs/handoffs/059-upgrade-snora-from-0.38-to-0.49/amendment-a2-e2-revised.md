# Developer Handoff supplement — RFC-059 A2: E2 re-aimed at the dim

**Read with** `implementation-handoff.md` and `amendment-a1-retarget-to-0.50.md`. This
supplement **replaces §2's E2 only**. E1 and E3 are unchanged and already captured; do not
recapture them at 0.38.

Authority: `rfcs/accepted/059-upgrade-snora-from-0.38-to-0.49.md` § Amendments, A2.
Ruling on your report: `.git-exclude/reviewed/189-review-088-e2-prediction-was-wrong.md`.

**This supplement is immutable.** Corrections after issue go in a new document.

## 1. Your report was right; the instruction was wrong

E2 told you to scroll **over the dialog**. snora's scroll change is in the **dim** — the
dark area *outside* the dialog card. Over the dialog, knotra's own widgets sit in front of
it: a `text_input` claims the cursor whenever it is over the field, which stops every lower
layer from seeing the wheel at any snora version.

Nothing you captured is wasted. `038-11`'s tooling check is what made this resolvable.

## 2. E2, revised

**Cursor on the dim, well outside the dialog card**, over the dashboard list behind it — the
same kind of point as E3's click, e.g. near the left edge beside the project rows. Send the
same wheel events you used before.

| Version | Expected |
|---|---|
| **0.38** | the **dashboard list behind scrolls**; the dialog stays open and unmoved |
| **0.50** | the **dialog closes** — snora 0.50's dim publishes the close message on scroll; the list behind does not scroll |

Capture before/after at each version, as with E1 and E3.

**The 0.50 row is a behaviour change, not a repair**, and I did not state it before: scroll
over the dim now dismisses the dialog. If it is confirmed, say so — D4's CHANGELOG entry
should mention it, because a user can now dismiss a dialog with the wheel.

## 3. If the revised E2 still does not reproduce at 0.38

**Record it and proceed with the upgrade.** Write it up as "no observable difference for
knotra", with the captures.

Do **not** investigate further, and do not adjust the claim to fit. E1 is the defect users
actually hit, it reproduced exactly as predicted, and the upgrade's justification does not
rest on E2. A second non-reproduction is a finding about my reading of snora's rendering,
not a blocker — and it is worth more written down plainly than explained away.

## 4. One thing left unexplained, and deliberately not yours to chase

Your attempts 2 and 3 put the cursor in the gap between the dialog's title row and its
`Name` label, where by my reading no widget claims the cursor — so the wheel should have
reached the list behind, and did not. **I cannot account for that**, it does not bear on the
upgrade, and it is recorded in A2 as unexplained. Do not spend time on it.

## 5. Everything else is unchanged

§3's bump, §4's Windows checks, §5's two comments, §6's CHANGELOG entry, §7's scope, §8's
gates and §9's report-back list all stand as written, with A1's version substitutions.
