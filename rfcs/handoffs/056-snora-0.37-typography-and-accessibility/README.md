# RFC-056 — implementation handoffs

Companion execution documents for
[`rfcs/accepted/056-snora-0.37-typography-and-accessibility.md`](../../accepted/056-snora-0.37-typography-and-accessibility.md),
per [RFC 000 § Companion handoffs](../../done/000-rfc-lifecycle-policy.md).

Status is **inherited from RFC-056**. These documents have no lifecycle of their
own and do not move between state folders.

## Stages

| Stage | Handoff | Status |
|---|---|---|
| 1 — the snora 0.38 bump | *predates this folder* — `.git-exclude/tasks/developer/077-rfc-056-stage-1-snora-0.38-bump.md` | complete |
| 2 — typography roles and the 12px floor | [`stage-2-typography-roles.md`](stage-2-typography-roles.md) | issued |
| 3 — line-height | not yet drafted | — |
| 4 — pointer targets | not yet drafted | — |

## Why Stage 1 is not here

`rfcs/handoffs/` was adopted after Stage 1 shipped. Stage 1's handoff is cited by
path in its review request and in `.git-exclude/reviewed/167-*`, both of which are
immutable under this project's review-artifact conventions — moving it would
break citations in documents that cannot be edited to follow.

Its location is recorded above rather than the file being relocated. The same
reasoning applies to every handoff numbered 001–077.
