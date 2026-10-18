# Developer Handoff addendum - RFC-063 Amendment A1

Corrects `implementation-handoff.md` §2 step 3 per RFC-063 Amendment A1. The rest of that
handoff stands.

**Put the version guard in `notes`, not `crates`.** It becomes the first step of `notes`,
with `if: github.event_name == 'push'`, before the CHANGELOG extraction. Its logic is
unchanged from §2 step 3, and its §5 checks are unchanged. Install the pinned toolchain in
`notes` the way `build` does, for `cargo metadata`.

`crates` then runs checkout, toolchain and Linux dependencies, authentication and the
per-crate publish, with **no guard step**. If you already implemented the guard in `crates`,
move it; do not leave a copy.
