// The comparison of the releases: generated from docs/baseline.md, and the same there and on the page.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { comparison, docsBlock, METRICS, multiple, outputs, pageSection, releaseRows, svg } from "./comparison.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const md = readFileSync(join(root, "docs/baseline.md"), "utf8").replace(/\r/g, "");
const page = readFileSync(join(root, "site/index.html"), "utf8").replace(/\r/g, "");

const ROW_20 = "| 2.0.0 | 2026-12-01 | 7 MB | 29 MB | 1.7 s | 0.06 s | 0.06 s | 9.3 MB | 2.2 MB | 9 ms |";
const without20 = md.replace(/^\| 2\.0\.0 \|.*\n/m, "");
const with20 = without20.replace(/^(\| 1\.1\.0 \| \d{4}-.*\n)/m, `$1${ROW_20}\n`);

// The page's charts read back: [{ title, rows: [[release, figure]] }].
const charts = (html) =>
  [...html.matchAll(/<figure class="m"><figcaption>(.*?)<\/figcaption><ol>(.*?)<\/ol><\/figure>/g)].map(([, title, ol]) => ({
    title,
    rows: [...ol.matchAll(/<b>(.*?)<\/b><span><i [^>]*><\/i>([^<]*)/g)].map((m) => [m[1], m[2]]),
  }));

test("the page, the docs and the picture are what docs/baseline.md gives", () => {
  for (const [path, want, have] of outputs()) {
    assert.equal(have, want, `${path} is stale: run node scripts/comparison.mjs`);
  }
});

test("every figure on the page is the Releases table's own", () => {
  const rows = releaseRows(md);
  const shown = charts(page);
  assert.equal(shown.length, METRICS.length);
  for (const [i, m] of METRICS.entries()) {
    assert.equal(shown[i].title, m.title);
    assert.ok(shown[i].rows.length >= 2, `${m.title}: at least 1.0 and 1.1`);
    for (const [name, fig] of shown[i].rows) {
      assert.equal(fig, rows.get(`${name}.0`)[m.column], `${m.title}, ${name}`);
    }
  }
});

test("the section sits in main, after the notes and before the footer", () => {
  const at = ['class="before"', 'class="cmp"', "</main>", 'class="foot"'].map((s) => page.indexOf(s));
  assert.ok(at.every((n) => n > 0));
  assert.deepEqual(at, [...at].sort((a, b) => a - b));
});

test("without a 2.0 row it builds: 1.0 and 1.1, 2.0 said to be coming, no 2.0 button", () => {
  const c = comparison(without20);
  assert.deepEqual(c.releases.map((r) => r.name), ["1.0", "1.1"]);
  const html = pageSection(c);
  assert.deepEqual(charts(html)[0].rows.map(([n]) => n), ["1.0", "1.1"]);
  assert.match(html, /2\.0, rewritten in Rust, joins them when it is released\./);
  assert.doesNotMatch(html, /Download 2\.0|v2\.0\.0/);
  assert.match(docsBlock(c), /2\.0\.0, rewritten in Rust, joins them once its release row is in\./);
  assert.doesNotMatch(docsBlock(c), /\| 2\.0\.0 \|/);
});

test("with a 2.0 row: 2.0 shown last and highlighted, and its download first", () => {
  const c = comparison(with20);
  const html = pageSection(c);
  assert.deepEqual(charts(html)[0].rows, [["1.0", "97 MB"], ["1.1", "96 MB"], ["2.0", "7 MB"]]);
  assert.equal((html.match(/<li data-r="2\.0" class="new"><b>2\.0<\/b>/g) || []).length, METRICS.length);
  assert.match(html, /1\.1 tuned for idle; 2\.0 is rewritten in Rust\./);
  assert.doesNotMatch(html, /joins them/);
  const buttons = [...html.matchAll(/<a class="mn" href="([^"]+)">([^<]+)<\/a>/g)].map((m) => [m[2], m[1]]);
  assert.deepEqual(buttons, [
    ["Download 2.0", "https://github.com/RicardoSilveira23/sobreiro/releases/tag/v2.0.0"],
    ["Download 1.1", "https://github.com/RicardoSilveira23/sobreiro/releases/tag/v1.1.0"],
  ]);
  assert.match(docsBlock(c), /^\| Private memory \| 97 MB \| 96 MB \| 7 MB, 14× less \|$/m);
  assert.match(svg(c), />7 MB</);
});

test("1.0 has no download: it was released before this repository was public", () => {
  assert.doesNotMatch(pageSection(comparison(with20)), /Download 1\.0|v1\.0\.0/);
});

test("bars are shares of each figure's largest", () => {
  for (const m of comparison(with20).metrics) {
    assert.equal(Math.max(...m.figures.map((f) => f.share)), 1, m.title);
  }
  assert.match(pageSection(comparison(with20)), /<b>2\.0<\/b><span><i style="--w:0\.072"><\/i>7 MB/);
});

test("a cell that is not a figure stops the build", () => {
  const bad = without20.replace(/^(\| 1\.1\.0 \| [^|]+\| )96 MB/m, "$1not measured");
  assert.throws(() => comparison(bad), /1\.1\.0, Private memory: "not measured"/);
});

test("multiples read as the metric reads", () => {
  const [mem, cpu, start] = METRICS;
  assert.equal(multiple(2.0, 14.9, cpu), "7.5× less");
  assert.equal(multiple(7, 96, mem), "14× less");
  assert.equal(multiple(0.47, 0.4, start), "1.2× slower");
  assert.equal(multiple(96, 97, mem), "about the same");
});

test("with a 2.0 row the multiples are against 1.1, and the switch offers 1.0", () => {
  const html = pageSection(comparison(with20));
  assert.match(html, /<span data-k="against"> Multiples are against <b>1\.1<\/b>\.<\/span>/);
  assert.match(html, /<li data-r="2\.0" class="new"><b>2\.0<\/b><span><i [^>]*><\/i>7 MB<em data-vs="1\.0" hidden>14× less<\/em><em data-vs="1\.1">14× less<\/em>/);
  assert.match(html, /<div class="vs" hidden><span>Compared with<\/span><button type="button" data-base="1\.0" aria-pressed="false">1\.0<\/button><button type="button" data-base="1\.1" aria-pressed="true">1\.1<\/button><\/div>/);
  // Each figure shows one multiple without script: the one against the base.
  for (const li of html.match(/<li [^]*?<\/li>/g)) {
    assert.ok((li.match(/<em data-vs="[^"]+">/g) || []).length <= 1, li);
  }
});

test("without a 2.0 row 1.1 is set against 1.0 and there is no switch", () => {
  const html = pageSection(comparison(without20));
  assert.doesNotMatch(html, /class="vs"/);
  assert.match(html, /<li data-r="1\.1" class="new"><b>1\.1<\/b><span><i [^>]*><\/i>2\.0 s<em data-vs="1\.0">7\.5× less<\/em>/);
});
