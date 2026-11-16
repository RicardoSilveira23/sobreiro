# Sobreiro

Windows tray app (formerly claude-tray) showing Claude Code's session and weekly limits and
GitHub Copilot's monthly AI credits. Plan: [docs/plan.md](docs/plan.md) (milestones at the end). The landing
page's design: [docs/design.md](docs/design.md); read it before styling anything in `site/`.

## Build rules

- Before a push, `npm run check:exposure` passes.
- The app is Rust (2.0 on), one workspace at the root: logic goes in `core/` with its tests,
  the bridge's exe in `bridge/`, the Windows shell in `shell/windows/`. Before a push:
  `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`. The compiler, the libraries and the toolkit stay pinned:
  [how it is built](docs/roadmap.md#where-it-lives-and-how-it-is-built).
- 1.x is C# / .NET 10 and ships from the branch `1.x`; its fixes go there. The C# under `src/`
  and `tests/` on `main` is 1.x's, kept for the fonts and icon the build reads until #105
  removes it; change it only for that.
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
