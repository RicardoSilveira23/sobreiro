# Review checklist

What to check on a change, beyond the tests. Each item has a trigger that can be read from the
diff: check only the items whose trigger matches. A bug that got past review adds one line here.

| When the diff touches | Check |
| --- | --- |
| Logic in `core/` | The behaviour has a test in `core/`, and `cargo test --workspace` passes. |
| A file format `core/` reads or writes | Files written by earlier releases still load, and a test holds a sample of the old format. |
| Reading a token, or any HTTP request | The token is read into memory per request and sent to its one address. It is never logged, stored or cached, and no new network call is added. |
| `bridge/` | The output is only the `rate_limits` fields and a timestamp. Nothing else from stdin is written anywhere. |
| Code that writes the user's `settings.json` | Only through Connect/Disconnect, with a backup, and only the `statusLine` key. |
| Polling, timers or a call to an external program | Polls start in the event loop, and every external call has a timeout. |
| Drawing in `shell/windows/` (pill, tooltip, flyout, menu) | Looked at on a real taskbar at 100%, 150% and 200% scale, in both themes, with the taskbar moved. The flyout closes on Esc and on focus loss. |
| Error handling or log lines | The log says what failed, without a token or a response body. |
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` | Versions stay pinned, and a new dependency has a stated reason. |
| `site/` | The change follows `docs/design.md`. |
| Any text or new file | `npm run check:exposure` passes. |
