<#
.SYNOPSIS
  Captures the right part of the live (dark) taskbar and a JSON of where things are, for the
  hero animation (Sobreiro.exe --media <folder> --plate <plate.json>).

.DESCRIPTION
  Needs Sobreiro running with its pill on (window "Sobreiro pill"). Reads window rectangles and
  UI Automation button rectangles; takes one screen copy (no cursor); never clicks, moves the
  mouse or leaves anything on screen. All coordinates in the JSON are screen pixels.
  Writes <OutDir>\taskbar-dark.png and taskbar-dark.json.
#>
param(
    [string]$OutDir
)

$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
if (-not $OutDir) { $OutDir = Join-Path $repo 'media\plates' }
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Path $OutDir | Out-Null }
$OutDir = (Resolve-Path $OutDir).Path

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class CapNative {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct MONITORINFO {
        public int cbSize; public RECT rcMonitor; public RECT rcWork; public uint dwFlags;
    }
    [DllImport("user32.dll", SetLastError = true)] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr h, uint flags);
    [DllImport("user32.dll")] public static extern bool GetMonitorInfo(IntPtr mon, ref MONITORINFO mi);
}
'@

# DPI-aware first, so every rectangle is in physical pixels.
$aware = $false
try { $aware = [CapNative]::SetProcessDpiAwarenessContext([IntPtr](-4)) } catch { }
if (-not $aware) { [void][CapNative]::SetProcessDPIAware() }

function Get-Rect([IntPtr]$h) {
    $r = New-Object CapNative+RECT
    [void][CapNative]::GetWindowRect($h, [ref]$r)
    return [ordered]@{ left = $r.Left; top = $r.Top; right = $r.Right; bottom = $r.Bottom }
}

$pillWnd = [CapNative]::FindWindow([NullString]::Value, 'Sobreiro pill')
if ($pillWnd -eq [IntPtr]::Zero -or -not [CapNative]::IsWindowVisible($pillWnd)) {
    Write-Host 'Start Sobreiro with the pill on, then run this again.'
    exit 1
}
$pill = Get-Rect $pillWnd

$tray = [CapNative]::FindWindow('Shell_TrayWnd', [NullString]::Value)
if ($tray -eq [IntPtr]::Zero) { Write-Host 'The taskbar window was not found.'; exit 1 }
$taskbar = Get-Rect $tray

$mi = New-Object CapNative+MONITORINFO
$mi.cbSize = [Runtime.InteropServices.Marshal]::SizeOf([type][CapNative+MONITORINFO])
$mon = [CapNative]::MonitorFromWindow($tray, 2)
[void][CapNative]::GetMonitorInfo($mon, [ref]$mi)
$monitor = [ordered]@{ left = $mi.rcMonitor.Left; top = $mi.rcMonitor.Top; right = $mi.rcMonitor.Right; bottom = $mi.rcMonitor.Bottom }
$workArea = [ordered]@{ left = $mi.rcWork.Left; top = $mi.rcWork.Top; right = $mi.rcWork.Right; bottom = $mi.rcWork.Bottom }

# Icon slots: UI Automation buttons under the taskbar.
$root = [System.Windows.Automation.AutomationElement]::FromHandle($tray)
$buttonCond = New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
    [System.Windows.Automation.ControlType]::Button)
$buttons = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $buttonCond)
$claude = $null; $copilot = $null; $seen = @()
foreach ($b in $buttons) {
    $name = $b.Current.Name
    $seen += $name
    $r = $b.Current.BoundingRectangle
    if ($r.IsEmpty -or $r.Width -le 0) { continue }
    $rect = [ordered]@{ left = [int]$r.Left; top = [int]$r.Top; right = [int]$r.Right; bottom = [int]$r.Bottom }
    # The tray buttons are to the right of the pill: a pinned "Sobreiro" taskbar button is not.
    if ($rect.left -lt $pill.right) { continue }
    if (-not $claude -and ($name -like 'Session *' -or $name -like 'Sobreiro*')) { $claude = $rect }
    elseif (-not $copilot -and $name -like 'AI credits*') { $copilot = $rect }
}
if (-not $claude) {
    Write-Host 'Sobreiro''s tray icon was not found among the taskbar buttons. Names seen:'
    $seen | ForEach-Object { Write-Host "  $_" }
    exit 1
}

$iconPx = [System.Windows.Forms.SystemInformation]::SmallIconSize.Width

# Capture: 480 px left of the pill to the taskbar's right edge, the taskbar's full height.
$capLeft = [Math]::Max($taskbar.left, $pill.left - 480)
$w = $taskbar.right - $capLeft
$h = $taskbar.bottom - $taskbar.top
$bmp = New-Object System.Drawing.Bitmap($w, $h, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($bmp)
try { $g.CopyFromScreen($capLeft, $taskbar.top, 0, 0, (New-Object System.Drawing.Size($w, $h))) }
finally { $g.Dispose() }

# Ground: the widest run of columns, left of the pill, each identical to its neighbour.
$pillCol = $pill.left - $capLeft
$rectAll = New-Object System.Drawing.Rectangle(0, 0, $w, $h)
$data = $bmp.LockBits($rectAll, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
try {
    $stride = $data.Stride
    $buf = New-Object byte[] ($stride * $h)
    [Runtime.InteropServices.Marshal]::Copy($data.Scan0, $buf, 0, $buf.Length)
} finally { $bmp.UnlockBits($data) }

function Test-SameColumns([int]$a, [int]$b) {
    for ($y = 0; $y -lt $h; $y++) {
        $i = $y * $stride + $a * 4; $j = $y * $stride + $b * 4
        if ($buf[$i] -ne $buf[$j] -or $buf[$i+1] -ne $buf[$j+1] -or $buf[$i+2] -ne $buf[$j+2]) { return $false }
    }
    return $true
}
$bestStart = 0; $bestLen = 1; $start = 0
for ($x = 1; $x -lt $pillCol; $x++) {
    if (-not (Test-SameColumns ($x - 1) $x)) { $start = $x }
    $len = $x - $start + 1
    if ($len -gt $bestLen) { $bestLen = $len; $bestStart = $start }
}
$ground = [ordered]@{ left = $capLeft + $bestStart; right = $capLeft + $bestStart + $bestLen }

$pngPath = Join-Path $OutDir 'taskbar-dark.png'
$bmp.Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

$light = (Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize' -ErrorAction SilentlyContinue).SystemUsesLightTheme
$dark = ($light -eq 0)

$plate = [ordered]@{
    image      = 'taskbar-dark.png'
    capturedAt = (Get-Date).ToString("yyyy-MM-dd'T'HH:mm:sszzz", [Globalization.CultureInfo]::InvariantCulture)
    dark       = $dark
    origin     = [ordered]@{ x = $capLeft; y = $taskbar.top }
    monitor    = $monitor
    workArea   = $workArea
    taskbar    = $taskbar
    band       = [ordered]@{ top = $pill.top; bottom = $pill.bottom }
    pill       = $pill
    icons      = [ordered]@{ claude = $claude; copilot = $copilot }
    ground     = $ground
    iconPx     = $iconPx
}
$jsonPath = Join-Path $OutDir 'taskbar-dark.json'
[IO.File]::WriteAllText($jsonPath, ($plate | ConvertTo-Json -Depth 5), (New-Object Text.UTF8Encoding($false)))

Write-Host ("Captured {0}x{1} at ({2},{3}); pill {4}..{5}; ground {6}..{7} ({8} px); dark={9}" -f `
    $w, $h, $capLeft, $taskbar.top, $pill.left, $pill.right, $ground.left, $ground.right, ($ground.right - $ground.left), $dark)
Write-Host $pngPath
Write-Host $jsonPath
