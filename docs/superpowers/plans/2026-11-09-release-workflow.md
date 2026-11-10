# Release workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A zip colleagues unzip and run, built by CI on every PR and published as a GitHub Release on a `v*` tag.

**Architecture:** One PowerShell script, `scripts/package.ps1`, publishes the tray and the bridge, stages one folder and zips it. `ci.yml` runs build, tests and the script on PRs and `main`; `release.yml` runs tests and the script on a tag and creates the release.

**Tech Stack:** .NET 10 SDK, PowerShell (Windows PowerShell 5.1 locally, pwsh 7 on runners), GitHub Actions (`windows-latest`), `gh`.

**Spec:** `docs/superpowers/specs/2026-11-09-release-workflow-design.md`

## Global Constraints

- Version `0.6.0` in `Directory.Build.props`; zip name `Sobreiro-<version>-win-x64.zip`.
- The zip holds exactly one folder `Sobreiro/` with `Sobreiro.exe`, `sobreiro-bridge.exe`, `Fonts/OFL-Inter.txt`, `Fonts/OFL-Unbounded.txt`. No `.pdb`, no other files.
- The tray in the zip is self-contained (runs without .NET installed); the README's two publish commands for Ricardo's own copy stay unchanged.
- Actions: `actions/checkout@v7`, `actions/setup-dotnet@v6` (`10.0.x`), `actions/upload-artifact@v7`.
- `ci.yml` has `permissions: contents: read`; only `release.yml` gets `contents: write`.
- The app makes no new network calls; nothing in the zip but the programs and the licences.

## Review Focus

1. A tag that doesn't match the version (forgot the bump, typo) must fail before anything is published, saying which two values differ.
2. Running the script twice, or after a failed run, must not leave stale files in the zip.
3. A repo path with spaces (`C:\Users\First Last\...`) must package correctly.
4. Windows PowerShell 5.1 (Ricardo's shell) and pwsh 7 (runners) must both work; 5.1's `Compress-Archive` writes `\` in entry names, so don't use it.
5. A `-rc.N` tag (a test build for colleagues) must publish a prerelease, not the latest release.

---

### Task 1: Package script and version 0.6.0

**Files:**
- Create: `scripts/package.ps1`
- Modify: `Directory.Build.props` (`<Version>0.5.0</Version>` → `0.6.0`)
- Modify: `.gitignore` (add `dist/`)

**Interfaces:**
- Produces: `scripts/package.ps1 [-OutputDir <path> = "dist"] [-Tag <string>]`. Writes `<OutputDir>/Sobreiro-<version>-win-x64.zip`, prints the zip's full path as its last output line, exits non-zero on any failure. With `-Tag`, first checks the tag is `v<version>` or `v<version>-<suffix>` and fails otherwise with `Tag v0.5.9 does not match version 0.6.0 in Directory.Build.props`.

- [ ] **Step 1: Write the check first.** Run `powershell -NoProfile -File scripts/package.ps1 -Tag v0.5.9` before the script exists. Expected: fails (file not found).

- [ ] **Step 2: Implement `scripts/package.ps1`.** `Set-StrictMode -Version Latest`, `$ErrorActionPreference = 'Stop'`, check `$LASTEXITCODE` after each `dotnet` call. Resolve paths from `$PSScriptRoot` (repo root is its parent) and quote every path. Version: `dotnet msbuild Directory.Build.props -getProperty:Version` (fallback: read the XML). Remove and recreate `<OutputDir>/stage` and the zip before building. Publish:
  - bridge: `dotnet publish src/Sobreiro.Bridge -c Release -r win-x64 --self-contained -p:PublishSingleFile=true -p:PublishReadyToRun=true -p:PublishTrimmed=true -o <stage>/bridge-out`
  - tray: `dotnet publish src/Sobreiro -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true -p:EnableCompressionInSingleFile=true -o <stage>/tray-out`
  Copy only `Sobreiro.exe`, `sobreiro-bridge.exe` and `Fonts/OFL-*.txt` into `<stage>/Sobreiro/`; zip with `[System.IO.Compression.ZipFile]::CreateFromDirectory(<stage>/Sobreiro, <zip>, Optimal, $true)` (load `System.IO.Compression.FileSystem` on 5.1).

- [ ] **Step 3: Verify the tag check.** `-Tag v0.5.9` fails with the message above, before any `dotnet publish` runs. `-Tag v0.6.0` and `-Tag v0.6.0-rc.1` pass the check.

- [ ] **Step 4: Verify the zip.** Run `powershell -NoProfile -File scripts/package.ps1` twice. List the entries with `ZipFile::OpenRead`. Expected: exactly the four entries from Global Constraints, all under `Sobreiro/` with `/` separators. `Sobreiro.exe` is self-contained (over 50 MB uncompressed; the publish folder has no `Sobreiro.runtimeconfig.json` requiring a framework).

- [ ] **Step 5: Verify it runs.** Extract to a scratch folder whose path has a space, set `SOBREIRO_DATA_DIR` and `CLAUDE_CONFIG_DIR` to scratch folders, run `Sobreiro.exe --snapshot <scratch>\look.png` and wait for it. Expected: exit code 0, PNG written. Delete the PNG afterwards.

- [ ] **Step 6: Build and test.** `dotnet build -c Release` (0 warnings) and `dotnet test -c Release`: all pass.

- [ ] **Step 7: Commit.** `git add scripts/package.ps1 Directory.Build.props .gitignore` and commit "Package script: a self-contained zip for colleagues (0.6.0)".

### Task 2: CI and release workflows

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: `scripts/package.ps1 -OutputDir dist [-Tag <tag>]` from Task 1; its last output line is the zip path.

- [ ] **Step 1: `ci.yml`.** Name `CI`. On `pull_request` and `push` to `main`; `concurrency` per ref with cancel-in-progress. One job on `windows-latest`, `shell: pwsh`: checkout, setup-dotnet `10.0.x`, `dotnet build -c Release`, `dotnet test -c Release --no-build`, `./scripts/package.ps1 -OutputDir dist`, upload `dist/*.zip` as artifact `Sobreiro-win-x64`, `retention-days: 14`.

- [ ] **Step 2: `release.yml`.** Name `Release`. On `push` of tags `v*`; `permissions: contents: write`. Same setup; `dotnet test -c Release`; `./scripts/package.ps1 -OutputDir dist -Tag $env:GITHUB_REF_NAME`; then `gh release create $env:GITHUB_REF_NAME <zip> --generate-notes --title "Sobreiro <version>"`, adding `--prerelease` when the tag has a `-` suffix. `GH_TOKEN: ${{ github.token }}`.

- [ ] **Step 3: Check syntax.** Parse both files as YAML (`python -c "import yaml,sys; [yaml.safe_load(open(f)) for f in sys.argv[1:]]" .github/workflows/*.yml`). Expected: no error.

- [ ] **Step 4: Commit.** "CI and release workflows: build, test and package on every PR; a release on a v tag".

The real check is the PR's own CI run (Task 3 of the review) and an `-rc.1` tag pushed from the branch.

### Task 3: Docs

**Files:**
- Modify: `README.md` (new "Install from a release" section before the build-from-source steps; keep the publish commands)
- Modify: `docs/plan.md` (milestone 4: the workflow is in, with the result)

- [ ] **Step 1: README "Install from a release".** Download `Sobreiro-<version>-win-x64.zip` from the repo's Releases; right-click → Properties → Unblock before extracting (otherwise SmartScreen stops it: "More info", then "Run anyway"; the app isn't signed yet); extract the `Sobreiro` folder to `%LOCALAPPDATA%\Programs\`; start `Sobreiro.exe`; Connect from the tray menu. Moving the folder later breaks the status line: choose Connect again. No .NET install needed. Also say CI builds the same zip on every PR (artifact `Sobreiro-win-x64`) and how a maintainer releases: bump `Directory.Build.props`, merge, tag `v<version>` on `main` and push the tag.

- [ ] **Step 2: plan.md.** Under milestone 4, add a "Progress (2026-11-09), version 0.6.0" note: the workflows and the package script, the zip is self-contained, colleagues get it from Ricardo while the repo is private. Fix the stale "v0.1.0 tag" line to "first release tag".

- [ ] **Step 3: Commit.** "Docs: install from a release; milestone 4 progress".
