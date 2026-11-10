# Release workflow: a zip colleagues can test (design)

2026-11-09. Approved by Ricardo the same day. Part of milestone 4 ("CI release workflow").

## Why

Ricardo wants colleagues at work to try Sobreiro. Today the only way to run it is to clone the
repo, install the .NET 10 SDK and run the two `dotnet publish` commands in the README. Colleagues
need a download they unzip and run.

## Goals

- Every PR and every push to `main` builds with 0 warnings and runs the tests on GitHub's Windows
  runners, and packages the zip, so a broken package shows up before a release.
- A `v*` tag publishes a GitHub Release with `Sobreiro-<version>-win-x64.zip` attached.
- The zip runs on a PC without .NET installed.
- The README says how to install from a release.

## Non-goals

- Code signing, an installer, winget, auto-update (milestone 5).
- Making the repo public (a later milestone 4 step, Ricardo's call).
- Changing how Ricardo refreshes his own copy: the README's two publish commands stay.

## Approaches

1. **One package script, called by both workflows (picked).** `scripts/package.ps1` publishes
   the tray and the bridge, stages them in one folder and zips it. CI runs it on every PR, so the
   release path is exercised daily, and anyone can build the same zip locally.
2. Publish steps written inline in each workflow. Fewer files, but the commands live twice and
   can't be run locally the same way.
3. Release only, no CI. Least work, but a broken build is found at release time.

## Design

- **`scripts/package.ps1 [-OutputDir dist]`** (PowerShell 5.1 and 7):
  - reads the version from `Directory.Build.props` (`dotnet msbuild -getProperty:Version`);
  - publishes the bridge as today (self-contained, single file, ReadyToRun, trimmed);
  - publishes the tray **self-contained**, single file, with native libraries inside and
    compression on (today's README build is framework-dependent and needs the runtime);
  - stages `Sobreiro/` with `Sobreiro.exe`, `sobreiro-bridge.exe` (beside it, where
    `BridgeLocator` looks first) and the font licences, no `.pdb`;
  - writes `<OutputDir>/Sobreiro-<version>-win-x64.zip` and prints its path.
- **`.github/workflows/ci.yml`**: on pull requests and pushes to `main`, `windows-latest`,
  .NET `10.0.x`: build Release (warnings are errors via `Directory.Build.props`), test, run the
  package script, upload the zip as a workflow artifact (kept 14 days).
- **`.github/workflows/release.yml`**: on a `v*` tag push: check the tag equals `v` + the version
  in `Directory.Build.props` (fail otherwise), test, package, then `gh release create` with the
  zip and generated notes. `permissions: contents: write` only here; CI is read-only.
- **README**: "Install from a release": download the zip, Properties → Unblock (an unsigned
  download is otherwise stopped by SmartScreen: "More info", then "Run anyway"), extract to
  `%LOCALAPPDATA%\Programs\Sobreiro`, start `Sobreiro.exe`, Connect. Keep it there: the status
  line points at the bridge's path.
- **Version 0.6.0**; after merge, tag `v0.6.0` on `main` to publish the first release.

## Constraints

- No new network calls in the app; the workflows only talk to GitHub.
- Nothing in the zip beyond the two programs and the licences: no tokens, no data folder.

## Left to the builder

Action versions (latest majors: checkout v7, setup-dotnet v6, upload-artifact v7), the script's
exact flags, and README wording.
