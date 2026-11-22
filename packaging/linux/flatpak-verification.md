# Flatpak verification checklist

RFC-068. This holds no manifest content — it tells you what to check in **your own**
Flatpak build, and what you should see. Run each step inside the sandbox (for example,
`flatpak run` your own build, or a shell inside it), not on the host.

1. **Your own Git settings apply.** Open a project folder anywhere under your home
   folder. Its status in knotra must match what `git status` shows for the same folder
   in an ordinary terminal, on the host.
2. **Your own `~/.config/git/ignore` applies.** Add a pattern there that matches a file
   only you have (not already in the project's own `.gitignore`). Create that file in a
   project knotra has open. It must **not** appear as a changed or untracked file.
3. **Check for updates reaches the network, as your own `git`/`jj` would.** Try it on a
   project whose remote is over SSH, and on one over HTTPS with a credential helper
   configured. Both must succeed with no prompt you would not also see running the same
   command yourself in a terminal on the host.
4. **A release point's tag is yours.** Save a release point on a project. Open the new
   tag (for example `git show <tag>` on the host) and confirm the tagger name and email
   match your own `user.name`/`user.email`, not a sandbox default.
5. **"Open in editor" opens your own editor.** With an external editor configured in
   Settings, open a file from knotra. The program that opens must be the one you
   configured, not a bundled fallback.
6. **Settings match the tar.gz build.** If you have used knotra's `.tar.gz` build
   before, your workspaces and settings must appear unchanged in the Flatpak build —
   same workspaces, same configuration. (If this is a first install, skip this step.)
7. **No network permission, same result.** With network access removed from the
   sandbox (however your own manifest grants or withholds `--share=network`), steps 1–5
   must still all work. knotra's own code never needs the network directly — only
   `git`/`jj` do, running on the host.
8. **The window shows knotra's icon.** In your dock or taskbar, the running window
   must show knotra's icon, not a generic placeholder.

Anything that does not match what is described above is worth reporting back, with
which step failed and what you saw instead.
