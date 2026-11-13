<#
.SYNOPSIS
  Builds the single file colleagues download: Sobreiro.exe.
.DESCRIPTION
  Publishes the bridge, then a self-contained tray that embeds it (BridgeExe), and copies
  Sobreiro.exe to OutputDir. The last output line is the exe's full path. Works in Windows
  PowerShell 5.1 and pwsh 7.
.PARAMETER OutputDir
  Where Sobreiro.exe (and a throwaway stage folder) go. Default: dist.
.PARAMETER Tag
  Optional git tag. Must be v<version> or v<version>-<suffix>, else the script fails.
.PARAMETER KeepBridge
  Also leaves sobreiro-bridge.exe in OutputDir, for scripts/baseline.ps1 to measure.
#>
param(
    [string]$OutputDir = 'dist',
    [string]$Tag,
    [switch]$KeepBridge
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$propsPath = Join-Path $repoRoot 'Directory.Build.props'

# Version: ask MSBuild, fall back to reading the XML.
$version = $null
try {
    $out = & dotnet msbuild "$propsPath" -getProperty:Version 2>$null
    if ($LASTEXITCODE -eq 0 -and $out) {
        $version = (($out | Out-String).Trim())
        if ($version -match '^\s*\{') {
            $version = (($out | Out-String) | ConvertFrom-Json).Properties.Version
        }
    }
} catch { $version = $null }
if (-not $version -or $version -notmatch '^\d+\.\d+\.\d+') {
    $xml = [xml](Get-Content -LiteralPath $propsPath -Raw)
    $version = ($xml.Project.PropertyGroup | ForEach-Object { $_.Version } | Where-Object { $_ } | Select-Object -First 1)
}
if (-not $version) { throw "Could not read Version from $propsPath" }
$version = "$version".Trim()

if ($Tag) {
    $escaped = [regex]::Escape($version)
    if ($Tag -notmatch "^v$escaped(-.+)?$") {
        throw "Tag $Tag does not match version $version in Directory.Build.props"
    }
}

if (-not [System.IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path (Get-Location).Path $OutputDir
}
$OutputDir = [System.IO.Path]::GetFullPath($OutputDir)
$stage = Join-Path $OutputDir 'stage'
$exe = Join-Path $OutputDir 'Sobreiro.exe'

if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
if (Test-Path -LiteralPath $exe) { Remove-Item -LiteralPath $exe -Force }
New-Item -ItemType Directory -Path $stage -Force | Out-Null

$bridgeOut = Join-Path $stage 'bridge-out'
$trayOut = Join-Path $stage 'tray-out'

& dotnet publish "$(Join-Path $repoRoot 'src/Sobreiro.Bridge')" -c Release -r win-x64 --self-contained `
    -p:PublishSingleFile=true -p:PublishReadyToRun=true -p:PublishTrimmed=true -o "$bridgeOut"
if ($LASTEXITCODE -ne 0) { throw "dotnet publish of the bridge failed (exit $LASTEXITCODE)" }

& dotnet publish "$(Join-Path $repoRoot 'src/Sobreiro')" -c Release -r win-x64 --self-contained true `
    -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true -p:EnableCompressionInSingleFile=true `
    -p:BridgeExe="$(Join-Path $bridgeOut 'sobreiro-bridge.exe')" -o "$trayOut"
if ($LASTEXITCODE -ne 0) { throw "dotnet publish of the tray failed (exit $LASTEXITCODE)" }

$extra = @(Get-ChildItem -LiteralPath $trayOut -Force | Where-Object { $_.Name -ne 'Sobreiro.exe' -and $_.Extension -ne '.pdb' })
if ($extra.Count -gt 0) { throw "Unexpected files in the tray publish output: $($extra.Name -join ', ')" }

Copy-Item -LiteralPath (Join-Path $trayOut 'Sobreiro.exe') -Destination $exe
if ($KeepBridge) {
    Copy-Item -LiteralPath (Join-Path $bridgeOut 'sobreiro-bridge.exe') -Destination (Join-Path $OutputDir 'sobreiro-bridge.exe') -Force
}
Remove-Item -LiteralPath $stage -Recurse -Force
Write-Output $exe
