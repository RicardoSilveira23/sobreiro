# Rendered media: Sobreiro draws its own pictures

Status: design, 2026-11-10. Branch `claude/rendered-media`, cut from `claude/copilot-usage`
(PR RicardoSilveira23/sobreiro#15); it merges after #15.

## Why

`Sobreiro.exe --snapshot` draws the details, the pill and the tray icons with the app's own WPF
views and embedded fonts. Ricardo found those renders very high fidelity and wants them as the
source of every product picture: the README, the site, the Sobreiro tile on his website's wall,
his posts. Screen recording needs him at the mouse, a backdrop window that has to be keyed out
(the key ate the panel's corners once), and a new recording after every change. A renderer is
repeatable: change the app, run one command, and every picture follows.

The second reason: the Copilot support (PR #15) is barely visible. The README and the manual
describe it in words; the home page never mentions it, and no picture anywhere shows it. People
should see at a glance how Sobreiro looks with Claude only, with Copilot only and with both, and
then what the options change.

## Goals

- One command, `scripts/media.ps1`, regenerates every picture into one folder, `media/`, and
  copies the site's picks into `site/assets/img/`.
- **Stills**, each a file of its own with a name that says what it shows, at 1× and 2×, on a
  transparent background: the details in Bars, Battery and Thermometer, dark and light, for
  Claude only, Copilot only and both; the pill; the tray icons.
- **Animations**, animated WebP and GIF, made of frames from the same renderer.
- A contact sheet, `media/index.html`, showing the whole set at real size, for Ricardo to pick
  from and for later reuse.
- The README and the home page show, clearly: how it looks with Claude, with Copilot, with both,
  then the options (style, bar colour, what to show, dark and light). The home page says it
  covers GitHub Copilot's AI credits.

## Non-goals

- Any motion the app does not make. Decided with Ricardo: animations show only real moments, and
  every change is a cut, as in the app. No bars filling, no ring sweeps, no sliding panel, no
  crossfades. The one continuous motion is the pointer, which a user really moves.
- Any change to how the running app looks or behaves.
- Placing pictures on Ricardo's website (the wall tile, posts). The set serves them later;
  that work happens there.
- Animations for the manual. Stills only there, where a page needs one.

## Anti-goals (a failure even if it works)

- A picture that differs from what the app shows. Every surface comes from the app's own view
  code with its real colours and fonts; nothing is redrawn by hand.
- A drawing of Windows: no fake taskbar, no fake right-click menu, no fake cursor. Where Windows
  appears, it is a real capture (the taskbar plate) or a real system asset (the arrow pointer).
- Pictures that change with the PC they are made on: accent colour, culture, time zone and the
  clock are pinned, so two runs give the same pictures.
- Jagged or haloed edges on any page ground, dark or light.

## Constraints

- Project rules (`CLAUDE.md`): logic goes in `Sobreiro.Core` with an xUnit test. The media mode,
  like `--snapshot`, starts no tray icon, takes no mutex, makes no network call and reads no
  credentials or live data: sample readings only.
- The site publishes only `site/` and loads nothing from other hosts, so its pictures must be
  copied into `site/assets/img/`.
- Ricardo's rules: plain words in alt text ("the details", not "flyout"); visual choices are made
  from real-size variants side by side; reduced motion gets a still.
- His display runs at 100%, so a capture of his taskbar exists only at 1×.

## Approaches

- **A. Inside the app, beside `--snapshot` (picked by Ricardo, 2026-11-10).** A new
  `Sobreiro.exe --media <folder>` mode renders every still and every animation frame as PNG with
  the same views `--snapshot` uses. `scripts/media.ps1` builds, runs it, and encodes the
  animations with Pillow in a Python virtual environment the script creates on first run (git
  ignores it). The catalogue of pictures (names, sample readings, timelines) lives in
  `Sobreiro.Core` with tests. Costs a few hundred lines in the shipped exe, as `--snapshot`
  already does, and Python to regenerate.
- **B. A separate tool project**, rendering and encoding in C# with ImageSharp. One language and
  nothing extra in the exe, but the views find their fonts through `pack://application:,,,/`,
  which resolves to the running program, so they would need changing to render from another
  program; and ImageSharp's licence carries conditions.
- Dropped: encoding inside the app (ships an encoder to every user).

## Decided with Ricardo

- Only real moments, as cuts.
- Approach A.
- Clear pictures of Claude only, Copilot only and both, plus the other options, in the README and
  on the site; the home page names Copilot's AI credits.
- Proposed and not objected to: building on `claude/copilot-usage`; a real capture of his taskbar
  under the hero animation; one `media/` folder with the site's picks copied into `site/`; GIFs on
  a solid ground, WebP transparent.

## The design

### The command

`powershell -File scripts/media.ps1` (works in Windows PowerShell 5.1 and pwsh):

1. `dotnet build src/Sobreiro -c Release`.
2. Run `Sobreiro.exe --media <work>`, `<work>` a temp folder. It writes every still as PNG, and
   for each animation its distinct frames as PNG plus a `timeline.json` (frame file and duration
   in ms, in order). A cut is one frame held; only pointer motion needs one frame per 1/60 s.
3. Run `scripts/encode-media.py <work> <out>` in `scripts/.venv-media` (Pillow pinned in
   `scripts/requirements-media.txt`). For each animation: `<name>.webp` (lossless, alpha, loops
   forever), `<name>.gif` on a solid ground, `<name>-still.png` (the poster for reduced motion).
4. Write `media/index.html`, the contact sheet.
5. Swap the result into `media/` whole, so pictures that are no longer made disappear; if any step
   fails, `media/` is left as it was and the script exits non-zero.
6. Copy each entry of `media/picks.json` (site file name → media file) into `site/assets/img/`.

`Sobreiro.exe --media` with the wrong arguments prints its usage and exits 2, like `--snapshot`.

### Pinned inputs

- The clock: one fixed instant (2026-11-10 14:00), except the hero, which uses the taskbar
  plate's capture time so its readings agree with the clock in the picture.
- Time zone `GMT Standard Time` (Lisbon), culture `en-GB` (24-hour times, "1 Dec"), set as the
  process's culture in media mode so views that read the current culture agree.
- Accent: `AccentPalette.WindowsBlue`, not this PC's accent.

### Sample readings

- Claude: a calm reading where both rows read On track (today's site shows session 15%, weekly
  45%).
- Copilot: 2,644 of 15,000 AI credits (17%), resetting on 1 December, read live (no cache note).
- The footer: Live usage off (`LiveState.Off`), no legacy connection, the reading taken one
  minute before the pinned clock. Every details still uses this footer, the `-accent` ones too
  (not the "Refused" sentence `--snapshot` puts on its accent columns). (My call: the footer
  most people see.)
- For the pill, the tray icons and the "day" animation, a ladder of readings: no reading yet, low,
  middle, high, used up.

### Stills

Under `media/stills/`, every file at `@1x` and `@2x`. 2× is the same layout rendered by
`RenderTargetBitmap` at 192 dpi, with the views' own `UseLayoutRounding`; nothing else changes.

The details card brings its own ground, a 1 px border and 10 px rounded corners, and no shadow
(the app's window is layered and transparent, so Windows adds none). A render of the card alone
therefore has clean alpha corners, and the stills carry no shadow.

- `details/<services>-<style>-<theme>[-accent]@<n>x.png`: services `claude`, `copilot`, `both`;
  style `bars`, `battery`, `thermometer`; theme `dark`, `light`; purple bars, plus `-accent`
  (Windows accent) for `both` in every style and theme. 24 pictures, 48 files.
- `pill/<services>-<theme>@<n>x.png`: the pill alone, transparent. At rest its face is the ink
  colour at alpha 1 (there only so clicks land on it), so the still is in effect its text, rings
  and level bar on a clear ground. That ink is chosen for a taskbar behind it, so the contact
  sheet shows these on the taskbar's colour, and they are meant to be placed on it.
- `icons/<claude|copilot>-<level>-<theme>-<px>.png`: the tray icon at each level of the ladder,
  drawn for a dark and a light taskbar, at 16, 20, 24 and 32 px (Windows at 100, 125, 150,
  200%), transparent; `@1x`/`@2x` does not apply. Plus `-x8` nearest-neighbour enlargements of
  the 16 px icon for posts.
- `taskbar/<show>-<services>.png`: the "Show" option on the plate: tray icon only, and tray icon
  and pill. 1× only (the plate's resolution).

### Animations

Only states the app really shows, held and cut. Holds are the builder's to tune (about 1.2 to
2 s each); Ricardo judges them on the contact sheet.

1. **`hero-<claude|both>`**, on the taskbar plate: the pill shows a reading; a new reading lands
   (the numbers and rings step); the pointer moves to the pill and clicks; the details appear
   above it where the app puts them (`FlyoutPlacement`); a new reading lands while they are open,
   and both update; the pointer clicks away and the details close. Loops. Replaces today's
   recorded demo. The canvas is the right part of the plate plus enough transparent room above it
   for the details, as in today's demo.
2. **`styles-<services>-<theme>`**: the details as Bars, then Battery, then Thermometer, looping.
   The motion version of the site's style switch.
3. **`day-<services>-<theme>`**: the pill and the tray icon through the ladder of readings, up to
   used up, then the reset.
4. **`copilot-joins-<theme>`**: the pill and the details with Claude only, then a GitHub sign-in
   is found and Copilot appears beside Claude.
5. **`theme-<services>`**: the details as Windows switches from dark to light and back.

Animations render at 1×; `styles` also at 2× for sharp screens. The hero is 1× only, as its plate
is. GIFs are made at 1× only, on the page's ground for their theme (the Night `--bg`: `#0b0b0b`
dark, `#ffffff` light); the hero's GIF comes in both.

Size: the hero WebP stays under about 500 KB (today's recorded `demo.webp` is 420 KB) and every
other animation under about 300 KB; the canvas is trimmed to what moves and the frames around
it. If one goes over, the plan trims its canvas or its pointer frames, not its colours. (My
call.)

### The pointer

Windows' default arrow, read from `%WINDIR%\Cursors\aero_arrow.cur` (its 32 px image, the 100%
size) with the hotspot the file gives, and no shadow. Taking the file rather than the current
cursor keeps the picture the same whatever scheme or shadow setting the PC has. It appears only
in the hero, moves to the pill along an eased path at 60 frames a second, and stays still while
the details are open. (My call.)

### The taskbar plate

- `media/plates/taskbar-dark.png` and `taskbar-dark.json`: one capture of the right part of
  Ricardo's taskbar (dark), made by `scripts/capture-taskbar.ps1` with Sobreiro running and its
  pill showing. The script records where the pill and Sobreiro's tray icon are (the pill window's
  rectangle; the icon's from `Shell_NotifyIconGetRect`), the screen's work area and the capture
  time.
- To composite, the renderer clears the pill's and the icon's rectangles with taskbar ground
  copied from an empty stretch of the same plate (the taskbar is flat across), then draws the
  rendered pill and icon there.
- The pill's width changes with the reading and the services. The cleared rectangle is the
  captured pill's rectangle widened to the widest pill the animation draws, and each pill is
  anchored the way `PillHost` anchors it on the real taskbar. Sobreiro's tray icons are redrawn
  in the slots the capture found; a slot that was not on the taskbar when it was captured (the
  Copilot icon, say, which Windows may keep in the overflow) is not invented. (My call.)
- Until the real plate exists, the hero is built and tested against a placeholder: a flat strip
  with a JSON of made-up rectangles, kept in the test fixtures and never written to `media/`.
  Capturing needs no clicks, only Sobreiro running with its pill on Ricardo's PC; it is the last
  step before the hero is shown to him. (My call.)
- The capture shows his clock and other tray icons, so he approves the plate before it is
  committed. A light plate is added the same way if he wants a light hero; until then the hero is
  dark only.
- The plate is the one input that does not regenerate; `media.ps1` uses the committed plate and
  never captures.

### The contact sheet

`media/index.html`, opened from disk: every still and animation at real size, grouped as above,
each with its file name, on dark and light grounds side by side, the pills on the taskbar's
colour. Plain HTML, no script needed, linking `../site/assets/foundation.css` for type and
ground.

### Where the pictures go (a starting proposal; Ricardo picks from the sheet)

- README: the `both` hero in place of `docs/images/readme-demo.*`; a "What it looks like" part:
  Claude only, Copilot only, both (Bars, dark), then the three styles, dark and light, purple and
  accent, tray icon only and with the pill. Paths point into `media/` directly.
- Home page: the `both` hero in place of `demo.webp` and `demo-still.png`; the panel with the
  style switch gains Claude, Copilot and both; one line saying it covers GitHub Copilot's AI
  credits.
- Manual: a still in the Copilot section.
- `docs/images/readme-demo.*` and the old `site/assets/img/panel-*.png` go when nothing points
  at them.

### Code layout

- `Sobreiro.Core/Media/`: the catalogue (every still and animation: name, services, style, theme,
  bar colour, scale, sample readings), the pinned clock, zone and culture, the reading ladder,
  the timelines (states and holds, the pointer path), and the file naming. All pure, all tested.
- `Sobreiro/Media/`: the WPF side: turns a catalogue entry into a view (the same `FlyoutView`,
  `PillView`, `IconRenderer` calls `Snapshot` makes), renders it at a scale, composites hero
  frames on the plate, writes PNG and `timeline.json`. `App.xaml.cs` routes `--media` as it routes
  `--snapshot`.
- `scripts/media.ps1`, `scripts/encode-media.py`, `scripts/requirements-media.txt`,
  `scripts/capture-taskbar.ps1`. `.gitignore` gains `scripts/.venv-media/`.

### Tests

In `Sobreiro.Tests`: the catalogue holds every combination named above and no duplicate names;
names match the pattern; Copilot-only entries give no Claude rows and `both` gives both; the
pinned clock, zone and culture give the expected strings (for example "1 Dec"); timelines have
positive holds and the pointer path runs at 60 fps and ends on the pill; every `picks.json`
entry names a picture the catalogue makes. The WPF rendering is checked by running the command
and looking at the sheet.

### Docs

- `docs/plan.md`: the site's "pictures are real captures ... there is no tool" bullet becomes:
  the pictures are rendered by `scripts/media.ps1`; to refresh them, run it.
- README "Building from source": one line for `--media` and `scripts/media.ps1`.
- The memory note `product-pages-real-captures`: rendered media is preferred over screen
  captures.

## Left to the builder

- Exact holds, the pointer's path and easing, and the hero's canvas size.
- Whether a frame repeated across a cut is stored once (it should be).
- The contact sheet's layout within the grouping above.
- How `encode-media.py` quantizes GIFs, as long as edges are clean on their ground.
- Trimming the set if a group turns out to add nothing, with a note in the plan.

## As built (2026-11-10)

Rulings made during the build that change what the text above says:

- The pointer moves up into the open details and closes them with a click on the desktop (R13, from Ricardo).
- The tray icon is drawn at the plate's small-icon size, 16 px at 100% scale (R10, from Ricardo).
- Icon slots on the plate come from UI Automation button names, not `Shell_NotifyIconGetRect`.
- The hero and the taskbar stills are cropped to what they show (R14).
- The README hero is baked from the plate's real pixels, with no drawn taskbar (R15); it uses the run's own plate.
- The site's picks are validated at run time by `media.ps1`, not by an xUnit test. Only a run with the committed plate refreshes the site and the README hero; a run with no plate or another plate rebuilds `media/` only.
- The site's panel gained a Claude Code / Copilot / Both switch and an options row (R16-R17).
- GIF delays are regrouped to multiples of 10 ms of at least 20 ms (browsers show 10 ms or less as 100 ms), and the ground colour is pinned in each GIF palette.
- The swap's stage sits inside `media/` (`.stage-*`, git-ignored) so every move is a rename; on a failure only what moved in is removed and each original is put back.
