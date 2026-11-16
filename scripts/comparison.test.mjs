// The comparison of the releases: generated from docs/baseline.md, and the same there and on the page.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { comparison, docsBlock, METRICS, multiple, outputs, pageSection, readmeBlock, releaseRows, svg } from "./comparison.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const md = readFileSync(join(root, "docs/baseline.md"), "utf8").replace(/\r/g, "");
const page = readFileSync(join(root, "site/index.html"), "utf8").replace(/\r/g, "");

// A stand-in 2.0.0 row: the spike's figures, used only to build the page as it will be.
const ROW_20 = "| 2.0.0 | 2026-12-01 | 7 MB | 29 MB | 1.7 s | 0.06 s | 0.06 s | 9.3 MB | 2.2 MB | 9 ms |";
const without20 = md.replace(/^\| 2\.0\.0 \|.*\n/m, "");
const with20 = without20.replace(/^(\| 1\.1\.0 \| \d{4}-.*\n)/m, `$1${ROW_20}\n`);

// The cards read back: [{ title, big, base, fig }].
const cards = (html) =>
  [...html.matchAll(/<figcaption>(.*?)<\/figcaption><p class="big"><b>(.*?)<\/b> (.*?)<\/p><div class="pair"><p><span>1\.1<\/span><i [^>]*><\/i><em>(.*?)<\/em><\/p><p class="new"><span>2\.0<\/span><i [^>]*><\/i><em>(.*?)<\/em><\/p><\/div>/g)].map(
    ([, title, x, word, base, fig]) => ({ title, big: `${x} ${word}`, base, fig }),
  );

test("the page and the docs are what docs/baseline.md gives", () => {
  for (const [path, want, have] of outputs()) {
    assert.equal(have, want, `${path} is stale: run node scripts/comparison.mjs`);
  }
});

test("the section sits in main, after the notes and before the footer", () => {
  const at = ['class="before"', '<section class="cmp', "</main>", 'class="foot"'].map((s) => page.indexOf(s));
  assert.ok(at.every((n) => n > 0));
  assert.deepEqual(at, [...at].sort((a, b) => a - b));
});

test("the words on 1.0 to 1.1 carry the table's figures", () => {
  const rows = releaseRows(md);
  const cpu = (b) => rows.get(b)["CPU per idle hour"];
  assert.ok(page.includes(`1.1 cut idle CPU from ${cpu("1.0.0")} to ${cpu("1.1.0")} an hour`));
});

test("without a 2.0 row it builds: the words, no rings, no picture, no 2.0 button", () => {
  const c = comparison(without20);
  const html = pageSection(c);
  assert.equal(cards(html).length, 0);
  assert.doesNotMatch(html, /class="charts"|Download 2\.0|v2\.0\.0/);
  assert.match(html, /<h2 id="cmp" class="r20"><svg class="rmark"[^]*?<\/svg>Why 2\.0 moves to Rust<\/h2>/);
  assert.match(html, /<section class="cmp solo"/);
  assert.match(html, /1\.1 cut idle CPU from 14\.9 s to 2\.0 s an hour, 7\.5× less, but most of its memory still went to WPF/);
  assert.doesNotMatch(html, /Bars start at zero/);
  assert.equal(svg(c), null);
  assert.match(docsBlock(c), /2\.0\.0, rewritten in Rust, is set against 1\.1 here once its release row is in\./);
});

test("with a 2.0 row: a card per figure, every figure the table's own, 2.0 against 1.1", () => {
  const c = comparison(with20);
  const rows = releaseRows(with20);
  const shown = cards(pageSection(c));
  assert.deepEqual(
    shown.map((s) => s.title),
    METRICS.map((m) => m.title),
  );
  for (const [i, m] of METRICS.entries()) {
    assert.equal(shown[i].base, rows.get("1.1.0")[m.column], m.title);
    assert.equal(shown[i].fig, rows.get("2.0.0")[m.column], m.title);
  }
  assert.deepEqual(
    shown.map((s) => s.big),
    ["14× less", "1.2× less", "7.8× faster", "18× smaller"],
  );
  assert.match(docsBlock(c), /^\| Private memory \| 96 MB \| 7 MB, 14× less \|$/m);
  assert.match(svg(c), />7 MB<tspan class="x" dx="8">14× less</);
});

test("the bars: zero-based shares of the larger, and 2.0's shrinks from 1.1's length", () => {
  const html = pageSection(comparison(with20));
  assert.match(html, /<span>1\.1<\/span><i style="--w:1"><\/i><em>96 MB<\/em><\/p><p class="new"><span>2\.0<\/span><i style="--w:0\.073;--from:1"><\/i><em>7 MB<\/em>/);
});

test("downloads: 2.0 then 1.1, each its tag; 1.0 predates the repository and has none", () => {
  const html = pageSection(comparison(with20));
  const buttons = [...html.matchAll(/<a class="mn" href="([^"]+)">([^<]+)<\/a>/g)].map((m) => [m[2], m[1]]);
  assert.deepEqual(buttons, [
    ["Download 2.0", "https://github.com/RicardoSilveira23/sobreiro/releases/tag/v2.0.0"],
    ["Download 1.1", "https://github.com/RicardoSilveira23/sobreiro/releases/tag/v1.1.0"],
  ]);
});

test("multiples read as the metric reads", () => {
  const [mem, cpu, start] = METRICS;
  assert.equal(multiple(2.0, 14.9, cpu), "7.5× less");
  assert.equal(multiple(7, 96, mem), "14× less");
  assert.equal(multiple(0.47, 0.4, start), "1.2× slower");
  assert.equal(multiple(96, 97, mem), "about the same");
});

test("a cell that is not a figure stops the build", () => {
  const bad = without20.replace(/^(\| 1\.1\.0 \| [^|]+\| )96 MB/m, "$1not measured");
  assert.doesNotThrow(() => comparison(bad), "memory is not read before 2.0 exists");
  const bad2 = with20.replace(/^(\| 2\.0\.0 \| [^|]+\| )7 MB/m, "$1not measured");
  assert.throws(() => comparison(bad2), /2\.0\.0, Private memory: "not measured"/);
});


test("the README says the same, and shows the picture and table once 2.0 has a row", () => {
  assert.doesNotMatch(readmeBlock(comparison(without20)), /comparison\.svg|\| 2\.0\.0/);
  const r = readmeBlock(comparison(with20));
  assert.match(r, /<img src="docs\/images\/comparison\.svg" width="760" alt="Private memory: 1\.1 96 MB, 2\.0 7 MB\./);
  assert.match(r, /^\| Sobreiro\.exe \| 168\.9 MB \| 9\.3 MB, 18× smaller \|$/m);
  assert.match(r, /^## Why 2\.0 moved to Rust$/m);
});

test("the heading carries the mark, inline, in classes only", () => {
  assert.match(pageSection(comparison(without20)), /class="rmark"/);
  const html = pageSection(comparison(with20));
  const svg = html.match(/<h2 id="cmp" class="r20"><svg class="rmark" viewBox="0 0 64 64" aria-hidden="true">(.*?)<\/svg>Why 2\.0 moved to Rust<\/h2>/)?.[1];
  assert.ok(svg, "the mark leads the Rust heading");
  assert.match(svg, /^<defs><clipPath id="rtile"><rect width="64" height="64" rx="14"\/><\/clipPath><\/defs><rect class="tile" width="64" height="64" rx="14"\/><rect class="fill" clip-path="url\(#rtile\)" width="64" height="64"\/><g class="trunk">/);
  assert.doesNotMatch(svg, /#[0-9a-f]{3,8}\b/i, "no colour in the markup");
});
