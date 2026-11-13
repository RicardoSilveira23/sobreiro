# One more GitHub sign-in: the Copilot language server's

Issue 16, part of 1.0. Written 2026-11-13.

## Why

On Ricardo's work PC (Copilot Business, the Copilot CLI in use, no `gh`) Sobreiro has never
sent GitHub a request. Every start from 0.7.0 to 0.10.0 logs
`copilot NoSignIn env=none credential-manager=none gh=none cache=used`: the figure on the pill
is the Copilot CLI's cached one, as old as the CLI's last use.

`cmdkey /list | findstr /i copilot` on that PC prints one line:
`Target: LegacyGeneric:target=copilot-language-server/oauth-token-key`. There is no entry named
`copilot-cli`, the only name Sobreiro matches. GitHub documents `copilot-cli` as the CLI's
service name; nothing found says what writes the other entry. Its name points at the Copilot
language server that editor plug-ins run.

## Goal

Find out, with one start on that PC, whether that entry gives live numbers, and keep it as a
sign-in if it does.

## Not goals

- No new network address. The token goes only to `GET https://api.github.com/copilot_internal/user`.
- No change to how a token is held: read into memory for one request, never stored, logged,
  printed, cached or renewed.
- The flyout's sentences and the Copilot CLI's cache stay as they are.

## What would make it a failure

- Reading or keeping any Credential Manager entry other than the one named here.
- A token, a target name or an account name in the log.

## Approaches

1. **Read that entry as one more sign-in (picked by Ricardo, 2026-11-13: "Yes, build it").**
   One start on the work PC answers the two unknowns: the shape of the entry's contents and
   whether GitHub's usage address accepts its token.
2. Read `~/.copilot/config.json`, the CLI's plaintext fallback. Not known to exist on that PC,
   and a file that may hold a token in the clear is a bigger step. Kept for later, if the first
   answers `refused` or `no-token`.
3. Leave it: the cached figure only. Dropped by Ricardo's yes.

## The design

- A fourth source, tried third: the three variables, the Copilot CLI's entry, **the language
  server's entry**, then `gh`. The first token GitHub accepts wins, as today.
- The entry is asked for by name: Credential Manager is given the filter
  `copilot-language-server/oauth-token-key*`, and only a generic entry whose name starts with
  `copilot-language-server/oauth-token-key` is read. Sobreiro never lists every entry to find
  it (the Copilot CLI's entry keeps its own, older rule).
- Its contents are decoded as the CLI's are, into a buffer that is cleared before the reader
  returns: UTF-8, then UTF-16, and the first text shaped like
  a GitHub token (`gho_`, `ghu_`, `ghp_`, `github_pat_`) is the token.
- The log line gains one word: `copilot <state> env=… credential-manager=…
  language-server=… gh=…`, with the same values as the others (`ok`, `refused`, `no-budget`,
  `none`, `no-token`, `error`, `skipped`).
- A listing that Credential Manager refuses is logged as `error`, for either entry, so it
  cannot pass for "no entry" (`none`). Before, the CLI's entry logged `none` for both.
- The project rule in `CLAUDE.md` widens from "the sign-in the Copilot CLI would use" to name
  this entry too. README, SECURITY and the manual list the new source.

## Shape in Core

- `CopilotSourceKind` gains `LanguageServer`.
- A new type holds the two entry names: the filter for each and whether a target name is the
  CLI's or the language server's. A second holds how each is looked for, and what a failed
  listing means: no entry, list everything (the CLI's entry only), or a failure.
  `CredentialStore` (the native calls, in the tray app) asks them.
- The order of the four sources is built in Core, so it has a test.

## Calls made here, not by Ricardo

- The match is the exact name seen, as a prefix, not any name under `copilot-language-server/`:
  one entry is known, so one entry is read.
- Its place in the order: after the CLI's own sign-in, before `gh`.
- If the entry holds several tokens, the first is used.

## Left to the builder

Names, and the wording of the three doc passages.
