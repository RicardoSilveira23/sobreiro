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

The window tests in `shell/windows/tests` need the foreground: run them with nothing else
running and the desktop left alone, or one fails for a reason that is not the change.

## The tray app (`shell/windows/`, `core/`)

1. One copy runs per data folder: a second launch only asks the first to show its details and
   exits. Quit the running copy from its tray menu first, and start it again when done.
2. `cargo run -p sobreiro`
3. Look at what changed on the real taskbar: the pill, its tooltip, the flyout (opens beside the
   pill, closes on Esc and on focus loss) and the tray menu.
4. A drawing change is looked at at 100%, 150% and 200% display scale and in both themes.
5. Pass: the figures show, nothing is clipped or misplaced, and the log has no new error.

## The bridge (`bridge/`)

`cargo test -p sobreiro-bridge`, then pipe a sample status line JSON into
`cargo run -p sobreiro-bridge` and read the file it writes. Pass: only the `rate_limits` fields
and a timestamp are there.

## The landing page (`site/`)

1. `npm run check`
2. `npm run dev`, then `npm run smoke -- http://localhost:8787`
3. Open `http://localhost:8787` and look at what changed at phone width (375 px) and at desktop
   width.

## Rules

- Never print or store a token while verifying.
- A failure is fixed in the code. A check is not skipped, loosened or retried until it passes.
