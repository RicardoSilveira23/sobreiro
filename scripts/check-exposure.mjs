// Keeps account identifiers, local PC paths and personal email addresses out of the tracked
// files and out of the commits. It reads every file `git ls-files` lists and reports a line for
// each hit; the exit code is 1 when there is one.
//
//   node scripts/check-exposure.mjs                   checks the tracked files
//   node scripts/check-exposure.mjs --commits <range> checks author and committer addresses
//
// Kinds: hex32 (32 hexadecimal digits), guid, email, user-path (a user's folder on Windows,
// macOS or Linux), dev-path (a folder under C:\Dev), pc-word (a word that names a PC, a tool's
// scratch folder or an account's hosting subdomain).

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// Files that are not read at all. One line of reason each.
const SKIPPED_PATHS = [
  // Generated; holds checksums that look like identifiers. Only these two, at the root.
  { path: "package-lock.json" },
  { path: "Cargo.lock" },
  // These two hold the patterns and the samples the check looks for.
  { path: "scripts/check-exposure.mjs" },
  { path: "scripts/check-exposure.test.mjs" },
];

// User folder names that are made up, used in examples and tests.
const ALLOWED_USER_NAMES = [
  "me", // example of a user folder
  "a", // example of a user folder
  "ana", // example of a user folder
  "x", // example of a user folder
  "from-windows", // example of a user folder
  "<name>", // placeholder in documentation
];

// Extensions of binary files, which are not read: the ones .gitattributes marks as binary, and
// .gif, which is tracked and binary too. Every other file is read.
const BINARY_EXTENSIONS = new Set(["png", "jpg", "jpeg", "webp", "gif", "ttf", "woff2", "ico"]);

// Addresses that are not a person's: the assistant's co-author line and GitHub's no-reply
// addresses.
const ALLOWED_ADDRESSES = ["noreply@anthropic.com"];
const NOREPLY_SUFFIX = "@users.noreply.github.com";

// Commit addresses allowed besides the no-reply suffix.
const ALLOWED_COMMIT_ADDRESSES = [
  "noreply@github.com", // GitHub's own committer: merge commits, web edits, its bots
];

// One value allowed in one file. A path matches exactly; the value is what the hit starts with.
const ALLOWED_IN_FILE = [
  {
    kind: "dev-path",
    value: "C:\\Dev\\my sobreiro",
    paths: [
      "core/tests/all/bridge_command.rs",
      "tests/Sobreiro.Tests/BridgeCommandTests.cs",
    ],
    reason: "a folder name with a space, made up for the bridge command tests",
  },
  {
    kind: "user-path",
    value: "c:/users/A",
    paths: [
      "core/tests/all/bridge_command.rs",
      "tests/Sobreiro.Tests/BridgeCommandTests.cs",
    ],
    reason: "a made-up user folder in capitals, in the bridge command tests",
  },
  {
    kind: "email",
    value: "gho_abc@github.com",
    paths: ["core/tests/all/copilot_token.rs"],
    reason: "a made-up token inside a sample URL, not an address",
  },
];

const HEX = "[0-9a-fA-F]";
const PATTERNS = [
  { kind: "hex32", re: new RegExp(`(?<!${HEX})${HEX}{32}(?!${HEX})`, "g") },
  {
    kind: "guid",
    re: new RegExp(`(?<!${HEX})${HEX}{8}-${HEX}{4}-${HEX}{4}-${HEX}{4}-${HEX}{12}(?!${HEX})`, "g"),
  },
  {
    kind: "email",
    re: /[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}/g,
  },
  {
    kind: "user-path",
    // Separators may be doubled (`C:\\Users\\x` in a string literal or JSON); case is ignored.
    // The name runs to the next separator or quote when one follows on the line (so it may hold
    // spaces); otherwise it is one word. Also `/c/Users` (Git Bash) and `/mnt/c/Users` (WSL).
    re: /(?:[A-Za-z]:[\\/]+Users[\\/]+|(?:\/mnt)?\/c\/Users\/|(?<![\w:])\/(?:Users|home)\/)(<name>|[\w.$%{}~-]+(?: [\w.$%{}~-]+)*(?=[\\/"'`])|[\w.$%{}~-]+)/gi,
  },
  {
    kind: "dev-path",
    re: /(?:[A-Za-z]:[\\/]+|(?<![\w.-])(?:\/mnt)?\/c\/)Dev[\\/]+[^\s"'`)<>|*?]*/gi,
  },
  {
    kind: "pc-word",
    re: /(?<![A-Za-z0-9_])DESKTOP(?![A-Za-z0-9_])/g,
  },
  {
    kind: "pc-word",
    // A machine name: DESKTOP- or LAPTOP- and 5 to 12 letters or digits, the suffix holding a
    // digit (any case) or the whole name upper case.
    re: /(?<![A-Za-z0-9_])(?:DESKTOP|LAPTOP)-(?=[A-Za-z0-9]{5,12}(?![A-Za-z0-9_]))[A-Za-z]*\d[A-Za-z0-9]*/gi,
  },
  {
    kind: "pc-word",
    re: /(?<![A-Za-z0-9_])(?:DESKTOP|LAPTOP)-[A-Z]{5,12}(?![A-Za-z0-9_])/g,
  },
  {
    kind: "pc-word",
    re: /(?<![A-Za-z0-9_])(?:worktrees|scratchpad|Claude-Data)(?![A-Za-z0-9_])/gi,
  },
  {
    kind: "pc-word",
    re: /(?<![A-Za-z0-9_-])(?:[A-Za-z0-9-]+\.)+workers\.dev(?![A-Za-z0-9_])/gi,
  },
];

const slashes = (s) => s.replace(/[\\/]+/g, "/").toLowerCase();

// "name@2x.png" is an image file, not an address; so is "name@1x-dark.gif" (the media folder's
// naming: a pixel ratio, then dark, light or still).
const IMAGE_FILE = /@\dx(?:-(?:dark|light|still))?\.(?:png|jpe?g|gif|webp|avif|svg|ico)$/i;

// `rest` is the line from the start of the match, for values that hold a space.
function allowed(kind, match, rest, path, groups) {
  if (kind === "user-path" && ALLOWED_USER_NAMES.includes(groups[1])) return true;
  if (kind === "email") {
    if (ALLOWED_ADDRESSES.includes(match) || match.endsWith(NOREPLY_SUFFIX)) return true;
    if (IMAGE_FILE.test(match)) return true;
  }
  return ALLOWED_IN_FILE.some((a) => {
    if (a.kind !== kind || !a.paths.includes(path)) return false;
    if (kind === "email") return match === a.value;
    // The value must end where the path name ends, not run on into a longer name.
    const rest2 = slashes(rest);
    const value = slashes(a.value);
    return rest2.startsWith(value) && !/[\w-]/.test(rest2.charAt(value.length));
  });
}

const skipped = (path) => SKIPPED_PATHS.some((s) => s.path === path);

const isBinary = (path) => BINARY_EXTENSIONS.has((path.match(/\.([^./]+)$/)?.[1] ?? "").toLowerCase());

// Text of a file's bytes: UTF-16 when they start with a byte-order mark, otherwise UTF-8.
export function decode(buffer) {
  if (buffer[0] === 0xff && buffer[1] === 0xfe) return buffer.subarray(2).toString("utf16le");
  if (buffer[0] === 0xfe && buffer[1] === 0xff) {
    const body = Buffer.from(buffer.subarray(2, 2 + ((buffer.length - 2) & ~1)));
    return body.swap16().toString("utf16le");
  }
  return buffer.toString("utf8");
}

// Reports the hits of one line of text; `line` 0 stands for the file's path.
function scan(hits, path, line, text) {
  for (const { kind, re } of PATTERNS) {
    for (const m of text.matchAll(re)) {
      if (allowed(kind, m[0], text.slice(m.index), path, m)) continue;
      hits.push({ path, line, kind, match: m[0] });
    }
  }
}

export function findExposures(files) {
  const hits = [];
  for (const { path, text } of files) {
    if (skipped(path)) continue;
    scan(hits, path, 0, path);
    if (isBinary(path)) continue;
    text.split("\n").forEach((line, i) => scan(hits, path, i + 1, line));
  }
  return hits;
}

// Each line is "<author address> <committer address>", as `git log --format='%ae %ce'` prints.
// Returns the lines in which an address is not a GitHub no-reply address
// or GitHub's own committer address.
export function badAuthors(lines) {
  return lines.filter((l) => l.trim() && l.trim().split(/\s+/).some((a) => !a.endsWith(NOREPLY_SUFFIX) && !ALLOWED_COMMIT_ADDRESSES.includes(a)));
}

function git(...args) {
  return execFileSync("git", args, { encoding: "utf8", maxBuffer: 1 << 28 });
}

function main(argv) {
  if (argv.some((a) => a.startsWith("--") && a !== "--commits")) {
    console.error("usage: check-exposure.mjs [--commits <range>]");
    return 2;
  }
  const at = argv.indexOf("--commits");
  if (at !== -1) {
    const range = argv[at + 1];
    if (!range) {
      console.error("usage: check-exposure.mjs --commits <range>");
      return 2;
    }
    const bad = badAuthors(git("log", "--format=%ae %ce", range).split("\n"));
    for (const line of bad) console.log(`commit  author  ${line}`);
    return bad.length ? 1 : 0;
  }
  const files = git("ls-files", "-z")
    .split("\0")
    .filter(Boolean)
    .map((path) => ({ path, text: isBinary(path) ? "" : decode(readFileSync(path)) }));
  const hits = findExposures(files);
  for (const h of hits) console.log(`${h.path}:${h.line}  ${h.kind}  ${h.match}`);
  return hits.length ? 1 : 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) process.exitCode = main(process.argv.slice(2));
