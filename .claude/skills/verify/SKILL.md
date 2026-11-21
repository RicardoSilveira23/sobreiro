---
name: verify
description: Run Sobreiro for real to confirm a change works - the tray app, the bridge or the landing page. Use before committing a change, after the tests pass, and whenever asked to verify, try or smoke-test a change.
---

# Verify a change in Sobreiro

Tests first, then the real thing. Run only the part the change touches, and report each command
with its real result, including anything not run.

## Always

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check:exposure
```

If a window test in `shell/windows/tests` fails, rerun that one test alone once and report both
results. Do not call a failure unrelated to the change without that.

## Never on the real data

The bridge and the tray app run on scratch folders, never on the user's data folder, token or
settings, and the installed copy is left running. Set both of these for every command in the
next two sections:

- `SOBREIRO_DATA_DIR`: the full path of a new empty folder outside the repo. A copy on its own
  data folder runs beside the installed one.
- `CLAUDE_CONFIG_DIR`: the full path of another new empty folder, so the build reads no Claude
  token and has no real `settings.json` to edit.

Both must be full paths. Any other value is ignored without an error and the run goes to the
real folders, so check both before the first command.

The build still polls with the Copilot sign-in, as the installed copy does. A change to token
handling or to a request is read through first and run only on a yes. So is any run without
`CLAUDE_CONFIG_DIR`, which only a change to the Claude polling itself needs.

## The bridge (`bridge/`)

`cargo test -p sobreiro-bridge`, then pipe a sample status line JSON, with reset times in the
future, into `cargo run -p sobreiro-bridge` and read `usage.json` in the scratch data folder.
If the file is not there, stop: the override was not taken. Pass: only the `rate_limits` fields
and a timestamp are there.

## The tray app (`shell/windows/`, `core/`)

1. Run the bridge step first, so the scratch data folder holds a reading.
2. `cargo run -p sobreiro`
3. Look at what changed on the real taskbar: the pill, its tooltip, the flyout (opens beside the
   pill, closes on Esc and on focus loss) and the tray menu. Connect and Disconnect are clicked
   only with `CLAUDE_CONFIG_DIR` set. Start with Windows is never clicked in the test copy: it
   writes the user's real sign-in entry, which no scratch folder moves. A change to it is
   verified by its tests.
4. A drawing change is looked at at 100%, 150% and 200% display scale and in both themes.
5. Pass: the figures show, nothing is clipped or misplaced, and the log has no new error.
6. Quit the test copy from its tray menu, or stop the process this run started by its id, never
   by name: the installed copy has the same name. Then delete the two scratch folders.

## The landing page (`site/`)

1. `npm run check`
2. `npm run dev` in the background, then `npm run smoke -- http://localhost:8787`. Always pass
   the address: without it the smoke check runs against the public site.
3. Open `http://localhost:8787` and look at what changed at phone width (375 px) and at desktop
   width.
4. Stop the dev server.

## Rules

- Never print or store a token while verifying.
- A failure is fixed in the code. A check is not skipped, loosened or retried until it passes.
