import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { checkSite, siteAddress } from "./check-site.mjs";

const HEAD =
  '<title>T</title><meta name="viewport" content="width=device-width">' +
  '<meta name="description" content="d">';

function page(head, body) {
  return `<!doctype html><html lang="en"><head>${HEAD}${head}</head><body>${body}</body></html>`;
}

// Builds a site in a temp folder from { "relative/path": "content" }, runs fn, cleans up.
function withSite(files, fn) {
  const dir = mkdtempSync(join(tmpdir(), "check-site-"));
  try {
    for (const [path, content] of Object.entries(files)) {
      const full = join(dir, path);
      mkdirSync(dirname(full), { recursive: true });
      writeFileSync(full, content);
    }
    return fn(dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const check = (files) => withSite(files, checkSite);
const IMG = '<img src="img/a.png" alt="a" width="10" height="10">';

test("clean site has no problems", () => {
  const problems = check({
    "index.html": page(
      '<link rel="stylesheet" href="assets/s.css">',
      `<h1 id="top">Home</h1>${IMG}<a href="manual/#limits">m</a><a href="manual/">m</a>`,
    ),
    "manual/index.html": page(
      '<link rel="stylesheet" href="../assets/s.css">',
      '<h2 id="limits">Limits</h2><a href="../index.html#top">home</a><a href="../#top">home</a>',
    ),
    "assets/s.css": "body { background: url(font/../bg.png); } a { background: url('bg.png?v=1'); }",
    "assets/bg.png": "x",
    "img/a.png": "x",
  });
  assert.deepEqual(problems, []);
});

test("missing local file", () => {
  const problems = check({
    "index.html": page(
      '<link rel="stylesheet" href="assets/missing.css">',
      '<img src="img/none.png" alt="" width="1" height="1">',
    ),
  });
  assert.deepEqual(problems, [
    "index.html: missing file assets/missing.css",
    "index.html: missing file img/none.png",
  ]);
});

test("directory link resolves to its index", () => {
  const files = { "index.html": page("", '<a href="manual/">m</a>') };
  assert.deepEqual(check({ ...files, "manual/index.html": page("", "") }), []);
  assert.deepEqual(check(files), ["index.html: missing file manual/"]);
});

test("fragment must exist", () => {
  const problems = check({
    "index.html": page("", '<a href="#nope">a</a><a href="#here">b</a><a href="manual/#limits">c</a><a href="manual/#ok">d</a><b id="here"></b>'),
    "manual/index.html": page("", '<b id="ok"></b>'),
  });
  assert.deepEqual(problems, [
    "index.html: missing anchor #nope",
    "index.html: missing anchor manual/#limits",
  ]);
});

test("no other host", () => {
  const problems = check({
    "index.html": page(
      '<link rel="stylesheet" href="https://fonts.googleapis.com/x"><script src="//cdn.example/x.js"></script>',
      '<img src="https://example.com/i.png" alt="" width="1" height="1">' +
        '<a href="https://github.com/RicardoSilveira23/sobreiro">gh</a><a href="mailto:a@b.c">m</a>',
    ),
    "assets/s.css": "a { background: url(https://example.com/x.png); }",
  });
  assert.deepEqual(problems, [
    "index.html: other host https://fonts.googleapis.com/x",
    "index.html: other host //cdn.example/x.js",
    "index.html: other host https://example.com/i.png",
    "assets/s.css: other host https://example.com/x.png",
  ]);
});

test("images need alt, width and height", () => {
  const problems = check({
    "index.html": page(
      "",
      '<img src="a.png" width="1" height="1"><img src="a.png" alt="" height="1"><img src="a.png" alt="" width="1"><img src="a.png" alt="" width="1" height="1">',
    ),
    "a.png": "x",
  });
  assert.deepEqual(problems, [
    "index.html: img a.png needs alt",
    "index.html: img a.png needs width",
    "index.html: img a.png needs height",
  ]);
});

test("page basics", () => {
  const problems = check({
    "index.html": "<!doctype html><html><head></head><body></body></html>",
  });
  assert.deepEqual(problems, [
    "index.html: missing <html lang>",
    "index.html: missing <title>",
    'index.html: missing <meta name="viewport">',
    'index.html: missing <meta name="description">',
  ]);
});

test("css url in a stylesheet resolves relative to the stylesheet", () => {
  const files = {
    "index.html": page('<link rel="stylesheet" href="assets/s.css">', ""),
    "assets/s.css": "a { background: url(img/bg.png); }",
  };
  assert.deepEqual(check({ ...files, "assets/img/bg.png": "x" }), []);
  assert.deepEqual(check({ ...files, "img/bg.png": "x" }), [
    "assets/s.css: missing file img/bg.png",
  ]);
});

test("root-absolute links are a problem: the pages work from any folder", () => {
  const problems = check({
    "index.html": page(
      '<link rel="stylesheet" href="/assets/s.css">',
      '<a href="/manual/">m</a><a href="//cdn.example/x">x</a>',
    ),
    "assets/s.css": "a { background: url(/assets/bg.png); }",
    "manual/index.html": page("", ""),
  });
  assert.deepEqual(problems, [
    "index.html: root-absolute link /assets/s.css",
    "index.html: root-absolute link /manual/",
    "assets/s.css: root-absolute link /assets/bg.png",
  ]);
});

test("a link must match the file's case exactly", () => {
  const problems = check({
    "index.html": page(
      '<link rel="stylesheet" href="Assets/Site.css">',
      '<a href="manual/">ok</a><a href="Manual/">bad</a>',
    ),
    "assets/Site.css": "a {}",
    "manual/index.html": page("", ""),
  });
  assert.deepEqual(problems, [
    "index.html: missing file Assets/Site.css",
    "index.html: missing file Manual/",
  ]);
});

test("other elements that make requests are checked too", () => {
  const problems = check({
    "index.html": page(
      "<style>@import 'https://a.example/x.css'; @import 'local.css'; b { background: url(https://b.example/i.png); }</style>",
      '<img src="a.png" srcset="https://c.example/a.png 2x, nope.png 1x" alt="" width="1" height="1">' +
        '<picture><source srcset="gone.webp"><source src="https://d.example/s"></picture>' +
        '<iframe src="https://e.example/"></iframe><video src="v.mp4" poster="missing-poster.png"></video>' +
        '<audio src="//f.example/a.mp3"></audio><object data="https://g.example/o"></object>' +
        '<embed src="https://h.example/e"><p style="background: url(\'https://i.example/p.png\')"></p>' +
        '<p style="background: url(none.png)"></p>',
    ),
    "a.png": "x",
    "v.mp4": "x",
    "local.css": "a {}",
    "s.css": "@import 'https://j.example/x.css'; @import \"gone.css\";",
  });
  assert.deepEqual(problems, [
    "index.html: other host https://a.example/x.css",
    "index.html: other host https://b.example/i.png",
    "index.html: other host https://c.example/a.png",
    "index.html: missing file nope.png",
    "index.html: missing file gone.webp",
    "index.html: other host https://d.example/s",
    "index.html: other host https://e.example/",
    "index.html: missing file missing-poster.png",
    "index.html: other host //f.example/a.mp3",
    "index.html: other host https://g.example/o",
    "index.html: other host https://h.example/e",
    "index.html: other host https://i.example/p.png",
    "index.html: missing file none.png",
    "s.css: other host https://j.example/x.css",
    "s.css: missing file gone.css",
  ]);
});

const ADDRESS = "https://example.test/";
const checkAt = (files) => withSite(files, (dir) => checkSite(dir, { address: ADDRESS }));

// The tags a shared link's card is made from, for the page at this path.
const share = (path = "", image = "og.png") =>
  '<meta property="og:title" content="T"><meta property="og:description" content="d">' +
  `<meta property="og:url" content="https://example.test/${path}">` +
  `<meta property="og:image" content="https://example.test/${image}">` +
  '<meta name="twitter:card" content="summary_large_image">';

test("each page names its own address as canonical", () => {
  const problems = checkAt({
    "index.html": page('<link rel="canonical" href="https://example.test/">' + share(), ""),
    "manual/index.html": page('<link rel="canonical" href="https://example.test/manual/">' + share("manual/"), ""),
    "og.png": "x",
  });
  assert.deepEqual(problems, []);
});

test("a missing or wrong canonical link is a problem", () => {
  const problems = checkAt({
    "index.html": page(share(), ""),
    "manual/index.html": page('<link rel="canonical" href="https://example.test/">' + share("manual/"), ""),
    "og.png": "x",
  });
  assert.deepEqual(problems, [
    'index.html: missing <link rel="canonical" href="https://example.test/">',
    "manual/index.html: canonical https://example.test/, wanted https://example.test/manual/",
  ]);
});

test("a page with no share card is a problem: a pasted link would show no picture or sentence", () => {
  const problems = checkAt({ "index.html": page('<link rel="canonical" href="https://example.test/">', "") });
  assert.deepEqual(problems, [
    'index.html: missing <meta property="og:title">',
    'index.html: missing <meta property="og:description">',
    'index.html: missing <meta property="og:url">',
    'index.html: missing <meta property="og:image">',
    'index.html: missing <meta name="twitter:card">',
  ]);
});

test("the share card names the page's own address and a picture this site serves", () => {
  const canonical = '<link rel="canonical" href="https://example.test/manual/">';
  const problems = checkAt({
    "index.html": page('<link rel="canonical" href="https://example.test/">' + share("", "missing.png"), ""),
    "manual/index.html": page(canonical + share(), ""),
    "other/index.html": page(
      '<link rel="canonical" href="https://example.test/other/">' +
        share("other/").replace("https://example.test/og.png", "https://elsewhere.test/og.png"),
      "",
    ),
    "og.png": "x",
  });
  assert.deepEqual(problems, [
    "index.html: og:image https://example.test/missing.png is not a file of this site",
    "manual/index.html: og:url https://example.test/, wanted https://example.test/manual/",
    "other/index.html: og:image https://elsewhere.test/og.png is not a file of this site",
  ]);
});

test("the address is the Worker's custom domain in wrangler.jsonc", () => {
  const config = (routes) => `{\n  // a comment\n  "name": "x",\n  "routes": ${routes}\n}`;
  withSite({ "wrangler.jsonc": config('[{ "pattern": "example.test", "custom_domain": true }]') }, (dir) => {
    assert.equal(siteAddress(join(dir, "wrangler.jsonc")), ADDRESS);
  });
  withSite({ "wrangler.jsonc": config("[]") }, (dir) => {
    assert.throws(() => siteAddress(join(dir, "wrangler.jsonc")), /custom domain/);
  });
});
