// The home page's first screen: the name at its own spacing with the meta beside it, the rule,
// then the field with the page's sentence, the download and its note, and the capture.
// Run: node --test scripts/first-screen.test.mjs
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8");
const html = read("../site/index.html");
const css = read("../site/assets/site.css");
const text = (fragment) => fragment.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ").trim();
const count = (haystack, needle) => haystack.split(needle).length - 1;

// From the opening tag with this class to the next opening tag with that class.
function between(from, to) {
  const start = html.indexOf(`class="${from}"`);
  const end = html.indexOf(`class="${to}"`, start);
  assert.ok(start > 0 && end > start, `${from} comes before ${to}`);
  return html.slice(start, end);
}

test("the name is text at its own spacing, with the two meta lines beside it", () => {
  const line = between("name-line", "rule");
  assert.match(line, /<h1 class="name">Sobreiro<\/h1>/);
  assert.doesNotMatch(html, /textLength/, "no lettering stretched across the column");
  assert.match(line, /<div class="meta">Free, for Windows 11<br>Needs GitHub Copilot with AI credits, or Claude Code on Pro, Max or Ultra<\/div>/);
});

test("the field sits directly under the rule", () => {
  assert.match(html, /<div class="rule" aria-hidden="true"><\/div>\s*<div class="field">/);
  assert.doesNotMatch(css, /\.field\s*\{[^}]*margin-top/, "the rule sets the gap");
});

test("the field holds the page's sentence, then the filled download with its note under it", () => {
  const field = between("field", "section");
  const sentence = /<meta property="og:image:alt" content="[^"]*then: ([^"]+)">/.exec(html)[1];
  assert.match(field, new RegExp(`<p class="line">${sentence}</p>`));
  const button = field.indexOf('class="btn fill"');
  const note = field.indexOf('class="small"');
  const capture = field.indexOf("<picture");
  assert.ok(field.indexOf('class="line"') < button && button < note && note < capture, "sentence, button, note, capture");
  assert.match(field, /<a class="btn fill" href="https:\/\/github\.com\/RicardoSilveira23\/sobreiro\/releases\/latest">Download from GitHub /);
  assert.equal(count(html, 'class="btn fill"'), 1, "one download button on the page");
});

test("on the field the button is filled with the field's text colour, its text in the field's colour", () => {
  const rule = /\.field \.btn\.fill\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
  assert.match(rule, /background:\s*var\(--on-desk\)/);
  assert.match(rule, /(^|[;\s])color:\s*var\(--desk\)/);
});

test("the unknown-publisher warning is said once, under the download", () => {
  assert.equal(count(text(html), "unknown publisher"), 1);
  assert.match(between("btn fill", "demo"), /<\/a>\s*<p class="small">Sobreiro\.exe, one file\. Until the app is signed, Windows may warn about an unknown publisher: More info, then Run anyway\.<\/p>/);
});

test("hover and click are a row in Reading it, not a line on the field; its rows on Style and Options say right-click", () => {
  assert.doesNotMatch(text(between("field", "section")), /Hover/);
  const reading = html.slice(html.indexOf('id="reading"'), html.indexOf('id="install"'));
  assert.match(reading, /<div class="row"><span class="k">Pill<\/span><span class="n">Beside the clock<\/span><p class="s">Hover it for when each limit resets, click it for the details.<\/p><\/div>/);
  assert.equal(count(text(html), "Beside the clock"), 1);
});

test("the README opens with the page's sentence and meta, word for word", () => {
  const readme = text(read("../README.md").split("## ")[0]);
  const sentence = text(/<p class="line">([\s\S]*?)<\/p>/.exec(html)[1]);
  assert.ok(readme.includes(sentence), "the sentence");
  assert.ok(readme.includes("Free, for Windows 11. Needs GitHub Copilot with AI credits, or Claude Code on Pro, Max or Ultra."), "the meta");
});
