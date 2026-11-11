// Pins what docs/design.md fixes for the landing page: its words, order, colours, sizes and motion.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (p) => readFileSync(join(root, p), "utf8");
const assets = join(root, "site", "assets");

const home = read("site/index.html");
const manual = read("site/manual/index.html");
const css = read("site/assets/site.css");
const js = read("site/assets/pill.js");
const readme = read("README.md");
const sources = readdirSync(assets)
  .filter((f) => /\.(css|js)$/.test(f))
  .map((f) => [`site/assets/${f}`, read(`site/assets/${f}`)]);

const text = (html) =>
  html
    .replace(/<script[\s\S]*?<\/script>/g, " ")
    .replace(/<svg[\s\S]*?<\/svg>/g, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/&amp;/g, "&")
    .replace(/\s+/g, " ")
    .trim();
// The inner HTML of the first element matching an opening tag, up to its closing tag.
const inner = (html, open, tag) => {
  const i = html.indexOf(open);
  assert.notEqual(i, -1, `missing ${open}`);
  const from = html.indexOf(">", i) + 1;
  return html.slice(from, html.indexOf(`</${tag}>`, from));
};
const count = (s, re) => (s.match(re) || []).length;
const tagOf = (html, key) => html.match(new RegExp(`<[^>]*data-k="${key}"[^>]*>`))?.[0] ?? "";

test("the header holds only the name", () => {
  const h = inner(home, '<header class="top"', "header");
  assert.equal(count(h, /<a[\s>]/g), 1);
  assert.match(h, /<a [^>]*href="\.\/"/);
  assert.match(h, /<img [^>]*sobreiro-mark\.svg/);
  assert.equal(text(h), "Sobreiro");
  assert.doesNotMatch(h, /<nav|<button/);
});

test("the headline is the page's one h1", () => {
  assert.equal(count(home, /<h1/g), 1);
  assert.ok(
    home.includes(
      '<h1 class="line">Your GitHub Copilot and Claude Code usage, always in view on your taskbar.</h1>',
    ),
  );
});

test("Download carries GitHub's mark, Manual sits beside it", () => {
  const acts = inner(home, '<div class="acts"', "div");
  const links = [...acts.matchAll(/<a [^>]*>[\s\S]*?<\/a>/g)].map((m) => m[0]);
  assert.equal(links.length, 2);
  assert.match(links[0], /class="dl"/);
  assert.match(links[0], /href="https:\/\/github\.com\/RicardoSilveira23\/sobreiro\/releases\/latest"/);
  assert.match(links[0], /<use href="#gh"\/>/);
  assert.equal(text(links[0]), "Download");
  assert.match(links[1], /class="mn"/);
  assert.match(links[1], /href="manual\/"/);
  assert.equal(text(links[1]), "Manual");
  assert.equal(count(home, /class="dl"/g), 1);
});

test("the meta line and the steps read as the design has them", () => {
  assert.equal(
    text(inner(home, '<p class="meta"', "p")),
    "Free, for Windows 11. Needs GitHub Copilot with AI credits, or Claude Code on Pro, Max or Ultra.",
  );
  const run = inner(home, '<div class="run"', "div");
  assert.ok(run.includes("<h2>Run it</h2>"));
  const lis = [...run.matchAll(/<li>([\s\S]*?)<\/li>/g)].map((m) => text(m[1]));
  assert.deepEqual(lis, ["1 Download Sobreiro.exe, one file", "2 Open it", "The pill appears beside the clock"]);
});

test("the parts come in the phone's order", () => {
  const at = ['class="say"', 'class="flow"', 'class="sends"', 'class="before"', 'class="foot"'].map((s) => {
    assert.equal(count(home, new RegExp(s, "g")), 1, s);
    return home.indexOf(s);
  });
  assert.deepEqual(at, [...at].sort((a, b) => a - b));
  const main = inner(home, '<main class="home"', "main");
  for (const s of ['class="say"', 'class="flow"', 'class="sends"', 'class="before"']) assert.ok(main.includes(s), `${s} inside main`);
  assert.ok(!main.includes('class="foot"') && !main.includes('class="top"'));
});

test("the pill's first reading is in the HTML", () => {
  const body = (key) => text(inner(home, tagOf(home, key), "b"));
  assert.equal(body("v5"), "15%");
  assert.equal(body("v7"), "45%");
  assert.equal(body("va"), "17%");
  assert.match(tagOf(home, "r5"), /stroke-dashoffset="85"/);
  assert.match(tagOf(home, "r7"), /stroke-dashoffset="55"/);
  assert.match(tagOf(home, "ba"), /style="height:17%"/);
  assert.match(tagOf(home, "mk"), /style="left:15%"/);
});

test("the diagram names both addresses", () => {
  const t = text(home);
  for (const s of [
    "api.anthropic.com",
    "/api/oauth/usage",
    "api.github.com",
    "/copilot_internal/user",
    "asks every 5 minutes",
  ]) {
    assert.ok(t.includes(s), s);
  }
});

test("the two notes say what the design says", () => {
  assert.equal(
    text(inner(home, '<div class="sends"', "div")),
    "What it sends Two requests, with a sign-in already on this PC The numbers come back and go to the taskbar Nothing else leaves: no telemetry, no update checks, no token stored, logged or renewed",
  );
  assert.equal(
    text(inner(home, '<div class="before"', "div")),
    "Before you install Both addresses are undocumented and can change, and Anthropic's terms don't allow third-party apps to use Claude sign-ins. Windows may warn about an unknown publisher until the app is signed.",
  );
  assert.equal(count(text(home), /unknown publisher/g), 1);
});

test("no other company's mark", () => {
  for (const m of home.matchAll(/<img [^>]*>/g)) {
    assert.match(m[0], /src="[^"]*sobreiro-mark\.svg"/, m[0]);
  }
  assert.equal(count(home, /<symbol/g), 1);
  assert.match(home, /<symbol id="gh"/);
  assert.equal(count(home, /<use /g), 1);
});

test("colours are the closed list", () => {
  const allowed = new Set(
    ["#ebe6ff", "#1d1447", "#4d457c", "#6f4df6", "#5a3dd1", "#ffffff", "#b9adf5", "#f6f3ff", "#1c1c1c", "#9a9a9a", "#a594ff", "#ffb020", "#ff6b6b"],
  );
  const hexes = (s) => (s.replace(/href="#[^"]*"/g, "").match(/#[0-9a-fA-F]{3,8}\b/g) || []).map((h) => h.toLowerCase());
  const roots = [...css.matchAll(/:root\s*\{[^}]*\}/g)].map((m) => m[0]).join("\n");
  const outside = css.replace(/:root\s*\{[^}]*\}/g, "");
  assert.deepEqual(hexes(outside), [], "hex outside :root in site.css");
  for (const h of hexes(roots)) assert.ok(allowed.has(h), h);
  assert.equal(new Set(hexes(roots)).size, allowed.size, "all thirteen are defined");
  for (const [name, src] of [["index.html", home], ["manual/index.html", manual], ...sources]) {
    if (name === "site/assets/site.css") continue;
    assert.deepEqual(hexes(src), [], `hex in ${name}`);
  }
  for (const [name, src] of [["index.html", home], ["manual/index.html", manual], ...sources]) {
    for (const m of src.matchAll(/rgba?\(\s*([^)]*)\)/g)) {
      assert.match(m[1], /^(255,\s*255,\s*255|111,\s*77,\s*246)\b/, `${name}: ${m[0]}`);
    }
  }
});

test("no other colour notation", () => {
  for (const [name, src] of [["index.html", home], ["manual/index.html", manual], ...sources]) {
    assert.doesNotMatch(src, /\b(hsl|hwb|lab|lch|oklab|oklch|color-mix|color)\(/i, name);
  }
});

test("the Download button's focus ring is ink", () => {
  assert.match(css, /\.dl:focus-visible\s*\{[^}]*outline-color: var\(--ink\)/);
});

test("the sizes the design fixes", () => {
  assert.match(css, /\.line\s*\{[^}]*clamp\(26px, 2\.9vw, 36px\)/);
  assert.match(css, /\.dl,\s*\.mn\s*\{[^}]*min-height: 54px/);
  assert.match(css, /\.brand\s*\{[^}]*min-height: 44px/);
  assert.match(css, /\.foot a\s*\{[^}]*min-height: 44px/);
  assert.match(css, /\.key\s*\{[^}]*max-width: 62ch/);
});

test("the page never stretches and has one theme", () => {
  for (const [name, src] of [["index.html", home], ["manual/index.html", manual], ...sources]) {
    assert.doesNotMatch(src, /\d(vh|svh|dvh|lvh)\b/, `${name}: viewport-height unit`);
    assert.doesNotMatch(src, /prefers-color-scheme|data-theme/, `${name}: theme`);
  }
});

// The stylesheet cut in two: the no-preference media block, and everything else.
const motionSplit = () => {
  const start = css.indexOf("@media (prefers-reduced-motion: no-preference)");
  assert.notEqual(start, -1);
  let depth = 0;
  let end = css.indexOf("{", start);
  for (let i = end; i < css.length; i++) {
    if (css[i] === "{") depth++;
    if (css[i] === "}" && --depth === 0) {
      end = i + 1;
      break;
    }
  }
  return { rest: css.slice(0, start) + css.slice(end), block: css.slice(start, end) };
};

test("a dot is invisible until it is animated", () => {
  assert.match(motionSplit().rest, /\.dot\s*\{[^}]*opacity: 0/);
});

test("motion stops for reduced motion", () => {
  const { rest, block } = motionSplit();
  assert.doesNotMatch(rest, /animation/);
  assert.match(block, /animation/);
  assert.match(js, /\(prefers-reduced-motion: reduce\)/);
  assert.match(js, /matches\)\s*return/);
});

test("the readings are the design's six", () => {
  assert.ok(js.includes("[[15, 45, 17], [38, 46, 17], [61, 48, 18], [74, 49, 18], [86, 51, 19], [93, 52, 19]]"));
  assert.match(css, /animation:[^;]*\b5s\b/);
});

test("fonts come from the site", () => {
  const faces = [...css.matchAll(/@font-face\s*\{[^}]*\}/g)].map((m) => m[0]);
  assert.equal(faces.length, 4);
  const want = { Inter: "inter", Unbounded: "unbounded", Sora: "sora", "JetBrains Mono": "jetbrains-mono" };
  for (const [family, file] of Object.entries(want)) {
    const f = faces.find((x) => x.includes(`font-family: '${family}'`) || x.includes(`font-family: ${family}`));
    assert.ok(f, family);
    assert.match(f, new RegExp(`src: url\\('fonts/${file}-latin-wght-normal\\.woff2'\\)`));
  }
});

test("the README opens with the page's sentence and meta", () => {
  const top = text(readme.split(/^## /m)[0]);
  assert.ok(top.includes("Your GitHub Copilot and Claude Code usage, always in view on your taskbar."));
  assert.ok(top.includes("Free, for Windows 11. Needs GitHub Copilot with AI credits, or Claude Code on Pro, Max or Ultra."));
});
