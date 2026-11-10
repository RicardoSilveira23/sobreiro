# One Sobreiro.exe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The release ships one file, `Sobreiro.exe`, which carries the status line bridge and the font licences inside it.

**Architecture:** `package.ps1` publishes the bridge, then publishes the tray with the bridge embedded as a resource. Connect (and tray start, when the status line already uses it) copies that bridge to `<data folder>\sobreiro-bridge.exe`, and the status line runs that copy. The connection state learns which bridge is this Sobreiro's, so a status line running an older copy (an unzipped 0.7.0 folder) gets Reconnect.

**Tech Stack:** C# / .NET 10, WPF + WinForms, xUnit, PowerShell, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-10-single-exe-design.md`

## Global Constraints

- Logic goes in `Sobreiro.Core` and gets an xUnit test (`CLAUDE.md`).
- `settings.json` is edited only by Connect/Disconnect, with a backup, and only the `statusLine` key. Installing the bridge never touches it.
- The bridge program itself does not change: still writes only `rate_limits` fields and a timestamp.
- Do not touch `src/Sobreiro/Flyout/` or `src/Sobreiro/TrayController.cs` (another session is working there).
- Build has 0 warnings (`TreatWarningsAsErrors`).
- Installed bridge path: `Path.Combine(AppPaths.DataDirectory(), "sobreiro-bridge.exe")`. Resource name: `sobreiro-bridge.exe`.
- Release asset name: `Sobreiro.exe`, no version. Version in `Directory.Build.props`: `0.8.0`.
- Baseline to beat: today's bridge runs in about 61 ms (20 runs through Git Bash, scratch data folder).

## Review Focus

- A bridge that is running while Sobreiro replaces it (Claude Code refreshing mid-update): the replace waits and succeeds, or fails cleanly with the old copy intact. Tests in Task 2.
- A status line written by hand, with backslashes, other casing or no quotes, around the installed path: still counts as this Sobreiro's. Test in Task 1.
- A data folder that doesn't exist yet when Connect runs on a fresh PC: the folder is created. Test in Task 2.
- A `settings.json` that is missing or isn't JSON at tray start: the start-up refresh does nothing and doesn't throw. Test in Task 2.
- A packaged exe that doesn't carry the bridge (stale incremental build): caught by the end-to-end check in Task 4.

---

### Task 1: Connection state knows this Sobreiro's bridge

**Files:**
- Modify: `src/Sobreiro.Core/AppPaths.cs`, `src/Sobreiro.Core/Connect/BridgeLocator.cs`, `src/Sobreiro.Core/Connect/BridgeCommand.cs`, `src/Sobreiro.Core/Connect/Connection.cs`, `src/Sobreiro.Core/Connect/ConnectMessage.cs`
- Test: `tests/Sobreiro.Tests/AppPathsTests.cs`, `BridgeLocatorTests.cs`, `BridgeCommandTests.cs`, `LegacyConnectionTests.cs`, `ConnectMessageTests.cs`

**Interfaces:**
- Produces:
  - `AppPaths.BridgeFile() -> string` — `<DataDirectory()>\sobreiro-bridge.exe`.
  - `BridgeLocator.Own(bool carried, string installedPath, string appDirectory, Func<string, bool> fileExists) -> string?` — `installedPath` when `carried`, else `Find(appDirectory, fileExists)`.
  - `BridgeCommand.Runs(string? command, string bridgeExePath) -> bool` — true when the command, after trimming whitespace and one pair of surrounding double quotes and turning `/` into `\`, equals the path ignoring case.
  - `ConnectionState.Elsewhere` — appended after `Legacy`: the status line runs a `sobreiro-bridge` that isn't this Sobreiro's.
  - `Connection.State(string settingsText, string? ownBridgePath) -> ConnectionState`. `State(string)` stays and means `State(text, null)` (null: can't tell, any `sobreiro-bridge` is Connected, as today).

- [ ] **Step 1: Write the failing tests**

```csharp
// AppPathsTests
[Fact] public void BridgeFile_is_in_the_data_folder() =>
    Assert.Equal(Path.Combine(AppPaths.DataDirectory(), "sobreiro-bridge.exe"), AppPaths.BridgeFile());

// BridgeLocatorTests
[Fact] public void Own_is_the_installed_copy_when_the_exe_carries_the_bridge() =>
    Assert.Equal(@"C:\Data\sobreiro-bridge.exe", BridgeLocator.Own(true, @"C:\Data\sobreiro-bridge.exe", App, _ => true));
[Fact] public void Own_is_the_bridge_beside_the_app_otherwise()   // carried false, beside exists -> beside path
[Fact] public void Own_is_null_when_not_carried_and_none_beside() // carried false, nothing exists -> null

// BridgeCommandTests
[Theory]
[InlineData("\"C:/Users/a/AppData/Local/Sobreiro/sobreiro-bridge.exe\"", true)]
[InlineData("C:\\Users\\a\\AppData\\Local\\Sobreiro\\sobreiro-bridge.exe", true)]
[InlineData("  \"c:/users/A/appdata/local/sobreiro/SOBREIRO-BRIDGE.EXE\" ", true)]
[InlineData("\"C:/Users/a/Downloads/Sobreiro/sobreiro-bridge.exe\"", false)]
[InlineData("\"C:/Users/a/AppData/Local/Sobreiro/sobreiro-bridge.exe\" | tee x", false)]
[InlineData(null, false)]
public void Runs_matches_the_path_however_it_is_written(string? command, bool expected) =>
    Assert.Equal(expected, BridgeCommand.Runs(command, @"C:\Users\a\AppData\Local\Sobreiro\sobreiro-bridge.exe"));

// LegacyConnectionTests (or a new OwnBridgeConnectionTests beside it)
[Fact] public void State_with_an_own_bridge_tells_connected_from_elsewhere()
{
    const string own = @"C:\Data\sobreiro-bridge.exe";
    Assert.Equal(ConnectionState.Connected, Connection.State(Settings("\"C:/Data/sobreiro-bridge.exe\""), own));
    Assert.Equal(ConnectionState.Elsewhere, Connection.State(Settings("\"C:/Users/a/Downloads/Sobreiro/sobreiro-bridge.exe\""), own));
    Assert.Equal(ConnectionState.Legacy, Connection.State(LegacySettings, own));
    Assert.Equal(ConnectionState.NotConnected, Connection.State(Settings("~/x.sh"), own));
    Assert.Equal(ConnectionState.Connected, Connection.State(Settings("\"C:/Users/a/Downloads/Sobreiro/sobreiro-bridge.exe\""), null));
}
[Fact] public void PlanConnect_from_another_sobreiro_bridge_is_a_reconnect_that_keeps_the_config()
// settings run "C:/Old/sobreiro-bridge.exe"; config has Previous + ChainedCommand;
// PlanConnect(settings, config, @"C:\Data\sobreiro-bridge.exe"): Reconnects == true, NewConfig == config,
// After's command == "\"C:/Data/sobreiro-bridge.exe\"".

// ConnectMessageTests
[Fact] public void Confirm_for_a_reconnect_from_another_folder_says_so()
// plan from the test above: Confirm(plan) contains
// "The status line runs a Sobreiro bridge from another folder. This points it at the one this Sobreiro uses."
// and does not contain "OpenHeadroom". The existing OpenHeadroom reconnect test keeps its line.
```

`Settings(cmd)` is a local helper building `{"statusLine":{"type":"command","command":<cmd as JSON string>}}`.

- [ ] **Step 2: Run them and see them fail**

Run: `dotnet test --filter "FullyQualifiedName~AppPathsTests|FullyQualifiedName~BridgeLocatorTests|FullyQualifiedName~BridgeCommandTests|FullyQualifiedName~LegacyConnectionTests|FullyQualifiedName~ConnectMessageTests"`
Expected: build errors for the missing members.

- [ ] **Step 3: Implement the members listed under Interfaces**

`PlanConnect` sets `Reconnects: BridgeCommand.IsBridge(oldCommand) && !BridgeCommand.Runs(oldCommand, bridgeExePath)`. If an existing test asserted `Reconnects == false` for a Connect over a `sobreiro-bridge` at a different path, that expectation changes on purpose; say so in the commit. `ConnectMessage.Confirm` picks the line by `BridgeCommand.IsLegacyBridge(SettingsJson.CommandOf(plan.Before))`: the existing OpenHeadroom line, else the new one above.

- [ ] **Step 4: Run all tests**

Run: `dotnet test`
Expected: all pass.

- [ ] **Step 5: Commit** — "Connect: tell this Sobreiro's bridge from an older copy elsewhere"

---

### Task 2: Install the bridge into the data folder

**Files:**
- Modify: `src/Sobreiro.Core/IO/AtomicFile.cs`
- Create: `src/Sobreiro.Core/Connect/BridgeInstall.cs`
- Test: `tests/Sobreiro.Tests/AtomicFileTests.cs`, create `tests/Sobreiro.Tests/BridgeInstallTests.cs`

**Interfaces:**
- Consumes: `BridgeCommand.Runs`, `SettingsJson.ReadStatusLine`, `SettingsJson.CommandOf` (Task 1 / existing).
- Produces:
  - `AtomicFile.Write(string path, Stream content)` — temp file beside `path`, then `File.Move(overwrite: true)`; 10 attempts 100 ms apart (a running bridge holds its exe for about 60 ms), shares the retry loop with `WriteAllText` (which keeps 5 × 20 ms). Creates the folder. Deletes the temp file on failure and rethrows.
  - `BridgeInstall.ResourceName = "sobreiro-bridge.exe"`.
  - `BridgeInstall.Ensure(Func<Stream> openSource, string path) -> bool` — false when `path` already holds the same bytes; otherwise writes through `AtomicFile.Write` and returns true. Throws `IOException` / `UnauthorizedAccessException` when it can't.
  - `BridgeInstall.ShouldRefresh(string settingsText, string installedPath) -> bool` — true when the status line command `Runs(installedPath)`; false otherwise, including on `FormatException`.

- [ ] **Step 1: Write the failing tests** (temp folder per test, deleted in `Dispose`, as `LegacyConnectionTests` does)

```csharp
[Fact] public void Ensure_writes_the_bridge_and_its_folder_when_missing()
// path = <dir>\data\sobreiro-bridge.exe (data\ doesn't exist); Ensure(() => new MemoryStream(bytes), path) == true;
// File.ReadAllBytes(path) == bytes
[Fact] public void Ensure_leaves_an_identical_bridge_alone()
// write bytes, set LastWriteTimeUtc to 2020-01-01; Ensure == false; LastWriteTimeUtc unchanged
[Fact] public void Ensure_replaces_other_bytes_of_the_same_length()
// existing = bytes with one byte flipped; Ensure == true; file == bytes
[Fact] public void Ensure_waits_for_a_bridge_that_is_running()
// open path with FileShare.None, close it after 250 ms on a Task; Ensure == true; file == bytes
[Fact] public void Ensure_throws_and_leaves_no_temp_file_when_the_bridge_stays_locked()
// hold FileShare.None for the whole call; Assert.ThrowsAny<IOException>; the folder holds only sobreiro-bridge.exe
[Theory]
[InlineData("{\"statusLine\":{\"type\":\"command\",\"command\":\"\\\"C:/Data/sobreiro-bridge.exe\\\"\"}}", true)]
[InlineData("{\"statusLine\":{\"type\":\"command\",\"command\":\"\\\"C:/Old/sobreiro-bridge.exe\\\"\"}}", false)]
[InlineData("{}", false)]
[InlineData("not json", false)]
[InlineData("", false)]
public void ShouldRefresh_only_when_the_status_line_runs_the_installed_copy(string settings, bool expected) =>
    Assert.Equal(expected, BridgeInstall.ShouldRefresh(settings, @"C:\Data\sobreiro-bridge.exe"));

// AtomicFileTests
[Fact] public void Write_from_a_stream_replaces_the_file() // existing file "old"; Write(path, stream of "new") -> "new"; no *.tmp left
```

- [ ] **Step 2: Run them and see them fail**

Run: `dotnet test --filter "FullyQualifiedName~BridgeInstallTests|FullyQualifiedName~AtomicFileTests"`
Expected: build errors for the missing members.

- [ ] **Step 3: Implement `AtomicFile.Write` and `BridgeInstall`**

`Ensure` compares lengths first, then the streams chunk by chunk (no hashing needed).

- [ ] **Step 4: Run all tests**

Run: `dotnet test`
Expected: all pass.

- [ ] **Step 5: Commit** — "Core: install the bridge into the data folder, waiting out a running one"

---

### Task 3: The app carries the bridge and the licences

**Files:**
- Modify: `src/Sobreiro/Sobreiro.csproj`, `src/Sobreiro/ConnectFlow.cs`, `src/Sobreiro/TrayMenu.cs`, `src/Sobreiro/App.xaml.cs`
- Create: `src/Sobreiro/EmbeddedBridge.cs`, `src/Sobreiro/LicenceWindow.cs`

**Interfaces:**
- Consumes: everything Task 1 and Task 2 produce.
- Produces (for Task 4): building `src/Sobreiro` with `-p:BridgeExe=<full path to sobreiro-bridge.exe>` embeds that file as the resource `sobreiro-bridge.exe`; without the property nothing is embedded. A `BridgeExe` that doesn't exist fails the build with `BridgeExe points at a file that doesn't exist: <path>`.

- [ ] **Step 1: `Sobreiro.csproj`**

Embed `$(BridgeExe)` with `LogicalName="sobreiro-bridge.exe"` when set, plus an `Error` target before build when it is set and missing. Embed `Fonts\OFL-*.txt` with `LogicalName="%(Filename)%(Extension)"` and drop the `CopyToOutputDirectory` line, so publish output holds only `Sobreiro.exe` (and the `.pdb`).

- [ ] **Step 2: `EmbeddedBridge` (internal static class)**

- `bool Carried` — `GetManifestResourceInfo(BridgeInstall.ResourceName) is not null` on this assembly.
- `Stream Open()` — the resource stream (throws `InvalidOperationException` when not carried).
- `string? OwnPath()` — `BridgeLocator.Own(Carried, AppPaths.BridgeFile(), AppContext.BaseDirectory, File.Exists)`.
- `string RefreshAtStart()` — returns the log word: `not-carried`, `not-used` (`ShouldRefresh` false; settings read with `Connection.ReadSettings(ClaudePaths.SettingsFile(...))`, missing file counts as not used), `current`, `written`, or `failed:<ExceptionType>`. Never throws.

- [ ] **Step 3: `ConnectFlow`**

`CurrentState()` uses `Connection.State(text, EmbeddedBridge.OwnPath())`. `Run(connect: true)` takes the bridge path from `EmbeddedBridge.OwnPath()` (null keeps today's `NoBridge` box). After the user answers Yes and before `Connection.Apply`, when `EmbeddedBridge.Carried`, call `BridgeInstall.Ensure(EmbeddedBridge.Open, bridge)`; it sits inside the existing `try`, so a failure shows the error box and leaves `settings.json` alone.

- [ ] **Step 4: `TrayMenu`**

`Legacy` and `Elsewhere` both show Reconnect, with Disconnect beside it. About keeps its text, ends with a blank line and `Read the font licences?`, and uses `MessageBoxButtons.YesNo`; Yes opens `LicenceWindow.ShowLicences()`. (The app has no WinForms visual styles, so `TaskDialog` isn't available; a Yes/No box keeps About as it looks today.)

- [ ] **Step 5: `LicenceWindow` (internal sealed WinForms `Form`)**

`static void ShowLicences()` shows it modally: title `Font licences`, no icon, centred on screen, about 560 × 520 logical pixels with `AutoScaleMode.Dpi`, a read-only multiline `TextBox` filling it with a vertical scroll bar, word wrap, `SystemFonts.MessageBoxFont`. Text: `OFL-Inter.txt`, a blank line, `OFL-Unbounded.txt`, read from the resources with line endings turned into `\r\n`.

- [ ] **Step 6: `App.xaml.cs`**

Right after the `migration ...` log line: `_log.Write("bridge " + EmbeddedBridge.RefreshAtStart());`.

- [ ] **Step 7: Build and test**

Run: `dotnet build -c Release` then `dotnet test -c Release --no-build`
Expected: 0 warnings, all tests pass.

- [ ] **Step 8: Commit** — "Tray: carry the bridge and the font licences inside Sobreiro.exe"

---

### Task 4: Package one exe; release workflow; 0.8.0

**Files:**
- Modify: `scripts/package.ps1`, `.github/workflows/release.yml`, `.github/workflows/ci.yml`, `Directory.Build.props`

**Interfaces:**
- Consumes: the `BridgeExe` property (Task 3).
- Produces: `scripts/package.ps1 [-OutputDir dist] [-Tag vX.Y.Z]` writes `<OutputDir>\Sobreiro.exe` and prints its full path as the last line.

- [ ] **Step 1: `package.ps1`**

Keep the version and tag checks. Publish the bridge exactly as today into `stage\bridge-out`, then the tray as today plus `-p:BridgeExe=<stage\bridge-out\sobreiro-bridge.exe>` into `stage\tray-out`. Fail if `tray-out` holds any file other than `Sobreiro.exe` and `*.pdb`. Copy `Sobreiro.exe` to `<OutputDir>`, removing an old one first; delete the stage; print the path. Drop the zip code and the compression assembly loads; update the synopsis.

- [ ] **Step 2: Workflows and version**

`release.yml`: check `dist\Sobreiro.exe` exists (throw otherwise) and pass it to `gh release create` in place of the zip; title and prerelease rule unchanged. `ci.yml`: upload `dist/Sobreiro.exe` (artifact name `Sobreiro-win-x64` unchanged). `Directory.Build.props`: `0.8.0`.

- [ ] **Step 3: Package and check the exe end to end (run in the background; it takes minutes)**

Run: `powershell -ExecutionPolicy Bypass -File scripts\package.ps1 -OutputDir <scratch>\dist`
Expected: last line `<scratch>\dist\Sobreiro.exe`; nothing else in `dist`. Record its size.

Then, with `SOBREIRO_DATA_DIR=<scratch>\data` and `CLAUDE_CONFIG_DIR=<scratch>\claude`, where `<scratch>\claude\settings.json` is `{"statusLine":{"type":"command","command":"\"<scratch/data with forward slashes>/sobreiro-bridge.exe\""}}`: start `dist\Sobreiro.exe`, wait about 5 s, then check
- `<scratch>\data\log.txt` has `bridge written`;
- `<scratch>\data\sobreiro-bridge.exe` exists, and piping the sample status line JSON into it prints `5h 12% · 7d 40%`;
- the average of 20 runs through Git Bash, against the 61 ms baseline;
- stop that tray process (by its path) and run it again: the log now says `bridge current`.

Never run these checks against the real `%USERPROFILE%\.claude` or `%LOCALAPPDATA%\Sobreiro`.

- [ ] **Step 4: Commit** — "Release: one Sobreiro.exe instead of the zip; 0.8.0"

---

### Task 5: Docs and site

**Files:**
- Modify: `README.md`, `site/index.html`, `site/manual/index.html`, `docs/plan.md`

- [ ] **Step 1: Rewrite every mention of the zip, unzipping, the `Sobreiro` folder, `sobreiro-bridge.exe` beside the app and the `Fonts` folder**

- Install: download `Sobreiro.exe` from the latest release, run it (SmartScreen: More info, then Run anyway; or Properties → Unblock first). It can live anywhere; `%LOCALAPPDATA%\Programs\Sobreiro` is a good home, and Start with Windows remembers where it is.
- Connect: Sobreiro puts the bridge in its data folder, `%LOCALAPPDATA%\Sobreiro\sobreiro-bridge.exe`, and points the status line at it. Moving `Sobreiro.exe` later doesn't break the status line.
- Upgrading from 0.7.0's zip: run the new `Sobreiro.exe`, choose Reconnect to Claude Code, then delete the old folder.
- Manual setup by hand: point `statusLine` at `"C:/Users/<you>/AppData/Local/Sobreiro/sobreiro-bridge.exe"`, then start (or restart) Sobreiro, which puts the bridge there.
- What it writes: the data folder also holds `sobreiro-bridge.exe`, copied from `Sobreiro.exe`.
- Uninstall: Disconnect, quit, delete `Sobreiro.exe` and `%LOCALAPPDATA%\Sobreiro`.
- Licence: the fonts and their OFL licences are inside `Sobreiro.exe`; About Sobreiro shows the licences.
- Build: the dev publish commands stay; `package.ps1` writes `dist\Sobreiro.exe`; release and CI produce `Sobreiro.exe`.
- `docs/plan.md`: a 0.8.0 progress note (one exe, bridge in the data folder, measured start-up and size from Task 4).

- [ ] **Step 2: Check**

Run: `npm run check`
Expected: pass. Then `git grep -n -i "zip\|unzip" -- README.md site docs/plan.md` shows nothing about the release download.

- [ ] **Step 3: Commit** — "Docs: install from one Sobreiro.exe"
