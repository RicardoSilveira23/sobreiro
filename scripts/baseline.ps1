<#
.SYNOPSIS
  Measures a release build and writes its row into docs/baseline.md.
.DESCRIPTION
  Packages Sobreiro.exe and the bridge (scripts/package.ps1), then runs src/Sobreiro.Baseline on
  them: time to the first pill, memory and processor time when idle, both exe sizes and the
  bridge's run time. The copy it starts has its own data folder, so an installed Sobreiro keeps
  running untouched; for those minutes the taskbar shows a second pill and a second tray icon.
  Takes about 12 minutes. How each figure is taken: docs/baseline.md. Works in Windows
  PowerShell 5.1 and pwsh 7.
.PARAMETER Label
  The row's name. Default: the exe's version. Name an experiment, such as "0.10.0, no compression".
.PARAMETER Section
  The heading in docs/baseline.md whose table gets the row: Releases (default) or Experiments.
.PARAMETER Exe
  Measure this Sobreiro.exe instead of packaging one. Bridge then names its bridge, if any.
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
    [switch]$NoTable
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot

if (-not $Exe) {
    $out = Join-Path $repoRoot 'dist/baseline'
    & (Join-Path $PSScriptRoot 'package.ps1') -OutputDir $out -KeepBridge | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Packaging failed (exit $LASTEXITCODE)" }
    $Exe = Join-Path $out 'Sobreiro.exe'
    $Bridge = Join-Path $out 'sobreiro-bridge.exe'
}

$toolArgs = @('--exe', $Exe, '--idle-minutes', $IdleMinutes, '--starts', $Starts)
if ($Bridge) { $toolArgs += @('--bridge', $Bridge) }
if ($Label) { $toolArgs += @('--label', $Label) }
if (-not $NoTable) { $toolArgs += @('--table', (Join-Path $repoRoot 'docs/baseline.md'), '--section', $Section) }

& dotnet run --project (Join-Path $repoRoot 'src/Sobreiro.Baseline') -c Release -- @toolArgs
if ($LASTEXITCODE -ne 0) { throw "The baseline run failed (exit $LASTEXITCODE)" }
