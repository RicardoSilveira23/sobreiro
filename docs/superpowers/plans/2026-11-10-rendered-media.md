# Rendered Media Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One command, `scripts/media.ps1`, renders every Sobreiro product picture (stills and
animations) with the app's own views into `media/`, and copies the site's picks into `site/`.

**Architecture:** A `--media <folder>` mode in `Sobreiro.exe`, beside `--snapshot`, renders PNG
stills and animation frames plus a `timeline.json` per animation and a `manifest.json`. What to
render (names, sample readings, timelines, plate geometry) is pure code in
`Sobreiro.Core/Media/` with xUnit tests; the WPF side in `src/Sobreiro/Media/` only turns entries
into pixels. A Python script with Pillow encodes animated WebP and GIF and writes the contact
sheet; `media.ps1` runs it all and swaps the result into `media/`.

**Tech Stack:** C# / .NET 10, WPF (`RenderTargetBitmap`), System.Drawing (tray icons), xUnit,
Python 3.13 + Pillow (pinned), Windows PowerShell 5.1.

**Spec:** `docs/superpowers/specs/2026-11-10-rendered-media-design.md`. Read it first.

## Global Constraints

- Only real moments: every state change is a cut. No eased bars, ring sweeps, sliding panels or
  crossfades. The pointer glide is the only continuous motion.
- `--media` starts no tray icon, takes no mutex, makes no network call, reads no credentials or
  live data. Sample readings only.
- Pinned: clock `2026-11-10T14:00:00+00:00`; zone `GMT Standard Time`; culture `en-GB`; accent
  `AccentPalette.WindowsBlue`. The hero uses the plate's `capturedAt` as its clock instead.
- Every surface comes from `FlyoutView.Build`, `PillView.Build` or `IconRenderer.Draw`. No view
  code is copied or changed for media; the running app looks the same.
- Stills: transparent, `@1x` and `@2x` (2× = `RenderTargetBitmap` at 192 dpi). Icons and the
  taskbar stills are exempt (pixel sizes; plate resolution).
- WebP lossless, alpha, `loop=0`. GIF 1× only, on `#0b0b0b` (dark) or `#ffffff` (light).
- Size budget: hero WebP about 500 KB, every other animation about 300 KB. Over budget is
  reported, not fatal.
- `media/plates/` and `media/picks.json` are inputs: the swap never deletes them.
- Alt text and page copy use plain words ("the details", not "flyout").
- Commit messages follow the repo's `Area: sentence` style (`Media: ...`) and end with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Tests: `dotnet test tests/Sobreiro.Tests --filter FullyQualifiedName~Media` for media tests;
  the full suite must stay green (`dotnet test`).

## Review Focus

1. A PC with another culture, accent or time: the pictures must not change. Pinned by
   `PinnedCultureWinsOverTheCurrentOne` (Task 1) and the two-run hash check (Task 6).
2. No plate yet (a fresh clone) or a broken plate JSON: a missing plate skips the hero and the
   taskbar stills with one line on stderr and still writes everything else; a broken one fails
   with the field it lacks. Pinned by `ParseNamesTheMissingField` (Task 4) and Task 5 step 6.
3. A pill wider than the one captured: it must grow leftwards with its right edge fixed, as on
   the real taskbar, never over the tray. Pinned by `ClearedPillWidensLeftKeepingTheRightEdge`
   (Task 4).
4. Glide lengths that don't divide by 1/60 s (350 ms): frame durations must still sum exactly,
   or loops drift. Pinned by `GlideDurationsSumToTheBeat` (Task 2).
5. Python missing or pip offline: `media.ps1` must stop before touching `media/`, saying what is
   missing. Pinned by Task 6 step 7.

---

### Task 1: Pinned inputs, sample readings and still names

**Files:**
- Create: `src/Sobreiro.Core/Media/MediaPins.cs`, `src/Sobreiro.Core/Media/SampleReadings.cs`,
  `src/Sobreiro.Core/Media/MediaStills.cs`
- Test: `tests/Sobreiro.Tests/MediaStillsTests.cs`, `tests/Sobreiro.Tests/SampleReadingsTests.cs`

**Interfaces:**
- Produces (namespace `Sobreiro.Core.Media`):
  - `enum Services { Claude, Copilot, Both }`
  - `static class MediaPins { DateTimeOffset Now; TimeZoneInfo Zone; CultureInfo Culture; AccentPalette Accent; }`
  - `static class SampleReadings`:
    - `IReadOnlyList<Limit> Calm(Services s, DateTimeOffset now)`: Claude session 15%
      resetting at `now + 141 min` (window 5 h), weekly 45% at `now + 74 h` (window 7 d);
      Copilot via `LimitList.FromCopilot(new CopilotReading(15000, 2644, <1 Dec 00:00 in
      MediaPins.Zone>, now - 1 min, CopilotSource.Live), now)`. All observed `now - 1 min`.
    - `IReadOnlyList<Limit> Rung(Services s, int level, DateTimeOffset now)`: levels 0-4 =
      none, low (8, 44, 750 credits), middle (27, 61, 2644), high (74, 80, 9450), used up
      (100, 93, 15000); level 0 returns `[]`.
    - `static readonly string[] LevelNames = ["none", "low", "middle", "high", "full"]`
    - `bool ShowClaude(Services s) => s != Services.Copilot`
  - `abstract record MediaStill { abstract string Path { get; } }` with
    - `DetailsStill(Services Services, FlyoutStyle Style, bool Dark, BarColourOption Colour, int Scale)`
      → `stills/details/{services}-{style}-{theme}[-accent]@{scale}x.png`
    - `PillStill(Services Services, bool Dark, int Scale)` → `stills/pill/{services}-{theme}@{scale}x.png`
    - `IconStill(bool Copilot, int Level, bool Dark, int Px, int Enlarge)` →
      `stills/icons/{claude|copilot}-{level name}-{theme}-{px}[-x8].png` (the icon is drawn
      for a dark or a light taskbar, as `--snapshot`'s two icon strips show)
    - `TaskbarStill(ShowOption Show, Services Services)` →
      `stills/taskbar/{tray-only|tray-and-pill}-{services}.png`
  - `static class MediaStills { IReadOnlyList<MediaStill> All { get; } }` (slugs lower case:
    `claude`, `copilot`, `both`, `bars`, `battery`, `thermometer`, `dark`, `light`)

- [ ] **Step 1: Write the failing tests**

```csharp
[Fact] public void DetailsCoverEveryServiceStyleAndThemeAtBothScales()
    => Assert.Equal(48, MediaStills.All.OfType<DetailsStill>().Count()); // 18 purple + 6 accent (Both), ×2 scales
[Fact] public void AccentIsOnlyForBoth()
    => Assert.All(MediaStills.All.OfType<DetailsStill>().Where(d => d.Colour == BarColourOption.Accent),
        d => Assert.Equal(Services.Both, d.Services));
[Fact] public void CountsPerSurface()
{
    Assert.Equal(12, MediaStills.All.OfType<PillStill>().Count());
    Assert.Equal(100, MediaStills.All.OfType<IconStill>().Count()); // 2 rings × 5 levels × 2 themes × (4 px + one x8)
    Assert.Equal(4, MediaStills.All.OfType<TaskbarStill>().Count()); // 2 shows × (Claude, Both)
}
[Fact] public void PathsAreUniqueAndLowerCase()
{
    var paths = MediaStills.All.Select(s => s.Path).ToList();
    Assert.Equal(paths.Count, paths.Distinct().Count());
    Assert.All(paths, p => Assert.Matches(@"^stills/(details|pill|icons|taskbar)/[a-z0-9@\-]+\.png$", p));
}
[Fact] public void NamesSayWhatTheyShow()
{
    Assert.Equal("stills/details/both-battery-light-accent@2x.png",
        new DetailsStill(Services.Both, FlyoutStyle.Battery, false, BarColourOption.Accent, 2).Path);
    Assert.Equal("stills/icons/copilot-full-dark-16-x8.png", new IconStill(true, 4, true, 16, 8).Path);
}
// SampleReadingsTests
[Fact] public void CalmClaudeReadsOnTrackOnBothRows()  // FlyoutModel.From(..., MediaPins.Now, MediaPins.Zone, MediaPins.Culture)
    // Session.Status!.Kind == StatusKind.OnTrack and Weekly.Status!.Kind == StatusKind.OnTrack
[Fact] public void CopilotOnlyHasNoClaudeRows()      // From(Calm(Copilot), ..., showClaude: false): ShowClaude false, Copilot not null
[Fact] public void BothHasClaudeAndCopilot()         // Session.HasData && Copilot is not null
[Fact] public void PinnedCultureWinsOverTheCurrentOne()
    // set CultureInfo.CurrentCulture = pt-PT (restore in finally); the credits row's ResetsIn or Detail
    // contains "1 Dec", and Copilot.Credits.UsedFigure equals what it is under en-GB
[Fact] public void RungZeroIsNoReadingAndRungFourIsUsedUp()
```

- [ ] **Step 2: Run, expect FAIL** (types not defined): `dotnet test tests/Sobreiro.Tests --filter FullyQualifiedName~Media`
- [ ] **Step 3: Implement the three files** with the values above. `MediaPins.Accent =>
  AccentPalette.WindowsBlue`. Sample readings live here, not in `Snapshot.cs`, which keeps its
  own.
- [ ] **Step 4: Run, expect PASS.** If "1 Dec" is formatted differently by `When`, assert the
  string the app really shows for that date under en-GB, not a new format.
- [ ] **Step 5: Commit** `Media: pinned inputs, sample readings and still names`

### Task 2: Animation timelines

**Files:**
- Create: `src/Sobreiro.Core/Media/Timeline.cs`, `src/Sobreiro.Core/Media/MediaAnimations.cs`
- Test: `tests/Sobreiro.Tests/MediaTimelineTests.cs`

**Interfaces:**
- Consumes: Task 1's `Services`, `SampleReadings`, `MediaPins`.
- Produces:
  - `enum AnimationKind { Hero, Styles, Day, CopilotJoins, Theme }`
  - `sealed record Scene(Services Services, IReadOnlyList<Limit> Limits, FlyoutStyle Style, bool Dark, bool DetailsOpen)`
  - `readonly record struct PointerSpot(double Dx, double Dy)`: offset in 1× px from the pill's centre
  - `abstract record Beat(int Ms)`; `sealed record Hold(Scene Scene, int Ms, PointerSpot? Pointer = null) : Beat(Ms)`;
    `sealed record Glide(Scene Scene, PointerSpot From, PointerSpot To, int Ms) : Beat(Ms)`
  - `sealed record Frame(Scene Scene, PointerSpot? Pointer, int Ms)`
  - `static class Timeline { IReadOnlyList<Frame> Frames(IReadOnlyList<Beat> beats); double Ease(double t); }`
  - `sealed record AnimationSpec(string Name, AnimationKind Kind, int Scale, IReadOnlyList<Beat> Beats, IReadOnlyList<string> GifGrounds, int BudgetKb, int PosterBeat)`
  - `static class MediaAnimations { IReadOnlyList<AnimationSpec> All { get; } IReadOnlyList<AnimationSpec> Hero(DateTimeOffset now); }`

Frames: a `Hold` is one frame. A `Glide` of `Ms` is `n = max(1, round(Ms * 60 / 1000))` frames;
frame `i` (0-based) lasts `round((i+1)·Ms/n) − round(i·Ms/n)` ms with the pointer at
`From + (To − From)·Ease((i+1)/n)`, so the last frame sits exactly on `To`. `Ease` is
ease-in-out cubic. Adjacent frames with the same `Scene` instance and pointer merge, summing `Ms`.

The set (names carry `@1x`/`@2x`): `hero-{claude,both}@1x` (dark; grounds `["dark","light"]`;
budget 500); `styles-{services}-{theme}@{1,2}x` (grounds `[theme]` at 1×, none at 2×);
`day-{services}-{theme}@1x`; `copilot-joins-{theme}@1x`; `theme-{services}@1x` (grounds
`["dark"]`). Budget 300 except the hero. Default holds (tune on the sheet): reading 1500 ms,
style 1600, rung 1000, glide in 450, open hold 2000, glide out 350, closed 800. The hero's
readings: A (13, 44, 2400 credits), B (15, 45, 2644), C (18, 45, 2644), clock = plate time,
so `MediaAnimations.Hero(DateTimeOffset now)` takes the clock; `All` uses `MediaPins.Now` and
the WPF side rebuilds the hero beats with the plate's time.

- [ ] **Step 1: Failing tests**

```csharp
[Fact] public void AHoldIsOneFrame()
[Fact] public void GlideDurationsSumToTheBeat()            // Glide 350 ms → 21 frames, Sum(Ms) == 350, every Ms in 16..17
[Fact] public void GlideEndsExactlyOnItsTarget()           // last frame Pointer == To
[Fact] public void EaseIsZeroHalfAndOneAtTheEnds()         // Ease(0)=0, Ease(.5)=.5, Ease(1)=1
[Fact] public void RepeatedFramesMerge()                   // Hold(s,500) + Hold(s,700) → one frame of 1200
[Fact] public void EveryAnimationHasPositiveBeatsAndAUniqueName()
[Fact] public void OnlyTheHeroHasAPointer()                // no Glide and no Hold.Pointer outside Kind.Hero
[Fact] public void StylesCycleBarsBatteryThermometer()     // Styles beats' Scene.Style sequence
[Fact] public void CopilotJoinsStartsClaudeOnlyAndEndsWithBoth()
[Fact] public void EachCutChangesOneThing()
    // between consecutive beats at most one of: the reading (Limits and Services together), Style, Dark, DetailsOpen
```

- [ ] **Step 2: Run, expect FAIL.**
- [ ] **Step 3: Implement** `Timeline.cs` and `MediaAnimations.cs` as above.
- [ ] **Step 4: Run, expect PASS.**
- [ ] **Step 5: Commit** `Media: timelines for the animations, cuts and one pointer glide`

### Task 3: Render the stills (`--media`)

**Files:**
- Create: `src/Sobreiro/Media/MediaMode.cs`, `src/Sobreiro/Media/MediaViews.cs`,
  `src/Sobreiro/Media/Raster.cs`, `src/Sobreiro/IconBitmaps.cs`
- Modify: `src/Sobreiro/App.xaml.cs:26-30` (route `--media`), `src/Sobreiro/Snapshot.cs`
  (its private `Pixels` moves to `IconBitmaps`)

**Interfaces:**
- Consumes: `MediaStills.All`, `SampleReadings`, `MediaPins`.
- Produces:
  - `MediaMode.Write(string outFolder, string? platePath) : int` (0 ok, 1 failure);
    `MediaMode.Usage() : int` prints `Usage: Sobreiro.exe --media <folder> [--plate <plate.json>]`, returns 2
  - `Raster.Png(FrameworkElement e, int scale) : byte[]` (measure, arrange, `RenderTargetBitmap`
    at `96·scale` dpi, `Pbgra32`, PNG); `Raster.Png(BitmapSource b) : byte[]`
  - `IconBitmaps.Source(RingsModel model, TaskbarColours colours, int size, int scale) : BitmapSource`
    (today's `Snapshot.Pixels` logic, nearest neighbour)
  - `MediaViews.Details(Scene s, BarColourOption colour) : FrameworkElement`,
    `MediaViews.Pill(Scene s) : FrameworkElement`, `MediaViews.Icon(bool copilot, IReadOnlyList<Limit> l, bool dark, int px, int scale) : BitmapSource`
  - `manifest.json` in the out folder: `{ "stills": [{ "path", "group", "dark" }], "animations": [] }`

- [ ] **Step 1:** Move `Pixels` to `IconBitmaps.Source`; `Snapshot` calls it. Run
  `Sobreiro.exe --snapshot <scratch>\sheet.png` before and after and compare file hashes: equal.
- [ ] **Step 2:** `App.xaml.cs`: `--media` routes like `--snapshot` (no tray, no mutex): args
  `--media <folder>` or `--media <folder> --plate <file>`, anything else → `Usage()`.
- [ ] **Step 3:** `MediaMode.Write` sets `CultureInfo.CurrentCulture`, `CurrentUICulture` and
  both `DefaultThread…` to `MediaPins.Culture`, then renders every `DetailsStill` and
  `PillStill` with `SampleReadings.Calm` at `MediaPins.Now`, footer `LiveState.Off`, colours
  `FlyoutColours.For(dark, colour, MediaPins.Accent)` / `TaskbarColours.For(!dark, false, Purple, null)`;
  every `IconStill` via `RingsModel.From` (Claude) or `RingsModel.Copilot` on `Rung(level)`.
  `TaskbarStill`s wait for Task 5. Encode in memory, then write (as `Snapshot.Write` does).
- [ ] **Step 4:** Build and run:
  `dotnet build src/Sobreiro -c Release` then
  `Start-Process -Wait -PassThru -NoNewWindow <exe> '--media', '<scratch>\m'` → ExitCode 0, and
  `(Get-ChildItem <scratch>\m\stills -Recurse -File).Count` = 160 (all but the 4 taskbar stills).
- [ ] **Step 5:** Look at three crops at real size: `both-bars-dark@1x`, `copilot-thermometer-light@2x`
  (corners clean on alpha), `pill/both-dark@1x` composited on `#1C1C1C`. Check a pixel at a
  card corner is partly transparent, not opaque black.
- [ ] **Step 6: Commit** `Media: --media renders the stills`

### Task 4: The taskbar plate model

**Files:**
- Create: `src/Sobreiro.Core/Media/TaskbarPlate.cs`
- Test: `tests/Sobreiro.Tests/TaskbarPlateTests.cs`, fixture `tests/Sobreiro.Tests/Fixtures/plate-placeholder.json`
  (+ a flat `#1C1C1C` `plate-placeholder.png`, 600×48, copied to output)

**Interfaces:**
- Produces:
  - `sealed record TaskbarPlate(string Image, DateTimeOffset CapturedAt, bool Dark, int OriginX, int OriginY, PixelRect Monitor, PixelRect WorkArea, PixelRect Taskbar, int BandTop, int BandBottom, PixelRect Pill, PixelRect? ClaudeIcon, PixelRect? CopilotIcon, int GroundLeft, int GroundRight)`:
    every rect in screen px; `Image` relative to the JSON's folder.
  - `static TaskbarPlate Parse(string json)`: throws `FormatException` naming the first missing field.
  - `PixelRect ClearedPill(int widestWidth)`: `Pill` widened leftwards to `widestWidth`, right edge kept.
  - `(int X, int Y) PillAt(int width, int height)`: right edge on `Pill.Right`, vertically centred in the band (as `PillPlacement.Place`).
  - `(int X, int Y) DetailsAt(int width, int height, int margin)`: `FlyoutPlacement.Place(Monitor, WorkArea, pillCentreX, pillCentreY, width, height, margin)`, `margin` from `FlyoutWindow`.

JSON shape (camelCase, rects as `{left, top, right, bottom}`): `image, capturedAt, dark, origin {x,y},
monitor, workArea, taskbar, band {top,bottom}, pill, icons {claude, copilot}, ground {left,right}`.

- [ ] **Step 1: Failing tests:** `ParsesThePlaceholder`, `ParseNamesTheMissingField` (no
  `pill` → message contains "pill"), `ClearedPillWidensLeftKeepingTheRightEdge`,
  `ClearedPillNeverShrinks` (narrower width → `Pill` unchanged), `PillAtCentresInTheBand`,
  `DetailsSitAboveTheTaskbar` (bottom ≤ `WorkArea.Bottom`).
- [ ] **Step 2: Run, expect FAIL.**  **Step 3: Implement** with `System.Text.Json`.
  **Step 4: Run, expect PASS.**
- [ ] **Step 5: Commit** `Media: the taskbar plate and where the pill and details sit on it`

### Task 5: Render the animation frames and the taskbar stills

**Files:**
- Create: `src/Sobreiro/Media/PlateCompositor.cs`, `src/Sobreiro/Media/ArrowCursor.cs`,
  `src/Sobreiro/Media/AnimationWriter.cs`
- Modify: `src/Sobreiro/Media/MediaMode.cs`

**Interfaces:**
- Consumes: Tasks 2-4.
- Produces:
  - `ArrowCursor.Load() : (BitmapSource Image, int HotX, int HotY)` from
    `%WINDIR%\Cursors\aero_arrow.cur`: read the ICONDIR (type 2), take the 32×32 entry, hotspot
    from the entry's planes/bit-count fields, decode by rewriting the type to 1 and using
    `IconBitmapDecoder`.
  - `PlateCompositor.Frame(TaskbarPlate plate, BitmapSource plateImage, Scene s, PointerSpot? p, int widestPill) : BitmapSource`:
    canvas = plate width × (plate height + room for the details), transparent above the
    taskbar; clears `ClearedPill` and the icon slots with columns copied from
    `GroundLeft..GroundRight`; draws the pill (`PillAt`), icons in captured slots only, the
    details (`DetailsAt`) when `DetailsOpen`, then the pointer at hotspot.
  - `AnimationWriter.Write(AnimationSpec spec, string folder, Func<Scene, PointerSpot?, BitmapSource> render)`:
    `animations/<name>/f000.png…` for distinct frames, `timeline.json` =
    `{ "frames": [{ "file", "ms" }], "poster": <frame index of PosterBeat>, "grounds": [...], "budgetKb": n }`;
    a frame shown twice is stored once.
- Non-hero animations render on a transparent canvas sized to the largest frame, the pill
  bottom-right under the details for `copilot-joins`, the pill and icon side by side for `day`.

- [ ] **Step 1:** `ArrowCursor.Load` and a quick look at the arrow at 4× in a scratch PNG.
- [ ] **Step 2:** Non-hero animations; run `--media`; `timeline.json` counts match
  `Timeline.Frames(...).Count` for two animations (check by hand against a test run).
- [ ] **Step 3:** Hero and taskbar stills against the placeholder plate:
  `--media <scratch>\m --plate tests\Sobreiro.Tests\Fixtures\plate-placeholder.json`.
- [ ] **Step 4:** Look at three hero frames (closed, pointer mid-glide, open) as small crops:
  details above the pill, pointer tip on the pill, no seam where the pill was cleared.
- [ ] **Step 5:** Manifest lists the animations.
- [ ] **Step 6:** Without `--plate`: hero and taskbar stills skipped with one stderr line,
  exit 0, everything else written. With a plate JSON missing `pill`: exit 1, message names it.
- [ ] **Step 7: Commit** `Media: animation frames, the hero on the taskbar plate`

### Task 6: Encode, contact sheet and the one command

**Files:**
- Create: `scripts/media.ps1`, `scripts/encode-media.py`, `scripts/requirements-media.txt`
  (`pillow==11.3.0`),
  `scripts/test_encode_media.py`, `media/picks.json` (`{}`)
- Modify: `.gitignore` (`scripts/.venv-media/`)

**Interfaces:**
- `encode-media.py <work> <out>`: copies `stills/`; per animation writes
  `animations/<name>.webp` (lossless, `loop=0`, per-frame `duration`, alpha kept),
  `animations/<name>-<ground>.gif` per ground (each frame alpha-composited on the ground
  colour, `quantize(255, MEDIANCUT, dither NONE)`, `disposal=1`), `animations/<name>-still.png`
  (the poster); writes `index.html`; prints each animation's size and `OVER BUDGET` where it is.
- `media.ps1 [-Python <exe>] [-SkipBuild]`: checks Python first; creates the venv and installs
  the requirements if absent; builds Release; runs the exe with `Start-Process -Wait -PassThru
  -NoNewWindow` (a WinExe returns at once under `&`) passing `--plate media\plates\taskbar-dark.json`
  when it exists; encodes into a staging folder; replaces `media\stills`, `media\animations`
  and `media\index.html` only (never `plates\` or `picks.json`); then copies each `picks.json`
  entry `{ "site/assets/img/<file>": "<media path>" }`. Any failure: exit 1, `media\` untouched.
- `index.html`: grouped as the spec says (details by services, pill, icons, taskbar,
  animations), real size (`width`/`height` from the 1× file), file name under each, dark and
  light grounds side by side, pills on `#1C1C1C`/`#F0F0F0`, links `../site/assets/foundation.css`.

- [ ] **Step 1: Failing Python test** (`python -m unittest scripts/test_encode_media.py` in the
  venv): three 8×8 frames (one with a transparent corner), durations 100/200/300 →
  `webp.n_frames == 3`, durations read back, `info["loop"] == 0`, the corner alpha 0 in the
  WebP and equal to the ground in the GIF; a 600 KB budget breach prints `OVER BUDGET`.
- [ ] **Step 2: Run, expect FAIL.**  **Step 3: Implement `encode-media.py`.**  **Step 4: PASS.**
- [ ] **Step 5:** Write `media.ps1`; run it end to end. Expect exit 0, `media\index.html`, and
  sizes printed.
- [ ] **Step 6:** Run it again and compare SHA-256 of every file under `media\stills`: identical.
- [ ] **Step 7:** `media.ps1 -Python C:\nope\python.exe` → exit 1, says Python was not found,
  `media\` timestamps unchanged.
- [ ] **Step 8:** Open `media\index.html` in the browser pane; one screenshot of each group
  at the sizes that matter, then delete the screenshots.
- [ ] **Step 9: Commit** the scripts, `.gitignore`, `picks.json` and the generated `media/`
  (`Media: one command renders every picture into media/`).

### Task 7: Capture the real taskbar plate (needs Ricardo's PC, Sobreiro running with its pill)

**Files:**
- Create: `scripts/capture-taskbar.ps1`, `media/plates/taskbar-dark.png`, `media/plates/taskbar-dark.json`

**Interfaces:** writes the JSON shape of Task 4.

- [ ] **Step 1:** Write the script: DPI-aware; `FindWindow(null, "Sobreiro pill")` for the pill
  rect; `Shell_TrayWnd` for the taskbar; `MonitorFromWindow`/`GetMonitorInfo` for monitor and
  work area; UI Automation buttons under the taskbar whose `Name` starts with `Session `, or
  `Sobreiro` (Claude icon) and `AI credits` (Copilot icon) for icon slots; band from the pill's
  top and bottom; `CopyFromScreen` of the taskbar from 480 px left of the pill to its right
  end; the ground = the widest run of identical columns left of the pill. If the pill window is
  missing, stop with "Start Sobreiro with the pill on, then run this again."
- [ ] **Step 2:** Run it, show Ricardo the plate PNG (it shows his clock and tray icons) and
  commit only on his yes. Then `media.ps1` again and look at the hero frames on the real plate.
- [ ] **Step 3: Commit** `Media: the taskbar plate, captured once`

### Task 8: Ricardo picks; README, site and docs

Not before Ricardo has looked at `media/index.html` and said what goes where. The starting
proposal is the spec's "Where the pictures go".

**Files:**
- Modify: `README.md` (hero; "What it looks like": Claude only, Copilot only, both, then styles,
  dark/light, purple/accent, tray only/with pill; "Building from source": one line for
  `scripts/media.ps1` and `--media`), `site/index.html` (hero, the panel's switch gains
  Claude/Copilot/Both, one line naming GitHub Copilot's AI credits), `site/manual/index.html`
  (a still in the Copilot section), `media/picks.json`, `docs/plan.md` (the site bullet: the
  pictures are rendered by `scripts/media.ps1`; to refresh them, run it)
- Delete: `docs/images/readme-demo.*` and old `site/assets/img/panel-*.png`, `demo.*`, once
  nothing points at them.

- [ ] **Step 1:** Fill `picks.json`; run `media.ps1`; the picks land in `site/assets/img/`.
- [ ] **Step 2:** Edit the README and pages with the `landing-and-readme` and `page-design`
  skills; alt text in plain words; `<picture>` with the still for reduced motion.
- [ ] **Step 3:** `node scripts/check-site.mjs` passes; preview the site with
  `python -m http.server` from `site/` in the browser pane, light and dark, viewport crops only.
- [ ] **Step 4:** Update the memory note `product-pages-real-captures`: rendered media is now
  preferred over screen captures, with why and how (outside the repo).
- [ ] **Step 5: Commit** `README and site: rendered pictures, with Copilot shown`
