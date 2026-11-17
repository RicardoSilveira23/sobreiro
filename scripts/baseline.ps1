<#
.SYNOPSIS
  Measures a release build and writes its row into docs/baseline.md.
.DESCRIPTION
  Builds the release (cargo build --release --locked), then runs tools/baseline on
  target/release's Sobreiro.exe and sobreiro-bridge.exe: time to the first pill, memory and
  processor time when idle, both exe sizes and the bridge's run time. The copy it starts has its own data folder, so an installed Sobreiro keeps
  running untouched; for those minutes the taskbar shows a second pill and a second tray icon.
  Takes about 12 minutes. How each figure is taken: docs/baseline.md. Works in Windows
  PowerShell 5.1 and pwsh 7.
.PARAMETER Label
  The row's name. Default: the exe's version. Name an experiment, such as "0.10.0, no compression".
.PARAMETER Section
  The heading in docs/baseline.md whose table gets the row: Releases (default) or Experiments.
.PARAMETER Exe
  Measure this Sobreiro.exe instead of building one. Bridge then names its bridge, if any.
.PARAMETER BridgeOnly
  Leaves the tray unstarted and takes only the two file sizes and the bridge's run time. It takes
  seconds, and a running Sobreiro can stay open.
.PARAMETER NoTable
  Print the row only.
#>
param(
    [string]$Label,
    [ValidateSet('Releases', 'Experiments')][string]$Section = 'Releases',
    [string]$Exe,
    [string]$Bridge,
    [int]$IdleMinutes = 10,
    [int]$Starts = 5,
    [switch]$BridgeOnly,
    [switch]$NoTable
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot

if (-not $Exe) {
    Push-Location $repoRoot
    try {
        & cargo build --release --locked | Out-Host
        if ($LASTEXITCODE -ne 0) { throw "The build failed (exit $LASTEXITCODE)" }
    }
    finally { Pop-Location }
    $Exe = Join-Path $repoRoot 'target/release/Sobreiro.exe'
    $Bridge = Join-Path $repoRoot 'target/release/sobreiro-bridge.exe'
}

$toolArgs = @('--exe', $Exe, '--idle-minutes', $IdleMinutes, '--starts', $Starts)
if ($BridgeOnly) { $toolArgs += @('--tray', 'size') }
if ($Bridge) { $toolArgs += @('--bridge', $Bridge) }
if ($Label) { $toolArgs += @('--label', $Label) }
if (-not $NoTable) { $toolArgs += @('--table', (Join-Path $repoRoot 'docs/baseline.md'), '--section', $Section) }

& cargo run --release --locked -p sobreiro-baseline --manifest-path (Join-Path $repoRoot 'Cargo.toml') -- @toolArgs
if ($LASTEXITCODE -ne 0) { throw "The baseline run failed (exit $LASTEXITCODE)" }
