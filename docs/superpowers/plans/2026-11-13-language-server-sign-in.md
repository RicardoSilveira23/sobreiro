# The language server's sign-in: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Sobreiro also tries the Credential Manager entry `copilot-language-server/oauth-token-key` as a GitHub sign-in, third of four, and says how it went in the log line.

**Architecture:** Core names the two Credential Manager entries and builds the order of the sources; the tray app's `CredentialStore` does the native listing and asks Core which names to keep. `CopilotPoll` is unchanged except for the log line.

**Tech Stack:** C# on .NET 10, xUnit, P/Invoke (`CredEnumerateW`).

**Spec:** `docs/superpowers/specs/2026-11-13-language-server-sign-in-design.md`

## Global Constraints

- The token goes only to `GET https://api.github.com/copilot_internal/user`. No new network call.
- A token is read into memory for one request and never stored, logged, printed, cached or renewed. Target names and account names are never logged or returned. Each blob copy is zeroed after use.
- Only a generic entry whose target name starts with `copilot-language-server/oauth-token-key` (ignoring case) is read for the new source, asked for with the filter `copilot-language-server/oauth-token-key*`. No listing of every entry for it.
- The Copilot CLI's entry keeps today's rule: the filter `copilot-cli*`, then, only where the CLI's folder exists, every entry with `copilot-cli` anywhere in its name.
- Order: `Env`, `CredentialManager`, `LanguageServer`, `Gh`.
- Log line: `copilot <state> env=<v> credential-manager=<v> language-server=<v> gh=<v>`.

## Review Focus

1. A PC with both entries: the CLI's accepted token must stop the chain before the language server's is read (Task 1, `An_accepted_cli_sign_in_leaves_the_language_server_unread`).
2. The language server's token refused by GitHub: the chain goes on to `gh`, and the state is `Refused` only if nothing is accepted (Task 1, `A_refused_language_server_sign_in_goes_on_to_gh`).
3. An entry that exists but holds no recognisable token: `language-server=no-token`, state `NoSignIn` (Task 1).
4. A name that merely resembles the entry (`copilot-language-server-x/…`, `my-copilot-language-server/oauth-token-key`) is not read; the CLI rule does not pick the language server's entry up, nor the reverse (Task 1, `CopilotCredentialNameTests`).
5. The native reader failing for the new source must not break the CLI's or `gh`'s turn (the poll already turns a throw into `error`; Task 2 keeps the reader from throwing on a failed listing).

---

### Task 1: Core: the names, the order and the log line

**Files:**
- Create: `src/Sobreiro.Core/Copilot/CopilotCredentialName.cs`, `tests/Sobreiro.Tests/CopilotCredentialNameTests.cs`
- Modify: `src/Sobreiro.Core/Copilot/CopilotToken.cs` (the enum; `CopilotSignIn.InOrder`), `src/Sobreiro.Core/Copilot/CopilotPoll.cs` (the log line)
- Modify: `tests/Sobreiro.Tests/CopilotPollTests.cs`, `tests/Sobreiro.Tests/CopilotProviderTests.cs` (log strings)

**Interfaces (produces):**
- `enum CopilotSourceKind { Env, CredentialManager, LanguageServer, Gh }`
- `static class CopilotCredentialName` with `const string CliFilter = "copilot-cli*"`, `const string LanguageServerFilter = "copilot-language-server/oauth-token-key*"`, `static bool IsCli(string? targetName)`, `static bool IsLanguageServer(string? targetName)`
- `static IReadOnlyList<CopilotSignIn> CopilotSignIn.InOrder(Func<TokenSearch> env, Func<TokenSearch> credentialManager, Func<TokenSearch> languageServer, Func<TokenSearch> gh)`

- [ ] **Step 1: Tests first.**
  - `CopilotCredentialNameTests`: `IsCli` is true for `copilot-cli/https://github.com:someone` and `Legacy copilot-cli token`, false for `copilot-language-server/oauth-token-key`, `` and null. `IsLanguageServer` is true for `copilot-language-server/oauth-token-key`, `Copilot-Language-Server/OAuth-Token-Key` and `copilot-language-server/oauth-token-key/github.com`; false for `copilot-language-server/other`, `copilot-language-server-x/oauth-token-key`, `my-copilot-language-server/oauth-token-key`, `copilot-cli/https://github.com:someone`, `` and null. The two filters equal the constants above.
  - `CopilotPollTests`: every expected log string gains ` language-server=skipped` (or the value the case gives) before ` gh=`. New: `An_accepted_cli_sign_in_leaves_the_language_server_unread` (CLI source with a token that is accepted, a throwing language-server source: `Ok`, log `env=none credential-manager=ok language-server=skipped gh=skipped`); `The_language_server_sign_in_is_used_when_the_cli_has_none` (`Ok`, log `env=none credential-manager=none language-server=ok gh=skipped`, and the one request carried that token); `A_refused_language_server_sign_in_goes_on_to_gh` (401 for the first token, 200 for the second: `Ok`, log `… language-server=refused gh=ok`); `A_language_server_entry_without_a_token_is_no_sign_in` (`NoSignIn`, `language-server=no-token`).
  - `The_sources_come_in_order`: `CopilotSignIn.InOrder(a, b, c, d)` gives kinds `Env, CredentialManager, LanguageServer, Gh`, each calling its own function.
- [ ] **Step 2: Run `dotnet test tests/Sobreiro.Tests -c Release`.** Expected: does not compile.
- [ ] **Step 3: Implement** the enum member, the name type, `InOrder` and the log line.
- [ ] **Step 4: Run `dotnet test tests/Sobreiro.Tests -c Release`.** Expected: all pass.

### Task 2: The reader and the wiring in the tray app

**Files:**
- Modify: `src/Sobreiro/Copilot/CredentialStore.cs`, `src/Sobreiro/TrayController.cs:103-108`

- [ ] **Step 1: `CredentialStore`.** One private reader takes the filter, the name test and whether a "not found" may fall back to listing every entry. `CopilotTokens()` calls it with `CliFilter`, `IsCli` and the fallback as today; `LanguageServerTokens()` calls it with `LanguageServerFilter`, `IsLanguageServer` and no fallback. Blob handling is unchanged (copy, decode with `CopilotToken.FromBlob`, zero).
- [ ] **Step 2: `TrayController.CopilotHost`** builds its sources with `CopilotSignIn.InOrder`.
- [ ] **Step 3: `dotnet build -c Release`**, 0 warnings; `dotnet test -c Release` passes. Commit Tasks 1 and 2 together.

### Task 3: The rule and the docs

**Files:**
- Modify: `CLAUDE.md:13-16`, `README.md:146-152`, `SECURITY.md:19-22`, `site/manual/index.html:161`, `docs/plan.md:405-406`
- Left alone: the roadmap's test count. An open pull request changes the same two lines; whichever merges second sets the number.

- [ ] **Step 1:** Each passage lists the four sources in order and says the language server's entry is asked for by name, without listing the others.
- [ ] **Step 2:** `dotnet test -c Release` and `npm run check` pass. Commit.
