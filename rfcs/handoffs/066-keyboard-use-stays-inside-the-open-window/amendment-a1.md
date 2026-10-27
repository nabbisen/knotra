# RFC-066 Handoff — Amendment A1: R10's comparison is between two builds

Amends `implementation-handoff.md`, Stage 3, "R10, screenshots", under the owner's updated
`.git-exclude/rules/project-instructions-gui-common.md` ("GUI Evidence and Automation
Rules"). Everything else stands.

## R10, replaced

**The question for steps 1–2:** after a window is opened **with the mouse**, does any focus
outline appear? The outline is the same in any language, so English answers it.

1. Capture `03-smart-pull` and `04-freezer`, English, on the **Stage 3 baseline build** and on
   the **Stage 3 build**. Use one script for both: same demo environment, same isolated config
   (theme and locale pinned in it), same window size, same backend. Mouse input means the X11
   backend through XWayland, per the rules.
2. Compare each pair pixel by pixel. Expected: identical. Then the committed Store set stands
   unchanged. If they differ, stop and report the bounding box and both images.

**Steps 3–4 are unchanged.** The Remove project dialog over History and the shortcuts sheet,
English and Japanese, are new screens whose pictures are what I review. Capture each once.

**Isolation:** as in Task 096 Amendment A1. knotra's folders come from `XDG_CONFIG_HOME` and
`XDG_DATA_HOME` with a hard-coded `knotra` folder; isolate those two plus `XDG_STATE_HOME` and
`XDG_CACHE_HOME`.
