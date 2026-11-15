// Checks the running site over HTTP. CI runs it after each deploy, rolling back if it fails, and
// on each pull request's preview.
//
//   npm run smoke                          the public address, from wrangler.jsonc
//   npm run smoke -- http://localhost:8787 (npm run dev)
//
// The public address needs no login. Staging (workers.dev and each pull request's preview) is
// behind a GitHub login: CI gets through with a Cloudflare Access service token, from
// CF_ACCESS_CLIENT_ID and CF_ACCESS_CLIENT_SECRET; without them, every page there is the login.
import { readdirSync } from "node:fs";
import { siteAddress } from "./check-site.mjs";

const base = process.argv[2] ?? siteAddress();

const { CF_ACCESS_CLIENT_ID: accessId, CF_ACCESS_CLIENT_SECRET: accessSecret } = process.env;
const headers =
  accessId && accessSecret ? { "CF-Access-Client-Id": accessId, "CF-Access-Client-Secret": accessSecret } : {};

async function get(path) {
  for (let attempt = 1; ; attempt++) {
    try {
      const res = await fetch(new URL(path, base), {
        redirect: "manual",
        signal: AbortSignal.timeout(20_000),
        headers,
      });
      if (res.status < 500 || attempt === 3) return res;
    } catch (error) {
      if (attempt === 3) throw error;
    }
    await new Promise((resolve) => setTimeout(resolve, 10_000));
  }
}

// Every page in site/, so a page added there is checked here.
const pages = readdirSync(new URL("../site/", import.meta.url), { recursive: true })
  .map((file) => file.replaceAll("\\", "/"))
  .filter((file) => file === "index.html" || file.endsWith("/index.html"))
  .map((file) => "/" + file.slice(0, -"index.html".length))
  .sort();

for (const path of pages) {
  const res = await get(path);
  if (res.status !== 200) throw new Error(`${path}: status ${res.status}`);
  if (!(await res.text()).includes("<h1")) throw new Error(`${path}: no <h1>`);
  console.log(`ok ${path}`);
}
const missing = await get("/no-such-page/");
if (missing.status !== 404) throw new Error(`/no-such-page/: status ${missing.status}, wanted 404`);
console.log("ok not found");
