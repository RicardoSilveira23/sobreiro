# Sobreiro's page (design)

2026-11-11. The design of Sobreiro's site: the landing page, `site/index.html`, and the manual
beside it. Ricardo approved the landing page's design on 2026-11-11. Build plan:
[superpowers/plans/2026-11-11-own-page.md](superpowers/plans/2026-11-11-own-page.md).

It replaces the look in [the first site design](superpowers/specs/2026-11-09-site-design.md).
Every colour and motion the page uses is written here, and every size that shapes it. A colour
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
| 6 | Footer | Two links, the name line and the disclaimer |

24 px between parts.

### Desktop (1000 px and wider): two columns

```
Header ............................................
Words                    |  Diagram
Before you install       |  What it sends
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

### The words

| Where | Text |
|---|---|
| Headline | Your GitHub Copilot and Claude Code usage, always in view on your taskbar. |
| Buttons | **Download** (the latest release on GitHub) and **Manual** (the manual page) |
| Meta line | Free, for Windows 11. Needs GitHub Copilot with AI credits, or Claude Code on Pro, Max or Ultra. |
| Run it | 1 Download Sobreiro.exe, one file. 2 Open it. Then, beside the mark: The pill appears beside the clock. |
| What it sends | Two requests, with a sign-in already on this PC. The numbers come back and go to the taskbar. Nothing else leaves: no telemetry, no update checks, no token stored, logged or renewed. |
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
| Violet | `#6f4df6` | Download's fill, Manual's border, the two rules, the step circles, the request dot, the pulse |
| Violet text | `#5a3dd1` | Manual's label, step numbers, footer links |
| On violet | `#ffffff` | Download's label and GitHub's mark |
| Line | `#b9adf5` | Wires, box borders, the line joining the steps |
| Box | `#f6f3ff` | The fill of the diagram's boxes |

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
on the page moves: no entrances, nothing on scroll, nothing on hover.

| Motion | Trigger | Length | Tells the reader |
|---|---|---|---|
| Request dots | The loop starts | 1 s, from 0.2 s | Sobreiro asks both addresses, at the same moment. Violet dots go down the two wires. |
| Answer dots | A request dot arrives | 0.3 s to turn ink, then 1.1 s back up | What comes back is a different thing from what went out, on the same path |
| Answer dot to the pill | The answers reach Sobreiro.exe (2.7 s) | 1 s, landing at 3.9 s | The numbers go to the taskbar and nowhere else |
| Rings and bar step | The answer dot lands | 0.9 s, slowing to a stop | Usage is what the rings draw. A change of colour takes 0.35 s, starting 0.25 s in. |
| Figures count | The answer dot lands | 0.8 s | The same reading, as a number |
| Marker on the scale | The answer dot lands | 0.9 s | Where the session stands against 70% and 90%, so the colour change has a reason |
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
| No script | The pill at 15%, 45% and 17%, purple, the marker at 15%. The dots still travel; the pill does not change. Ricardo kept this as it is. |
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

## Decided by Ricardo

All on 2026-11-11.

| Decision | His words |
|---|---|
| Sobreiro gets a design of its own, suited to what the tool does | The page before "was shooved into this project by a collective design system phylosfy" |
| A light lavender ground, violet as accent only | A violet field had "too much purple it becomes very too strong" |
| Headline, buttons and steps on the left, the diagram on the right | "B for content, A for content layout" |
| Download carries GitHub's mark; Manual is a second button beside it; the header holds only the name | "since thats where it happens" |
| The pill is drawn and live, not a capture | "dont get fixated on the current assets ... rendering the circles growing and changing colors that what I think is appealing" |
| Rings grow and figures count up each time an answer lands, in the app's colours and at its thresholds | |
| No large empty bands; the page does not stretch to the window's height | |
| Headline 36 px on a desktop, 26 px on a phone | 46 px "seems too huge" |
| The two notes sit beside the content on one row, each under a violet rule: "Before you install" under the words, "What it sends" under the diagram | |
| The steps sit under a "Run it" heading and end level with the diagram's bottom | |
| No Claude or Copilot logos; product names in plain text | |
| With no script the dots still travel and the pill stays at its first reading | "the style is to stay" |
| The manual is rewritten in this look, and the README updated to match, in the same change | "also rewrite the manual and update readme before we open PR" |
| A public repository names no other repository | |

On the drawn pill: the rule for this project's pictures was that only the app draws them
([rendered media design](superpowers/specs/2026-11-10-rendered-media-design.md)). Drawing the
pill on this page is his decision, for this page's diagram. The rule holds everywhere else.

On the logos: Anthropic's trademark guidelines ask for approval before its marks are used, and
GitHub retired the standalone Copilot logo.

## Not settled

- The manual's look above is drawn from the landing page's. Ricardo has not seen it rendered yet.
- A dark theme. The approved page is light only, and the theme switch is gone.
- The share picture (`og.png`) still shows the earlier dark page, and the README's hero the
  earlier purple field.
