# Shell comparison, on paper

Written 2026-11-13 for step 1 of [roadmap.md](roadmap.md#step-1-compare-on-paper): the candidates
for the next shell, scored on the roadmap's seven points. Nothing was built for the table. It
ends in a [shortlist](#shortlist) and in
[what it leaves open](#what-this-leaves-open-for-the-spikes) for the spikes. Each spike of step 2
puts its figures in its row and a note under
[What the spikes measured](#what-the-spikes-measured).

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
| **A. C# with Avalonia, Native AOT** | 40 to 80 MB (E); exe 20 to 40 MB (E); start under 0.4 s (E) | Possible on paper: borderless transparent window, window handle and per-monitor DPI (P). Being owned by the taskbar is not documented. Native code: about today's | Windows: icon, tooltip, menu, click (P). macOS: icon and menu, no text in the menu bar (P). Linux: on desktops with a status notifier, GNOME needs an extension (P). No system notifications (P) | All of it: 4,220 lines and 956 tests, untouched | No new language. A build on each system, with that system's linker (P) | MIT (P). Fits | Knows C# and .NET. The toolkit is new to him, its XAML is close to WPF's |
| **B. Rust with Slint** | 15 to 40 MB (E); exe 5 to 15 MB (E); start under 0.2 s (E) | Unproven: frameless and always on top (P), a transparent window on Windows is not documented, the window handle comes through a feature marked unstable (P). Native code: all of today's again, in Rust | Tray element since June 2026 (P). Windows: icon, tooltip, menu, click (P). macOS: the same, plus text in the menu bar (P). Linux: status notifier only, GNOME needs an extension (P). No notifications (P) | None of the code. 4,220 lines and 956 tests rewritten; the JSON fixtures are reused | A second toolchain, simple to install. A build on each system | GPL 3 (P). Fits | Has not worked in Rust |
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
  render in software (P).
- **B.** The toolkit publishes "less than 300 KiB" of memory for its runtime (P). That is the
  runtime on a small device, not a Windows process with a window, fonts and a tray icon.
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
- **A** and **C** document the other three needs. **B** documents frameless and always on top;
  transparency on Windows is not in its documentation, and the window handle is reached through
  a feature its documentation says may change between minor releases. Its software renderer is
  selectable on desktop and leaves out some effects (P).
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
  2026-11-13 its documentation lists icon, tooltip, title and a click as well.
- System notifications: only **C** has them built in, and its documentation says a message may
  not appear at all, depending on the system. Today's shell shows a balloon from its tray icon.

### Core and its tests

- Core is already marked compatible with Native AOT, and the bridge, which links Core, ships
  that way (checked in the repository on 2026-11-13). **A** and **F** need nothing from it.
- **B**, **C** and **D** rewrite 4,220 lines and 956 tests. Only the JSON fixtures carry over.
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
near 40 MB, B has to idle at 20 MB with its window and UI Automation included.

## What this leaves open for the spikes

For [step 2](roadmap.md#step-2-build-the-shortlist) (issue 18). The table settles none of these.

- **Every memory, size and start figure of A, B and F.** All are estimates. Each spike runs the
  baseline script on the PC the 1.0.0 row was measured on. F's are measured now.
- **Whether the taskbar takes each window.** Owned by the taskbar, per-pixel transparent, not
  taking focus, right on monitors of different scale: no toolkit documents the first, and B
  does not document the second. It is the first thing each spike tries, before any drawing.
- **What the pill's 75 MB is made of.** How much is the WPF window and how much the UI
  Automation client, and whether asking through the system's own interface costs less than
  through WPF's managed one. Answered by F's spike: the window is about 53 MB, UI Automation 3
  to 5 MB, and the system's interface is 1 to 3 MB cheaper than the managed client.
- **What the flyout costs, in every candidate.** New from F's spike: a WPF flyout opened once
  brings back all the memory the plain pill saved. A and B have to be measured after their
  flyout has been opened and closed, not only before.
- **A:** software against GPU rendering, in memory and in idle processor time; whether the shell
  publishes with Native AOT without warnings; where system notifications come from.
- **B:** a transparent frameless window on Windows; whether the unstable window-handle feature
  is something to build on; whether the software renderer draws the pill's rings and text as
  they look today; the tray element's age.
- **F:** how much drawing code the pill takes, with the app's own fonts (answered: 241 lines);
  whether the exe can shrink while the flyout stays WPF (answered: by 17 MB safely); what a
  hand-drawn flyout would cost in accessibility if it ever followed (answered in part: a
  UI Automation provider written by hand, which the spike did not write). Still open for F:
  mixed scaling, and a flyout that does not bring WPF's memory back.
- **Processor time.** No candidate has an idle figure. The current shell's is 1.4 s per hour
  without the pill (M) and 7.3 s with it (M); the 5 s target stays open on the current shell
  whatever is chosen here.
- **Not for the spikes:** the menu bar text on macOS that A lacks, the tray on Linux, and
  signing on three systems matter in 3.0. They are in the table so that the decision in issue 19
  is made knowing them.
