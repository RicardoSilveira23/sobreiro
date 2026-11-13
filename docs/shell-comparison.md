# Shell comparison, on paper

Written 2026-11-13 for step 1 of [roadmap.md](roadmap.md#step-1-compare-on-paper): the candidates
for the next shell, scored on the roadmap's seven points. Nothing was built for it. It ends in a
[shortlist](#shortlist) and in [what it leaves open](#what-this-leaves-open-for-the-spikes) for
the spikes.

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
| **F. C#, the pill on a plain system window** (added here) | 25 to 50 MB (E), above a floor of 19 MB (M); exe 168.9 MB and start 0.40 s (M) while the flyout stays WPF | Possible: a per-pixel transparent window is a system feature. The rings and text are drawn by hand. Native code: more than today, a few hundred lines (E) | Today's on Windows. Not a shell for other systems: there it becomes E | All of it | Nothing new | Nothing new. Fits | Knows C#; the drawing calls are new to him |

Versions read: Avalonia 12.1.6 (2026-11-09), Slint 1.18.1 (2026-09-21), Qt 6.12.0 (2026-09-30),
.NET 10.

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
- **F.** The 19 MB floor is the current exe with its pill off (M). The estimate adds a plain
  window, its bitmap and the UI Automation client. How the 75 MB splits between the WPF window
  and UI Automation was not measured cleanly: a copy that kept the window and asked nothing sat
  at 76 MB (M), so the window looks like most of it.
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
  baseline script on the PC the 1.0.0 row was measured on.
- **Whether the taskbar takes each window.** Owned by the taskbar, per-pixel transparent, not
  taking focus, right on monitors of different scale: no toolkit documents the first, and B
  does not document the second. It is the first thing each spike tries, before any drawing.
- **What the pill's 75 MB is made of.** How much is the WPF window and how much the UI
  Automation client, and whether asking through the system's own interface costs less than
  through WPF's managed one. F's spike answers it for all three.
- **A:** software against GPU rendering, in memory and in idle processor time; whether the shell
  publishes with Native AOT without warnings; where system notifications come from.
- **B:** a transparent frameless window on Windows; whether the unstable window-handle feature
  is something to build on; whether the software renderer draws the pill's rings and text as
  they look today; the tray element's age.
- **F:** how much drawing code the pill takes, with the app's own fonts; whether the exe can
  shrink while the flyout stays WPF; what a hand-drawn flyout would cost in accessibility if it
  ever followed.
- **Processor time.** No candidate has an idle figure. The current shell's is 1.4 s per hour
  without the pill (M) and 7.3 s with it (M); the 5 s target stays open on the current shell
  whatever is chosen here.
- **Not for the spikes:** the menu bar text on macOS that A lacks, the tray on Linux, and
  signing on three systems matter in 3.0. They are in the table so that the decision in issue 19
  is made knowing them.
