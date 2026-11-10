# Going public Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ready the repo to go public: GPL-3.0 everywhere, plus the files a public repo has,
while saying plainly that only Ricardo changes it.

**Architecture:** Mostly text files. One small app change: the exe embeds `LICENSE`, and About
Sobreiro names the licence. Repo settings come after the merge and are not part of this plan.

**Tech Stack:** Markdown, GitHub issue forms (YAML), CFF 1.2.0, C# / .NET 10 WinForms (the
licence window).

**Spec:** [docs/superpowers/specs/2026-11-10-going-public-design.md](../specs/2026-11-10-going-public-design.md)

## Global Constraints

- Licence: GPL-3.0-or-later. The `LICENSE` text is GitHub's canonical GPL-3.0 text:
  `gh api licenses/gpl-3.0 --jq .body`.
- Copyright line: "Copyright 2026 Ricardo Silveira".
- Releases up to 0.8.1 were MIT. Say so wherever the licence is named, and nowhere else.
- Only Ricardo changes the repo. Pull requests are limited to collaborators, while issues and
  forks are open to everyone. Never write anything that invites pull requests.
- Addresses: site `https://sobreiro.ricardosilveira.dev`, manual
  `https://sobreiro.ricardosilveira.dev/manual/`, repo `https://github.com/RicardoSilveira23/sobreiro`,
  private security form `https://github.com/RicardoSilveira23/sobreiro/security/advisories/new`.
- Voice: the README's. Plain words, short sentences, British spelling ("licence" for the noun),
  no hype. First person ("I") in `CONTRIBUTING.md` and `SECURITY.md`, which speak for Ricardo.
- PR #21 (open) changes README lines 16-23, 110-185, 213-231 and `docs/plan.md` 334-358. Keep
  this branch's edits outside those hunks so both merge cleanly.

## Review Focus

- The single-file exe: `scripts/package.ps1` bundles the app dll, so the embedded `LICENSE.txt`
  must survive it. Check with `dotnet build` and the resource list, not only the source.
- A missing resource: `LicenceWindow.Read` already prints "<name> is missing." Keep that
  behaviour; never throw from About.
- Issue forms that GitHub rejects show no form at all. Every `body` item needs a `type` and an
  `id`, and `attributes.label`. Dropdowns and inputs need unique ids.
- Stale facts in the security text. Each claim in `SECURITY.md` must match the README's "What
  it reads, what it sends" section; the reviewer checks them line by line.
- A leftover "MIT". After Task 1, `git grep -n "MIT"` may only hit `docs/brief.md` (the
  original brief, kept as written), the "up to 0.8.1" notes and specs/plans that record history.

---

### Task 1: GPL-3.0 everywhere

**Files:**
- Replace: `LICENSE`
- Modify: `src/Sobreiro/Sobreiro.csproj:15` (embed `LICENSE`)
- Modify: `src/Sobreiro/LicenceWindow.cs` (GPL first, title "Licences")
- Modify: `src/Sobreiro/TrayMenu.cs:17-20` (AboutText) and `:198` (the question)
- Modify: `README.md:14` (badge) and the `## Licence` section (`:241-243`)
- Modify: `docs/plan.md:13` and `:191`
- Create: `CITATION.cff`

**Interfaces:**
- Produces: the embedded resource name `LICENSE.txt` in the Sobreiro assembly.

- [ ] **Step 1: LICENSE.** Write the canonical text: `gh api licenses/gpl-3.0 --jq .body > LICENSE`,
  then check that it starts with "GNU GENERAL PUBLIC LICENSE" and ends with the "How to Apply"
  section. Keep LF line endings (see `.gitattributes`).
- [ ] **Step 2: Embed it.** In `Sobreiro.csproj`, next to the OFL line:
  `<EmbeddedResource Include="..\..\LICENSE" LogicalName="LICENSE.txt" />`.
- [ ] **Step 3: Licence window.** Title `"Licences"`. Text: `Read("LICENSE.txt")`, a blank line,
  then the two OFL texts as now. Update the summary comment: "The licence texts carried inside
  Sobreiro.exe: Sobreiro's own (GPL-3.0) and the fonts'."
- [ ] **Step 4: About.** `AboutText` becomes (exact copy):

  ```
  Shows Claude Code's session and weekly limits and GitHub Copilot's monthly AI credits. {AboutLine}

  Free software under the GNU GPL v3 or later, with no warranty.

  Not affiliated with or endorsed by Anthropic or GitHub.

  Fonts: Inter and Unbounded, SIL Open Font License.
  ```

  The question at the end of the About box becomes `Read the licences?`.
- [ ] **Step 5: Build and look inside.**
  Run: `dotnet build src/Sobreiro/Sobreiro.csproj -c Release`, then list the resources:
  `powershell -NoProfile -Command "[Reflection.Assembly]::LoadFile((Resolve-Path 'src/Sobreiro/bin/Release/net10.0-windows/Sobreiro.dll')).GetManifestResourceNames()"`
  (adjust the folder to where the build puts it).
  Expected: the build succeeds, and the list includes `LICENSE.txt`, `OFL-Inter.txt`, `OFL-Unbounded.txt`.
- [ ] **Step 6: Tests still pass.** Run: `dotnet test`. Expected: all pass, same count as on `main`.
- [ ] **Step 7: README.** Badge: `<a href="LICENSE"><img src="https://img.shields.io/badge/licence-GPL--3.0-6f4df6" alt="GPL-3.0 licence"></a>`.
  Licence section, first paragraph (exact copy):

  ```
  Copyright 2026 Ricardo Silveira. Sobreiro is free software under the
  [GNU General Public License v3.0 or later](LICENSE): you may use it, change it and share it,
  and any copy you hand out, changed or not, stays under the same licence, with its source.
  Releases up to 0.8.1 were under MIT.
  ```

  Keep the rest of the section (fonts, not affiliated, the name).
- [ ] **Step 8: Plan.** `docs/plan.md:13`: "...then published as open source under GPL-3.0
  (MIT up to 0.8.1), which colleagues can install." `:191`: "- Licence: GPL-3.0-or-later,
  chosen 2026-11-10 so copies stay open; MIT up to 0.8.1."
- [ ] **Step 9: CITATION.cff.** CFF 1.2.0: `cff-version`, `message` ("If you use Sobreiro in
  your work, you can cite it like this."), `title: Sobreiro`, `type: software`, one author
  (`given-names: Ricardo`, `family-names: Silveira`), `abstract` (the site's meta description:
  "A Windows 11 tray app that keeps GitHub Copilot's monthly AI credits and Claude Code's
  session and weekly limits in view on your taskbar."), `repository-code`, `url` (the site),
  `license: GPL-3.0-or-later`, `keywords` (the topics from the spec). No `version` or
  `date-released`, because they would go stale with every release.
- [ ] **Step 10: Leftovers.** Run: `git grep -n "MIT"`. Expected: only `docs/brief.md`, the
  "up to 0.8.1" notes, and history in `docs/superpowers/`.
- [ ] **Step 11: Commit.** `git add -A` the files above;
  message "Licence: GPL-3.0-or-later (MIT up to 0.8.1)".

### Task 2: The public repo's files

**Files:**
- Create: `CONTRIBUTING.md`, `SECURITY.md`
- Create: `.github/ISSUE_TEMPLATE/bug.yml`, `feature.yml`, `config.yml`
- Modify: `README.md` (new `## Issues and security` section, just above `## Licence`)
- Modify: `docs/plan.md` (a going-public note at the end of milestone 4, just above `### 5. After release`)

**Interfaces:**
- Consumes: Task 1's GPL wording ("GPL-3.0", "the GNU GPL v3 or later").

- [ ] **Step 1: CONTRIBUTING.md.** Exact copy:

  ```markdown
  # Contributing

  Sobreiro is a one-person project. I make the changes to this repository myself, so pull
  requests are open to collaborators only. You can still help:

  - **Report a bug or suggest an idea** in [Issues](https://github.com/RicardoSilveira23/sobreiro/issues/new/choose).
    For a bug, say which version you run (About Sobreiro shows it) and on which Windows.
  - **Report a security problem privately**, as [SECURITY.md](SECURITY.md) says, never in a
    public issue.
  - **Fork it.** Sobreiro is free software under the [GPL-3.0](LICENSE), so you can change it
    however you like. Any copy you hand out, changed or not, keeps the same licence and comes
    with its source. [Building from source](README.md#building-from-source) has the steps.

  Not every idea gets built, and a reply can take a few days.
  ```

- [ ] **Step 2: SECURITY.md.** Sections, in this order:
  - `# Security`
  - `## Reporting a problem`: report it privately through GitHub's
    [form](https://github.com/RicardoSilveira23/sobreiro/security/advisories/new), never in a
    public issue. "I'm one person, so I reply when I can, usually within a week."
  - `## Supported versions`: "Only the latest release gets fixes."
  - `## What matters most`: Sobreiro reads two sign-ins it doesn't own. Five bullets, each
    checked against the README's "What it reads, what it sends" and `CLAUDE.md`:
    1. Claude Code's access token: read from `.credentials.json` into memory for each request,
       sent only to `api.anthropic.com`, never renewed, logged or stored.
    2. A GitHub token, found the way the Copilot CLI finds it: sent only to `api.github.com`,
       never renewed, logged or stored.
    3. Those two addresses, every 5 minutes, are its only network calls.
    4. Connect to Claude Code changes one key, `statusLine`, in Claude Code's `settings.json`,
       after asking and with a backup.
    5. The bridge writes only the `rate_limits` fields and a timestamp to `usage.json`.
  - Close: "Problems with the addresses themselves belong to Anthropic or GitHub." Then link the
    README section [What it reads, what it sends](README.md#what-it-reads-what-it-sends).
- [ ] **Step 3: Issue forms.**
  - `bug.yml`: `name: Bug`, `description: Something Sobreiro does wrong`, `labels: [bug]`.
    Body:
    - `input` `version`, "Sobreiro version", described as "About Sobreiro shows it", required.
    - `input` `windows`, "Windows version", placeholder "Windows 11 24H2", required.
    - `textarea` `what`, "What happened", required.
    - `textarea` `expected`, "What you expected".
    - `textarea` `log`, "Lines from log.txt", with `render: text`, described as "Right-click the
      pill, Open data folder. The log holds no tokens, but read it before you paste."
    Check the menu item's real name in `src/Sobreiro/TrayMenu.cs` before writing it.
  - `feature.yml`: `name: Idea`, `description: Something you'd like Sobreiro to do`,
    `labels: [enhancement]`. Body: a `markdown` note ("Sobreiro is a one-person project, so
    not every idea gets built."), a required `textarea` `idea` ("What would you like?") and a
    `textarea` `why` ("What would it help with?").
  - `config.yml`: `blank_issues_enabled: false`. Two `contact_links`: "The manual" (the manual
    address; about: "Installing, settings, what it reads and sends, known limits") and "A
    security problem" (the private form; about: "Report it privately, not in an issue").
- [ ] **Step 4: Check the YAML.** Every form has `name`, `description` and `body`. Every body
  item that isn't `markdown` has a unique `id` and an `attributes.label`. Indentation uses
  spaces only. Run `git diff --check` and expect no output.
- [ ] **Step 5: README section.** Just above `## Licence`, exact copy:

  ```markdown
  ## Issues and security

  Sobreiro is a one-person project: bugs and ideas go in
  [Issues](https://github.com/RicardoSilveira23/sobreiro/issues/new/choose), and pull requests
  are open to collaborators only. [CONTRIBUTING.md](CONTRIBUTING.md) says more. Report a
  security problem privately, as [SECURITY.md](SECURITY.md) says.
  ```

- [ ] **Step 6: Plan note.** At the end of milestone 4, just above `### 5. After release`:
  "Going public (2026-11-10): design in
  [superpowers/specs/2026-11-10-going-public-design.md](superpowers/specs/2026-11-10-going-public-design.md)."
  Then the order from the spec, as a numbered list:
  1. Merge PR #21, then this PR.
  2. Make the repo public.
  3. Repo settings: description, homepage, topics; pull requests collaborators only; wiki and
     discussions off; private vulnerability reporting; the `main` ruleset (CI `build`, no
     force-push, no deletion).
  4. Remove `noindex` from the site's pages.
  5. Apply to SignPath Foundation for signing.
- [ ] **Step 7: Commit.** Message: "Public repo files: contributing, security, issue forms".
