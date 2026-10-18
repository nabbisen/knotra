# Privacy Policy

**Effective date:** 18 October 2026

This policy covers knotra in every form it is distributed: the source code, the
archives attached to GitHub releases, and the Microsoft Store package.

## Summary

knotra does not collect information about you. It has no accounts, no analytics
or telemetry, no advertising, and no crash reporting, and it never sends data to
the developer or to anyone acting on the developer's behalf.

## Network access

knotra itself does not open network connections.

It works with your repositories by running Git (`git`) and Jujutsu (`jj`) on your
computer. When **you** start an operation that needs a remote repository —
fetching updates, Smart Pull, or pushing a release tag — those programs contact
the remote repositories you have configured, using your own Git or Jujutsu
settings and credentials.

knotra never reads, stores or transmits your credentials. Git and Jujutsu handle
them exactly as they do when you use them directly.

knotra's automatic background refresh only reads the state of your repositories
on your computer. It does not contact remote repositories.

## Information stored on your computer

knotra saves the following on your device so that it can keep working between
sessions:

- **Settings** — such as language, theme, refresh interval, the paths to your
  editor and merge tool, and dashboard preferences.
- **Workspaces** — the names you give them, and the names and folder paths of the
  projects you add to them.
- **Operation history** — which operations ran, on which projects, when, and how
  they ended, including messages from Git or Jujutsu.

This information stays on your device. knotra does not upload it anywhere.

| System  | Settings and workspaces                 | Operation history                               |
|---------|-----------------------------------------|-------------------------------------------------|
| Windows | `%APPDATA%\knotra\`                     | `%LOCALAPPDATA%\knotra\history\`                |
| macOS   | `~/Library/Application Support/knotra/` | `~/Library/Application Support/knotra/history/` |
| Linux   | `~/.config/knotra/`                     | `~/.local/share/knotra/history/`                |

On Linux, `$XDG_CONFIG_HOME` and `$XDG_DATA_HOME` are used instead when they are
set. For the Microsoft Store version, Windows may keep these files in a folder
that is private to the app.

You can remove any of this information at any time by deleting those folders.

knotra's diagnostic log is written to standard error only. knotra does not save
it to a file.

## Other programs knotra starts

knotra runs Git and Jujutsu, and — when you ask it to open a file — the editor or
merge tool you have configured. These are separate programs with their own
behaviour, governed by their own terms.

## The Microsoft Store "full trust" capability

The Microsoft Store version declares the `runFullTrust` capability. A desktop
application needs it to read your project folders, run Git and Jujutsu, and open
your editor. It does not change anything described in this policy.

## Changes to this policy

If this policy changes, the updated version will be published at the same
address with a new effective date. Every change is recorded in the project's
public history.

## Contact

For questions about this policy, open an issue at
<https://github.com/nabbisen/knotra/issues>.
