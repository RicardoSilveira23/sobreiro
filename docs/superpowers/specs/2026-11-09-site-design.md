# Site: Sobreiro's docs on the web (design)

2026-11-09. Part of milestone 4 ("README with screenshots, setup, limits"). Ricardo settled the
hosting, the content split and the look in the brainstorm; the rest is marked as my call.

## Why

Ricardo wants colleagues at work to try Sobreiro. They need one place that says what it is,
lets them download it, and walks them through setting it up, without reading the repo. The
shape is a landing page with the way to run it, and a manual beside it. That shape sets the
content and structure only. The look is Ricardo's Night design and the
words are written fresh in his voice from Sobreiro's own docs.

For now the site is for Ricardo to check locally. It goes live the day the repo goes public
(milestone 4), together with the releases it links to, and so does his personal site.

## Goals

- A home page that shows what Sobreiro is, the real app, a download, the three install steps,
  what it reads and sends, and the main known limits.
- A manual with everything a user does, said once.
- Deployed by GitHub Pages from `site/` once switched on, with no build step.
- Pictures that are the app's own rendering, crisp on scaled screens, and reproducible with one
  command.
- `docs/plan.md` updated.

## Non-goals

- Going live now. The repo is private and Pages on a private repo needs a paid plan; the deploy
  stays off until the repo is public.
- A custom domain, analytics, a search box, a blog or a changelog page.
- GitHub Copilot on the site, even as "coming later" (Ricardo left it off).
- The 5-hour-only display option, which was dropped: never mentioned.

## Anti-goals

- Anything that looks generated: run the tells in the design system's `language.md` on
  every page.
- Any request to a third party from the pages: no font CDN, no scripts or styles from elsewhere,
  no trackers. The app makes one network call and the site should match that spirit.
- A token, or anything from `.credentials.json`, in any picture or text.
- The private reason behind the name. The site may say Sobreiro is Portuguese for cork oak,
  whose trunk in section is the tray's two rings, as the README does.
- Copy, phrases or layout taken from another project's site.

## Approaches

**Hosting** (Ricardo picked 1):

1. GitHub Pages from `site/`, deployed by a workflow. No account, secret or
   login to set up; the address is `https://ricardosilveira23.github.io/sobreiro/`; live once the
   repo is public. No per-PR preview: preview locally.
2. A Cloudflare Worker. Previews while private, but another API
   token and Access setup, and public only on a domain he doesn't have yet. Colleagues still
   couldn't download anything while the repo is private.

**Stack** (my call, unchallenged): plain HTML and CSS in `site/`, hand-written, no build step
and no framework. Astro like his other sites was the alternative; for two pages it adds a build
and a dependency tree for nothing.

**How the home page shows the app** (Ricardo picked B from three real-size candidates in
`docs/candidates/site/`, on 2026-11-09):

- A, on the ground: the captures on the plain ground right of the intro. Quiet, but the app
  reads small and the purple is only the rule.
- **B, one field (picked):** one `--sobreiro` tile across the column holding the app as it sits
  on screen: the flyout above a strip of taskbar with the pill and the tray icon, and a short
  heading and sentence at its bottom left. Night's "one colour field per piece of work, the
  product shown large and real".
- C, the wall: a tile each for the flyout, the pill and the icon. Three tiles of one purple were
  loud and the small tiles mostly empty.

## The design

### Colour and type

- Night's foundation: copy the design system's `tokens/foundation.css` into `site/assets/`,
  without the other projects' tile colours. Sobreiro's own tokens go in `site/assets/sobreiro.css`:
  - `--sobreiro-text`: `#a594ff` on dark grounds, `#5a3dd1` on light, the app's own purple.
    It takes every place `--red` has on the personal site: the rule, link and nav hover, the
    current page, the theme switch on hover.
  - `--sobreiro` (from the foundation, `#6f4df6` dark, `#5a3dd1` light) is the fill: the field
    and the button's hover, with `--on-sobreiro: #fff` as text (5.16:1 and above). `--bg` on
    the fill falls short, unlike the personal site's red.
  - No `--red` anywhere on these pages.
- Fonts: Inter and Unbounded, latin `wght` variable woff2 from `@fontsource-variable/inter` and
  `@fontsource-variable/unbounded` 5.3.0 (the files Ricardo's website uses), self-hosted in
  `site/assets/fonts/` with their OFL licences.

### Parts (all from the design system's `patterns.md`, with their measures)

Header, Lettering (SOBREIRO; with 8 letters it needs a larger size than the personal site's 15,
so the letters fill the column without wide gaps: the candidate's `font-size="120"` in a
`0 0 1000 94` box), Rule, Intro, Section, Row, Button, Footer, Theme switch, and the CV page's
layout for the manual. The mark is `scripts/sobreiro-mark.svg`, the app icon's own drawing.

Two parts are new to the family, and go into `patterns.md` once Ricardo approves the look:

- **Field:** the picked B. A `--sobreiro` tile, 10px radius, 28px padding, across the column,
  40px under the button. Left, at the bottom: a heading in Unbounded 700 (the tile name's
  measure) and one sentence in Inter 14 to 15px at .9 opacity, 34ch at most. Right: the flyout
  above the taskbar strip, right-aligned, at the pictures' 1x CSS size. Under 720px it stacks,
  the pictures left-aligned. No motion.
- **Code:** the manual's commands and JSON. A `pre` on `--surface`, 10px radius, 18px padding,
  `ui-monospace, Consolas, monospace` at 14px, scrolling sideways inside itself, never the page.

Motion is Night's 1 (the lettering settles in), 2 (theme fade), 6 (sun or moon turns) and 7
(the button's arrow), all under `prefers-reduced-motion: no-preference`. Nothing else moves.

### Pictures

`--snapshot` today draws one sheet at 1x from a fixed sample. The site needs crisp crops and one
consistent reading, so the app gets a second form: `Sobreiro.exe --snapshot-site <folder>`. It
writes, at 2x (192 DPI), from the sheet's sample reading (session 27%, weekly 74%) with Live
usage off:

- `flyout-dark.png`, `flyout-light.png`: the Bars flyout in Sobreiro purple, each theme, with
  transparent corners outside its 10px radius;
- `strip-dark.png`, `strip-light.png`: the taskbar pill and the tray icon side by side on that
  theme's taskbar colour, as the sheet draws them, at the same reading.

The pictures show a 100% taskbar drawn at 2x, so the proportions are real: the tray icon is drawn
at its 32 px size, to show as 16 CSS px beside the pill, not scaled up. The flyout is drawn alone,
with transparent corners; the strip alone, on its taskbar colour, at the height the sheet's pill
row has. The output is deterministic: a fixed "now" and a fixed culture that gives the 24-hour
form the sheet shows ("Ends Tue 23:21"), so a re-run changes no pixel unless the app's look
changed. (My calls.)

The PNGs are committed under `site/assets/img/`, since CI can't render WPF. To refresh them:
build the tray, then `Sobreiro.exe --snapshot-site site/assets/img`, documented in the README's
building section. The sheet itself is unchanged. Any logic this adds outside drawing (argument
handling, the fixed clock) goes in `Sobreiro.Core` with an xUnit test; the drawing is checked by
running it.

The page shows each at half its pixel size, with `width` and `height` set, and the flyout and
strip for its own theme (the flyout follows Windows the same way). Each picture is a pair of
`<img>`, one for each theme, shown by the same CSS as the theme switch; both carry the same
`alt`, and only the visible one reaches a screen reader. A `<picture>` can't follow the switch.

### Pages and their content

Every fact comes from `README.md`, `docs/plan.md`, the release design
(`docs/superpowers/specs/2026-11-09-release-workflow-design.md` on the release branch) and the
code; nothing from memory. Short, plain, sentence case, Ricardo's voice.

**Home, `site/index.html`:**

1. Header: the mark as the home link ("Sobreiro, home"), then Manual, GitHub, theme switch.
2. Lettering, rule.
3. Intro: "How much of Claude Code's session and weekly limits you have left, always in view on
   the Windows taskbar." Meta right: "For Windows", one line. No Windows version: the pill was
   checked on Windows 11 only, which the manual's known limits say. No licence claim either:
   the repo has no `LICENSE` file yet, and adding it belongs to making the repo public; the
   footer gains "MIT licence" then. (My calls.)
4. The button: "Download from GitHub →", to
   `https://github.com/RicardoSilveira23/sobreiro/releases/latest`. A right arrow, since it opens
   the release page, not a file: the zip's name carries the version, so there is no stable
   direct link. No version number anywhere on the site, so it never goes stale.
5. The field (above). Heading "Two rings and a flyout"; sentence on the outer ring being the
   session, the inner the week, and clicking for the rest.
6. Install, count "3 steps": Rows. Download and extract (Unblock in Properties first, then
   extract to `%LOCALAPPDATA%\Programs\Sobreiro` and keep it there, since the status line points
   at the bridge's path); Start Sobreiro (SmartScreen: More info, then Run anyway, until it is
   signed; where the pill and the tray icon appear); Connect to Claude Code (right-click, what
   it shows and backs up; when the numbers appear). Needs: Claude Code signed in on a Pro, Max
   or Ultra plan, or behind a gateway with a spend limit.
7. What it reads and sends: Rows, from the README's Live usage section and the build rules: the
   one address, the access token read into memory per request and never stored, logged or
   renewed; the bridge writing only `rate_limits` and a timestamp; nothing else leaves the
   machine; Anthropic's terms and the undocumented address stated plainly, as the README does.
8. Known limits: the three that matter most to a colleague (Live usage paused means terminal
   only; Pro, Max and Ultra only; the pill on the primary monitor's bottom taskbar), then the
   Button "All N known limits →" to the manual, N counted from the manual's list.
9. Footer: GitHub and "Open an issue" links; one `--mute` sentence that Sobreiro is the
   Portuguese name of the cork oak, whose trunk in section is the tray's two rings; "Not
   affiliated with or endorsed by Anthropic."

**Manual, `site/manual/index.html`**, one long page laid out like the CV page: the listing title
MANUAL, then two columns aligned to their bottoms, one sentence on the left and, where the CV has
its contact lines, the section names as in-page links in the same meta form (plain 15px lines,
purple on hover), going under the sentence below 720px (my call). Then Sections of Rows with
detail lists, and Code blocks where a user types or pastes something. Sections, in order:

1. Install (the home's steps with their detail)
2. Connect to Claude Code: Connect, Disconnect, Reconnect, manual setup (the JSON), what both
   refuse to change
3. What it shows: tray icon and its colours, the taskbar pill and when it hides, the flyout, the
   status badges (each badge and what it means), the styles (Bars, Battery, Thermometer), bar
   colour, Show, toasts
4. Live usage: what it does, when it fetches and backs off, what the flyout's last line says
5. What it reads and sends, and the data folder: `usage.json`, `config.json`, the log
6. Uninstall (new, written from the code): Disconnect, untick Start with Windows, Quit, delete
   the app folder and the data folder
7. Known limits: all of them

(An "Upgrading from OpenHeadroom" section was dropped on Ricardo's word: OpenHeadroom was never
published, so there is no one to upgrade.)

The header's Manual link is the current page there; "← Home" closes the page above the footer.

**README** gets shorter in the same PR: what it is, one picture, links to the site and the
latest release, "What it reads, what it sends" (IT reads the repo), building from source, the
layout and the licence. The picture is `site/assets/img/flyout-dark.png` by relative path. Every
user step moves to the manual.

The install steps (the self-contained zip, `Sobreiro.exe` and `sobreiro-bridge.exe` side by side,
no .NET needed) follow the release design Ricardo approved, which is not merged yet. The release
branch also edits the README ("Install from a release"), so the README part is done last, on top
of `main` once that PR has merged. If it is still open when the rest is ready, ask Ricardo
whether to wait or to split the README into a follow-up PR.

### Deploy

`.github/workflows/pages.yml`:

- Pull requests touching `site/**` or the workflow: run the site check.
- Push to `main` touching the same, or a manual run: the check, then deploy `site/` with
  `actions/configure-pages`, `actions/upload-pages-artifact` (path `site`) and
  `actions/deploy-pages`, with `pages: write` and `id-token: write` on the deploy job only.
- The deploy job runs only when the repository variable `PAGES` is `"true"`, the same switch
  Ricardo's other repos use (`DEPLOY`). Turning it on is part of making the repo public:
  Settings, Pages, Source "GitHub Actions", then `gh variable set PAGES --body true`.
- Actions pinned by commit SHA with the version in a comment.

**The site check**, `scripts/check-site.mjs`, Node with no dependencies, written test-first:

- every local `href` and `src` in `site/**/*.html` and every `url()` in its CSS resolves to a
  file in `site/`;
- nothing is loaded from another host (`<link>`, `<script>`, `<img>`, CSS `url()`); links to
  other sites are fine;
- every `<img>` has `alt`, `width` and `height`;
- each page has a `<title>`, `lang`, a viewport tag and a meta description.

### Local preview

`python -m http.server` on `site/` (and on `docs/candidates/site/` for the candidates), through a
`.claude/launch.json` that is not committed. All links in the site are relative, so the same
files work locally and under `/sobreiro/` on Pages.

## Checks before showing Ricardo

- The page-design skill's audit passes (sameness table, motion list, colour field loudness on
  captures, facts counted, the tells, the rest of the language) on both pages.
- Captures with `capture.mjs`: each section in light and dark at rest, each at 390px, and the
  lettering's middle and end frames. `pageWidth` equals `innerWidth` at 390px.
- Contrast: every text colour 4.5:1 on its ground in both themes, including white on the field.
- `node scripts/check-site.mjs` passes; `dotnet build` (warnings are errors) and `dotnet test`
  pass.

## After Ricardo approves the look

- The design system's `patterns.md` gains Field and Code, and `language.md`'s Sobreiro row says it
  now has a site; the reason goes in memory.
- `docs/plan.md`: milestone 4 gets the site, its address, and the steps to switch Pages on.
- The candidate page stays in `docs/candidates/site/` as the record of the pick.

## Left to the builder

The exact wording within the voice and facts above, the Rows' keys, file names under
`site/assets/`, the favicon sizes (from `scripts/sobreiro-mark.svg`), the code and order of the
check script, and how `--snapshot-site` composes its pictures inside the app.

## Revision after Ricardo's first look (2026-11-09)

Ricardo's verdict on the built home page: "flyout" is jargon, the drawn pictures don't show the
app in its place, the install reads as work when it is download and run, and the
page doesn't make him want to try it. Agreed with him:

- **Real captures from his PC replace the drawn ones.** A recording of his screen's bottom-right
  corner (2560x1440 at 100%, the pointer drawn in) while he used the app gave a 13-second demo
  (`demo.webp`: hover for the tooltip, click for the details, right-click, Style, Battery) and
  stills: the taskbar with the pill and tray icon, the tooltip, the menu, and the details in Bars,
  Battery and Thermometer. Under reduced motion the demo is a still (`demo-still.png`) through
  `<picture>` with a `prefers-reduced-motion: no-preference` source. Nothing else from his screen
  is kept; the frames were deleted after cutting.
- **`--snapshot-site` goes.** With real captures nothing uses it, so its commits are reverted and
  the PR leaves the app untouched (YAGNI; my call).
- **Plain words.** "The details" (or "the details panel") instead of "flyout" on both pages.
- **The home follows a product page's usual order** in Night: what it is, see it working, run it, what it
  sends, known limits. The field holds the demo. A new section, "On your taskbar", shows the
  stills as captioned figures on the plain ground. "Run it" is two steps (download; unzip and run
  Sobreiro.exe) and one line of what else to know, pointing at the manual.
- The manual's install matches (two steps, then the details: SmartScreen, keep the folder where
  it is if you Connect, no .NET), and it uses "the details" throughout.

## Second revision, after Ricardo's second look (2026-11-09)

He found the home not refined: too many screenshots, and they didn't belong to the page. On
`docs/candidates/site-pictures/` he compared two fields and two ways to show the styles, and took A
and 1:

- The screen behind each capture is cut out, so the app's windows sit straight on the page. In dark,
  the details panel's ground is almost the page's, so it reads as part of the site.
- The field is the desktop: the purple ground, with the Windows taskbar (its own pixels, #1c1c1c
  under a #404040 line) along the field's foot and the real pill, tray and clock at its right end.
  The details, the tooltip and the menu open onto the purple.
- "On your taskbar" and its six figures go. "Reading it" replaces them: rows for 5h, 7d, the badge
  and the style, beside one details panel whose Style switch (Bars, Battery, Thermometer) swaps it.
  The home shows two pictures instead of seven.
- The demo is recorded again at 60 frames a second; at 10 it looked sluggish.
- Not taken: B, the field as the grey screen corner (quieter, but no purple); 2, the three styles in
  a row (four pictures).

## Third revision, the light theme (2026-11-09)

Ricardo found the light theme off: the cut-outs' corners showed jagged light notches, and the purple
field might be too much on white.

- **The corners.** The captures were keyed off a #191919 backdrop, 13 levels from the panel's
  #262626 border, so the key took the corners' anti-aliased pixels too: a binary staircase with
  holes. It hid on the dark page and showed on white, where the dark page also lost the border at
  each corner. Each corner of the details panel is painted again as Windows 11 draws it, an 8px
  radius with the 1px border around the #0b0b0b ground, its coverage supersampled into alpha: in
  the three panel pictures, the still, and the four demo frames that draw a corner (the other 235
  are unchanged). A CSS radius couldn't do it: it would cut the border off on dark and can't reach
  into the demo. A new recording needs a backdrop far from the windows' own colours.
- **The field in light.** The light theme darkened the field to the light tile colour, #5a3dd1, so
  white text would read; on white that made it the heaviest thing on the page. On
  `docs/candidates/site-field/` he compared four (A, as it was; B, the dark theme's #6f4df6; C, the
  light purple #a594ff with dark text; D, a pale #ede8fe with dark text, where the demo's light
  menus fade into the field) and took C. The field's colour is `--desk` in `sobreiro.css`, its text
  `--on-desk`: the purple tile and white on dark, #a594ff and `--ink` (7.7:1) on light.
