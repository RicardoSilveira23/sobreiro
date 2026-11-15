import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { findExposures, badAuthors, decode } from "./check-exposure.mjs";

const script = fileURLToPath(new URL("./check-exposure.mjs", import.meta.url));

const one = (text, path = "docs/a.md") => findExposures([{ path, text }]);

test("a 32-digit hexadecimal id is reported with its path and line", () => {
  const hits = one("first\nid: 0123456789abcdef0123456789abcdef\n");
  assert.deepEqual(hits, [
    { path: "docs/a.md", line: 2, kind: "hex32", match: "0123456789abcdef0123456789abcdef" },
  ]);
});

test("an id inside a longer run of hexadecimal digits is not an id", () => {
  assert.deepEqual(one("commit 0123456789abcdef0123456789abcdef01234567"), []);
});

test("a GUID is reported", () => {
  const hits = one("x\ny\n{12345678-90ab-cdef-1234-567890abcdef}");
  assert.equal(hits.length, 1);
  assert.equal(hits[0].kind, "guid");
  assert.equal(hits[0].line, 3);
});

test("an email address is reported", () => {
  const hits = one("write to someone@example.org now");
  assert.deepEqual(hits, [
    { path: "docs/a.md", line: 1, kind: "email", match: "someone@example.org" },
  ]);
});

test("a user folder is reported in each spelling", () => {
  for (const text of [
    "C:\\Users\\mel\\file",
    "/c/Users/mel/file",
    "/Users/mel/file",
    "/home/mel/file",
  ]) {
    const hits = one(text);
    assert.equal(hits.length, 1, text);
    assert.equal(hits[0].kind, "user-path", text);
    assert.equal(hits[0].line, 1);
  }
});

test("a made-up user folder is let through", () => {
  for (const name of ["me", "a", "ana", "x", "from-windows", "<name>"]) {
    assert.deepEqual(one(`C:\\Users\\${name}\\file`), [], name);
    assert.deepEqual(one(`/home/${name}/file`), [], name);
  }
});

test("the same name with another folder is not", () => {
  const hits = one("C:\\Users\\mel\\AppData");
  assert.equal(hits.length, 1);
  assert.equal(hits[0].kind, "user-path");
  assert.equal(one("C:\\Users\\ana2\\x").length, 1);
});

test("a path under C:\\Dev is reported", () => {
  const hits = one("see C:\\Dev\\something\\src");
  assert.equal(hits.length, 1);
  assert.equal(hits[0].kind, "dev-path");
});

test("the bridge command tests' own folder is let through, in those files only", () => {
  const text = "C:\\Dev\\my sobreiro\\bridge.exe";
  assert.deepEqual(one(text, "core/tests/all/bridge_command.rs"), []);
  assert.equal(one(text, "docs/a.md").length, 1);
});

test("each word that names a PC or a tool's scratch folder is reported", () => {
  for (const word of ["DESKTOP", "worktrees", "scratchpad", "Claude-Data"]) {
    const hits = one(`a ${word} b`);
    assert.equal(hits.length, 1, word);
    assert.equal(hits[0].kind, "pc-word", word);
    assert.equal(hits[0].match, word);
  }
});

test("a concrete workers.dev host is reported, with all its labels", () => {
  for (const host of ["sobreiro.some-name.workers.dev", "Some-Name.WORKERS.DEV"]) {
    const hits = one(`see https://${host}/x`);
    assert.equal(hits.length, 1, host);
    assert.equal(hits[0].kind, "pc-word", host);
    assert.equal(hits[0].match, host);
  }
});

test("the bare word workers.dev is not reported", () => {
  assert.deepEqual(one("the account's workers.dev host"), []);
});

test("the placeholder forms of a workers.dev host are not reported", () => {
  assert.deepEqual(one("<name>.workers.dev"), []);
  assert.deepEqual(one("https://pr-1-sobreiro.<WORKERS_SUBDOMAIN>"), []);
});

test("a word that only contains one of those is not reported", () => {
  assert.deepEqual(one("WM_DESKTOPSOMETHING and DESKTOP_X"), []);
  assert.deepEqual(one("desktop and Desktop"), []);
});

test("the allowed addresses pass", () => {
  assert.deepEqual(one("Co-Authored-By: Claude <noreply@anthropic.com>"), []);
  assert.deepEqual(one("12345+someone@users.noreply.github.com"), []);
});

test("the made-up address in the Copilot token test passes, elsewhere it does not", () => {
  const files = [
    { path: "core/tests/all/copilot_token.rs", text: "url: https://x:gho_abc@github.com" },
    { path: "core/src/a.rs", text: "url: https://x:gho_abc@github.com" },
  ];
  const hits = findExposures(files);
  assert.equal(hits.length, 1);
  assert.equal(hits[0].path, "core/src/a.rs");
});

test("the two lock files at the root are skipped", () => {
  const text = "0123456789abcdef0123456789abcdef";
  assert.deepEqual(one(text, "package-lock.json"), []);
  assert.deepEqual(one(text, "Cargo.lock"), []);
});

test("a lock file in a folder is read", () => {
  const text = "0123456789abcdef0123456789abcdef";
  assert.equal(one(text, "sub/package-lock.json").length, 1);
  assert.equal(one(text, "sub/Cargo.lock").length, 1);
});

test("a binary file is skipped by its extension", () => {
  const text = "0123456789abcdef0123456789abcdef";
  for (const ext of ["png", "jpg", "jpeg", "webp", "gif", "ttf", "woff2", "ico", "PNG"]) {
    assert.deepEqual(one(text, `a/b.${ext}`), [], ext);
  }
});

test("a file with a NUL byte but no binary extension is still read", () => {
  assert.equal(one("0123456789abcdef0123456789abcdef\0more", "a.bin").length, 1);
});

test("UTF-16 text is decoded by its byte-order mark and read", () => {
  const le = Buffer.concat([Buffer.from([0xff, 0xfe]), Buffer.from("see C:\\Users\\mel\\x", "utf16le")]);
  assert.equal(decode(le), "see C:\\Users\\mel\\x");
  assert.equal(one(decode(le), "notes.txt")[0]?.kind, "user-path");
  const be = Buffer.from("see C:\\Users\\mel\\x", "utf16le").swap16();
  assert.equal(decode(Buffer.concat([Buffer.from([0xfe, 0xff]), be])), "see C:\\Users\\mel\\x");
  assert.equal(decode(Buffer.from("plain \u00e9")), "plain \u00e9");
});

test("a file name like name@2x.png is not an address, any other address in an image name is", () => {
  assert.deepEqual(one("src: hero@2x.png and logo@1x.webp and a@1x-dark.gif and b@1x-still.png"), []);
  assert.equal(one("first.last@example.com-dark.png").length, 1);
  const hits = one("first.last@example.com.png");
  assert.equal(hits.length, 1);
  assert.equal(hits[0].kind, "email");
});

test("the user folder name runs to the next separator", () => {
  const hits = one("C:\\Users\\ana silva\\x");
  assert.equal(hits.length, 1);
  assert.equal(hits[0].match, "C:\\Users\\ana silva");
  assert.deepEqual(one("C:\\Users\\ana\\x"), []);
  assert.deepEqual(one('"C:\\Users\\ana" and more'), []);
});

test("Git Bash and WSL spellings of the folders are reported", () => {
  assert.equal(one("/c/Dev/proj/x")[0]?.kind, "dev-path");
  assert.equal(one("/mnt/c/Dev/proj/x")[0]?.kind, "dev-path");
  assert.equal(one("/mnt/c/Users/mel/x")[0]?.kind, "user-path");
  assert.deepEqual(one("/mnt/c/Users/me/x"), []);
});

test("machine names are reported in any case", () => {
  for (const name of ["DESKTOP-AB12CD3", "desktop-ab12cde", "LAPTOP-QWERTYUI", "DESKTOP-ABCDEFG"]) {
    const hits = one(`host ${name} here`);
    assert.ok(hits.length >= 1 && hits.length <= 2, name);
    assert.ok(hits.every((h) => h.kind === "pc-word"), name);
    assert.ok(hits.some((h) => h.match === name), name);
  }
  assert.deepEqual(one("a laptop and a desktop"), []);
  for (const word of ["desktop-like", "desktop-first", "Desktop-class", "laptop-sized"]) {
    assert.deepEqual(one(`a ${word} thing`), [], word);
  }
});

test("the bare upper-case word DESKTOP is reported whatever follows it", () => {
  for (const word of ["DESKTOP-PC", "DESKTOP-HOME", "DESKTOP-01", "DESKTOP-AB1", "DESKTOP-ABCDEFGHIJKLM", "DESKTOP-"]) {
    const hits = one(`a ${word} thing`);
    assert.ok(hits.length >= 1, word);
    assert.ok(hits.every((h) => h.kind === "pc-word" && h.match.startsWith("DESKTOP")), word);
  }
});

test("the folder words are reported in any case", () => {
  for (const word of ["Worktrees", "SCRATCHPAD", "claude-data"]) {
    assert.equal(one(`a ${word} b`).length, 1, word);
  }
});

test("a tracked file's path is checked too, as line 0", () => {
  const hits = one("clean text", "docs/scratchpad/0123456789abcdef0123456789abcdef.md");
  assert.deepEqual(
    hits.map((h) => [h.line, h.kind]),
    [
      [0, "hex32"],
      [0, "pc-word"],
    ],
  );
  assert.deepEqual(one("clean text", "docs/ok.md"), []);
});

test("an unknown flag prints a usage line and exits 2", () => {
  const run = spawnSync(process.execPath, [script, "--bogus"], { encoding: "utf8" });
  assert.equal(run.status, 2);
  assert.match(run.stderr, /usage/);
});

test("the script and its test, which hold the patterns, are skipped", () => {
  const text = "C:\\Users\\mel";
  assert.deepEqual(one(text, "scripts/check-exposure.mjs"), []);
  assert.deepEqual(one(text, "scripts/check-exposure.test.mjs"), []);
});

test("a no-reply author passes and any other address is reported", () => {
  assert.deepEqual(badAuthors(["1+a@users.noreply.github.com 1+a@users.noreply.github.com"]), []);
  assert.deepEqual(badAuthors(["me@example.org 1+a@users.noreply.github.com"]), [
    "me@example.org 1+a@users.noreply.github.com",
  ]);
  assert.deepEqual(badAuthors(["", "1+a@users.noreply.github.com me@example.org"]), [
    "1+a@users.noreply.github.com me@example.org",
  ]);
});

test("GitHub's own committer address passes", () => {
  assert.deepEqual(badAuthors(["1234567+someone@users.noreply.github.com noreply@github.com"]), []);
  assert.deepEqual(badAuthors(["1234567+someone@users.noreply.github.com noreply@example.com"]), [
    "1234567+someone@users.noreply.github.com noreply@example.com",
  ]);
});

test("doubled backslashes, as in a string literal or JSON, are caught", () => {
  assert.deepEqual(one("C:\\\\Users\\\\mel\\\\x").map((h) => h.kind), ["user-path"]);
  assert.deepEqual(one("C:\\\\Dev\\\\proj").map((h) => h.kind), ["dev-path"]);
  assert.deepEqual(one("C:\\\\Users\\\\me\\\\x"), []);
});

test("the allowed bridge folder is let through with doubled backslashes too", () => {
  assert.deepEqual(one("C:\\\\Dev\\\\my sobreiro\\\\b.exe", "core/tests/all/bridge_command.rs"), []);
});

test("Windows paths and workers.dev are caught whatever their case", () => {
  assert.equal(one("c:\\users\\mel\\x")[0]?.kind, "user-path");
  assert.equal(one("C:\\USERS\\mel\\x")[0]?.kind, "user-path");
  assert.equal(one("c:\\dev\\proj")[0]?.kind, "dev-path");
  assert.equal(one("C:\\DEV\\proj")[0]?.kind, "dev-path");
  assert.equal(one("Some-Name.WORKERS.DEV")[0]?.kind, "pc-word");
  assert.deepEqual(one("the desktop and Desktop"), []);
});

test("an allowed name or value must match exactly, not as a prefix", () => {
  assert.equal(one("C:\\Users\\First Last\\x")[0]?.kind, "user-path");
  const bridge = "core/tests/all/bridge_command.rs";
  assert.deepEqual(one("C:\\Dev\\my sobreiro\\x", bridge), []);
  assert.equal(one("C:\\Dev\\my sobreiro-anything\\x", bridge).length, 1);
  const token = "core/tests/all/copilot_token.rs";
  assert.deepEqual(one("https://x:gho_abc@github.com", token), []);
  assert.equal(one("https://x:gho_abc@github.com.xyz", token).length, 1);
});
