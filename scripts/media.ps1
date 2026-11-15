<#
.SYNOPSIS
  Renders every Sobreiro product picture into media\ (stills, animations, contact sheet).
.DESCRIPTION
  Builds Sobreiro, runs `Sobreiro.exe --media`, encodes the result with Pillow, then swaps
  media\stills, media\animations and media\index.html in whole. media\plates\ and
  media\picks.json are inputs and are never deleted. Any failure leaves media\ as it was
  (exit 1; if putting the old ones back fails, the stage folder is kept and its path printed).
  Then copies each media\picks.json "copy" entry into the repo and the baked README hero
  ("bake", made from the run's own plate) to docs\images. Only the committed plate
  (media\plates\taskbar-dark.json) refreshes those; any other plate, or none, is a trial run
  that touches media\ only.
.PARAMETER Python
  The Python to use (default: python). It creates scripts\.venv-media on the first run.
.PARAMETER SkipBuild
  Use the Release build that is already there.
.PARAMETER Plate
  The taskbar plate JSON (default: media\plates\taskbar-dark.json when it exists, else none:
  then the hero animations and taskbar stills are skipped, and so are the site picks and the
  README hero). A plate other than the committed one is a trial run: media\ is rebuilt, site\
  and docs\images are not touched.
#>
param(
    [string]$Python = 'python',
    [switch]$SkipBuild,
    [string]$Plate
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$media = Join-Path $root 'media'
$venv = Join-Path $PSScriptRoot '.venv-media'
$venvPython = Join-Path $venv 'Scripts\python.exe'
# The stage lives beside the folders it swaps with, so every move is a rename on one volume.
$stage = Join-Path $media ('.stage-' + [guid]::NewGuid().ToString('N'))
$keepStage = $false

function Fail([string]$message) {
    Write-Host "media.ps1: $message" -ForegroundColor Red
    exit 1
}

# Runs a native command and stops the script if it exits non-zero.
function Invoke-Native([string]$what, [string]$exe, [string[]]$arguments) {
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & $exe @arguments | Out-Host
    $code = $LASTEXITCODE
    $ErrorActionPreference = $previous
    if ($code -ne 0) { throw "$what failed (exit $code)." }
}

try {
    # 1. Python first, before anything else is touched.
    $found = $null
    try {
        $found = (& $Python --version) 2>$null
    } catch {
        $found = $null
    }
    if (-not $found -or $LASTEXITCODE -ne 0) {
        Fail "Python was not found ('$Python'). Install Python 3 or pass -Python <path to python.exe>."
    }

    if (-not $Plate) {
        $default = Join-Path $media 'plates\taskbar-dark.json'
        if (Test-Path $default) { $Plate = $default }
    }
    if ($Plate -and -not (Test-Path $Plate)) { Fail "The plate '$Plate' does not exist." }
    $committedPlate = Join-Path $media 'plates\taskbar-dark.json'
    $refreshPicks = $false
    if (-not $Plate) {
        Write-Host 'No plate: the hero, the README hero and the site picks were not refreshed.'
    } elseif ((Resolve-Path $Plate).Path -ne (Resolve-Path $committedPlate -ErrorAction SilentlyContinue).Path) {
        Write-Host 'Trial plate: media\ is rebuilt, but the site picks and the README hero were not refreshed.'
    } else {
        $refreshPicks = $true
    }

    # 2. The virtual environment.
    if (-not (Test-Path $venvPython)) {
        Write-Host 'Creating scripts\.venv-media ...'
        Invoke-Native 'Creating the virtual environment' $Python @('-m', 'venv', $venv)
    }
    # Every run: quick when satisfied, and picks up a changed requirements file.
    try {
        Invoke-Native 'pip install' $venvPython @('-m', 'pip', 'install', '--disable-pip-version-check', '-q', '-r', (Join-Path $PSScriptRoot 'requirements-media.txt'))
    } catch {
        throw "The Pillow install failed (pip install -r scripts/requirements-media.txt): $($_.Exception.Message)"
    }

    # 3. Build.
    $exe = Join-Path $root 'src\Sobreiro\bin\Release\net10.0-windows\Sobreiro.exe'
    if (-not $SkipBuild) {
        Invoke-Native 'dotnet build' 'dotnet' @('build', (Join-Path $root 'src\Sobreiro'), '-c', 'Release', '-v', 'q', '--nologo')
    }
    if (-not (Test-Path $exe)) { throw "Sobreiro.exe not found at $exe (build it first)." }

    # 4. Render. A WinExe returns at once under `&`, so wait for it.
    New-Item -ItemType Directory -Force $media | Out-Null
    $work = Join-Path $stage 'work'
    $out = Join-Path $stage 'out'
    New-Item -ItemType Directory -Force $work, $out | Out-Null
    $errFile = Join-Path $stage 'media-stderr.txt'
    $exeArgs = @('--media', ('"{0}"' -f $work))
    if ($Plate) { $exeArgs += @('--plate', ('"{0}"' -f (Resolve-Path $Plate).Path)) }
    $proc = Start-Process -FilePath $exe -ArgumentList $exeArgs -Wait -PassThru -NoNewWindow -RedirectStandardError $errFile
    if (Test-Path $errFile) { Get-Content $errFile | ForEach-Object { Write-Host $_ } }
    if ($proc.ExitCode -ne 0) { throw "Sobreiro.exe --media failed (exit $($proc.ExitCode))." }

    # 5. Encode and write the contact sheet.
    $picksFile = Join-Path $media 'picks.json'
    $encodeArgs = @((Join-Path $PSScriptRoot 'encode-media.py'), $work, $out)
    if ($refreshPicks -and (Test-Path $picksFile)) {
        $plateFull = (Resolve-Path $Plate).Path
        $plateJson = Get-Content $plateFull -Raw | ConvertFrom-Json
        $encodeArgs += @($picksFile, (Join-Path (Split-Path -Parent $plateFull) $plateJson.image))
    }
    Invoke-Native 'Encoding' $venvPython $encodeArgs
    foreach ($needed in 'stills', 'animations', 'index.html') {
        if (-not (Test-Path (Join-Path $out $needed))) { throw "The encoder did not write $needed." }
    }

    # Every pick must name a picture the new output has, before anything is swapped.
    # picks.json: { "copy": { repo path: media path }, "bake": [ { out, ... } ] }.
    $copyList = @()
    $bakeList = @()
    if ($refreshPicks -and (Test-Path $picksFile)) {
        $picks = Get-Content $picksFile -Raw | ConvertFrom-Json
        if ($picks.copy) { $copyList = @($picks.copy.PSObject.Properties) }
        if ($picks.bake) { $bakeList = @($picks.bake) }
        foreach ($p in $copyList) {
            if (-not (Test-Path (Join-Path $out $p.Value))) { throw "picks.json: '$($p.Value)' is not in the new media output." }
        }
        foreach ($b in $bakeList) {
            $baked = Join-Path (Join-Path $out 'bake') (Split-Path -Leaf $b.out)
            foreach ($ext in '.webp', '.png') {
                if (-not (Test-Path ($baked + $ext))) { throw "picks.json: the bake '$($b.out)$ext' was not written." }
            }
        }
    }

    # 6. Swap in whole. Remember what moved out and what moved in; if anything fails, remove
    #    only what moved in and put each moved-out original back, each in its own try.
    $old = Join-Path $stage 'old'
    New-Item -ItemType Directory -Force $old | Out-Null
    $names = 'stills', 'animations', 'index.html'
    $movedOut = @()
    $movedIn = @()
    try {
        foreach ($n in $names) {
            $target = Join-Path $media $n
            if (Test-Path $target) { Move-Item $target (Join-Path $old $n) -ErrorAction Stop; $movedOut += $n }
        }
        foreach ($n in $names) {
            Move-Item (Join-Path $out $n) (Join-Path $media $n) -ErrorAction Stop
            $movedIn += $n
        }
    } catch {
        $swapError = $_.Exception.Message
        $restoreFailed = @()
        foreach ($n in $movedIn) {
            try { Remove-Item (Join-Path $media $n) -Recurse -Force -ErrorAction Stop } catch { $restoreFailed += $n }
        }
        foreach ($n in $movedOut) {
            try { Move-Item (Join-Path $old $n) (Join-Path $media $n) -ErrorAction Stop } catch { $restoreFailed += $n }
        }
        if ($restoreFailed.Count -gt 0) {
            $keepStage = $true
            throw "The swap failed ($swapError) and could not put back: $($restoreFailed -join ', '). The originals are in $old - copy them back into media\."
        }
        throw "The swap failed ($swapError); media\ is as it was."
    }

    # 7. The site's picks and the baked README hero (checked above). The bake stays in the stage:
    #    it is copied from there, not kept in media\.
    foreach ($p in $copyList) {
        $to = Join-Path $root $p.Name
        New-Item -ItemType Directory -Force (Split-Path -Parent $to) | Out-Null
        Copy-Item (Join-Path $media $p.Value) $to -Force
    }
    foreach ($b in $bakeList) {
        $to = Join-Path $root $b.out
        New-Item -ItemType Directory -Force (Split-Path -Parent $to) | Out-Null
        foreach ($ext in '.webp', '.png') {
            Copy-Item ((Join-Path (Join-Path $out 'bake') (Split-Path -Leaf $b.out)) + $ext) ($to + $ext) -Force
        }
    }
    Write-Host 'Done: media\ is up to date.'
    exit 0
} catch {
    Write-Host "media.ps1: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
} finally {
    if ($keepStage) {
        Write-Host "media.ps1: kept $stage (the originals are in its old\ folder)." -ForegroundColor Red
    } elseif (Test-Path $stage) {
        Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue
    }
}
