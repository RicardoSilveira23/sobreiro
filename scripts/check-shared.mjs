// A check that the shared stylesheet files match their lock, and that the
// project's own CSS neither restyles a shared rule nor uses a raw colour.
//
// Usage: node check-shared.mjs <lock-file> <own.css>...
//
// The lock is JSON: { "files": { "<path relative to the lock's folder>": "sha256-<hex>" } }.
// Paths are posix style and may climb with "../". Hashes are taken over the text
// with CRLF turned into LF, so a checkout with Windows line endings still matches.
// A file named shared.overrides.json beside the lock may list shared selectors the
// project restyles on purpose: { "<selector>": "<reason>" }.
//
// No packages, no network.

import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function hashOf(text) {
  const lf = text.replace(/\r\n/g, "\n");
  return "sha256-" + createHash("sha256").update(lf).digest("hex");
}

// ---- reading CSS ----------------------------------------------------------

function stripComments(css) {
  let out = "";
  let quote = "";
  for (let i = 0; i < css.length; i++) {
    const c = css[i];
    if (quote) {
      out += c;
      if (c === "\\") out += css[++i] ?? "";
      else if (c === quote) quote = "";
    } else if (c === '"' || c === "'") {
      quote = c;
      out += c;
    } else if (c === "/" && css[i + 1] === "*") {
      const end = css.indexOf("*/", i + 2);
      i = end < 0 ? css.length : end + 1;
      out += " ";
    } else {
      out += c;
    }
  }
  return out;
}

// Splits text at a separator that is outside quotes, parentheses and brackets.
function splitTop(text, sep) {
  const parts = [];
  let depth = 0;
  let quote = "";
  let cur = "";
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quote) {
      cur += c;
      if (c === "\\") cur += text[++i] ?? "";
      else if (c === quote) quote = "";
    } else if (c === '"' || c === "'") {
      quote = c;
      cur += c;
    } else if (c === "(" || c === "[") {
      depth++;
      cur += c;
    } else if (c === ")" || c === "]") {
      depth--;
      cur += c;
    } else if (c === sep && depth <= 0) {
      parts.push(cur);
      cur = "";
    } else {
      cur += c;
    }
  }
  parts.push(cur);
  return parts;
}

// Collapses whitespace and puts one space around the combinators > + ~.
function normaliseSelector(sel) {
  let out = "";
  let depthParen = 0;
  let depthBracket = 0;
  let quote = "";
  for (let i = 0; i < sel.length; i++) {
    const c = sel[i];
    if (quote) {
      out += c;
      if (c === "\\") out += sel[++i] ?? "";
      else if (c === quote) quote = "";
      continue;
    }
    if (c === '"' || c === "'") {
      quote = c;
      out += c;
    } else if (c === "[") {
      depthBracket++;
      out += c;
    } else if (c === "]") {
      depthBracket--;
      out += c;
    } else if (c === "(") {
      depthParen++;
      out += c;
    } else if (c === ")") {
      depthParen--;
      out += c;
    } else if (depthBracket === 0 && (c === ">" || c === "~" || (c === "+" && depthParen === 0))) {
      out += ` ${c} `;
    } else {
      out += c;
    }
  }
  return out
    .replace(/\s+/g, " ")
    .replace(/\s*,\s*/g, ", ")
    .replace(/\(\s+/g, "(")
    .replace(/\s+\)/g, ")")
    .trim();
}

// At-rules whose body holds ordinary rules.
const NESTING_AT_RULES = new Set(["media", "supports", "container", "layer", "scope", "document", "starting-style"]);

// Reads CSS into a flat list of rules:
//   { selectors: string[] | null, prelude, decls: [{ property, value }] }
// selectors is null for blocks that are not selectors (@font-face, keyframe steps).
function readRules(css) {
  const text = stripComments(css);
  const rules = [];
  let pos = 0;

  // Reads statements until the closing brace of the current block (or the end).
  // `kind` is "rules" (a list of rules and at-rules) or "decls" (a rule body, which
  // may also hold nested rules).
  function block(kind, owner) {
    const decls = [];
    while (pos < text.length) {
      // Gather the next statement: up to a top-level ; { or }.
      let start = pos;
      let depth = 0;
      let quote = "";
      let end = -1;
      let stop = "";
      for (; pos < text.length; pos++) {
        const c = text[pos];
        if (quote) {
          if (c === "\\") pos++;
          else if (c === quote) quote = "";
        } else if (c === '"' || c === "'") quote = c;
        else if (c === "(" || c === "[") depth++;
        else if (c === ")" || c === "]") depth--;
        else if (depth <= 0 && (c === ";" || c === "{" || c === "}")) {
          end = pos;
          stop = c;
          break;
        }
      }
      if (end < 0) {
        end = text.length;
        pos = text.length;
      }
      const statement = text.slice(start, end).trim();
      if (stop === "}") {
        if (kind === "decls" && statement) pushDecl(decls, statement);
        pos++;
        return decls;
      }
      if (stop === ";" || stop === "") {
        pos++;
        if (kind === "decls" && statement) pushDecl(decls, statement);
        continue;
      }
      // stop === "{": a nested block with `statement` as its prelude.
      pos++;
      openBlock(statement, owner);
    }
    return decls;
  }

  function pushDecl(decls, statement) {
    const i = statement.indexOf(":");
    if (i < 0) return;
    decls.push({ property: statement.slice(0, i).trim(), value: statement.slice(i + 1).trim() });
  }

  function openBlock(prelude, parent) {
    if (prelude.startsWith("@")) {
      const name = /^@([\w-]+)/.exec(prelude)?.[1].toLowerCase() ?? "";
      if (NESTING_AT_RULES.has(name)) {
        block("rules", parent);
      } else if (/keyframes$/.test(name)) {
        const label = normaliseSelector(prelude);
        // Steps: from, to, 50% are not selectors.
        while (pos < text.length) {
          const save = pos;
          const close = text.indexOf("}", pos);
          const open = text.indexOf("{", pos);
          if (open < 0 || (close >= 0 && close < open)) {
            pos = close < 0 ? text.length : close + 1;
            break;
          }
          const step = normaliseSelector(text.slice(save, open));
          pos = open + 1;
          const decls = block("decls", null);
          rules.push({ selectors: null, prelude: `${label} ${step}`, decls });
        }
      } else {
        const decls = block("decls", null);
        rules.push({ selectors: null, prelude: normaliseSelector(prelude), decls });
      }
      return;
    }
    const selectors = splitTop(prelude, ",")
      .map(normaliseSelector)
      .filter(Boolean);
    const rule = { selectors, prelude: selectors.join(", "), decls: [] };
    rules.push(rule);
    rule.decls = block("decls", rule);
  }

  block("rules", null);
  return rules;
}

export function selectorsOf(css) {
  const out = [];
  for (const rule of readRules(css)) if (rule.selectors) out.push(...rule.selectors);
  return out;
}

// ---- colours ----------------------------------------------------------------

// Every CSS named colour and system colour, lower case.
const NAMED = `
  aliceblue antiquewhite aqua aquamarine azure beige bisque black
  blanchedalmond blue blueviolet brown burlywood cadetblue chartreuse chocolate
  coral cornflowerblue cornsilk crimson cyan darkblue darkcyan darkgoldenrod
  darkgray darkgreen darkgrey darkkhaki darkmagenta darkolivegreen darkorange darkorchid
  darkred darksalmon darkseagreen darkslateblue darkslategray darkslategrey darkturquoise darkviolet
  deeppink deepskyblue dimgray dimgrey dodgerblue firebrick floralwhite forestgreen
  fuchsia gainsboro ghostwhite gold goldenrod gray green greenyellow
  grey honeydew hotpink indianred indigo ivory khaki lavender
  lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan lightgoldenrodyellow lightgray
  lightgreen lightgrey lightpink lightsalmon lightseagreen lightskyblue lightslategray lightslategrey
  lightsteelblue lightyellow lime limegreen linen magenta maroon mediumaquamarine
  mediumblue mediumorchid mediumpurple mediumseagreen mediumslateblue mediumspringgreen mediumturquoise mediumvioletred
  midnightblue mintcream mistyrose moccasin navajowhite navy oldlace olive
  olivedrab orange orangered orchid palegoldenrod palegreen paleturquoise palevioletred
  papayawhip peachpuff peru pink plum powderblue purple rebeccapurple
  red rosybrown royalblue saddlebrown salmon sandybrown seagreen seashell
  sienna silver skyblue slateblue slategray slategrey snow springgreen
  steelblue tan teal thistle tomato turquoise violet wheat
  white whitesmoke yellow yellowgreen canvas canvastext linktext visitedtext
  activetext buttonface buttontext buttonborder field fieldtext highlight highlighttext
  selecteditem selecteditemtext mark marktext graytext accentcolor accentcolortext
`.split(/\s+/).filter(Boolean);
const HEX = /#(?:[0-9a-f]{8}|[0-9a-f]{6}|[0-9a-f]{3,4})(?![\w-])/gi;
const FUNC = /(?<![\w-])(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color-mix|color|light-dark)\(/gi;
const NAME = new RegExp(`(?<![\\w-])(?:${NAMED.join("|")})(?![\\w(-])`, "gi");

// Properties that carry a colour; named colours are looked for only in these.
const COLOUR_PROPERTIES = new Set([
  "color", "background", "background-color", "background-image", "border", "outline",
  "outline-color", "fill", "stroke", "box-shadow", "text-shadow", "text-decoration",
  "text-decoration-color", "caret-color", "accent-color", "column-rule", "column-rule-color",
  "filter", "-webkit-text-fill-color",
]);
function carriesColour(property) {
  const p = property.toLowerCase();
  return COLOUR_PROPERTIES.has(p) || p.startsWith("border-") || p.startsWith("-webkit-text-stroke");
}

// Blanks out url(...) and quoted text, keeping the length.
function maskValue(value) {
  let out = "";
  let i = 0;
  while (i < value.length) {
    const c = value[i];
    if (c === '"' || c === "'") {
      let j = i + 1;
      while (j < value.length && value[j] !== c) j += value[j] === "\\" ? 2 : 1;
      out += " ".repeat(Math.min(j + 1, value.length) - i);
      i = j + 1;
    } else if (/^url\(/i.test(value.slice(i, i + 4))) {
      let j = i + 4;
      let q = "";
      for (; j < value.length; j++) {
        if (q) {
          if (value[j] === "\\") j++;
          else if (value[j] === q) q = "";
        } else if (value[j] === '"' || value[j] === "'") q = value[j];
        else if (value[j] === ")") break;
      }
      out += " ".repeat(Math.min(j + 1, value.length) - i);
      i = j + 1;
    } else {
      out += c;
      i++;
    }
  }
  return out;
}

// The text of a function call starting at `from`, up to its closing parenthesis.
function callAt(text, from) {
  let depth = 0;
  for (let i = from; i < text.length; i++) {
    if (text[i] === "(") depth++;
    else if (text[i] === ")" && --depth === 0) return text.slice(from, i + 1);
  }
  return text.slice(from);
}

// Every colour literal in a declaration value, in order of appearance.
function literalsIn(value, named) {
  const masked = maskValue(value);
  const found = [];
  for (const m of masked.matchAll(HEX)) found.push({ at: m.index, text: m[0] });
  for (const m of masked.matchAll(FUNC)) found.push({ at: m.index, text: callAt(value, m.index) });
  if (named) for (const m of masked.matchAll(NAME)) found.push({ at: m.index, text: m[0] });
  // A literal inside a colour function is part of that function's one report.
  const out = [];
  let until = -1;
  for (const f of found.sort((a, b) => a.at - b.at)) {
    if (f.at < until) continue;
    out.push(f.text);
    if (f.text.endsWith(")")) until = f.at + f.text.length;
  }
  return out;
}

// One entry per colour literal used outside a custom property's value.
// `value` is the literal itself, not the whole declaration.
export function colourLiterals(css) {
  const out = [];
  for (const rule of readRules(css)) {
    for (const d of rule.decls) {
      if (d.property.startsWith("--")) continue;
      for (const lit of literalsIn(d.value, carriesColour(d.property))) {
        out.push({ selector: rule.prelude, property: d.property, value: lit });
      }
    }
  }
  return out;
}

// ---- the check --------------------------------------------------------------

function readText(file) {
  return readFileSync(file, "utf8");
}

export function checkShared(lockFile, ownFiles) {
  const problems = [];
  const lockDir = dirname(resolve(lockFile));
  const lock = JSON.parse(readText(lockFile));
  const files = lock.files ?? {};
  let patternsPath = null;

  for (const required of ["patterns.css", "foundation.css", "check-shared.mjs"]) {
    if (!Object.keys(files).some((key) => basename(key) === required)) {
      problems.push(`shared.lock.json: must list ${required}`);
    }
  }

  for (const [key, expected] of Object.entries(files)) {
    const path = resolve(lockDir, key);
    if (!existsSync(path)) {
      problems.push(`${key}: missing`);
      continue;
    }
    if (hashOf(readText(path)) !== expected) problems.push(`${key}: changed, it is a shared file`);
    if (basename(key) === "patterns.css") patternsPath = path;
  }

  const shared = new Set(patternsPath && existsSync(patternsPath) ? selectorsOf(readText(patternsPath)) : []);
  const overridesPath = join(lockDir, "shared.overrides.json");
  const overrides = existsSync(overridesPath) ? JSON.parse(readText(overridesPath)) : {};
  const ownSelectors = new Set();

  for (const own of ownFiles) {
    if (!existsSync(own)) {
      problems.push(`${own}: missing`);
      continue;
    }
    const css = readText(own);
    const reported = new Set();
    for (const sel of selectorsOf(css)) {
      ownSelectors.add(sel);
      if (shared.has(sel) && !(sel in overrides) && !reported.has(sel)) {
        reported.add(sel);
        problems.push(`${own}: restyles the shared ${sel}`);
      }
    }
    for (const c of colourLiterals(css)) {
      problems.push(`${own}: raw colour ${c.value} in ${c.selector} { ${c.property} }`);
    }
  }

  for (const sel of Object.keys(overrides)) {
    if (!ownSelectors.has(sel) || !shared.has(sel)) {
      problems.push(`shared.overrides.json: ${sel} overrides nothing`);
    }
  }

  return problems;
}

// ---- command line -----------------------------------------------------------

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [lockFile, ...own] = process.argv.slice(2);
  if (!lockFile) {
    console.error("usage: node check-shared.mjs <lock-file> <own.css>...");
    process.exit(2);
  }
  const problems = checkShared(lockFile, own);
  if (problems.length) {
    console.log(problems.join("\n"));
    process.exit(1);
  }
  const count = Object.keys(JSON.parse(readText(lockFile)).files ?? {}).length;
  console.log(`shared ok: ${count} files`);
}
