# Say on the pill why the numbers stopped

Issue 14, part of 1.0. Written 2026-11-13.

## Why

Both usage addresses are undocumented and can change, and Claude Code's sign-in expires when no
terminal has used it for a while. Today the pill keeps showing its last figures, so an
eight-hour-old session figure looks current. The flyout footer names some of the causes, in
small print, and does not say what to do.

## Goals

- Each way the numbers can stop has its own state in `Sobreiro.Core`, with a test.
- The pill says which one it is, in two short words, where the figures were.
- The flyout says what to do about it.

## Not goals

- The tray icon and the tooltips stay as they are.
- With Live usage off nothing changes: numbers that wait for Claude Code's next response are not
  a fault.
- No new network call, and no change to how a token is read. Sobreiro still never renews,
  stores or logs one.

## What would make it a failure

- A word that flashes on the pill because one poll was missed.
- A figure that is still good being hidden.

## Approaches

1. **Words replace the figures (picked by Ricardo, 2026-11-13).** The two lines of a block
   become the reason and its ring or bar goes empty. The last figures stay in the flyout with
   their age. Nothing old is shown as if it were current.
2. Figures stay, dimmed, with the word beside them. Nothing is lost, but the pill grows by
   about 50 px and still shows old numbers.
3. A mark on the pill, the reason in the tooltip. Dropped: the reason would need a hover.

## The states

Claude, from the Live usage poll:

| State | When | Pill | Flyout |
|---|---|---|---|
| Expired | the sign-in's expiry has passed | `sign-in` / `expired` | Claude Code's sign-in has expired. Use Claude Code in a terminal and it renews it. |
| NoSignIn | no sign-in file, or one that can't be read | `no` / `sign-in` | Claude Code isn't signed in on this PC. Run `claude` in a terminal and sign in. |
| Refused | 401 or 403 | `sign-in` / `refused` | Anthropic didn't accept Claude Code's sign-in. Use Claude Code in a terminal so it gets a new one. |
| Offline (new) | no connection, or no answer within 10 seconds | `no` / `network` | Sobreiro can't reach Anthropic. Check the connection; it tries again every 2 minutes. |
| BadAnswer (was Failing) | any other refusal, or an answer in a shape Sobreiro doesn't know | `no` / `answer` | Anthropic's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro. |
| RateLimited | 429 | `asked` / `to wait` | Anthropic asked Sobreiro to wait. Nothing to do: it asks again by itself. |

Copilot has the same states without Expired, with GitHub in place of Anthropic and the Copilot
CLI or `gh` as the place to sign in.

## The rule

- **Claude's block shows the words** when Live usage is on, its last poll ended in one of the
  states above, and the newest Claude reading is over 15 minutes old or there is none. Until
  then it keeps its figures. A status line that still delivers fresh numbers keeps the figures
  whatever the poll says.
- **Copilot's block shows the words** when it has a reading, its last poll ended in one of the
  states above, and that reading is over 24 hours old. Its budget is monthly, so a figure from
  the Copilot CLI's cache stays good for hours; a PC where that cache is the only source must
  not lose its figure. With no reading there is no Copilot block, as today.
- **The flyout** says its sentence from the first failed poll, as today, in the places it
  already uses: the footer for Claude, the line under the figures for Copilot. Its figures and
  their age stay.

## Shape in Core

- `LiveState` and `CopilotState` each split Failing into Offline and BadAnswer. The clients
  already tell the two apart (`Unreachable` and `Failed`).
- One new type in `Sobreiro.Core/Display` holds the rule and the words: given a state, the
  readings and the time, it answers the two pill words or nothing.
- `PillModel` carries the words for each block; `PillView` draws them in the label font.
- `LiveStateText` and `CopilotStateText` get the new sentences.

## Calls made here, not by Ricardo

- The two thresholds, 15 minutes and 24 hours.
- A 5xx answer counts as BadAnswer, not Offline: the address answered.
- RateLimited gets words too, though the issue does not list it.
- The flyout keeps the sentence where it is instead of gaining a notice at the top.

## Left to the builder

The exact Copilot sentences, names of the new type and its members, and the pill's drawing
details. The states are rendered at real size for Ricardo to check the words before the pull
request.
