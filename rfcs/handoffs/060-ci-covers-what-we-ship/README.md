# RFC-060 — implementation handoffs

Companion execution documents for
[`rfcs/done/060-ci-covers-what-we-ship.md`](../../done/060-ci-covers-what-we-ship.md),
per [RFC 000 § Companion handoffs](../../done/000-rfc-lifecycle-policy.md).

Status is **inherited from RFC-060** — now Implemented (main: d51aac7).

**Three stages, in order.** Each is independently verifiable and separately reviewed.
Stage 1 additionally requires Task 080 to have landed.

| Handoff | Covers | Status |
|---|---|---|
| [`stage-1-advisory-gate.md`](stage-1-advisory-gate.md) | `deny.toml`, the key-presence check, `advisories.yaml` (D1, D2) | complete |
| [`stage-2-platform-matrix.md`](stage-2-platform-matrix.md) | Windows/macOS test jobs, the symlink test port (D3) | complete |
| [`stage-1-and-2-follow-up.md`](stage-1-and-2-follow-up.md) | Stage 1 F1/F2, Stage 2's `if: always()`, and the Windows re-run | complete |
| [`stage-3-release-order-and-actions.md`](stage-3-release-order-and-actions.md) | release publishes only after all builds; action majors (D4, D5) | complete |
