# The Copilot credential reader: what a work PC taught

Issue 16, part of 1.0. Written 2026-11-13.

## What was looked into

On Ricardo's work PC (Copilot Business, Copilot used almost only inside Visual Studio's chat,
no `gh`) Sobreiro has never sent GitHub a request. Every start from 0.7.0 to 0.10.0 logs
`copilot NoSignIn env=none credential-manager=none gh=none cache=used`: the figure on the pill
is the one in the Copilot CLI's cache file.

What that PC showed, with target names, sizes and yes/no answers only:

- Credential Manager has no entry named `copilot-cli`. It has one Copilot entry,
  `copilot-language-server/oauth-token-key`.
- A test build that read that entry as a fourth sign-in logged `language-server=no-token`.
- The entry is 44 characters of base64, about 32 bytes: a key, not a token. The token it
  protects is kept somewhere else.
- The Copilot CLI's own `config.json` there names no signed-in account.

So on that PC no sign-in Sobreiro can use exists: the Copilot CLI is not signed in, and the
editor's sign-in is protected. The install notes already say it: for live numbers, sign in with
the Copilot CLI or `gh`.

## Decided

**Sobreiro does not read `copilot-language-server/oauth-token-key`, and does not go after the
token it protects** (Ricardo, 2026-11-13: "Yes, replace it"). The key alone is useless, and
reading the protected token would mean undoing a protection another product put there on
purpose, in a format that is undocumented and can change. The Copilot CLI's entry stays the
only one read in Credential Manager; a test pins that the other name is never taken for it.

## Kept from the attempt

Three things that stand without the fourth source:

1. **A listing Credential Manager refuses is an error, not "nothing found".** The reader used
   to return no tokens for any failure, so the log said `credential-manager=none` both for "no
   entry" and for "Windows refused". It now throws on a refusal and the poll logs `error`. On
   the PC above that difference was the first thing in doubt.
2. **Decoded credential text is cleared.** Decoding made two strings of an entry's whole
   contents, which cannot be cleared; only the byte copy was zeroed. It now decodes into a
   rented buffer, searches it in place, makes one string (the token, if there is one) and
   clears the buffer.
3. **The listing decision is in Core, with tests.** When nothing is named `copilot-cli*`, every
   entry is listed and matched, but only on a PC where the Copilot CLI has left its folder.
   That decision was inline in the tray app's native reader. Which two folders count as "the
   CLI has been used" is still checked in the tray app, without a test.

## Not goals

- No new network call, no new place a token is read from, and no change to the three sources
  or their order. The log line keeps its three words.
- The flyout's sentences stay as they are.

## Calls made here, not by Ricardo

- A refused listing throws a fixed-text exception rather than returning a new kind of result:
  the poll already turns a throwing source into `error` and never logs the exception.
- An empty Credential Manager (a listing of everything that finds nothing) is "no entry", not
  a failure.

## Open, for later

My reading, unconfirmed: on PCs like that one the cache file is refreshed by the editor's
Copilot, not by the Copilot CLI, which is why the cached figure keeps up with use. If so, the
flyout's "From the Copilot CLI" and "No GitHub sign-in Sobreiro can use" read harsher than the
state deserves. No change is proposed here.
