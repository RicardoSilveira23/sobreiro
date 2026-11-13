# Security

## Reporting a problem

Report it privately through GitHub's
[form](https://github.com/RicardoSilveira23/sobreiro/security/advisories/new), never in a
public issue. I'm one person, so I reply when I can, usually within a week.

## Supported versions

Only the latest release gets fixes.

## What matters most

Sobreiro reads two sign-ins it doesn't own, so these matter most:

- Claude Code's access token is read from `.credentials.json` into memory for each request and
  sent only to `api.anthropic.com`. It is never renewed, logged or stored.
- A GitHub token is looked for in this order: the `COPILOT_GITHUB_TOKEN`, `GH_TOKEN` and
  `GITHUB_TOKEN` variables, the Copilot CLI's sign-in in Windows Credential Manager (finding it
  may mean listing the Credential Manager entries), the entry
  `copilot-language-server/oauth-token-key` there (asked for by its name), then `gh auth token`.
  It is sent only to `api.github.com`, and is never renewed, logged or stored.
- Those two addresses, every 5 minutes and when the details open on a reading over a minute
  old, are its only network calls.
- Connect to Claude Code changes one key, `statusLine`, in Claude Code's `settings.json`, after
  asking and with a backup.
- The bridge writes only the `rate_limits` fields and a timestamp to `usage.json`.

Problems with the addresses themselves belong to Anthropic or GitHub. The README's section
[What it reads, what it sends](README.md#what-it-reads-what-it-sends) has the detail.
