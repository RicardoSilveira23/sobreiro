# Going public: design

Date: 2026-11-10. Short design: what Ricardo and Claude agreed in conversation, written down.

## Why

The site is about to go live at `sobreiro.ricardosilveira.dev` (PR #21). Its Download, GitHub
and issue links only work once this repository is public. Before that, the repo needs what
a public project has. It must also stay Ricardo's: he alone changes
it, and anyone else can open issues and fork.

## Goals

- A public repo with what public projects have: `CONTRIBUTING.md`, `SECURITY.md`, `CITATION.cff`, issue
  templates, a description, a homepage, topics, private security reports and a rule on `main`.
- Only Ricardo changes the repo. Issues are open to everyone, and so are forks.
- Forks stay open: the licence becomes GPL-3.0.

## Non-goals

- Rewriting history. Ricardo's email is already on his CV, and the lines about colleagues and
  the company are not sensitive. History, the 21 PRs and the docs (`docs/superpowers/` included)
  go public as they are.
- A fresh repository. The repo stays at `RicardoSilveira23/sobreiro`.
- Code signing. It can be applied for once the repo is public, but that is a separate step.

## Anti-goals

- Inviting contributions: no PR template and no "how to send a PR" guide.
- Anything that would make outsiders think their pull requests are reviewed.

## Approaches

**Licence.** MIT (what the repo has), Apache-2.0 or GPL-3.0. Apache-2.0 adds a patent
grant and a contribution clause, which do nothing for a one-person tray app that takes no
contributions. MIT lets a fork become closed. **Ricardo picked GPL-3.0**, because a copy that
someone hands out must stay open. Nothing bundled stands in the way: no NuGet packages, the
OFL fonts and the MIT .NET runtime. The free code signing for open source accepts it. Copies
already handed out (0.6.0 to 0.8.1) stay MIT.

**Keeping it his.** Turning pull requests off would remove his own way of merging Claude's
branches. Leaving them open invites PRs he won't take. **Picked:** GitHub's "Collaborators only"
setting for pull requests (February 2026). He has no collaborators, so only he and bots can
open them.

**History.** Rewrite authors to his GitHub noreply address, publish a fresh repo, or keep it as
is. A rewrite in place would leave the old commits in the 21 PRs, and a fresh repo costs the PR
history and the setup. **Picked: keep as is.**

## What gets built (one PR)

- `LICENSE`: the GPL-3.0 text. The README badge and its Licence section say GPL-3.0, and so does
  `docs/plan.md`.
- The app: About Sobreiro gains one line, "Free software under the GNU GPL v3, with no
  warranty", and the licence window shows the GPL-3.0 text before the font licences. The exe
  then carries its own licence, as the GPL asks.
- `CONTRIBUTING.md`: short, in Ricardo's voice. Sobreiro is a one-person project and he makes
  the changes. Bugs and ideas go in issues, and anyone may fork it under GPL-3.0. It points to
  the README's Building from source.
- `SECURITY.md`: report privately through GitHub's form. Fixes go into the latest release only.
  The sensitive parts are the token handling, the two network addresses and the `settings.json`
  edit. Replies are best effort from one person.
- `.github/ISSUE_TEMPLATE/`: `bug.yml` (version, Windows version, what happened, what was
  expected, optional log lines, with a reminder that logs hold no tokens), `feature.yml`, and
  `config.yml` (no blank issues; links to the manual and to the private security form).
- `CITATION.cff`.
- README: a short "Issues and security" section linking to `CONTRIBUTING.md` and `SECURITY.md`.
- `docs/plan.md`: a going-public checklist under milestone 4, which already says "Make the repo
  public".

## Repo settings (after merging, with Ricardo's OK for each)

- Description: "Copilot and Claude Code usage on your Windows taskbar". Homepage:
  `https://sobreiro.ricardosilveira.dev`. Topics: windows, windows-11, tray-app, taskbar,
  claude-code, github-copilot, usage, rate-limits, dotnet, csharp, wpf.
- Pull requests: Collaborators only. Issues on, Wiki and Discussions off.
- Once public: private vulnerability reporting on, and a ruleset on `main`: the CI
  `build` check before merge, no force-push, no deletion. Secret scanning and push protection
  turn on by themselves for public repos.

## Order

1. Merge PR #21 (the domain), then this PR.
2. Ricardo makes the repo public.
3. Apply the settings above.
4. A one-line PR per page removes `noindex` from the site.

## Left to the builder (Claude's calls)

- `GPL-3.0-or-later`, the FSF's own wording, in `CITATION.cff` and the README. No per-file headers:
  `LICENSE` and the README carry the notice.
- The wording of every new file, in the voice of the README.
- Whether the ruleset and the PR setting go through `gh api` or the dashboard. The
  collaborators-only field isn't in GitHub's documented REST schema yet, so that one may stay a
  dashboard step.
