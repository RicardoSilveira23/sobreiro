# Sobreiro's page (design)

2026-11-11. The design of Sobreiro's site: the landing page, `site/index.html`, and the manual
beside it.

It replaces the site's first look. Every colour and motion the page uses is written here, and every size that shapes it. A colour
that is not here is not used.

## Why

The page is for someone who heard of Sobreiro and wants to know three things before installing
a small tool from a stranger:

| They ask | The page answers with |
|---|---|
| What is it? | One sentence, and the pill as it looks on a taskbar |
| What does it do with my sign-in? | A diagram of the two requests, and "What it sends" |
| How do I get it? | Download, and three steps |

Readers are developers on Windows 11 who use Claude Code or GitHub Copilot.

The page it replaces wore a look made to be shared by several projects. This one is designed
from what the tool does: it asks two addresses for numbers and draws them as rings.

## Goals

- The first screen says what it is, shows it, and offers the download, at phone and desktop
  width.
- The diagram shows the real path: which addresses are asked, how often, and where the answer
  goes.
- The rings are the thing people like, so they are seen working: growing, and changing colour
  at the real thresholds.
- Everything is readable with no script. Every control is at least 44 px tall.

## It fails if

- A reader cannot tell, without scrolling far, what leaves their PC.
- The purple is so strong that the pill and the Download button stop standing out.
- There are tall empty bands between parts, or the page stretches to fill the window.
- The pill on the page shows a colour, threshold or figure style the app does not have.
- Anything shifts while the figures change.
- It needs script, or motion, to make sense.

## The page

One page, light only. Content is at most 1152 px wide with 16 px at each side.

### Phone (393 px): one column, in this order

| # | Part | Holds |
|---|---|---|
| 1 | Header | The mark (32 px) and the name. Nothing else. |
| 2 | Words | Headline, Download and Manual, the meta line, "Run it" with its three steps |
| 3 | Diagram | The pill with its key and colour scale, Sobreiro.exe, the two addresses |
| 4 | What it sends | Three lines, each with its dot |
| 5 | Before you install | One short paragraph |
| 6 | How heavy each release is | One line, a bar chart per figure, a download per release |
| 7 | Footer | Two links, the name line and the disclaimer |

24 px between parts.

### Desktop (1000 px and wider): two columns

```
Header ............................................
Words                    |  Diagram
Before you install       |  What it sends
How heavy each release is, across both columns ...
Footer ............................................
```

- The right column is 490 px. The left takes the rest. 56 px between columns, 32 px between
  rows.
- "Run it" sits at the foot of the words, so the last step ends level with the diagram's bottom.
- Each note sits under the thing it explains, below a 2 px violet rule. The two rules are on
  one line.
- The page is as tall as its content. It never stretches to the window.

### The diagram, top to bottom

| Part | Holds |
|---|---|
| Pill | `5h 15%` over `7d 45%`, the two rings, `AI 17%` over `1 Dec`, the level bar |
| Key | **5h** Session, the outer ring · **7d** Week, the inner ring · **AI** Copilot's monthly credits |
| Colour scale | Purple to 70%, amber to 90%, red after, labelled 70% and 90%. A "5h" marker stands at the session figure. |
| Wire | |
| Sobreiro.exe | The mark, the file name, "asks every 5 minutes" |
| Two wires, side by side | |
| Two address boxes, side by side | **Claude Code usage** `api.anthropic.com` `/api/oauth/usage` and **Copilot AI credits** `api.github.com` `/copilot_internal/user` |

The scale is hidden from screen readers. They get one line instead: "A ring is purple below
70%, amber from 70% and red from 90%."

### How heavy each release is

What the releases bought, so the download is chosen on numbers. `scripts/comparison.mjs` writes
the section from the Releases table in `docs/baseline.md`, and a test fails when the page and
the table differ: no figure is typed on the page.

| Part | Holds |
|---|---|
| 1.0 to 1.1 | One short paragraph: what 1.1 changed (idle), with the CPU figures and the start's cost |
| Why 2.0 moved to Rust | A heading led by the mark (36 px) in Rust, then one short paragraph: WPF held most of 1.1's memory, three other shells were measured, the Rust one rested lowest. No figures in it. |
| Caption | "Each bar starts at zero; shorter is lighter." |
| Cards | Private memory, CPU per idle hour, start to the pill, `Sobreiro.exe`. Each: the name, the multiple large ("14×", "less" under it), and two bars, 1.1 over 2.0, each with its figure at the tip. One column, two from 600 px. From 1000 px the words take the left column (at most 62 characters) and the cards sit two by two on the right, 1 : 1.25, 56 px apart, the cards top-aligned with the heading. |
| Downloads | A Manual-shaped button per release with a GitHub Release, newest first, sharing the row's width equally; "How it is measured" on its own line under them |

- The multiple is the headline and the bars are its proof: linear, from zero, each a share of
  the larger figure, so a 14× gap shows as a sliver beside a full bar. 1.1's bar is Line,
  2.0's Violet. Each card is a diagram box: Box fill, 1.5 px Line border, 8 px corners.
- Until 2.0.0 has a row, only the two paragraphs and the 1.1 button show: one release has
  nothing to be set against.
- A row is measured from the file attached to the release, so a row means a download. 1.0 was
  released before this repository was public and has no button.
- 2.0's bars play the diagram's 5-second loop: each appears at 1.1's length, shrinks to its own
  in 0.9 s on the page's easing, holds, and fades out before the loop starts again, so it is
  never seen growing. The figure at its tip counts down with it and the multiple counts up
  (script). The word sits under the multiple, so nothing beside it moves. With no
  script the bars still play and the figures stand; with reduced motion nothing moves.

| Thing | Size |
|---|---|
| Card | 14 px padding above, 16 px at the sides and below; 12 px between cards |
| Multiple | Unbounded 700, 32 px, Ink; its word under it, 15 px Soft |
| Bars | 10 px tall, square at zero, 4 px corners at the tip; release label Sora 600 13 px |
| Figures | 13 px, tabular, 8 px after the bar; 2.0's Inter 600 Ink, 1.1's Soft |

### The words

| Where | Text |
|---|---|
| Headline | Your GitHub Copilot and Claude Code usage, always in view on your taskbar. |
| Buttons | **Download** (the latest release on GitHub) and **Manual** (the manual page) |
| Meta line | Free, for Windows 11. Needs GitHub Copilot with AI credits, or Claude Code on Pro, Max or Ultra. |
| Run it | 1 Download Sobreiro.exe, one file. 2 Open it. Then, beside the mark: The pill appears beside the clock. |
| What it sends | Two requests, with a sign-in already on this PC. The numbers come back and go to the taskbar. Nothing else leaves: no telemetry, no update checks, no token stored, logged or renewed. |
| How heavy each release is | Generated from `docs/baseline.md`: what 1.1 changed, then 2.0 against 1.1. Buttons: Download 2.0, Download 1.1. |
| Before you install | Both addresses are undocumented and can change, and Anthropic's terms don't allow third-party apps to use Claude sign-ins. Windows may warn about an unknown publisher until the app is signed. |
| Footer | Open an issue. By Ricardo Silveira. Sobreiro is the Portuguese name of the cork oak, whose trunk in section is the tray's two rings. Not affiliated with or endorsed by Anthropic or GitHub. |

## The look

### Colours: the whole list

The page:

| Name | Value | Used for |
|---|---|---|
| Ground | `#ebe6ff` | The page, a light lavender |
| Ink | `#1d1447` | Text, the answer dot, the scale's marker |
| Soft | `#4d457c` | Smaller text: meta line, notes, key, addresses, footer |
| Violet | `#6f4df6` | 2.0's bars in the comparison, Download's fill, Manual's border, the two rules, the step circles, the request dot, the pulse |
| Violet text | `#5a3dd1` | Manual's label, step numbers, footer links |
| On violet | `#ffffff` | Download's label and GitHub's mark |
| Line | `#b9adf5` | Wires, box borders, the line joining the steps, the comparison's card borders and 1.1's bars |
| Box | `#f6f3ff` | The fill of the diagram's boxes |
| Rust | `#b7410e` | The 2.0 mark's tile in the comparison, and nothing else |

Violet is an accent. It never fills an area larger than the Download button.

The pill, as the app draws it on a dark taskbar. Purple, amber and red are the dark values in
`src/Sobreiro.Core/Colours/NightPalette.cs`:

| Name | Value | Used for |
|---|---|---|
| Taskbar | `#1c1c1c` | The pill's ground |
| Pill text | `#ffffff` | Labels and figures |
| Dim | `#9a9a9a` | The reset date |
| Track | white at 20% (rings), 22% (bar) | The empty part of a ring or the bar |
| Purple | `#a594ff` | A ring or the bar below 70% |
| Amber | `#ffb020` | From 70% |
| Red | `#ff6b6b` | From 90% |

The three status colours appear only in the pill and on the colour scale.

### Type

All four faces are openly licensed, served from the site, Latin range only.

| Face | Where |
|---|---|
| Sora | The name (700, 20 px), the headline (600), section headings (600, 16 px), key labels (600, 17 px), step numbers (600, 14 px), the scale's marker (600, 12 px) |
| Inter | Everything else: body text 16 px on a 1.45 line, button labels (600, 17 px), the pill's labels (400) |
| Unbounded | The pill's figures only (700). It is the app's figure face. |
| JetBrains Mono | `Sobreiro.exe` (600, 15 px) and the addresses (400, 11 px) |

### Sizes that matter

| Thing | Size |
|---|---|
| Headline | 26 px on a phone, 36 px on a desktop (2.9% of the window's width between the two). Line height 1.12, at most 20 characters to a line, letter spacing -0.025 em. |
| Buttons | 54 px tall, 12 px corners. GitHub's mark is 24 px. |
| Other links | Name in the header and footer links: at least 44 px tall |
| Pill text | 5% of the window's width, from 18 px to 28 px (about 20 px at 393 px) |
| Rings | 16% of the window's width, from 56 px to 88 px (63 px at 393 px). Stroke is 5 in a 52-unit square; radii 21 and 12.5. |
| Level bar | 0.45 em wide, 2.4 em tall, 5 px corners |
| Pill | 8 px corners. 24 px above and below on a phone, 20 px on a desktop. Each figure keeps a width of 2.9 em, so nothing shifts as it changes. |
| Colour scale | 8 px tall, three parts in the ratio 70 : 20 : 10 with 2 px between |
| Diagram boxes | 1.5 px border, 8 px corners |
| Wires | 2 px wide; 36 px long on a phone, 28 px on a desktop. The dot is 10 px. |
| Steps | 30 px circles with a 1.5 px violet border, joined by a 2 px line. The third is the mark at 30 px. |
| Notes | 14.5 px text, at most 62 characters to a line, under a 2 px rule with 14 px between |
| Meta line | 14 px, at most 44 characters to a line |
| Keyboard focus | A 2 px outline in the text's colour, 3 px away. On the Download button it is Ink, since white would not show on the ground. |

## Motion

The diagram plays one loop of 5 seconds. It stands for the app's real 5 minutes. Nothing else
on the page moves: no entrances, nothing on scroll, nothing on hover. The comparison's
2.0 bars loop beside it, on the same 5 seconds.

| Motion | Trigger | Length | Tells the reader |
|---|---|---|---|
| Request dots | The loop starts | 1 s, from 0.2 s | Sobreiro asks both addresses, at the same moment. Violet dots go down the two wires. |
| Answer dots | A request dot arrives | 0.3 s to turn ink, then 1.1 s back up | What comes back is a different thing from what went out, on the same path |
| Answer dot to the pill | The answers reach Sobreiro.exe (2.7 s) | 1 s, landing at 3.9 s | The numbers go to the taskbar and nowhere else |
| Rings and bar step | The answer dot lands | 0.9 s, slowing to a stop | Usage is what the rings draw. A change of colour takes 0.35 s, starting 0.25 s in. |
| Figures count | The answer dot lands | 0.8 s | The same reading, as a number |
| Marker on the scale | The answer dot lands | 0.9 s | Where the session stands against 70% and 90%, so the colour change has a reason |
| 2.0's bars | The loop starts | Fade in 0.4 s at 1.1's length, shrink 0.9 s from 0.4 s, fade out from 4.6 s | How much lighter 2.0 is than 1.1, figure by figure |
| 2.0's mark | With the bars' shrink | 0.9 s, back to Violet in the fade | The trunk turns once as the tile goes from Violet to Rust: this release is the Rust one |
| 2.0's figures and the multiples | With the bars' shrink | 0.9 s | The figure counts from 1.1's down to 2.0's, and the multiple up from 1.0×: always 1.1's figure over the one shown |
| Pulse | The answer dot lands | 0.6 s | A new reading has just arrived. A violet outline grows 12 px from the pill and fades from 55% to nothing. |

The rings, bar and marker share one easing, `cubic-bezier(.3, .7, .2, 1)`. Dots move at a
steady speed.

### The readings

Six example readings, one per loop, 30 seconds in all. Then the session resets and it starts
again.

| Loop | 5h | 7d | AI | Outer ring |
|---|---|---|---|---|
| 1 | 15% | 45% | 17% | Purple |
| 2 | 38% | 46% | 17% | Purple |
| 3 | 61% | 48% | 18% | Purple |
| 4 | 74% | 49% | 18% | Amber |
| 5 | 86% | 51% | 19% | Amber |
| 6 | 93% | 52% | 19% | Red |

The inner ring and the bar stay purple throughout.

### Without script, and with reduced motion

| Reader | Sees |
|---|---|
| No script | The pill at 15%, 45% and 17%, purple, the marker at 15%. The dots still travel; the pill does not change. |
| Reduced motion | The same pill. No dots, no pulse, nothing moving. |

The page says everything in both cases: the wires, the boxes, the key and "What it sends" carry
it without the motion.

## The manual

`site/manual/index.html` wears the same look: the same ground, colours, faces, header and
footer. Its words stay as they are; they are the app's facts. Nothing on it moves.

| Part | How it looks |
|---|---|
| Header | The mark and the name, linking home. Nothing else. |
| Title | "Manual", in Sora 600 at the headline's size, with its one-line lead in Soft |
| Contents | The seven section links in a wrapping row, Violet text, each at least 44 px tall |
| Section | Starts under a 2 px Violet rule. Heading in Sora 600, 20 px. 40 px above the rule. |
| Row | Label, name, then text. From 1000 px: three columns of 96 px, 240 px and the rest. Narrower: label and name on one line, text under them. 1 px Line rule between rows. |
| Label | Sora 600, 13 px, Violet text |
| Name | Inter 600, 16 px, Ink |
| Text | Inter 16 px on a 1.6 line, Ink, at most 62 characters wide. Lists under it: 15 px, Soft. |
| Code | JetBrains Mono 13 px in a Box with a 1.5 px Line border and 8 px corners. It scrolls sideways inside itself. |
| Captures | The app's own pictures, unchanged, with 8 px corners. Captions 14 px, Soft. |
| Home link | At the foot, shaped like the Manual button |

The captures the landing page no longer holds sit in "What it shows", each beside the row it
explains:

| Row | Capture |
|---|---|
| Details | Copilot's details (already there) |
| Style | The details panel with its two switches, Shows and Style |
| Bar colour | "Light, with Windows" and "Your Windows accent" |
| Show | The two taskbars: the pill and the tray icon, and just the tray icon |

The switches are buttons at least 44 px tall: pressed is a Violet fill with white text, not
pressed a 1.5 px Violet border with Violet text. The panel's box is 504 px tall, the tallest
picture, so nothing shifts when a switch is pressed. With no script the first picture shows and
the switches stay hidden.

## The page must not

- Show another company's logo. The one exception is GitHub's mark on the Download button:
  unmodified, white, 24 px. "Claude Code" and "GitHub Copilot" are plain text.
- Use violet as a ground, for the page or for a section.
- Stretch to the window's height, or leave tall empty bands between parts.
- Put anything in the header besides the name.
- Set the headline above 36 px.
- Show the pill with a colour, threshold or figure face the app does not have.
- Use a colour outside the two lists above, or load a font from another server.
- Move anything outside the motion table.
- Hide content behind script or a tap.

## Decisions

| Decision | Reason |
|---|---|
| Sobreiro gets a design of its own, suited to what the tool does | The page before was made to be shared by several projects |
| A light lavender ground, violet as accent only | A violet field was too strong |
| Headline, buttons and steps on the left, the diagram on the right | |
| Download carries GitHub's mark; Manual is a second button beside it; the header holds only the name | The download is on GitHub |
| The pill is drawn and live, not a capture | Rings that grow and change colour are what appeals, not the current assets |
| Rings grow and figures count up each time an answer lands, in the app's colours and at its thresholds | |
| No large empty bands; the page does not stretch to the window's height | |
| Headline 36 px on a desktop, 26 px on a phone | 46 px is too large |
| The two notes sit beside the content on one row, each under a violet rule: "Before you install" under the words, "What it sends" under the diagram | |
| The steps sit under a "Run it" heading and end level with the diagram's bottom | |
| No Claude or Copilot logos; product names in plain text | |
| With no script the dots still travel and the pill stays at its first reading | The style stays |
| The manual is rewritten in this look, and the README updated to match, in the same change | |
| A public repository names no other repository | |
| Violet stays the brand; Rust marks only the 2.0 release, on its line in the comparison | A rust brand would sit beside the pill's amber and red and read as a warning; the mark, favicon, share picture and app icon stay violet |
On the drawn pill: the rule for this project's pictures was that only the app draws them. Drawing the
pill on this page is the exception, for this page's diagram. The rule holds everywhere else.

On the logos: Anthropic's trademark guidelines ask for approval before its marks are used, and
GitHub retired the standalone Copilot logo.

## Not settled

- The manual's look above is drawn from the landing page's and is not rendered yet.
- A dark theme. The page is light only, and the theme switch is gone.
- The share picture (`og.png`) still shows the earlier dark page, and the README's hero the
  earlier purple field.
