# Diagnostics log Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A tray that disappears leaves a trace: an append-only `log.txt` in the data folder, kept in Release builds, with start, exit, startup failure, unhandled exceptions and session ending, plus a visible message box when startup fails.

**Architecture:** Formatting, the exception walk, the size cap and the repeat budget live in `OpenHeadroom.Core/Diagnostics` with xUnit tests. `src/OpenHeadroom/App.xaml.cs` only creates the log and wires the events. Each entry is opened, appended and closed, so a process that dies right after a write keeps it.

**Tech Stack:** C# / .NET 10, xUnit, WPF `Application`, `Microsoft.Win32.SystemEvents`, WinForms `MessageBox`.

**Spec:** the task brief of 2026-11-09 (tray found not running twice, nothing in the event log; every failure log is `Debug.WriteLine`, which Release strips), quoted in Global Constraints below; `CLAUDE.md`; `docs/plan.md` (Security rules).

## Global Constraints

- C# / .NET 10. Logic goes in `OpenHeadroom.Core` and gets an xUnit test. Core stays `IsAotCompatible`.
- Nullable enabled, warnings as errors. `dotnet build` ends with 0 warnings; `dotnet test` passes.
- Log file: `%LOCALAPPDATA%\OpenHeadroom\log.txt`, or `log.txt` in `OPENHEADROOM_DATA_DIR`. Capped at about 256 KB (262144 bytes), then rolled to `log.1.txt`, replacing it.
- Each entry starts with one line: ISO timestamp, the PID, the event.
- Record exception type names and stack traces only. Never exception messages (`Message`, `ToString()`, `Data`), file contents, usage numbers, or anything from `.credentials.json`.
- No network calls. The log is local only.
- Never touch `C:\Dev\openheadroom\publish\`, the real `%LOCALAPPDATA%\OpenHeadroom\` or the real `%USERPROFILE%\.claude\`. Tests use temp folders; manual runs set `OPENHEADROOM_DATA_DIR` to a scratch folder.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Decisions this plan makes (not in the spec)

- **Entry format.** Header line `2026-11-09T14:03:12.345+00:00 pid=1234 <event>`: local time with offset, `yyyy-MM-dd'T'HH:mm:ss.fffzzz`, invariant culture. An exception follows as indented lines: `  <FullName>` for the first, `  inner <FullName>` for the rest, each followed by its stack trace lines trimmed and indented by four spaces, or `    (no stack trace)`. CR and LF in the event become spaces.
- **Exception walk.** Breadth-first over `InnerException`, and `InnerExceptions` for `AggregateException`, at most 8 exceptions. A thrown object that is not an `Exception` is recorded by its type name only.
- **Entry cap.** An entry over 16384 chars is cut to that length and ends `  (cut)`. Every entry ends with `\n`.
- **Roll rule.** Before an append, if the file exists, is not empty, and its size plus the entry's UTF-8 bytes would pass the cap, move it to `log.1.txt` (overwrite). A failed move is ignored and the append goes ahead. UTF-8 without BOM. Open with `FileShare.ReadWrite | FileShare.Delete` so a second launch can append. `Append` never throws.
- **Repeat budget.** Dispatcher and unobserved task exceptions each get 20 entries per run; the 20th carries `; later ones not logged`. A UI exception that repeats on every frame can't churn the disk.
- **Events** (all from `App`):
  - `start version=0.2.0 instance=<AppPaths.InstanceId()>` on every launch except `--snapshot`, which logs nothing.
  - `already-running` on a second launch, before it exits.
  - `exit code=<n>` at the top of `OnExit`.
  - `startup-failed` with the exception when `new TrayController()` throws; then the message box, then `Shutdown(1)`.
  - `unhandled-exception terminating=true|false` with the exception (`AppDomain.CurrentDomain.UnhandledException`).
  - `unobserved-task-exception` with the exception (`TaskScheduler.UnobservedTaskException`; not marked observed, behaviour unchanged).
  - `dispatcher-exception handled` with the exception (still `Handled = true`).
  - `session-ending reason=Logoff|Shutdown` from `Application.OnSessionEnding` (WPF quits at WM_QUERYENDSESSION, so WM_ENDSESSION never arrives).
  - The two `Debug.WriteLine` calls in `App.xaml.cs` become `show-event-failed` and `show-request-failed` with the exception. The other `Debug.WriteLine` calls (TrayController, TrayMenu) stay; routing them is a follow-up.
- **Message box.** WinForms `MessageBox`, title `OpenHeadroom`, `MessageBoxIcon.Error`, text from Core: `OpenHeadroom couldn't start ({TypeName}).\n\nThe details are in {logPath}`, or, when the `startup-failed` entry couldn't be written, `DiagnosticsMessage.StartupFailedUnlogged` naming the data folder (added in the final review's fix wave). Shown inside a try/catch, after the mutex is released, so a relaunch can try again.
- **No version bump.** PR #6 moves the version to 0.3.0; this PR leaves `Directory.Build.props` alone to stay mergeable.

## Review Focus

- A Release build: nothing may hang off `DEBUG` or `Debug.*`. The log must be written by a Release `OpenHeadroom.exe`.
- A data folder that doesn't exist yet (first run): `Append` creates it. Test: `Append_creates_the_folder`.
- The log locked or read-only (Notepad open on it, a second launch writing): `Append` returns false, never throws. Test: `Append_returns_false_when_the_path_is_a_directory`.
- An exception whose `Message` holds a path or token: none of it may reach the file. Test: `Format_never_includes_the_message`.
- A UI exception on every frame: the budget stops it after 20. Test: `LogBudget_lets_the_limit_through_then_stays_quiet`.

---

### Task 1: Core diagnostics log

**Files:**
- Create: `src/OpenHeadroom.Core/Diagnostics/LogEntry.cs`, `src/OpenHeadroom.Core/Diagnostics/DiagnosticsLog.cs`, `src/OpenHeadroom.Core/Diagnostics/LogBudget.cs`, `src/OpenHeadroom.Core/Diagnostics/DiagnosticsMessage.cs`
- Modify: `src/OpenHeadroom.Core/AppPaths.cs` (add `LogFile()`)
- Test: `tests/OpenHeadroom.Tests/LogEntryTests.cs`, `tests/OpenHeadroom.Tests/DiagnosticsLogTests.cs`, `tests/OpenHeadroom.Tests/LogBudgetTests.cs`, `tests/OpenHeadroom.Tests/AppPathsTests.cs`

**Interfaces:**
- Produces (namespace `OpenHeadroom.Core.Diagnostics`):
  - `public static class LogEntry { public const int MaxChars = 16384; public const int MaxExceptions = 8; public static string Format(DateTimeOffset at, int pid, string @event, object? exception = null); }`
  - `public sealed class DiagnosticsLog { public const long DefaultMaxBytes = 262144; public DiagnosticsLog(string path, long maxBytes = DefaultMaxBytes); public string FilePath { get; } public string PreviousFilePath { get; } public bool Append(string entry); public bool Write(string @event, object? exception = null); }` — `Write` formats with `DateTimeOffset.Now` and `Environment.ProcessId`. `PreviousFilePath` is `log.1.txt` beside `log.txt` (name without extension + `.1` + extension).
  - `public sealed class LogBudget { public LogBudget(int limit); public string? Next(string @event); }` — thread-safe (`Interlocked`).
  - `public static class DiagnosticsMessage { public static string StartupFailed(string exceptionTypeName, string logPath); }`
  - `AppPaths.LogFile()` → `Path.Combine(DataDirectory(), "log.txt")`.

- [ ] **Step 1: Write the failing tests**

`LogEntryTests` (use `new DateTimeOffset(2026, 11, 9, 14, 3, 12, 345, TimeSpan.FromHours(1))`, pid 1234; throw real exceptions to get stack traces):
- `Format_starts_with_iso_time_pid_and_event`: `Format(at, 1234, "start version=0.2.0 instance=OpenHeadroom")` equals `"2026-11-09T14:03:12.345+01:00 pid=1234 start version=0.2.0 instance=OpenHeadroom\n"`.
- `Format_flattens_line_breaks_in_the_event`: event `"a\r\nb"` gives header `... pid=1234 a  b`.
- `Format_adds_the_type_and_stack_trace`: a thrown `InvalidOperationException` gives a line `"  System.InvalidOperationException"` then at least one line starting `"    at "`.
- `Format_never_includes_the_message`: an exception thrown with message `"secret C:\\Users\\x token=abc"` and `Data["k"] = "secret"`; the output contains none of `secret`, `token=abc`, `C:\\Users\\x`.
- `Format_walks_inner_and_aggregate_exceptions`: `AggregateException(new IOException(), new InvalidOperationException("m", new ArgumentException()))` lists `System.AggregateException`, `inner System.IO.IOException`, `inner System.InvalidOperationException`, `inner System.ArgumentException` in that order.
- `Format_stops_after_eight_exceptions`: a chain of 12 nested exceptions has 8 type lines.
- `Format_records_a_non_exception_by_type_only`: `Format(at, 1, "unhandled-exception terminating=true", "boom")` has `"  System.String"` and no `boom`.
- `Format_marks_a_missing_stack_trace`: an unthrown `new Exception()` gives `"    (no stack trace)"`.
- `Format_cuts_an_entry_over_the_cap`: an event of 20000 `x` gives length ≤ `MaxChars + 10`, ends with `"  (cut)\n"`.

`DiagnosticsLogTests` (temp folder per test like `AtomicFileTests`; a small cap such as 100 bytes where rolling matters):
- `Append_creates_the_folder_and_writes_utf8_without_bom`.
- `Append_adds_to_the_end`: two appends, both in order.
- `Append_rolls_to_log_1_when_the_cap_would_be_passed`: cap 100, append 60 bytes, then 60 bytes: `log.txt` holds only the second, `log.1.txt` the first.
- `Append_replaces_an_older_log_1`.
- `Append_never_rolls_an_empty_file`: an entry bigger than the cap goes into an empty `log.txt`, no `log.1.txt`.
- `Append_returns_false_when_the_path_is_a_directory`: `FilePath` is an existing directory; `Append` returns false, no exception.
- `Previous_file_path_sits_beside_the_log`: `log.txt` → `log.1.txt`.
- `Write_uses_this_process_id`: the line contains `pid={Environment.ProcessId} ` and the event.

`LogBudgetTests`:
- `LogBudget_lets_the_limit_through_then_stays_quiet`: limit 3: `Next("e")` returns `"e"`, `"e"`, `"e; later ones not logged"`, then `null`, `null`.

`AppPathsTests` (existing `[Collection("env")]` class, same set/restore pattern):
- `LogFile_lives_in_the_data_directory`: with the variable set to a temp folder, `AppPaths.LogFile()` is `<folder>\log.txt`.
- `Log_written_with_the_override_lands_in_the_scratch_folder`: with the variable set to a temp folder, `new DiagnosticsLog(AppPaths.LogFile()).Write("start version=test instance=x")` returns true and `<folder>\log.txt` contains `start version=test instance=x`.

`DiagnosticsMessage`: add `StartupFailed_names_the_type_and_the_log` to `LogEntryTests`: `StartupFailed("InvalidOperationException", @"C:\x\log.txt")` equals `"OpenHeadroom couldn't start (InvalidOperationException).\n\nThe details are in C:\\x\\log.txt"`.

- [ ] **Step 2: Run the tests and see them fail**

Run: `dotnet test --filter "FullyQualifiedName~LogEntry|FullyQualifiedName~DiagnosticsLog|FullyQualifiedName~LogBudget|FullyQualifiedName~AppPaths"`
Expected: build fails, the types don't exist.

- [ ] **Step 3: Implement the four types and `AppPaths.LogFile()`** to the Interfaces and Decisions above. `DiagnosticsLog.Append` takes a lock, creates the folder, applies the roll rule, opens with `FileMode.Append, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete`, writes the UTF-8 bytes, and catches every exception (return false).

- [ ] **Step 4: Run the tests and see them pass**

Run: `dotnet test`
Expected: all tests pass, 0 warnings.

- [ ] **Step 5: Commit** `Core: diagnostics log with size cap, exception walk and repeat budget`.

### Task 2: Wire the tray, the message box, and the docs

**Files:**
- Modify: `src/OpenHeadroom/App.xaml.cs`
- Modify: `README.md` (new `## Diagnostics log` section just before `## Known limits`; it sits outside the lines PR #6 changes)
- Modify: `docs/plan.md` (Security rules: one bullet after the `usage.json` line)

**Interfaces:**
- Consumes: everything Task 1 produces.

- [ ] **Step 1: Wire `App`** to the Events list in Decisions. `private readonly DiagnosticsLog _log = new(AppPaths.LogFile());` is fine because the snapshot branch never calls it. Subscribe the `AppDomain`, `TaskScheduler` and `SystemEvents` handlers right after the snapshot branch and the `start` line, through named methods so `OnExit` can unsubscribe `SystemEvents`. Version: `typeof(App).Assembly.GetName().Version?.ToString(3)`, as `TrayMenu` does. In the startup catch: log, release the mutex as today, then `System.Windows.Forms.MessageBox.Show(DiagnosticsMessage.StartupFailed(ex.GetType().Name, _log.FilePath), "OpenHeadroom", MessageBoxButtons.OK, MessageBoxIcon.Error)`, then `Shutdown(1)`. Remove `using System.Diagnostics;` if nothing else needs it.

- [ ] **Step 2: Build Release** — `dotnet build -c Release` ends with 0 warnings.

- [ ] **Step 3: README and plan.** README section, plain words: where the file is (and `OPENHEADROOM_DATA_DIR`), why it exists, what an entry holds (time, process id, start with version, exit code, startup failure, unexpected errors, Windows ending the session), that errors are recorded by type name and stack trace only with no messages, usage numbers or file contents, and the 256 KB roll to `log.1.txt`. `docs/plan.md` Security rules bullet: `log.txt` records times, the version, exit codes and exception type names with stack traces; never messages, usage numbers, file contents or tokens.

- [ ] **Step 4: Commit** `Tray: write the diagnostics log; show a box when startup fails` and `Docs: diagnostics log`.

### Final verification (controller, not a subagent)

- `dotnet build` and `dotnet build -c Release`: 0 warnings. `dotnet test`: all pass.
- Run the Release `OpenHeadroom.exe` from `src/OpenHeadroom/bin/Release/...` with `OPENHEADROOM_DATA_DIR` set to a scratch folder: `start` appears. Launch it again: the second PID logs `start`, `already-running`, `exit code=0`. Send `WM_QUERYENDSESSION` (logoff) to that PID's top-level windows only: `session-ending` then `exit code=0` appear. Stop it, read the log, delete the scratch folder.
