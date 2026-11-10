# Milestone 1 (Prototype) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A bridge that turns Claude Code's status line JSON into `usage.json` (passing any chained status line through), and a tray icon that shows the session % with a tooltip.

**Architecture:** `OpenHeadroom.Core` holds every decision (parsing, the file format, the provider-neutral limit list, the text shown, finding Git Bash) and is unit-tested. `OpenHeadroom.Bridge` and `OpenHeadroom` (tray) are thin shells over it. Data flows one way: stdin → bridge → `%LOCALAPPDATA%\OpenHeadroom\usage.json` → tray.

**Tech Stack:** C# / .NET 10 (SDK 10.0.403), xUnit, System.Text.Json (`JsonDocument` / `Utf8JsonWriter` only, so Core stays AOT-safe without source generators), WPF app hosting a WinForms `NotifyIcon`, System.Drawing for the icon.

**Spec:** [docs/plan.md](../../plan.md) (milestone 1, "Data source", "UI", "Stack", "Security rules"), [CLAUDE.md](../../../CLAUDE.md).

## Global Constraints

- C# / .NET 10. Projects under `src/`, tests under `tests/`; logic goes in `OpenHeadroom.Core` and gets an xUnit test.
- Data comes only from the status line JSON via the bridge. No network calls at runtime.
- Never read `~/.claude/.credentials.json`. Never log, print, cache or commit any token.
- The bridge writes only `rate_limits` fields and a timestamp. Never the rest of stdin.
- Never edit the user's `settings.json` in this milestone (Connect/Disconnect is milestone 2; the README documents the manual setup).
- `usage.json` and `config.json` live in `%LOCALAPPDATA%\OpenHeadroom\`. The environment variable `OPENHEADROOM_DATA_DIR`, when set, replaces that folder (for tests and trials).
- Core targets `net10.0` with `<IsAotCompatible>true</IsAotCompatible>`; it uses no reflection-based JSON.
- No Anthropic or Claude logo; product text says "Claude Code" only in plain words.
- Nullable enabled, warnings as errors in every project (`Directory.Build.props`).
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Decisions this plan makes (not in the spec)

- **usage.json shape** mirrors the status line so one parser reads both:
  `{"observed_at": <unix s>, "rate_limits": {"five_hour": {"used_percentage": n, "resets_at": n}, "seven_day": {...}, "spend_limit": {"used_percentage": n, "resets_at": n, "used_usd": n, "limit_usd": n, "period": "monthly"}}}`.
  Written from the parsed model, so only known fields can reach disk. `period` is kept only when it is `daily`, `weekly` or `monthly`.
- **Chained command** lives in `config.json` as `{"chained_command": "..."}` (milestone 2's Connect writes it; for now by hand).
- **Shell for the chain:** the same one Claude Code uses on Windows: Git Bash when found, else `powershell.exe`. Git Bash is found from `CLAUDE_CODE_GIT_BASH_PATH`, then `bash.exe` on `PATH` (skipping `System32` and `WindowsApps`, which hold the WSL launcher), then `git.exe` on `PATH` (`<git>\..\bin\bash.exe`), then `%ProgramFiles%\Git\bin\bash.exe`.
- **Bridge output:** with a chain, the chain's stdout and exit code pass through unchanged. Without one (or when it cannot start): `5h 8% · 7d 44%`, absent windows as `–`, plus ` · spend 63%` when a spend limit is present. Exit 0.
- **Display rounding:** floor, clamped to 0–100; the icon shows `!` at 100 and `–` with no data.
- **Times:** English words and day names (`ddd`, invariant), clock in the user's short time pattern; a time on another local day gets the day name.
- **Native AOT:** this machine has no MSVC linker, so the bridge is published self-contained, single-file, ReadyToRun for now, with `PublishAot` left for milestone 2.

## Review Focus

- `rate_limits` absent (a session's first call) or stdin not JSON at all → no write, default line, exit 0. (Task 1 and Task 4 tests.)
- A window whose `resets_at` has passed while the app sits idle → shown as reset at 0%, not as the stale %. (Task 3 test.)
- The tray reads `usage.json` while the bridge replaces it → read retries or keeps the last snapshot, never crashes. (Task 2 test: read with the file locked returns null; the tray keeps its previous state.)
- `used_percentage` above 100 (spend limit can exceed it) or negative → clamped for display, never an exception. (Task 3 test.)
- Fields of the wrong type (`"used_percentage": "8"`, `"resets_at": null`) → that field is null or the window is dropped, never an exception. (Task 1 test.)

## File Structure

```text
Directory.Build.props                         net10.0 defaults, nullable, warnings as errors
OpenHeadroom.slnx
src/OpenHeadroom.Core/
  OpenHeadroom.Core.csproj
  AppPaths.cs                                 data dir, usage.json, config.json paths
  Usage/RateLimits.cs                         LimitWindow, SpendLimit, RateLimits, UsageSnapshot records
  Usage/RateLimitsJson.cs                     parse status line / usage.json; write usage.json JSON
  Usage/UsageFile.cs                          atomic write, tolerant read
  Limits/Limit.cs                             provider-neutral Limit + LimitList.From
  Display/StatusLineText.cs                   bridge's default line
  Display/TrayIconModel.cs                    icon text, weekly fraction, has-data
  Display/TooltipText.cs                      tooltip lines, ≤127 chars
  Display/When.cs                             time formatting shared by tooltip
  Bridge/BridgeConfig.cs                      config.json chained_command
  Bridge/BashLocator.cs                       find Git Bash
  Bridge/ChainRunner.cs                       run chained command, capture stdout + exit code
  Bridge/BridgeRunner.cs                      the bridge's whole flow
src/OpenHeadroom.Bridge/
  OpenHeadroom.Bridge.csproj                  AssemblyName openheadroom-bridge
  Program.cs
src/OpenHeadroom/
  OpenHeadroom.csproj                         WinExe, UseWPF, UseWindowsForms
  app.manifest                                PerMonitorV2
  App.xaml, App.xaml.cs                       single instance, starts TrayController
  TrayController.cs                           NotifyIcon, menu, watcher, refresh timer
  IconRenderer.cs                             draws TrayIconModel to an Icon
  TaskbarTheme.cs                             SystemUsesLightTheme
tests/OpenHeadroom.Tests/
  OpenHeadroom.Tests.csproj
  RateLimitsJsonTests.cs, UsageFileTests.cs, LimitListTests.cs, StatusLineTextTests.cs,
  TrayIconModelTests.cs, TooltipTextTests.cs, BridgeConfigTests.cs, BashLocatorTests.cs,
  ChainRunnerTests.cs, BridgeRunnerTests.cs
```

---

### Task 1: Solution scaffold, model and JSON

**Files:**
- Create: `Directory.Build.props`, `OpenHeadroom.slnx`, `src/OpenHeadroom.Core/OpenHeadroom.Core.csproj`, `src/OpenHeadroom.Core/Usage/RateLimits.cs`, `src/OpenHeadroom.Core/Usage/RateLimitsJson.cs`, `tests/OpenHeadroom.Tests/OpenHeadroom.Tests.csproj`, `tests/OpenHeadroom.Tests/RateLimitsJsonTests.cs`

**Interfaces:**
- Produces (namespace `OpenHeadroom.Core.Usage`):
  - `public sealed record LimitWindow(double UsedPercentage, DateTimeOffset? ResetsAt);`
  - `public sealed record SpendLimit(double UsedPercentage, DateTimeOffset? ResetsAt, double? UsedUsd, double? LimitUsd, string? Period);`
  - `public sealed record RateLimits(LimitWindow? FiveHour, LimitWindow? SevenDay, SpendLimit? Spend);`
  - `public sealed record UsageSnapshot(RateLimits Limits, DateTimeOffset ObservedAt);`
  - `public static class RateLimitsJson { RateLimits? ParseStatusLine(string json); UsageSnapshot? ParseUsageFile(string json); string WriteUsageFile(UsageSnapshot s); }`

- [ ] **Step 1: Scaffold.** `Directory.Build.props` sets `TargetFramework net10.0`, `Nullable enable`, `ImplicitUsings enable`, `TreatWarningsAsErrors true`, `LangVersion latest`. Core: class library with `IsAotCompatible true`. Tests: `dotnet new xunit` (keep the template's package versions), `net10.0`, reference Core. Add both to `OpenHeadroom.slnx` (`dotnet new sln --format slnx` then `dotnet sln add`). Delete template `UnitTest1.cs` / `Class1.cs`.

- [ ] **Step 2: Write the failing tests** in `RateLimitsJsonTests.cs`:

```csharp
const string Full = """
{"session_id":"abc","transcript_path":"C:/x.jsonl","cwd":"C:/p",
 "rate_limits":{"five_hour":{"used_percentage":23.5,"resets_at":1738425600},
                "seven_day":{"used_percentage":41.2,"resets_at":1738857600},
                "spend_limit":{"used_percentage":62.8,"resets_at":1740787200,"used_usd":314.12,"limit_usd":500,"period":"monthly"}}}
""";

[Fact] void ParseStatusLine_reads_all_three_windows()
// FiveHour == new LimitWindow(23.5, DateTimeOffset.FromUnixTimeSeconds(1738425600)); SevenDay 41.2/1738857600;
// Spend == new SpendLimit(62.8, FromUnixTimeSeconds(1740787200), 314.12, 500, "monthly")
[Fact] void ParseStatusLine_returns_null_without_rate_limits()      // """{"session_id":"abc"}""" → null
[Fact] void ParseStatusLine_returns_null_for_non_json()             // "not json", "", "[]" → null
[Fact] void ParseStatusLine_returns_null_when_no_window_has_a_percentage() // {"rate_limits":{}} → null
[Fact] void ParseStatusLine_keeps_windows_independently()          // only seven_day → FiveHour null, SevenDay set
[Fact] void ParseStatusLine_drops_window_with_non_numeric_percentage() // five_hour used_percentage "8" → FiveHour null
[Fact] void ParseStatusLine_keeps_window_without_valid_resets_at()  // resets_at null or "soon" → ResetsAt null
[Fact] void ParseStatusLine_drops_unknown_period()                 // period "hourly" → Period null; dollars absent → null
[Fact] void WriteUsageFile_contains_only_rate_limit_fields_and_timestamp()
// write snapshot parsed from Full: output has no "session_id", "transcript_path", "cwd";
// JsonDocument root has exactly the properties "observed_at" and "rate_limits"
[Fact] void UsageFile_round_trips()
// ParseUsageFile(WriteUsageFile(s)) == s for a snapshot with all three windows and ObservedAt FromUnixTimeSeconds(1738420000)
[Fact] void ParseUsageFile_returns_null_without_observed_at_or_rate_limits()
```

- [ ] **Step 3: Run** `dotnet test` — expected: compile errors (types missing).
- [ ] **Step 4: Implement** the records and `RateLimitsJson` with `JsonDocument` (catch `JsonException`; a non-object root → null) and `Utf8JsonWriter` (omit null fields; numbers written as JSON numbers; `observed_at` and `resets_at` as Unix seconds). `ParseUsageFile` reuses the `rate_limits` element parser.
- [ ] **Step 5: Run** `dotnet test` — expected: all pass.
- [ ] **Step 6: Commit** `Core: rate limits model and JSON`.

### Task 2: Paths and usage.json on disk

**Files:**
- Create: `src/OpenHeadroom.Core/AppPaths.cs`, `src/OpenHeadroom.Core/Usage/UsageFile.cs`, `tests/OpenHeadroom.Tests/UsageFileTests.cs`

**Interfaces:**
- Consumes: `RateLimitsJson`, `UsageSnapshot` (Task 1).
- Produces:
  - `public static class AppPaths { string DataDirectory(); string UsageFile(); string ConfigFile(); }` — `OPENHEADROOM_DATA_DIR` if set and non-empty, else `Path.Combine(LocalApplicationData, "OpenHeadroom")`; files `usage.json`, `config.json`.
  - `public static class UsageFile { void Write(string path, UsageSnapshot snapshot); UsageSnapshot? Read(string path); }`

- [ ] **Step 1: Failing tests** (each in its own temp directory, deleted in `Dispose`):
  - `Write_then_Read_round_trips`
  - `Write_creates_missing_directory`
  - `Write_replaces_existing_file_and_leaves_no_temp_files` (write twice; directory holds exactly `usage.json`)
  - `Read_returns_null_for_missing_file`
  - `Read_returns_null_for_corrupt_file` (`"{"`)
  - `Read_returns_null_while_file_is_locked` (open with `FileShare.None`, then Read → null, no throw)
  - `AppPaths_uses_OPENHEADROOM_DATA_DIR_when_set` (set and restore the variable; mark the class `[Collection("env")]` with `DisableParallelization`)
- [ ] **Step 2: Run** — expected: compile errors.
- [ ] **Step 3: Implement.** `Write`: create directory; write UTF-8 (no BOM) to `<path>.<Guid:N>.tmp` in the same folder; `File.Move(tmp, path, overwrite: true)`; on `IOException`/`UnauthorizedAccessException` retry up to 5 times, 20 ms apart; on final failure delete the temp file and rethrow. `Read`: open with `FileShare.ReadWrite | FileShare.Delete`; any IO or parse failure → null.
- [ ] **Step 4: Run** — expected: pass.
- [ ] **Step 5: Commit** `Core: usage.json atomic write and tolerant read`.

### Task 3: Limit list and the text the user sees

**Files:**
- Create: `src/OpenHeadroom.Core/Limits/Limit.cs`, `src/OpenHeadroom.Core/Display/When.cs`, `src/OpenHeadroom.Core/Display/StatusLineText.cs`, `src/OpenHeadroom.Core/Display/TrayIconModel.cs`, `src/OpenHeadroom.Core/Display/TooltipText.cs`, tests `LimitListTests.cs`, `StatusLineTextTests.cs`, `TrayIconModelTests.cs`, `TooltipTextTests.cs`

**Interfaces:**
- Consumes: Task 1 records.
- Produces (namespaces `OpenHeadroom.Core.Limits`, `OpenHeadroom.Core.Display`):
  - `public enum LimitKind { Session, Weekly, Spend }`
  - `public sealed record Limit(string Provider, LimitKind Kind, string Label, double Percent, DateTimeOffset? ResetsAt, DateTimeOffset ObservedAt, bool IsReset) { int DisplayPercent { get; } }` — `DisplayPercent` = floor of `Percent`, clamped 0–100.
  - `public static class LimitList { IReadOnlyList<Limit> From(UsageSnapshot? snapshot, DateTimeOffset now); }` — order Session, Weekly, Spend; provider `"Claude Code"`; labels `"Session"`, `"Weekly"`, `"Spend"`; a window with `ResetsAt <= now` gets `IsReset = true`, `Percent = 0`, `ResetsAt` kept.
  - `public static class StatusLineText { string Render(RateLimits? limits); }`
  - `public sealed record TrayIconModel(string Text, double? WeeklyFraction, bool HasData) { static TrayIconModel From(IReadOnlyList<Limit> limits); }`
  - `public static class TooltipText { const int MaxLength = 127; string Render(IReadOnlyList<Limit> limits, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture); }`
  - `internal static class When { string Format(DateTimeOffset t, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture); }` (tests see it via `InternalsVisibleTo`)

- [ ] **Step 1: Failing tests.** Use `now = 2026-11-09T12:00:00Z`, `TimeZoneInfo.Utc`, `CultureInfo.GetCultureInfo("en-US")` unless stated.
  - LimitList: `From_null_is_empty`; `From_maps_windows_in_order_with_labels`; `From_marks_passed_window_as_reset_at_zero` (five_hour resets 11:00Z, 30% → IsReset, Percent 0); `From_skips_absent_windows`.
  - `DisplayPercent`: 8.9 → 8; 99.6 → 99; 100 → 100; 130 → 100; -5 → 0.
  - StatusLineText: `{5h 8.4, 7d 44}` → `"5h 8% · 7d 44%"`; only 7d → `"5h – · 7d 44%"`; null → `"5h – · 7d –"`; with spend 62.8 → `"5h 8% · 7d 44% · spend 62%"`.
  - TrayIconModel: no limits → `("–", null, false)`; session 8, weekly 44 → `("8", 0.44, true)`; session 100 → `"!"`; session 99.6 → `"99"`; only weekly → `("–", 0.44, true)`; weekly 130 → WeeklyFraction 1.0.
  - When: same day 14:20Z → `"2:20 PM"`; 2026-11-14T00:00Z → `"Sat 12:00 AM"`; with `pt-PT` culture same day → `"14:20"`.
  - TooltipText:
    - no limits → `"OpenHeadroom · no data yet"`
    - session 8 (resets 14:20Z), weekly 44 (resets 2026-11-14T00:00Z), observed 11:58Z → `"Session 8% · resets 2:20 PM\nWeekly 44% · resets Sat 12:00 AM\nas of 11:58 AM"`
    - session absent → first line `"Session –"`
    - session reset at 11:00Z → first line `"Session 0% · reset 11:00 AM"`
    - spend 62.8 resetting 2026-11-16T00:00Z → a third line `"Spend 62% · resets Mon 12:00 AM"` before `as of`
    - every output `Length <= 127` (a culture whose short time pattern is long, e.g. a custom `CultureInfo` clone with `ShortTimePattern = "HH:mm:ss 'oclock' zzz"`, still fits: truncate to 127).
- [ ] **Step 2: Run** — expected: compile errors.
- [ ] **Step 3: Implement.** Separator is ` · ` (U+00B7 with spaces); dash is `–` (U+2013).
- [ ] **Step 4: Run** — expected: pass.
- [ ] **Step 5: Commit** `Core: limit list, status line, tooltip and icon text`.

### Task 4: Bridge (config, Git Bash, chain, runner, exe)

**Files:**
- Create: `src/OpenHeadroom.Core/Bridge/BridgeConfig.cs`, `BashLocator.cs`, `ChainRunner.cs`, `BridgeRunner.cs`; `src/OpenHeadroom.Bridge/OpenHeadroom.Bridge.csproj`, `Program.cs`; tests `BridgeConfigTests.cs`, `BashLocatorTests.cs`, `ChainRunnerTests.cs`, `BridgeRunnerTests.cs`

**Interfaces:**
- Consumes: `RateLimitsJson`, `UsageFile`, `AppPaths`, `StatusLineText`.
- Produces (namespace `OpenHeadroom.Core.Bridge`):
  - `public sealed record BridgeConfig(string? ChainedCommand) { static BridgeConfig Load(string path); }` — missing/corrupt/non-string → `ChainedCommand` null; blank → null.
  - `public static class BashLocator { string? Find(Func<string, string?> getEnv, Func<string, bool> fileExists); }`
  - `public sealed record ChainResult(int ExitCode, string Output);`
  - `public static class ChainRunner { ProcessStartInfo StartInfo(string command, string? bashPath); ChainResult? Run(string command, string stdin, string? bashPath); }` — bash: `FileName = bashPath`, `ArgumentList = ["-c", command]`; no bash: `FileName = "powershell.exe"`, `ArgumentList = ["-NoProfile", "-NonInteractive", "-Command", command]`; redirect stdin/stdout, `UseShellExecute false`, `CreateNoWindow true`, UTF-8 encodings. `Run` returns null when the process cannot start; stderr is not redirected (Claude Code's `--debug` sees it).
  - `public sealed class BridgeRunner(string usagePath, string configPath, Func<string, string, ChainResult?> runChain) { int Run(string stdin, TextWriter stdout, DateTimeOffset now); }`

- [ ] **Step 1: Failing tests.**
  - BridgeConfig: `Load_reads_chained_command`; `Load_missing_file_is_empty`; `Load_corrupt_or_wrong_type_is_empty` (`{"chained_command": 5}`); `Load_blank_command_is_null`.
  - BashLocator (fake env dictionary and a `HashSet<string>` of existing files, case-insensitive):
    - `Prefers_CLAUDE_CODE_GIT_BASH_PATH_when_it_exists`
    - `Finds_bash_on_PATH` (`PATH = @"C:\Program Files\Git\usr\bin;C:\Windows\System32"`, bash in both → Git's)
    - `Skips_System32_and_WindowsApps_bash` (`SystemRoot = C:\Windows`; only those two have bash → falls through to `null` when nothing else exists)
    - `Derives_bash_from_git_on_PATH` (`C:\Program Files\Git\cmd\git.exe` exists → `C:\Program Files\Git\bin\bash.exe`)
    - `Falls_back_to_ProgramFiles_Git`
    - `Returns_null_when_nothing_found`
  - ChainRunner: `StartInfo_uses_bash_dash_c` and `StartInfo_falls_back_to_powershell` (assert `FileName` and `ArgumentList`); `Run_passes_stdin_and_returns_stdout_and_exit_code` — integration, `bash -c "cat; exit 3"` with stdin `"hello"` → `("hello", 3)`, using `BashLocator.Find(Environment.GetEnvironmentVariable, File.Exists)`; skip (return early with a note) if no bash; `Run_returns_null_when_executable_missing` (bashPath `C:\nope\bash.exe`).
  - BridgeRunner (temp dir; `runChain` fake records its arguments):
    - `Writes_usage_and_prints_default_line_without_chain` → usage.json parses to `observed_at == now`; stdout `"5h 23% · 7d 41% · spend 62%\n"`; returns 0
    - `Does_not_write_without_rate_limits` → no usage.json; stdout `"5h – · 7d –\n"`; returns 0
    - `Keeps_previous_usage_when_rate_limits_absent` (existing usage.json unchanged)
    - `Runs_chain_with_full_stdin_and_passes_output_and_exit_code` → fake receives the exact stdin and the configured command; stdout is the fake's output verbatim; return equals the fake's exit code; usage.json still written
    - `Falls_back_to_default_line_when_chain_cannot_start` (fake returns null) → default line, 0
    - `Still_prints_when_usage_write_fails` (usage path is an existing directory) → default line, 0
    - `Usage_file_never_contains_other_stdin_fields` → the written file contains none of `session_id`, `transcript_path`, `cwd`, `abc`, `C:/x.jsonl`
- [ ] **Step 2: Run** — expected: compile errors.
- [ ] **Step 3: Implement** Core classes. `BridgeRunner.Run` catches every exception around the write and around the chain, so stdout always gets a line.
- [ ] **Step 4: Bridge exe.** `OpenHeadroom.Bridge.csproj`: `OutputType Exe`, `AssemblyName openheadroom-bridge`, `InvariantGlobalization true`, `IsAotCompatible true`, reference Core. `Program.cs`: read all stdin as UTF-8; stdout as `StreamWriter(Console.OpenStandardOutput(), new UTF8Encoding(false))`; `new BridgeRunner(AppPaths.UsageFile(), AppPaths.ConfigFile(), (cmd, input) => ChainRunner.Run(cmd, input, BashLocator.Find(Environment.GetEnvironmentVariable, File.Exists)))`; outer catch prints `StatusLineText.Render(null)` and returns 0. Add to the solution.
- [ ] **Step 5: Run** `dotnet test` — expected: pass. Then the smoke test (Git Bash):

```bash
export OPENHEADROOM_DATA_DIR="$(mktemp -d)"
dotnet build src/OpenHeadroom.Bridge -c Release
echo '{"session_id":"s","rate_limits":{"five_hour":{"used_percentage":8,"resets_at":1999999999},"seven_day":{"used_percentage":44,"resets_at":1999999999}}}' \
  | src/OpenHeadroom.Bridge/bin/Release/net10.0/openheadroom-bridge.exe
cat "$OPENHEADROOM_DATA_DIR/usage.json"
```
Expected: prints `5h 8% · 7d 44%`; `usage.json` holds only `observed_at` and `rate_limits`.
- [ ] **Step 6: Commit** `Bridge: write usage.json and chain the previous status line`.

### Task 5: Tray app

**Files:**
- Create: `src/OpenHeadroom/OpenHeadroom.csproj`, `app.manifest`, `App.xaml`, `App.xaml.cs`, `TrayController.cs`, `IconRenderer.cs`, `TaskbarTheme.cs`

**Interfaces:**
- Consumes: `AppPaths`, `UsageFile.Read`, `LimitList.From`, `TrayIconModel.From`, `TooltipText.Render`.
- Produces: `OpenHeadroom.exe` (no public API).

- [ ] **Step 1: Project.** `net10.0-windows`, `OutputType WinExe`, `UseWPF true`, `UseWindowsForms true`, `ApplicationManifest app.manifest` (PerMonitorV2 DPI awareness), reference Core, add to the solution. `App.xaml`: `ShutdownMode="OnExplicitShutdown"`, no `StartupUri`.
- [ ] **Step 2: App.** `OnStartup`: single instance via `Mutex` named `Local\OpenHeadroom.Tray` (second instance exits at once); create `TrayController`. `OnExit`: dispose it.
- [ ] **Step 3: TrayController.** `NotifyIcon` visible; context menu `Open data folder` (`explorer.exe` on `AppPaths.DataDirectory()`, creating it first) and `Quit`. Refresh = `UsageFile.Read` → if null and a previous snapshot exists, keep the previous one (a read that lost a race must not blank the icon) → `LimitList.From(snapshot, DateTimeOffset.Now)` → icon + `TooltipText.Render(..., TimeZoneInfo.Local, CultureInfo.CurrentCulture)`. Triggers: start; `FileSystemWatcher` on the data directory, filter `usage.json`, `Changed | Created | Renamed`, debounced 250 ms onto the UI thread; a `DispatcherTimer` every 30 s (resets and "as of" move on without a write); `SystemEvents.UserPreferenceChanged`. Create the data directory if missing so the watcher can start. Dispose the previous `Icon` after swapping (and `DestroyIcon` the HICON).
- [ ] **Step 4: IconRenderer.** `Icon Render(TrayIconModel model, bool lightTaskbar, int size)`: size from `SystemInformation.SmallIconSize.Width`; transparent background; text colour white on a dark taskbar, near-black (`#1A1A1A`) on a light one, grey (`#8A8A8A`) when `!HasData`; bold Segoe UI sized so two digits fill the width; weekly bar along the bottom edge, height `max(2, size/8)`, a 35 %-alpha track the full width and a solid fill of `WeeklyFraction × width`. No colour levels yet (milestone 2).
- [ ] **Step 5: TaskbarTheme.** `static bool IsLight()` reads `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\SystemUsesLightTheme` (missing → false).
- [ ] **Step 6: Verify.** `dotnet build -c Release` clean; run the app with `OPENHEADROOM_DATA_DIR` pointing at the smoke-test folder from Task 4; pipe a new reading through the bridge and confirm the icon and tooltip update within a second; a viewport screenshot of the tray area.
- [ ] **Step 7: Commit** `Tray: icon with session % and tooltip`.

### Task 6: README and publish

**Files:**
- Modify: `README.md` (Status, "Try the prototype": build/publish commands, the manual `settings.json` snippet with a forward-slash path, `config.json` for chaining an existing status line, `OPENHEADROOM_DATA_DIR`)
- Create: `docs/references.md` (what the two reference repos taught; data-source check)

- [ ] **Step 1:** Publish both: `dotnet publish src/OpenHeadroom.Bridge -c Release -r win-x64 --self-contained -p:PublishSingleFile=true -p:PublishReadyToRun=true -o publish/bridge` and the tray likewise to `publish/tray`. Time 20 bridge runs on sample stdin; record the median in the README.
- [ ] **Step 2:** Write the README section; settings snippet:

```json
"statusLine": { "type": "command", "command": "\"C:/path/to/publish/bridge/openheadroom-bridge.exe\"" }
```
- [ ] **Step 3: Commit** `README: try the prototype`.
