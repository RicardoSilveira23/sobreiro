// Guards the static site: local links and files exist, fragments exist, nothing loads from
// another host, images carry alt/width/height, pages have the basics, and each page's canonical
// link is its own address on the Worker's custom domain (wrangler.jsonc), with the tags a shared
// link's card is made from.
// Usage: node scripts/check-site.mjs [dir]   (default: site)
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, posix, relative, sep } from "node:path";
import { pathToFileURL } from "node:url";

const OTHER_HOST = /^(https?:)?\/\//i;
const OTHER_SCHEME = /^[a-z][a-z0-9+.-]*:/i;

function walk(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? walk(join(dir, e.name)) : [join(dir, e.name)],
  );
}

const attr = (tag, name) => {
  const m = new RegExp(String.raw`\s${name}\s*=\s*(?:"([^"]*)"|'([^']*)')`, "i").exec(tag);
  return m ? (m[1] ?? m[2]) : null;
};

// True when the file exists with exactly this case in every path segment: Windows and macOS
// match names case-insensitively, the Worker serving the site does not.
function isFileExact(root, target) {
  if (!existsSync(target) || !statSync(target).isFile()) return false;
  let dir = root;
  for (const segment of relative(root, target).split(sep)) {
    if (!readdirSync(dir).includes(segment)) return false;
    dir = join(dir, segment);
  }
  return true;
}

// The file a reference points at (a directory means its index.html), or null when missing.
function resolve(root, fromFile, path) {
  let target = join(dirname(fromFile), path);
  if (path.endsWith("/") || (existsSync(target) && statSync(target).isDirectory())) {
    target = join(target, "index.html");
  }
  return isFileExact(root, target) ? target : null;
}

// Which attributes of which elements make a request.
const REF_ATTRS = {
  link: ["href"],
  a: ["href"],
  script: ["src"],
  img: ["src", "srcset"],
  source: ["src", "srcset"],
  iframe: ["src"],
  video: ["src", "poster"],
  audio: ["src"],
  track: ["src"],
  embed: ["src"],
  object: ["data"],
};

function checkRef(root, file, name, ref, { skipExternal = false } = {}) {
  if (OTHER_HOST.test(ref)) return skipExternal ? [] : [`${name}: other host ${ref}`];
  if (OTHER_SCHEME.test(ref)) return []; // mailto:, tel:, data:
  const [withoutFragment, fragment] = ref.split("#");
  const path = withoutFragment.split("?")[0];
  // Relative links keep the pages working from any folder, opened as files too.
  if (path.startsWith("/")) return [`${name}: root-absolute link ${ref}`];
  let target = file;
  if (path) {
    target = resolve(root, file, path);
    if (!target) return [`${name}: missing file ${path}`];
  }
  if (fragment && target.endsWith(".html")) {
    const ids = [...readFileSync(target, "utf8").matchAll(/\sid\s*=\s*["']([^"']+)["']/gi)];
    if (!ids.some((m) => m[1] === fragment)) return [`${name}: missing anchor ${path}#${fragment}`];
  }
  return [];
}

const isCanonical = (tag, rest) => tag === "link" && /^canonical$/i.test(attr(rest, "rel") ?? "");

// The page's own address: index.html is its folder's address.
function checkCanonical(html, name, address) {
  const want = new URL(name.replace(/(^|\/)index\.html$/, "$1"), address).href;
  const found = [...html.matchAll(/<link\b([^>]*)>/gi)]
    .filter(([, rest]) => isCanonical("link", rest))
    .map(([, rest]) => attr(rest, "href"));
  if (!found.length) return [`${name}: missing <link rel="canonical" href="${want}">`];
  return found.filter((href) => href !== want).map((href) => `${name}: canonical ${href}, wanted ${want}`);
}

// The card a pasted link shows: a title, a sentence, the page's own address and a picture this
// site serves. The picture's address is absolute, as the services that draw the card need.
function checkShare(root, html, name, address) {
  const metas = [...html.matchAll(/<meta\b([^>]*)>/gi)].map(([, rest]) => ({
    key: attr(rest, "property") ?? attr(rest, "name"),
    content: attr(rest, "content"),
  }));
  const value = (key) => metas.find((m) => m.key === key)?.content ?? null;
  const problems = [];
  for (const key of ["og:title", "og:description", "og:url", "og:image", "twitter:card"]) {
    if (!value(key)) problems.push(`${name}: missing <meta ${key.startsWith("og:") ? "property" : "name"}="${key}">`);
  }
  const want = new URL(name.replace(/(^|\/)index\.html$/, "$1"), address).href;
  const url = value("og:url");
  if (url && url !== want) problems.push(`${name}: og:url ${url}, wanted ${want}`);
  const image = value("og:image");
  if (image && !(image.startsWith(address) && isFileExact(root, join(root, image.slice(address.length))))) {
    problems.push(`${name}: og:image ${image} is not a file of this site`);
  }
  return problems;
}

function checkPage(root, file, name, address) {
  const html = readFileSync(file, "utf8").replace(/<!--[\s\S]*?-->/g, "");
  const problems = [];
  if (address) problems.push(...checkCanonical(html, name, address), ...checkShare(root, html, name, address));
  if (!/<html\b[^>]*\slang\s*=/i.test(html)) problems.push(`${name}: missing <html lang>`);
  if (!/<title\b[^>]*>\s*\S/i.test(html)) problems.push(`${name}: missing <title>`);
  for (const meta of ["viewport", "description"]) {
    if (!new RegExp(String.raw`<meta\b[^>]*\sname\s*=\s*["']${meta}["']`, "i").test(html)) {
      problems.push(`${name}: missing <meta name="${meta}">`);
    }
  }
  for (const [, css] of html.matchAll(/<style\b[^>]*>([\s\S]*?)<\/style>/gi)) {
    problems.push(...checkCssText(root, file, name, css));
  }
  for (const [, tagName, rest] of html.matchAll(/<([a-z][a-z0-9]*)\b([^>]*)>/gi)) {
    const tag = tagName.toLowerCase();
    if (isCanonical(tag, rest)) continue; // an address, not a request: checkCanonical has it
    const refs = [];
    for (const a of REF_ATTRS[tag] ?? []) {
      const value = attr(rest, a);
      if (value === null) continue;
      // srcset is "url 2x, url 1x"; the others are a single url
      refs.push(...(a === "srcset" ? value.split(",").map((c) => c.trim().split(/\s+/)[0]) : [value]));
    }
    for (const ref of refs.filter(Boolean)) {
      problems.push(...checkRef(root, file, name, ref, { skipExternal: tag === "a" }));
    }
    const style = attr(rest, "style");
    if (style) problems.push(...checkCssText(root, file, name, style));
    if (tag === "img") {
      for (const a of ["alt", "width", "height"]) {
        if (attr(rest, a) === null) problems.push(`${name}: img ${attr(rest, "src") ?? "(no src)"} needs ${a}`);
      }
    }
  }
  return problems;
}

// Every reference in a piece of CSS: @import "x" and url(x).
function checkCssText(root, file, name, css) {
  const refs = [
    ...[...css.matchAll(/@import\s+(['"])(.*?)\1/g)].map((m) => m[2]),
    ...[...css.matchAll(/url\(\s*(['"]?)(.*?)\1\s*\)/g)].map((m) => m[2]),
  ];
  return refs
    .filter((ref) => ref && !ref.startsWith("data:"))
    .flatMap((ref) => checkRef(root, file, name, ref));
}

// The site's public address, from the Worker's custom-domain route. wrangler.jsonc keeps its
// comments on lines of their own.
export function siteAddress(config = new URL("../wrangler.jsonc", import.meta.url)) {
  const lines = readFileSync(config, "utf8").split(/\r?\n/).filter((l) => !l.trim().startsWith("//"));
  const domains = (JSON.parse(lines.join("\n")).routes ?? []).filter((r) => r.custom_domain);
  if (domains.length !== 1) throw new Error(`${config}: wanted one custom domain in routes, found ${domains.length}`);
  return `https://${domains[0].pattern}/`;
}

// address: when given, each page must name its own address there as canonical.
export function checkSite(dir, { address } = {}) {
  const files = walk(dir).sort();
  const name = (f) => relative(dir, f).split(sep).join(posix.sep);
  const pages = files.filter((f) => f.endsWith(".html"));
  const sheets = files.filter((f) => f.endsWith(".css"));
  return [
    ...pages.flatMap((f) => checkPage(dir, f, name(f), address)),
    ...sheets.flatMap((f) => checkCssText(dir, f, name(f), readFileSync(f, "utf8"))),
  ];
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const dir = process.argv[2] ?? "site";
  const problems = checkSite(dir, { address: siteAddress() });
  if (problems.length) {
    console.log(problems.join("\n"));
    process.exit(1);
  }
  const pages = walk(dir).filter((f) => f.endsWith(".html")).length;
  console.log(`site ok: ${pages} pages`);
}
