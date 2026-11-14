# Shell comparison, on paper

Written 2026-11-13 for step 1 of [roadmap.md](roadmap.md#step-1-compare-on-paper): the candidates
for the next shell, scored on the roadmap's seven points. Nothing was built for the table. It
ends in a [shortlist](#shortlist) and in
[what it leaves open](#what-this-leaves-open-for-the-spikes) for the spikes. Each spike of step 2
puts its figures in its row and a note under
[What the spikes measured](#what-the-spikes-measured).

The decision taken from it, on 2026-11-14, is in [roadmap.md](roadmap.md#the-decision): B.

## The bar

Measured on 2026-11-13, in [baseline.md](baseline.md) (the 1.0.0 row and the note "Taskbar asked
on change"):

| The current shell | Private memory | CPU per idle hour |
|---|---|---|
| As shipped, pill shown | 94 to 97 MB | 14.9 s (7.3 s with the taskbar asked on change) |
| Same exe, pill switched off | 19 MB | 1.4 s |

So .NET, Core, the polling and the tray icon together idle at 19 MB. The other 75 to 78 MB is
the WPF pill window and its questions to UI Automation. Two things follow:

- A new shell is judged on what its pill window costs, not on its language. "Is the pill
  possible, and at what cost" is the point that matters most below.
- The roadmap's decision rule asks for 40% less private memory than 1.0: **58 MB or less**. A
  candidate that needs Core rewritten must also reach half the memory of the one that keeps
  Core.

## How to read the table

Each figure carries one letter:

- **M** measured on this project, as [baseline.md](baseline.md) describes.
- **P** published by the toolkit, or by the maker of the system, in its own documentation. Read
  on 2026-11-13.
- **R** reported by someone else: one source, another app, another PC. Not checked here.
- **E** an estimate made for this table. The reasoning is in the notes under the table.

No source gave a measured idle figure for a small Windows app in any of the toolkits, so every
memory, size and start figure of a candidate that does not exist yet is an **E**.

## The table

| Candidate | Idle memory, exe size, start | The pill | Tray, menu bar, notifications | Core and its tests | Toolchain, CI, signing | Licence | Maintainer |
|---|---|---|---|---|---|---|---|
| **A. C# with Avalonia, Native AOT** (spiked) | 26 MB after 10 minutes with software rendering, 2 to 3 MB more once the flyout has been opened; 103 MB with GPU rendering (M). Hidden copies level off at 33 MB, or rest at 16 to 17 MB with a collection every 30 seconds (M); exe 34.9 MB in three files (M); start 0.06 s (M) | Built, and seen inside the taskbar (M): owned by it, transparent per pixel, takes no focus. Mixed scaling not checked. The toolkit sets a window's owner and style itself when it shows it, so ours are put back afterwards. Native code: 27 imports against today's 34 (M) | Windows: icon, tooltip, click and a menu the toolkit draws itself, built (M). macOS: icon and menu, no text in the menu bar (P). Linux: on desktops with a status notifier, GNOME needs an extension (P). No system notifications (P) | All of it: 4,220 lines and 956 tests, untouched, and published with Native AOT without a warning (M) | No new language. A build on each system, with that system's linker (P). The toolkit's build step reports usage to its maker unless a variable is set (P) | MIT (P). Fits | Knows C# and .NET. The toolkit is new to him; the spike wrote its windows in C#, as today's shell does |
| **B. Rust with Slint** (spiked) | 7 MB after 10 minutes with software rendering, about 1 MB more once the flyout has been opened; 246 MB with GPU rendering (M). A resting level: nothing in it collects garbage (M); exe 9.3 MB in one file (M); start 0.06 s (M) | Built, and seen inside the taskbar (M): owned by it from birth, transparent per pixel, takes no focus. Mixed scaling not checked. Being born owned goes through a hook the toolkit marks as unstable (P). Its tooltip opens inside the pill's own window (M). Native code: 36 calls in all against today's 34 imports (M) | Windows: icon, tooltip, click and the system's own menu, built (M); each icon needs its picture before the app's loop starts (M). macOS: the same, plus text in the menu bar (P). Linux: status notifier only, GNOME needs an extension (P). No notifications (P) | None of the code. For the spike, 3,148 of its lines were rewritten as 4,426 lines of Rust, with 518 tests passing and the JSON fixtures reused (M). Not reviewed line by line | A second toolchain, installed in a minute (M). 261 crates from the public registry in the build (M). A build on each system | GPL 3 (P). Fits; the crates in the build are permissive or GPL 3 (M) | Has not worked in Rust |
| **C. C++ with Qt** | 15 to 35 MB (E); 49 MB in 19 files (R), or one exe of 15 to 25 MB (E); start under 0.3 s (E) | Possible on paper: frameless translucent window, window handle, per-monitor DPI by default (P). Being owned by the taskbar is not documented. Native code: all of today's again, in the language those calls are written for | The longest-proven. Icon, tooltip, menu, click and a message balloon on all three (P). macOS: no text in the menu bar (P). Linux: status notifier or the older tray, limits on GNOME (P) | None of the code. 4,220 lines and 956 tests rewritten; fixtures reused | The heaviest: a build system, the toolkit installed on each CI runner, a deployment tool per system; one exe means compiling the toolkit | LGPL 3 (P). Fits | Has not worked in C++ or Qt |
| **D. Go with a small tray and window library** | 10 to 25 MB (E); exe 8 to 15 MB (E); start under 0.2 s (E) | Only by hand: no small library was found that draws a transparent window, so the window, the drawing and the text are native calls. The most native code of all | Tray libraries: icon, menu, click on all three; title text on macOS and Linux only, tooltip on macOS and Windows only (P). Notifications from another library | None of the code. 4,220 lines and 956 tests rewritten; fixtures reused | The simplest on Windows. A C compiler on macOS, and on Linux for some libraries (P), so a build on each system anyway | Apache 2.0 and BSD libraries (P). Fit | Has not worked in Go |
| **E. A native shell per system** | Windows, today's shell: 94 to 97 MB, 168.9 MB, 0.40 s (M). macOS: 13 MB for a plain menu bar app (R) | Exists (M). Native code: what is there | The best fit on each: today's on Windows, a menu bar item with text on macOS (P), a status notifier on Linux | All of it on Windows. Elsewhere: Core as a native library under the system's shell (P for the library, not found for calling it from Swift), or rewritten there | Three toolchains and three shells to keep. The most code | No toolkit to license. Fits | C# yes; has not worked in Swift |
| **F. C#, the pill on a plain system window** (added here; spiked) | 27 MB until the flyout is first opened, 94 MB from then on (M); exe 170.2 MB (M); start 0.25 s (M) | Built, and seen inside the taskbar (M): owned by it, transparent per pixel, takes no focus. Mixed scaling not checked. The rings and text are drawn by hand. Native code: 15 more imports and about 280 more lines than today (M) | Today's on Windows. Not a shell for other systems: there it becomes E | All of it | Nothing new | Nothing new. Fits | Knows C#; the drawing calls are new to him |

Versions read: Avalonia 12.1.6 (2026-11-09), Slint 1.18.1 (2026-09-21), Qt 6.12.0 (2026-09-30),
.NET 10.

## What the spikes measured

One note per spike of [step 2](roadmap.md#step-2-build-the-shortlist), added as each is run.

### F. C#, the pill on a plain system window

Spiked on 2026-11-13, on the PC the baseline rows come from. In short: the pill costs about 5 MB
on a plain window where it costs about 60 MB on WPF, and the app idles at 27 MB instead of
94 MB, until the flyout is opened once. The flyout is still WPF, and from its first opening the app
sits at 94 MB again.

**What was built.** The 1.0.0 tray with only the pill taken off WPF, on the branch
`spike/shell-f-plain-window`, which is never merged.

- The pill is a window of a class the app registers itself: layered, so every pixel carries its
  own transparency; owned by the taskbar; marked as never taking focus; placed in physical
  pixels by the same tracker as today.
- Its face is drawn by hand with the system's drawing library (GDI+), in the app's own fonts
  read from the exe: the rings, the labels and figures, the stale mark, Copilot's block and
  bar, and the lighter ground under the pointer.
- The taskbar is asked through UI Automation's own COM interface, not through WPF's managed
  client. A second plain window hears the taskbar being made again.
- The tray icon, the menu, the flyout, Core and the bridge are untouched.
- Switches in the environment bring back the WPF window or the managed client, so one exe gave
  every comparison below.

**The taskbar takes the window.** Checked first, before any drawing, with a test window: its
owner is the taskbar, it lies wholly inside it and above it, the foreground window did not
change, and a screenshot shows the taskbar through the clear part of the picture. Then the
spike itself, with every other Sobreiro on the PC closed, seen in three screenshots: the pill in
its place left of the tray icons; its ground lighter with the pointer on it, the foreground
window unchanged; and the flyout open beside it. A script put the pointer there and sent the
click. Nobody clicked it by hand, and the menu on a right click, a full-screen app, a hidden
taskbar and a restart of the taskbar were not tried.

**The baseline run**, beside the row it is measured against, both in
[baseline.md](baseline.md) under Experiments:

| | 1.0.0, taskbar asked on change | The spike |
|---|---|---|
| Private memory | 94 MB | 27 MB |
| Working set | 170 MB | 96 MB |
| CPU per idle hour | 7.3 s | 5.4 s |
| First pill, first start | 0.48 s | 0.33 s |
| First pill, later starts | 0.39 s | 0.25 s |
| Sobreiro.exe | 168.9 MB | 170.2 MB |

27 MB is 72% under 1.0's 97 MB, where the decision rule asks for 40%. The run never opens the
flyout.

**After the flyout has been opened once, the memory is back.** A copy of the spike with its
pill shown stood at 24.5 MB. Its flyout was opened and closed once: 94 MB a few seconds later,
and 92 to 94 MB in each of the five minutes after. So 27 MB is the figure for an app whose
flyout has not been opened since it started. Keeping it needs the flyout off WPF too, or in a
process of its own that ends when the flyout closes. Neither was tried.

**What the pill's memory is made of.** Seven copies of the spike's exe ran side by side for 7
minutes, each on its own data folder, signed out of Claude, their pills hidden because an
installed pill had the place. Private memory at the end of minute 7:

| Pill window | The taskbar is asked through | Private memory |
|---|---|---|
| None: pill switched off in the config | nothing | 21 MB |
| Plain | nothing | 23 MB |
| Plain | the system's interface | 25 MB |
| Plain | WPF's managed client | 28 MB |
| WPF | nothing | 76 MB |
| WPF | the system's interface | 79 MB |
| WPF | WPF's managed client | 80 MB |

- The WPF window is about 53 MB, whichever way the taskbar is asked. That is the pill's cost.
- UI Automation is 3 to 5 MB: about 3 MB through the system's interface, and 1 to 3 MB more
  through the managed client. Both gave the same answer about the taskbar.
- The plain window, its picture and the drawing library are about 2 MB together.
- Processor time was 2.0 to 2.2 s per idle hour in the copies that asked nothing and 3.6 to
  4.4 s in those that asked, with no difference between the two windows or the two clients
  that Windows can count. Four copies from another measurement ran beside these seven, so
  these figures compare the seven with each other and are not a baseline.

**How much code.** Counted on the spike's branch against `main`:

| Part | Today, on WPF | The spike |
|---|---|---|
| The face | 201 lines | 241 lines |
| The window | 181 lines, 7 native imports | about 370 lines, 22 native imports |
| The question to UI Automation | 52 lines | about 95 lines |
| The pill in all | 877 lines | about 1,160 lines |
| Native imports in the tray app | 34 | 49 |

The hand-drawn face is within one pixel of the WPF face in width and height at 100%, 150% and
200%, in both themes: the two were rendered side by side into one picture and compared by eye.

**Whether the exe can shrink while the flyout stays WPF.** A little, safely. Published to a
folder instead of one file, the same build is 174 MB without the bridge: base class libraries
50 MB, WPF 45 MB, the runtime 31 MB, Windows Forms and its drawing 27 MB, the frameworks'
messages in other languages 17 MB, Sobreiro itself 3 MB.

- Leaving out the other languages is a supported setting: 152.8 MB against 170.2 MB for the
  same build with them. That exe ran as a tray copy for half a minute; its flyout was not
  opened.
- Trimming unused code is refused by the SDK for a project that uses Windows Forms or WPF.
  Forced past the refusal it gave 100.9 MB; that exe started and idled like the others, but
  its question to UI Automation stopped answering and its flyout was not tried. Not a path to
  ship on.
- Compression is the measured trade in [baseline.md](baseline.md): 73.7 MB on disk for 83 MB
  more memory.
- Windows Forms is there for the tray icon and its menu. Taking it out would save up to 27 MB
  (E) and means that much more code by hand.

**Where it fell short.**

- The flyout, above: one opening and the saving is gone until the app is started again.
- Screen readers get less. The WPF pill gives UI Automation its two lines as text ("5h" and
  "7d" with their figures). The plain window gives a bare pane with a title and nothing
  inside. Saying more means writing a UI Automation provider by hand, and a hand-drawn flyout
  would need one for every row and button.
- The tooltip is the system's plain one. It opened over the pill, inside the taskbar, where
  today's opens above the taskbar.
- Mixed scaling was not checked: the PC has one display at 100%. The window is sized in
  physical pixels from the taskbar's own scale, and the face was drawn at 150% and 200% only
  as pictures.
- 5.4 s of processor time per idle hour is one run against one run of 7.3 s, and still over
  the 5 s target.
- The spike's exe carries the three font files twice, 1.3 MB: once for WPF and once for the
  hand-drawn face.
- It is not a shell for macOS or Linux.

### A. C# with Avalonia, Native AOT

Spiked on 2026-11-13, on the same PC. In short: with the toolkit told to render in software,
the whole shell idles at 26 MB with its pill shown, where today's idles at 103 MB, and opening
the flyout adds 2 to 3 MB. With the toolkit's default, rendering through the graphics card, it
idles at 103 MB. It publishes with Native AOT without a warning and shows its pill 0.06 s after
it is started. The 26 MB still holds garbage the runtime has not collected: hidden copies
level off at about 33 MB, and rest at 16 to 17 MB when the app asks for a collection every 30
seconds.

**What was built.** A new project beside today's, on the branch `spike/shell-a-avalonia`, which
is never merged. It links Core as it is and builds only the tray icon, the pill and the flyout.

- The pill is one of the toolkit's windows: borderless, clear per pixel, never activated. Its
  face is built from the toolkit's controls to the WPF face's measures, in the app's own fonts:
  the rings, the labels and figures, the stale mark, Copilot's block and bar, the lighter
  ground under the pointer, and a tooltip.
- It is placed by today's tracker, moved onto the toolkit's timer and dispatcher, with the same
  native calls and the same placement from Core.
- The taskbar is asked through UI Automation's own COM interface. Native AOT has no built-in
  COM, so the four interfaces are declared for the compiler to generate at build time.
- The flyout is a second window of the toolkit, made the first time it is opened: the card with
  bars, battery or thermometer, placed by Core beside the point of the click, closed by Esc or
  by losing the focus.
- The tray icons, their tooltips, the click and a short menu are the toolkit's own. The rings in
  the icon are drawn with the toolkit.
- Core reads, polls and words everything, as today, with this PC's sign-ins.
- Left out: Connect and Disconnect, the alerts, the settings in the menu other than Style, the
  first-run migration, the bridge inside the exe, the log, and the events for a change of
  theme, clock or display (a redraw every 30 seconds stands in). The spike refuses to start
  without a data folder of its own.
- A switch in the environment picks software or GPU rendering, so one exe gave both rows below.
  Others, added after the baseline run, pick how UI Automation's objects are released, which
  thread asks, and whether the app asks for a collection.

**The taskbar takes the window.** Checked first, before any drawing, with a test window in each
way of rendering: its owner is the taskbar, it lies wholly inside it and above it, the
foreground window did not change, and a screenshot shows the taskbar through the half-clear
part of the picture and around its round corners.

- The toolkit sets a window's owner and its extended style itself each time it shows the
  window. Set before, ours were gone. Set right after, they stay, and the tracker checks them
  again at every placement. Nothing in the toolkit's documentation promises that.
- Then the spike itself, with every other Sobreiro on the PC closed, seen in three
  screenshots: the pill in today's place left of the tray icons, 75 by 39 pixels where today's
  is 75 by 38; its ground lighter with the pointer on it and the tooltip open above the
  taskbar, the foreground window unchanged; and the flyout open beside it after a real click
  sent by a script at the pointer. A second click closed it.
- Not seen by eye: the tray icon, which Windows put among the hidden icons. The shell answered
  with its place, and a left and a right click sent as the tray's own message opened the flyout
  and the menu. The menu was seen in one screenshot.
- Nobody clicked anything by hand. A full-screen app, a hidden taskbar and a restart of the
  taskbar were not tried.

**The baseline run**, beside the row it is measured against, all three in
[baseline.md](baseline.md) under Experiments:

| | 1.0.0, tiered compilation off | The spike, software rendering | The spike, GPU rendering |
|---|---|---|---|
| Private memory | 103 MB | 26 MB | 103 MB |
| Working set | 173 MB | 63 MB | 125 MB |
| CPU per idle hour | 3.0 s | 1.4 s | 2.5 s |
| First pill, first start | 0.57 s | 0.07 s | 0.12 s |
| First pill, later starts | 0.47 s | 0.06 s | 0.11 s |
| Sobreiro.exe | 168.9 MB | 22.1 MB, and 12.8 MB of libraries | 22.1 MB, and 17.9 MB of libraries |

26 MB is 73% under 1.0's 97 MB, where the decision rule asks for 40%. Rendering through the
graphics card costs about 77 MB and buys nothing for a pill that changes a few times an hour.
Software rendering is a setting the toolkit documents (P); it is not its default.

**After the flyout has been opened once, the memory stays down.** A copy with its pill shown
stood at 14.0 MB. Its flyout was opened by a real click: 17.0 MB three seconds later, 17.3 MB
after it was closed. Two hidden copies side by side stood at 19 MB and 22 MB after seven
minutes, the second with one opening behind it. So the flyout costs 2 to 3 MB, and keeps them:
its window is hidden, not closed. With GPU rendering the same opening moved a copy from 88.0 MB
to 88.8 MB.

**What the memory is made of.** Eight copies ran side by side for 7 minutes, each on its own
data folder, signed out of Claude, their pills hidden because an installed pill had the place.
Private memory at the end of minute 7, and processor time from the end of minute 1, scaled to
an hour:

| Build | Rendering | Pill | The taskbar is asked | Flyout | Private memory | CPU per idle hour |
|---|---|---|---|---|---|---|
| Native AOT | software | switched off in the config | no | not opened | 10 MB | 0.3 s |
| Native AOT | software | window | no | not opened | 14 MB | 0.5 s |
| Native AOT | software | window | yes | not opened | 19 MB | 2.2 s |
| Native AOT | software | window | yes | opened once | 22 MB | 1.8 s |
| Native AOT | GPU | window | yes | not opened | 98 MB | 1.8 s |
| Native AOT | GPU | window | yes | opened once | 104 MB | 2.9 s |
| On the .NET runtime | software | window | yes | not opened | 23 MB | 1.9 s |
| On the .NET runtime | software | window | yes | opened once | 27 MB | 3.1 s |

- Everything that is not the pill is 10 MB, where today's exe with its pill off is 19 to 21 MB.
- The toolkit's pill window is about 4 MB. F's plain window was about 2 MB.
- Native AOT is not what makes it small: the same code on the .NET runtime, in one file of
  96.8 MB, idles at 23 MB. What Native AOT gives is the start (0.1 to 0.2 s of processor time
  in the first minute against 1.5 s), a working set of 55 MB against 90 MB, and the size.
- Windows counts processor time in steps of 0.016 s, so the last column tells 0.5 s from 2 s
  and little more. The rows with an opening include the opening.

**The 26 MB is a point on a slope.** Private memory in the baseline run went from 15.7 MB after
the first minute to 26.3 MB after the tenth, about 1 MB a minute. Two hidden copies left for 40
minutes rose the same way, reached 33 MB by minute 23 and stayed between 32 and 34 MB from
there. A trace of the runtime's own counters says why: in seven minutes its garbage collector
had not run once, and the garbage waiting for it grew by what private memory grew, about
150 KB for each question to UI Automation. The runtime's first collection waits for an amount
of garbage sized on the processor's cache, which is large on this PC.

- With a collection every 30 seconds, hidden copies stayed at 16 to 17 MB for the ten minutes
  they were watched, where the copy beside them without one reached 21 MB. Windows counted no
  difference in processor time.
- That resting level was not measured with the baseline script: the row is the build that
  never asks for a collection.
- Three things were tried before the trace and changed nothing: releasing UI Automation's
  objects as soon as an answer is read, asking from one thread that lives as long as the app,
  and three collector settings given through the environment.
- A copy that never asks UI Automation rises too, slowly: 13.0 MB to 17.5 MB in 40 minutes.
- Today's shell rises as well: 103 MB after the ten minutes of its baseline run, and as the
  installed copy, in use, 115 MB after 45 minutes and 137 MB after four hours. Whether for the
  same reason was not looked into.

**Native AOT.** Published on the PC with `dotnet publish -r win-x64` in about 20 seconds. The
compiler that checks for code Native AOT cannot keep gave no warning, for the spike, for Core
or for the toolkit. The one warning in the build is the C# compiler's, for an outdated call in
the spike's own picture mode. Two things needed doing for it: the COM interfaces generated at
build time, above, and the window events passed to Windows as plain function pointers.

**The files.** Not one exe: `Sobreiro.exe` at 22.1 MB, the toolkit's drawing library at 11.1 MB
and its text-shaping library at 1.7 MB beside it, 34.9 MB together. Its library for the
graphics card is 5.1 MB more and is not needed for software rendering: a copy without it ran
and opened its flyout. Today's release is one file of 168.9 MB that carries the bridge; the
spike does not carry the bridge, which is 2.2 MB. Linking the two libraries into the exe was
not tried.

**System notifications**, on paper. The toolkit has none (P), and its tray icon has no balloon.
Today's alerts are a balloon from the tray icon, which under Windows is one native call on an
icon the shell knows. Two ways to keep them:

- A notification icon of the app's own, registered with that same call, beside or in place of
  the toolkit's. About 60 lines and one import (E).
- The toolkit's icon, reached from outside: the spike found it registered under the toolkit's
  hidden window with the number 1. That leans on something the toolkit does not document.

On macOS and Linux a notification is a different call on each, and belongs to 3.0.

**How much code.** Counted on the spike's branch against `main`:

| Part | Today, on WPF | The spike |
|---|---|---|
| The face | 201 lines | 191 lines |
| The window | 181 lines, 7 native imports | 109 lines |
| The tracker and its native calls | 443 lines, 12 native imports | 480 lines, 23 native imports shared with the flyout |
| The question to UI Automation | 52 lines | 224 lines, 2 native imports |
| The pill in all | 877 lines | 1,004 lines |
| The flyout | 633 lines, 8 native imports | 413 lines, without the hand-set baselines |
| Tray icons, menu, polling and start | 993 lines | 516 lines, with a short menu |
| Native imports in the tray app | 34 | 27 |

The counts include the switches of the experiments. The face is the same code as today's with
other type names. The pictures the spike renders of itself at 100%, 150% and 200%, in both
themes, match today's pill in width; the text lines needed their height set by hand to match
in height. They were compared by eye, not pixel by pixel.

**F's pill with this flyout** was not built. The variant would keep F's hand-drawn pill and use
the toolkit for the flyout and the tray only. The table above bounds what it could save: the
toolkit's pill window is about 4 MB where F's is about 2 MB, so about 2 MB (E), for F's 370
lines of window, its 15 more imports and its bare pane for screen readers. Its drawing library
was not tried with Native AOT.

**Where it fell short.**

- Mixed scaling was not checked: the PC has one display at 100%. The window is placed in
  physical pixels and the toolkit scales its content by the monitor it is on; the face was
  drawn at 150% and 200% only as pictures.
- The window cannot be lower than 39 pixels at 100%: Windows keeps that minimum for a window
  of the kind the toolkit makes. The pill is 38, so it sits one pixel higher than today's.
  Lifting the minimum means handling one more window message. The spike did not find the
  toolkit's hook for that in this version, and did not take over the window's messages itself.
- The pill's right click opens nothing. The menu belongs to the toolkit's tray icon and cannot
  be opened from elsewhere, so the pill would need a second menu of its own.
- The menu is drawn by the toolkit in its own style, not the system's menu. It opens a window
  of the toolkit: 18.5 MB with it open in a hidden copy.
- A click gives the pill the keyboard focus inside the app, although Windows never activates
  it. An open flyout therefore closes on the press, and the spike ignores the release that
  follows. Today's pill does not do that.
- The owner and the style are put back after the toolkit, as said above: it works on this
  version, 12.1.6, and is not promised.
- The toolkit's build step sends its maker a report at every build: a hash of the project's
  name, a machine identifier, the versions (P). `AVALONIA_TELEMETRY_OPTOUT=1` switches it off;
  the spike's first two builds ran before that was seen. Nothing is sent when the app runs.
- Three files to ship where the release, the README's download link and the bridge inside the
  exe all assume one.
- The flyout's figures and units sit on one line of text instead of the WPF card's hand-set
  baselines, and Connect, the alerts and the rest of the menu were not built, so the line
  counts above are a floor.

Screen readers get what they get today: UI Automation finds the pill as a window with its two
lines as text ("5h" and "7d" with their figures).

### B. Rust with Slint

Spiked on 2026-11-13 and 14, on the same PC. In short: the whole shell idles at 7 MB with its
pill shown, where today's idles at 103 MB and A's at 26 MB, and opening the flyout adds about
1 MB. That is with the toolkit's software renderer; through the graphics card it idles at
246 MB. It is one exe of 9.3 MB and shows its pill 0.06 s after it is started. The price is
the one the table names: none of Core carries over. For the spike, about two thirds of it was
written again in Rust.

**What was built.** A Rust program beside today's projects, on the branch
`spike/shell-b-slint`, which is never merged. It builds only the tray icons, the pill and the
flyout, and brings the part of Core they need with it, rewritten.

- The pill is one of the toolkit's windows: borderless, clear per pixel, never activated. Its
  face is written in the toolkit's own language to the WPF face's measures, in the app's own
  fonts: the rings, the labels and figures, the stale mark, Copilot's block and bar, the
  lighter ground under the pointer, and a tooltip.
- It is placed by today's tracker, written again on the toolkit's timer and event loop, with
  the same native calls and the same placement rules.
- The taskbar is asked through UI Automation's own COM interface, from a thread of its own.
  The bindings crate for Windows already declares the interfaces, and a COM object is
  released when its value goes out of scope.
- The flyout is a second window of the toolkit, made the first time it is opened: the card
  with bars, battery or thermometer, placed beside the point of the click, closed by Esc or
  by losing the focus.
- The tray icons, their tooltips, the click and a short menu are the toolkit's tray element.
  The menu is the system's own. The rings in the icon are worked out pixel by pixel in the
  spike, because the toolkit draws into windows only.
- The rewritten logic reads, polls and words everything, as Core does, with this PC's
  sign-ins. The two requests go out through an HTTP crate set to use Windows' own TLS.
- Left out: Connect and Disconnect, the alerts, the settings in the menu other than Style, the
  first-run migration, the bridge inside the exe, the log, and the events for a change of
  theme, clock or display (a redraw every 30 seconds stands in). The spike refuses to start
  without a data folder of its own.
- The renderer is chosen when the exe is built. The measured exe carries the software
  renderer only; a second build adds the toolkit's OpenGL renderer, behind a switch in the
  environment.

**The taskbar takes the window.** Checked first, before any drawing, with a test window in each
way of rendering: its owner is the taskbar, it lies wholly inside it and above it, the
foreground window did not change, and a screenshot shows the taskbar through the half-clear
part of the picture and around its round corners. A transparent window on Windows is not in
the toolkit's documentation. It works, in software rendering too.

- The toolkit lets the app change a window's attributes before the window is made. With that
  the pill is born owned by the taskbar and is not activated when it is shown: nothing has to
  be put back afterwards, as A needed. That hook belongs to a feature the toolkit marks as
  unstable, one that may change between minor releases. The window handle comes through a
  feature that is not marked so.
- The two style bits that keep the pill out of Alt+Tab and away from the focus are added with
  a native call once the window exists. The tracker checks them and the owner at every
  placement and never had to put one back.
- Then the spike itself, with every other Sobreiro on the PC closed, seen in three
  screenshots: the pill in today's place left of the tray icons, 76 by 38 pixels where today's
  is 75 by 38; its ground lighter with the pointer on it, the foreground window unchanged;
  and the flyout open beside it after a real click sent by a script at the pointer. A second
  click closed it.
- Not seen by eye: the tray icons, which Windows put among the hidden icons. The shell
  answered with their place, and a left and a right click sent as the tray's own message
  opened the flyout and the menu. The menu was seen in one screenshot.
- Nobody clicked anything by hand. A full-screen app, a hidden taskbar and a restart of the
  taskbar were not tried.

**The baseline run**, beside the row it is measured against and A's, all in
[baseline.md](baseline.md) under Experiments:

| | 1.0.0, tiered compilation off | A, software rendering | The spike, software rendering | The spike, GPU rendering |
|---|---|---|---|---|
| Private memory | 103 MB | 26 MB | 7 MB | 246 MB |
| Working set | 173 MB | 63 MB | 29 MB | 103 MB |
| CPU per idle hour | 3.0 s | 1.4 s | 1.7 s | 1.7 s |
| First pill, first start | 0.57 s | 0.07 s | 0.06 s | 0.41 s |
| First pill, later starts | 0.47 s | 0.06 s | 0.06 s | 0.23 s |
| Sobreiro.exe | 168.9 MB | 22.1 MB, and 12.8 MB of libraries | 9.3 MB | 9.7 MB |

7 MB is 93% under 1.0's 97 MB, where the decision rule asks for 40%. Rendering through the
graphics card costs about 240 MB on this PC and buys nothing for a pill that changes a few
times an hour. The 0.3 s between A's and the spike's processor time is less than two runs of
one build have differed by.

**Against the decision rule.** B rewrites Core, so the rule asks two things of it: a pill that
works, and half the memory of the candidate that keeps Core, or less.

- Half of A's row is 13 MB. Half of the 16 to 17 MB A's hidden copies rest at, when the app
  asks for a collection every 30 seconds, is 8 MB.
- The spike ended its baseline run at 6.5 MB, which the row rounds to 7 MB, and that is a
  resting level (below). It is under both.
- Its pill works on this PC: taken by the taskbar, placed, clicked, with the flyout beside it.
  What is missing is under "Where it fell short"; the tooltip is the part of the pill itself
  that does not work as today's does.

**After the flyout has been opened once, the memory stays down.** A copy with its pill shown
stood at 5.0 MB. Its flyout was opened by a real click: 5.9 MB three seconds later, and 5.9 MB
two minutes after it was closed. So the flyout costs about 1 MB, and keeps it: its window is
hidden, not closed.

**What the memory is made of.** Six copies ran side by side for 7 minutes, each on its own data
folder, signed out of Claude, their pills hidden because an installed pill had the place.
Private memory at the end of minutes 5 and 7, and processor time from the end of minute 1,
scaled to an hour:

| Rendering | Pill | The taskbar is asked | Flyout | Private memory, minute 5 | Minute 7 | CPU per idle hour |
|---|---|---|---|---|---|---|
| software | switched off in the config | no | not opened | 3.5 MB | 4.9 MB | under 0.2 s |
| software | window | no | not opened | 4.2 MB | 4.2 MB | under 0.2 s |
| software | window | yes | not opened | 4.8 MB | 5.7 MB | 2.2 s |
| software | window | yes | opened once | 5.7 MB | 6.7 MB | 1.2 s |
| GPU | window | yes | not opened | 243 MB | 244 MB | 1.3 s |
| GPU | window | yes | opened once | 288 MB | 289 MB | 1.8 s |

- Everything that is not the pill is about 4 MB, where A's is 10 MB and today's exe with its
  pill off is 19 to 21 MB.
- The toolkit's pill window is about half a megabyte, and the question to UI Automation about
  as much. That question is nearly all of the idle processor time: a copy that never asks
  uses less than Windows can count.
- Most copies gain about 1 MB between their fifth and their seventh minute, once, when the
  second round of polls comes. The copy that never asks the taskbar did not. What the megabyte
  is made of was not looked into.
- The flyout copies opened their flyout by themselves, five seconds after starting.

**The 7 MB is a resting level.** Private memory in the baseline run was 5.6 MB for five
minutes, stepped to 6.5 MB by the seventh and stayed there. A hidden signed-in copy watched
for 16 minutes made the same single step, 5.5 MB to 6.5 MB in its sixth minute, and stood at
6.4 MB at the end. Nothing in this shell collects garbage, so there is no slope to wait out.

**Three things the toolkit did that were not expected.**

- It makes its tray icon once, on the first turn of the app's loop, from the picture the icon
  has at that moment. Without a picture it gives up, says so on the error stream, and never
  tries again: the first copies had no Copilot icon. The spike now gives each icon empty rings
  before the loop starts. The toolkit's documentation describes the icon as made when a
  picture is first set, which is not what this version did.
- A window hidden behind the toolkit's back, as today's tracker hides the pill, is never
  painted, and the toolkit keeps trying for as long as a change waits to be drawn. A hidden
  signed-in copy used 10 s of processor time per idle hour that way, all on the UI thread.
  The tracker now holds a new face back until the pill is shown again: 1.2 s per idle hour in
  the same hidden signed-in copy, watched for three minutes. A pill hidden behind a
  full-screen app would have met this.
- With GPU rendering, the test window of the first step was drawn and see-through. Seventy
  minutes later nothing rendered through the graphics card was drawn any more, in the pill,
  the flyout or that same test window, while windows rendered in software were. The display
  had gone to sleep in between, with nobody at the PC. Whether that is the reason is not
  known, and it was not looked into further: the software renderer is the one to ship.

**The rewrite of Core.** The spike needed the limits, the usage file, the config, the colours,
every display model, the taskbar rules, the poller and both providers with their sign-ins:
3,148 lines of C# as the files stand, 2,717 without blank lines, against the 4,220 this
document counts for all of Core. Left out were the alerts, the bridge, Connect, the
diagnostics log, the migration and the media pictures.

- It came to 4,426 lines of Rust, 3,920 without blank lines, and 518 tests that pass: about
  510 of the 540 test methods that cover those files, by the port's own count, and 8 new
  ones. The JSON fixtures are read from where they are.
- It was written by machine in about half an hour, type for type from the C#, and nobody has
  read it line by line. Only the two HTTP clients and the type that holds a token were read
  against the rules on tokens: the same two addresses and headers as today, no redirects, no
  cookies, and a token type that cannot be printed, copied or written to a file.
- What .NET gives and Rust's standard library does not, and where each came from: time zones
  and calendar arithmetic (a crate, which has no zone database of its own: the test of a
  clock change in Lisbon was not ported), the user's short time format (two Windows calls), a
  watcher on the data folder (30 lines over a native call), HTTP, TLS and JSON (crates), and
  a poll that runs off the UI thread and answers on it (a thread and a closure handed back).
- Differences that remain: money is rounded half-to-even on an exact tie, where .NET rounds
  away from zero; `config.json` is written with Unix line ends; a request under way cannot be
  cancelled before its 10 s timeout.
- The 30 test methods not ported need an HTTP test server, a made-up culture, the zone
  database, or cancellation by exception.

**The toolchain.** Installed with one command in about a minute: 600 MB, and 470 MB of fetched
crates after the first build. The first build took 1 minute 7 seconds, the ones after it about
40 seconds each. The build links 261 crates from the public registry, each under a permissive
licence or GPL 3; its working folder is 2.2 GB. The spike was not built by CI.

**The files.** One exe of 9.3 MB, with the three fonts inside and nothing beside it. Today's
release is one file of 168.9 MB that carries the bridge; the spike does not carry the bridge,
which is 2.2 MB.

**System notifications**, on paper. The toolkit has none (P), and its tray element keeps its
icon to itself. The two ways A's note names hold here too:

- A notification icon of the app's own, registered with one native call, for the balloon.
- The toolkit's icon, reached from outside: the spike's test found each one registered under
  a message-only window of the toolkit's, with the number 1. That leans on something the
  toolkit does not document.

**How much code.** Counted on the spike's branch against `main`, as the files stand:

| Part | Today, on WPF | The spike |
|---|---|---|
| The face | 201 lines | 237 lines in the toolkit's language, 73 of Rust to fill it in |
| The window and the tracker | 624 lines, 19 native imports | 410 lines, and 198 for 22 native calls shared with the flyout |
| The question to UI Automation | 52 lines | 120 lines, 2 native calls |
| The pill in all | 877 lines | 1,038 lines |
| The flyout | 633 lines, 8 native imports | 514 lines in the toolkit's language, 194 of Rust |
| Tray icons, menu, polling and start | 993 lines | 633 lines, 7 native calls |
| The logic | Core, as it is | 4,426 lines, 5 native calls |
| Native imports or calls in all | 34 | 36 |

The toolkit's language is wordy by the line: one property a line. The pictures the spike
renders of itself at 100%, 150% and 200%, in both themes, match A's pictures within three
pixels of width over four pills; the flyout card is 479 pixels high where A's is 472. They
were compared by eye, not pixel by pixel.

**Where it fell short.**

- The tooltip opens inside the pill's own window, which is 76 by 38 pixels: it covers the pill
  and is cut off, where today's opens above the taskbar. The toolkit's tooltip is drawn in
  the window it belongs to. A tooltip above the taskbar would be a window of the spike's own,
  or the system's.
- Mixed scaling was not checked: the PC has one display at 100%. The window is placed in
  physical pixels and the toolkit scales its content by the monitor it is on; the face was
  drawn at 150% and 200% only as pictures.
- The pill's right click opens nothing. The menu belongs to the toolkit's tray element and
  cannot be opened from elsewhere, so the pill would need a second menu of its own.
- Being born owned and not activated goes through the hook the toolkit marks as unstable, as
  said above.
- The toolkit has no setting for figures of equal width, which the WPF card uses so a
  changing figure does not shift its line, and no way to set two sizes of text on one
  baseline: a figure and its unit are two blocks side by side.
- The software renderer does not clip to round corners: the fill of Copilot's bar is square
  at the bottom.
- The toolkit's tray element was five months old in the version used. Each one comes with a
  hidden window of the toolkit's default size, whose cost was not measured apart.
- Windows' message that the session is ending does not reach the app through the toolkit.
  The spike reads the pill window's messages first for it, with two native calls.
- The pill is one pixel wider than today's.
- Connect, the alerts and the rest of the menu were not built, and a third of Core was not
  rewritten, so the line counts above are a floor.
- The Rust has no reader: the maintainer has not worked in the language, and the rewrite was
  not reviewed.

Screen readers get nearly what they get today: UI Automation finds the pill as a window with
four texts, each label and each figure by itself ("5h", its figure, "7d", its figure), where
today's gives two lines.

## Notes on each point

### Idle memory, exe size and start time

- **Why the estimates are wide.** The measurement above says the window is the cost: in WPF, one
  small transparent window and the toolkit under it are most of 75 MB. A toolkit that brings its
  own renderer and text engine can cost the same again, and nothing published says it does not.
  The low end of each range assumes the toolkit's software renderer; the high end, its default
  GPU renderer.
- **A.** Two reports, far apart: an empty Native AOT app at about 20 MB of memory and an 18 MB
  package (R, no system or version given), and a tray app from 2024, not Native AOT, at 150 to
  200 MB with its window hidden (R). The toolkit publishes no desktop figure. It can be told to
  render in software (P). The spike measured both ways of rendering: see
  [What the spikes measured](#a-c-with-avalonia-native-aot).
- **B.** The toolkit publishes "less than 300 KiB" of memory for its runtime (P). That is the
  runtime on a small device, not a Windows process with a window, fonts and a tray icon. The
  spike measured the process: see [What the spikes measured](#b-rust-with-slint).
- **C.** One report of an empty window: 4.1 MB of memory, which kind not said, and 49 MB on disk
  in 19 files when shipped with the toolkit's libraries beside the exe (R, March 2025).
- **D.** Nothing was found, measured or published.
- **E.** One author reports 13 MB for a menu bar app written against the older macOS toolkit and
  56 MB for the same app in the newer declarative one (R).
- **F.** The 19 MB floor is the current exe with its pill off (M). The estimate added a plain
  window, its bitmap and the UI Automation client. The spike measured it, and the split of the
  75 MB: see [What the spikes measured](#f-c-the-pill-on-a-plain-system-window).
- **UI Automation rides along with every candidate.** The pill has to know where the tray icons
  and the task buttons are, in any language. Today it asks through WPF's managed client.

### The pill

What the pill needs from a toolkit: a borderless window with per-pixel transparency that does
not take focus; its window handle, so it can be owned by the taskbar and placed inside it; the
right size on monitors of different scale; and a way to put questions to UI Automation.

- No toolkit documents a window owned by another program's window. In every candidate that part
  is the same native calls the tray app makes today: the pill is 857 lines, and 28 of the tray
  app's 34 native imports go to the window manager.
- **A** and **C** document the other three needs. A's spike built them on Windows; the size on
  monitors of different scale was not checked. **B** documents frameless and always on top;
  transparency on Windows is not in its documentation, and the window handle is reached through
  a feature its documentation says may change between minor releases. Its software renderer is
  selectable on desktop and leaves out some effects (P). B's spike built all three on Windows
  as well: the transparency works in both ways of rendering, and the unstable feature turned
  out to be needed for the window's owner and for not activating it, not for its handle.
- **D** has the calls and nothing above them: the window procedure, the drawing, the text and
  the questions to UI Automation would all be written by hand, in a language that is not the
  usual one for them.
- **F** is the same hand-written window as D, in the language the rest of the app is in, with
  Core beside it.

### Tray, menu bar and notifications

- Linux is the same story in every candidate: the tray icon needs a desktop that implements the
  status notifier protocol, and GNOME needs an extension for it (P, three toolkits). The roadmap
  already says so.
- Text in the macOS menu bar is what stands in for the pill there. Of the toolkits, only **B**
  documents it; **A** and **C** document an icon and a menu. A native shell has it from the
  system.
- **B**'s tray element is under five months old. The roadmap said it offers a menu only; on
  2026-11-13 its documentation lists icon, tooltip, title and a click as well. The spike built
  the icon, the tooltip, the click and the menu on Windows, and found the icon is made only if
  it has its picture when the app starts.
- System notifications: only **C** has them built in, and its documentation says a message may
  not appear at all, depending on the system. Today's shell shows a balloon from its tray icon.
  Where **A** would get them is in its
  [spike's note](#a-c-with-avalonia-native-aot).

### Core and its tests

- Core is already marked compatible with Native AOT, and the bridge, which links Core, ships
  that way (checked in the repository on 2026-11-13). **A** and **F** need nothing from it.
- **B**, **C** and **D** rewrite 4,220 lines and 956 tests. Only the JSON fixtures carry over.
  B's spike rewrote about two thirds of it, by machine: what that came to is in its
  [note](#b-rust-with-slint).
- **E** keeps Core on Windows. On macOS and Linux .NET can publish Core as a native library (P);
  no documentation was found for calling one from Swift.

### Toolchain, CI and signing

- Every candidate ends in one build per system on that system's own runner: Native AOT does not
  compile across systems (P), and the others need each system's compiler or libraries.
- Signing does not tell the candidates apart. Windows: the same certificate question as issue 9,
  whatever the language. macOS: the developer programme at 99 US dollars a year (P), in every
  candidate.
- **C** adds the most: a second language, a build system, the toolkit fetched on each runner,
  and a deployment step per system. The open-source edition gets only the first patch releases
  of a long-term release (P).

### Licence

All fit GPL-3.0-or-later. These are readings, not legal advice.

- **A:** MIT.
- **B:** offered under GPL version 3 only, among other options. Sobreiro's own files stay
  "or later"; the exe as shipped would be under version 3.
- **C:** the modules a tray app needs are under LGPL version 3, which is GPL 3 with added
  permissions. Sobreiro publishes its whole source, so linking it into one exe is allowed too.
- **D:** Apache 2.0 and BSD libraries.

### Maintainer

Ricardo, asked on 2026-11-13: he has worked in C# and .NET, and in Python; in none of Rust, C++,
Qt, Go or Swift. So **A**, **E** on Windows and **F** are in a language he can review, and
every other candidate is one he would learn while maintaining it.

Python did not get a row: with a window toolkit it carries an interpreter and that toolkit's
libraries, which is the "large runtime of its own" the roadmap leaves out. No figure was
collected for it.

## Shortlist

Three, for the spikes of step 2:

1. **A. C# with Avalonia, published with Native AOT.** The candidate that keeps Core, and the
   only one of the three that is a shell for all three systems without a rewrite. It is the one
   the others have to beat. Its risk is the first column: nothing says its window is cheaper
   than WPF's.
2. **F. C#, the pill on a plain system window.** Added by this comparison, because the
   measurement points at it: if the WPF window is most of the 75 MB, taking only the pill off
   WPF may meet the 58 MB bar with Core, the flyout and the release process as they are. It is
   the cheapest spike and it answers the question the other two depend on: what a pill costs
   when no toolkit is under it. It is not a path to macOS or Linux by itself.
3. **B. Rust with Slint.** The one candidate that rewrites Core. It has the lowest expected
   memory of the toolkits, the only documented menu bar text on macOS, and a licence that fits.
   It also has the most unknowns in the pill column, and it is a language the maintainer has not
   worked in.

Left off:

- **C. C++ with Qt** is the runner-up to B, and takes its place if B's window cannot be made
  transparent or owned by the taskbar in the first days of its spike. It loses to B on the
  maintainer's column (manual memory management in a language he cannot review), on the heavier
  build and on the missing menu bar text; it wins on the age of its tray support.
- **D. Go** has no toolkit to compare: its pill is F's hand-written window in a language new to
  the maintainer, with Core rewritten as well.
- **E. A native shell per system** is not a spike. On Windows it is the current shell, which the
  decision rule already keeps as the fallback; on the other systems it is where F leads, and
  3.0 waits for demand.

Under the decision rule, B wins only with a working pill at half of A's memory or less. With
19 MB measured for everything that is not the pill, that is a narrow target: if A or F lands
near 40 MB, B has to idle at 20 MB with its window and UI Automation included. A's spike has
since landed at 26 MB in the baseline run, so the target is 13 MB. At the 16 to 17 MB its
hidden copies rest at when the app asks for a collection, it would be 8 MB. B's spike landed at
7 MB, with its window and UI Automation included: under both.

## What this leaves open for the spikes

For [step 2](roadmap.md#step-2-build-the-shortlist) (issue 18). The table settles none of these.

- **Every memory, size and start figure of A, B and F.** All are estimates. Each spike runs the
  baseline script on the PC the 1.0.0 row was measured on. All three are measured now.
- **Whether the taskbar takes each window.** Owned by the taskbar, per-pixel transparent, not
  taking focus, right on monitors of different scale: no toolkit documents the first, and B
  does not document the second. It is the first thing each spike tries, before any drawing.
  All three windows are taken; none was checked on monitors of different scale.
- **What the pill's 75 MB is made of.** How much is the WPF window and how much the UI
  Automation client, and whether asking through the system's own interface costs less than
  through WPF's managed one. Answered by F's spike: the window is about 53 MB, UI Automation 3
  to 5 MB, and the system's interface is 1 to 3 MB cheaper than the managed client.
- **What the flyout costs, in every candidate.** New from F's spike: a WPF flyout opened once
  brings back all the memory the plain pill saved. A and B have to be measured after their
  flyout has been opened and closed, not only before. A's costs 2 to 3 MB, and keeps them. B's
  costs about 1 MB, and keeps it.
- **A:** software against GPU rendering, in memory and in idle processor time (answered: 26 MB
  and 1.4 s an hour against 103 MB and 2.5 s); whether the shell publishes with Native AOT
  without warnings (answered: it does); where system notifications come from (answered on
  paper: one native call for a balloon, as today). Still open for A: mixed scaling, a menu on
  the pill's right click, shipping three files where today's release is one, and a baseline
  run of the build that asks for a collection every 30 seconds.
- **B:** a transparent frameless window on Windows (answered: it works, in software rendering
  too); whether the unstable window-handle feature is something to build on (answered in part:
  the handle does not need it, the owner at birth does, and it held on the version used);
  whether the software renderer draws the pill's rings and text as they look today (answered:
  within three pixels of A's pictures, with square ends on Copilot's bar); the tray element's
  age (answered: it works, with one fault at start that the spike steps around). Still open
  for B: mixed scaling, a tooltip above the taskbar, a menu on the pill's right click, and
  who reads the Rust.
- **F:** how much drawing code the pill takes, with the app's own fonts (answered: 241 lines);
  whether the exe can shrink while the flyout stays WPF (answered: by 17 MB safely); what a
  hand-drawn flyout would cost in accessibility if it ever followed (answered in part: a
  UI Automation provider written by hand, which the spike did not write). Still open for F:
  mixed scaling, and a flyout that does not bring WPF's memory back.
- **Processor time.** No candidate had an idle figure. The current shell's was 1.4 s per hour
  without the pill (M) and 7.3 s with it (M), and is 3.0 s with it since tiered compilation
  was switched off (M), which meets the 5 s target. A's spike used 1.4 s with its pill shown
  (M), and B's 1.7 s (M). F's 5.4 s was measured before that switch and is not comparable.
- **Not for the spikes:** the menu bar text on macOS that A lacks, the tray on Linux, and
  signing on three systems matter in 3.0. They are in the table so that the decision in issue 19
  is made knowing them.
