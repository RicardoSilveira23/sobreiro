# Sobreiro

Windows tray app (formerly claude-tray) showing Claude Code's session and weekly limits and
GitHub Copilot's monthly AI credits. Plan: [docs/plan.md](docs/plan.md) (milestones at the end);
background: [docs/brief.md](docs/brief.md). The landing page's design:
[docs/design.md](docs/design.md); read it before styling anything in `site/`.

## Build rules

- 1.x is C# / .NET 10: projects under `src/`, tests under `tests/`, logic in `Sobreiro.Core`
  with an xUnit test. It ships from the branch `1.x`, and its fixes go there. On `main` the C#
  is the reference for the port and changes only with it.
- 2.0 is Rust, one workspace at the root: logic goes in `core/` with its tests, the bridge's
  exe in `bridge/`, the Windows shell in `shell/windows/`. Before a push: `cargo fmt --all`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. The
  compiler, the libraries and the toolkit stay pinned:
  [how it is built](docs/roadmap.md#where-it-lives-and-how-it-is-built).
- Data comes from the status line JSON via the bridge and from `GET
  https://api.anthropic.com/api/oauth/usage`, which the tray asks every 5 minutes with Claude
  Code's access token. Copilot's numbers come from `GET https://api.github.com/copilot_internal/user`,
  every 5 minutes, with the sign-in the Copilot CLI would use, and from the CLI's cache file. No
  other network calls.
- Read only the access token from `.credentials.json` (in the Claude config dir), into memory,
  per request, and send it to that one address. The same goes for a GitHub token, sent only to
  `api.github.com`. Never renew either, and never log, print, store, cache or commit any token.
- The bridge writes only `rate_limits` fields and a timestamp. Never the rest of stdin.
- Edit the user's `settings.json` only through Connect/Disconnect, with a backup, and only the
  `statusLine` key.
- The first-run migration from OpenHeadroom (`Sobreiro.Core/Migration`) never touches `settings.json`.
  A status line still running `openheadroom-bridge` counts as connected; Reconnect repoints it.
