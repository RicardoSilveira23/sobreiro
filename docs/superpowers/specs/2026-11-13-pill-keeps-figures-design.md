# The pill keeps its figures when the numbers stop

Part of 1.0. Written 2026-11-13. It changes the pill half of
[the design merged that morning](2026-11-13-pill-says-why-design.md); the states, the rule for
when, and the flyout's sentences from that design stay.

## Why

Ricardo put a build on his work PC, saw Claude's block read `sign-in` / `expired` where its
figures had been, and wrote: "I would prefer to keep the numbers just maybe have a * or some
other visual cue that the numbers are not updated instead of that message "sign-in expired"".

The last figures are still worth a glance. The cause and what to do about it are in the flyout,
which already says both.

## Goals

- A block whose numbers have stopped keeps its last figures.
- It carries one cue that says "not updated", readable without a hover.
- The flyout stays the place that names the cause.

## Not goals

- The flyout's sentences, the tray icon, the tooltips and the polling stay as they are.
- No new network call, and no change to how a token is read.

## What would make it a failure

- An old figure that looks current: the cue missed at a glance.
- A cue that flashes because one poll was missed.

## Approaches

Five cues were drawn at real size, in both themes, for both blocks:

1. `*` after the figure, colours kept. The `*` alone is easy to miss beside a ring in full
   colour.
2. **`*` after the figure, and the ring and bar keep their fill in grey (picked by Ricardo,
   2026-11-13).** The figures stay at full strength; the colour going is seen without reading.
3. Figures dimmed and the fill grey, no `*`. Harder to read on a taskbar.
4. Figures dimmed, the fill grey, and `*`. Says it three times.
5. The fill dashed, colours kept. Busy at 26 px.

## The design

- **A stopped Claude block** shows `5h 37%*` and `7d 78%*`: each figure it has, then `*`. Its
  rings keep their sweep, drawn in the taskbar's ink at half strength in place of the level
  colour. The track does not change.
- **A stopped Copilot block** shows `AI 40%*` over its reset date, which is unchanged. Its bar
  keeps its height, in the same grey.
- **One cue for every cause.** The pill no longer says which; the flyout does, from the first
  failed poll, as today.
- **The rule for when is unchanged:** two polls in a row that brought nothing, the last ended
  in one of the stopped states, and the newest figure is over 15 minutes old for Claude Code or
  over 24 hours old for Copilot.
- **Nothing to keep:** a Claude block with no session or weekly figure shows `5h –` and `7d –`
  with empty rings and no `*`, as it does before a first reading. A line with no figure never
  gets a `*`. With no Copilot reading there is no Copilot block, as today.

## Shape in Core

- The type that held the two words becomes the rule alone: given a state, the readings, the
  time and the misses, it answers whether a block's figures have stopped. The word pairs go.
- `PillModel` keeps the figures and the fractions of a stopped block and carries a flag per
  line and per block; it no longer empties the rings or the bar.
- `PillView` draws the `*` in the label font after a flagged figure, and a flagged block's fill
  in grey.

Ricardo approved this design on 2026-11-13: the cue from the drawn variants; one cue for every
cause, the unchanged rule and plain dashes when there is nothing to keep as put to him.

## Calls made here, not by Ricardo

- The grey is the ink at half strength, as drawn in the variant he picked.
- The `*` is not added to Copilot's reset date: the date is not a reading that goes stale.

## Left to the builder

Names of the type and its members, and the drawing details. The stopped pills are rendered at
real size in both themes on the snapshot sheet before the pull request.
