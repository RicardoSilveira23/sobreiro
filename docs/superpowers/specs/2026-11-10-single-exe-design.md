# One Sobreiro.exe, no zip (design)

2026-11-10. Asked for by Ricardo after installing 0.7.0 on his work laptop.

## Why

The 0.7.0 release is `Sobreiro-0.7.0-win-x64.zip`, which unzips to a `Sobreiro\` folder holding
`Sobreiro.exe` (74 MB), `sobreiro-bridge.exe` (17 MB) and `Fonts\` (the two font licences).
Ricardo wants to download one exe and run it.

## Goals

- The release asset is one file, `Sobreiro.exe`. It runs the tray, connects the status line and
  feeds usage through the bridge, with nothing beside it.
- The status line stays as fast as today: the bridge runs on every refresh (about 61 ms per run
  today, measured through Git Bash).
- Installs from the zip keep working, and Reconnect moves them to the new bridge.

## Non-goals

- Native AOT for the bridge (plan.md's original idea): it needs the MSVC linker, which this PC
  doesn't have. It can come later on top of this design.
- An installer, auto-update, signing.

## Approaches

1. **Sobreiro.exe carries the bridge and copies it into the data folder (picked).** The status
   line runs `%LOCALAPPDATA%\Sobreiro\sobreiro-bridge.exe`, the same console program as today.
   Start-up is unchanged, and the path stays put wherever the user keeps or moves `Sobreiro.exe`.
   Costs: the exe grows by the compressed bridge, and the app writes a program into its data
   folder.
2. **A bridge mode, `Sobreiro.exe --bridge`.** No second file anywhere. But `Sobreiro.exe` is a
   windowed program, so starting it on every refresh can flash Windows' busy cursor; each run
   opens a 74 MB compressed bundle and JITs code the bridge build has precompiled; and the status
   line points at wherever the download sits, so moving or replacing the exe breaks it.
3. **Keep two files, ship them side by side without a zip.** Two downloads; not what was asked.

## Design

- **Build.** `scripts/package.ps1` publishes the bridge as today, then publishes the tray with
  `-p:BridgeExe=<path>`, which embeds it as the resource `sobreiro-bridge.exe`. Output:
  `dist/Sobreiro.exe` only. A build without the property has no embedded bridge and finds one
  beside the exe or in `..\bridge\` as today, so the README's two dev publish commands and
  Ricardo's own status line keep working.
- **Install (Core, tested).** Copy the embedded bridge to `<data folder>\sobreiro-bridge.exe`
  when it is missing or its bytes differ: write a temp file beside it, then replace, retrying a
  few times because a bridge that is running can't be overwritten. It runs on Connect (after the
  user says Yes, before `settings.json` is written) and at tray start when the status line already runs that path, so a newer
  `Sobreiro.exe` updates the bridge and a deleted copy comes back. Disconnect leaves the copy in
  the data folder with `usage.json`. `settings.json` is still edited only by Connect/Disconnect,
  with a backup, and only `statusLine`.
- **Connection state.** Connected means the status line runs this Sobreiro's bridge (the
  installed copy, or in a dev build the one found beside it). A status line running another
  `sobreiro-bridge` (an old zip folder) or `openheadroom-bridge` gets Reconnect and Disconnect in
  the menu, as the OpenHeadroom case does today; until then the old bridge keeps writing the same
  `usage.json`. The confirm box says which bridge it moves from. The tooltip's legacy line stays
  OpenHeadroom-only, so `TrayController.cs` is untouched.
- **Fonts.** The `.ttf` files are already WPF resources inside the exe; only the two OFL licence
  texts sat beside it. They become resources too, shown from About Sobreiro (a button that opens
  a read-only text window). The OFL FAQ accepts an About box.
- **Release.** The asset is `Sobreiro.exe`, without the version: a browser saves it under that
  name, and `releases/latest/download/Sobreiro.exe` stays a stable link. The version is in the
  release title and in About. CI keeps the exe as its artifact. Version 0.8.0.
- **Docs.** README install and upgrade steps (download, Unblock/SmartScreen, keep it anywhere,
  run, Connect; from the zip: Reconnect, then delete the old folder), what Sobreiro writes (the
  bridge copy), the licence section; the site's home page and manual; `docs/plan.md` progress.
- **Measure.** Bridge start-up from the installed copy against today's 61 ms, and the size of
  `Sobreiro.exe`.

## Left to the builder

Resource and class names, retry counts, the licence window's look (plain, in the About box's
voice), README wording.
