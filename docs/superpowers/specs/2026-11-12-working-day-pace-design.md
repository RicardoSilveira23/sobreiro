# Pace over weekdays only (design)

2026-11-12. Asked for by Ricardo after showing Sobreiro to coworkers, who use it only on working
days.

## Why

The pace is a straight line over calendar time: `percent / share of the window gone`
(`Sobreiro.Core/Display/Pace.cs`). For someone who uses it Monday to Friday, the two days off
bend that line both ways:

- **False alarm.** The weekly limit reset on Monday 09:00; on Thursday 09:00 it stands at 58%.
  Calendar pace: 72 h of 168 h gone, heading for 135%, badge "Ends Fri…". Over weekdays: 72 h of
  120 h gone, heading for 97%, "On track".
- **False calm.** A reset on Friday followed by a quiet weekend makes Monday look far too
  comfortable.

## Goals

- A setting, **Pace › Every day / Weekdays only**, in the tray menu. Every day is the default,
  and with it nothing changes.
- With Weekdays only, the projection counts Monday to Friday and skips Saturday and Sunday, for
  every limit whose window is longer than a day: the weekly limit, Copilot's monthly AI credits,
  and a weekly or monthly spend limit.
- The badge, the "Ends …" time, the tick or faint fill on the bars, and the alert text all
  follow, since they all read the one projection.

## Non-goals

- Public holidays and days off: they need a calendar per country, and the app makes no other
  network calls.
- Working hours (09:00 to 18:00), and choosing which days count. Either can be added later
  beside this switch.
- A setting per service. One install has one setting.

## Anti-goal

- Anyone who never opens the Pace menu sees a different badge than they do today.

## Approaches

1. **A Weekdays switch (picked by Ricardo, 2026-11-12).** Monday to Friday count as whole days.
   One menu entry and one config key, and nothing to explain.
2. **Choosable days.** Seven tick boxes. Covers a Sunday-to-Thursday or four-day week, at the
   cost of a fiddlier menu for a case nobody raised.
3. **Days and hours.** The most exact for the weekly limit, but two more settings, and an
   "Ends …" time that jumps to the next morning.

## Design

- **Working time.** Between two moments, working time is the part that falls on Monday to Friday
  in the PC's time zone, midnight to midnight. A day made shorter or longer by a clock change
  counts as long as it is.
- **Projection.** The window runs from `ResetsAt - Window` to `ResetsAt`.
  `share = working time from the window's start to ObservedAt / working time in the whole window`,
  and `projected = Percent / share`. Below 5% of working time gone it stays "Too early", as today.
- **Run-out time.** When the projection passes 100%, the working time still needed is
  `working time gone × (100 - Percent) / Percent`. `RunsOutAt` is the moment that much working
  time has passed after `ObservedAt`, walking forward and skipping days off, so it never lands on
  a Saturday or Sunday.
- **Which limits.** Windows longer than one day. The 5-hour session and a daily spend limit keep
  calendar time, as does any window that holds no working time at all.
- **Setting.** `PaceDays { EveryDay, Weekdays }` in `Sobreiro.Core`, on `AppConfig` like `Show`:
  key `pace`, values `everyday` and `weekdays`; a missing or unknown value reads as every day.
- **Menu.** A "Pace" submenu after "Bar colour", with "Every day" and "Weekdays only", built and
  ticked the way "Show" is.
- **Plumbing.** `Pace.Project` takes the setting and the time zone. `LimitStatus.For` and
  `FlyoutModel.From` pass them on; the tray passes the config's value, and so does the alert
  policy. The rendered media and the snapshot tool keep every day.

## What it does at the edges (Weekdays only)

- **Use on a day off.** Use rises while working time stands still, so the projection gets
  stricter. That is correct: the allowance for the working days is being spent.
- **Only days off left before the reset.** All working time has gone, so the projection is the
  current figure.
- **A reset that falls on a weekend.** "Too early" until 5% of the working time has gone: six
  hours into Monday for the weekly limit.
- **Evenings.** Whole days count, nights included, so at the end of a working day the pace reads
  a little strict and is right again the next morning. Every day mode does the same today. It is
  up to about 12 points of the weekly window and about 3 of the monthly one.

## Tests (xUnit, `PaceTests` and `AppConfigTests`)

- The two cases under Why, with the figures given there.
- Use on a Saturday; only a weekend left; a weekend reset reading "Too early" then a projection.
- A run-out time that would land on Saturday lands on Monday instead.
- The session limit and a daily spend limit give the same result in both modes.
- Copilot's monthly window: working time counted across a whole month.
- A week with a clock change.
- Config: the key round-trips, an unknown value reads as every day, and a config without the key
  loads as every day. Saving writes the key every time, as it does `show`.

## Left to the builder

- How working time is summed (day by day is fine: a window is at most a month).
- Where the setting sits in the signatures, as long as every day stays the default argument.
- The line for `docs/plan.md` under Flyout and Menu.
