// Pins what docs/design.md fixes for the manual: its parts, its words' count, the panel and its switches.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (p) => readFileSync(join(root, p), "utf8");

const html = read("site/manual/index.html");
const css = read("site/assets/manual.css");
const site = read("site/assets/site.css");

const text = (s) => s.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ").trim();
const count = (s, re) => (s.match(re) || []).length;
const inner = (src, open, tag) => {
  const i = src.indexOf(open);
  assert.notEqual(i, -1, `missing ${open}`);
  const from = src.indexOf(">", i) + 1;
  return src.slice(from, src.indexOf(`</${tag}>`, from));
};
const section = (id) => inner(html, `<section class="section" id="${id}"`, "section");
const rule = (sheet, selector) => {
  const esc = selector.replaceAll(".", String.raw`\.`);
  const re = new RegExp(String.raw`(?:^|[}\s,])${esc}\s*(?:,[^{]*)?\{([^}]*)\}`, "gm");
  return [...sheet.matchAll(re)].map((m) => m[1]).join(" ");
};

test("the manual wears the site's look", () => {
  const a = html.indexOf('<link rel="stylesheet" href="../assets/site.css">');
  const b = html.indexOf('<link rel="stylesheet" href="../assets/manual.css">');
  assert.ok(a !== -1 && b > a, "site.css then manual.css");
  assert.match(html, /<script src="\.\.\/assets\/panel\.js" defer><\/script>/);
  assert.equal(count(html, /<script(?![^>]*\ssrc=)/g), 0, "no inline script");
  const h = inner(html, '<header class="top"', "header");
  assert.equal(count(h, /<a[\s>]/g), 1);
  assert.match(h, /<a [^>]*href="\.\.\/"/);
});

test("the seven sections are there, in order", () => {
  const ids = ["install", "connect", "shows", "live", "data", "uninstall", "limits"];
  const found = [...html.matchAll(/<section class="section" id="([^"]+)"/g)].map((m) => m[1]);
  assert.deepEqual(found, ids);
  const nav = inner(html, '<nav aria-label="Contents"', "nav");
  assert.deepEqual([...nav.matchAll(/href="#([^"]+)"/g)].map((m) => m[1]), ids);
});

test("the words are the app's facts, unchanged", () => {
  assert.equal(count(html, /class="row"/g), 43);
  assert.equal(count(section("limits"), /class="row"/g), 6);
  assert.match(inner(section("limits"), "<h2", "h2"), /<small>6<\/small>/);
  assert.equal(count(text(html), /A ring is purple below 70%, amber from 70% and red from 90%\./g), 1);
});

test("the panel shows one picture and never shifts", () => {
  const panel = inner(html, '<div class="panel"', "div");
  const imgs = [...panel.matchAll(/<img [^>]*>/g)].map((m) => m[0]);
  assert.equal(imgs.length, 9);
  for (const i of imgs) assert.match(i, /data-services="[a-z]+"[^>]*data-style="[a-z]+"/, i);
  const shown = imgs.filter((i) => !/\shidden\b/.test(i));
  assert.equal(shown.length, 1);
  assert.match(shown[0], /data-services="both"/);
  assert.match(shown[0], /data-style="bars"/);
  assert.match(rule(css, ".panel"), /height: 504px/);
});

test("the switches are real buttons, hidden until script runs", () => {
  const groups = [...html.matchAll(/<p class="styles"[^>]*>/g)].map((m) => m[0]);
  assert.equal(groups.length, 2);
  for (const g of groups) assert.match(g, /\shidden\b/);
  assert.equal(count(html, /<button type="button"/g), 6);
  for (const k of ['data-services="both"', 'data-style="bars"']) {
    assert.match(html, new RegExp(`<button type="button" ${k} aria-pressed="true">`));
  }
  assert.equal(count(html, /aria-pressed="true"/g), 2);
  assert.match(rule(css, ".styles button"), /min-height: 44px/);
});

test("the captures sit in What it shows", () => {
  const s = section("shows");
  const want = {
    "option-light.png": "Light, with Windows",
    "option-accent.png": "Your Windows accent",
    "option-tray-and-pill.png": "The pill and the tray icon",
    "option-tray-only.png": "Just the tray icon",
  };
  for (const [file, caption] of Object.entries(want)) {
    const name = file.replace(".", String.raw`\.`);
    const re = new RegExp(String.raw`<figure>\s*<img [^>]*${name}[^>]*>\s*<figcaption>${caption}</figcaption>\s*</figure>`);
    assert.match(s, re, file);
  }
});

test("the way home is a button", () => {
  assert.match(html, /<a class="back mn" href="\.\.\/">/);
  assert.match(site, /\.dl,\s*\.mn\s*\{[^}]*min-height: 54px/);
});
