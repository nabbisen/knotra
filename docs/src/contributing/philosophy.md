# Design Philosophy

**Unix philosophy** — one screen, one job; one module, one responsibility.

**Accessible by Default** — text + icon on every status indicator (never colour alone); keyboard-complete; visible focus; error messages state what, where, and what next.

**Local-first** — no cloud dependency; only VCS operations cause network traffic.

**Transparency** — every VCS command executed is logged with stdout/stderr/exit-code; recovery hints include literal shell commands.

**Safety over speed** — failed freeze leaves repositories unchanged; rollback is tested as rigorously as the forward path; never overwrites existing tags.

**Simplicity over generality** — not a general Git GUI; single-repository deep features are intentionally out of scope.
