#!/usr/bin/env node
// sync-official-tool-logos.mjs — bundle first-party tool logos under /brand/tools/.
//
// For tools with status official|verified, downloads logos from curated overrides
// or the vendor homepage (og:image, apple-touch-icon, favicon), writes
// public/brand/tools/{slug}.png, and optionally updates tools.logo_url.
//
// Usage:
//   node scripts/sync-official-tool-logos.mjs [--apply] [--force] [--slug <slug>]
//
// Env: DATABASE_URL (or SUPABASE_URL + SUPABASE_SERVICE_KEY via verify-tool-official helpers)

import { readFileSync, mkdirSync, existsSync, copyFileSync, writeFileSync } from "node:fs";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const OVERRIDES = JSON.parse(
  readFileSync(resolve(ROOT, "scripts/official-tool-logo-overrides.json"), "utf8"),
);
const BRAND_DIR = resolve(ROOT, "public/brand/tools");
const FRONTEND_BRAND_DIR = resolve(ROOT, "frontend/public/brand/tools");
const MIN_BYTES = 400;

function parseEnvFile(path) {
  const out = {};
  try {
    for (const raw of readFileSync(path, "utf8").split("\n")) {
      const line = raw.trim();
      if (!line || line.startsWith("#")) continue;
      const eq = line.indexOf("=");
      if (eq <= 0) continue;
      out[line.slice(0, eq).trim()] = line
        .slice(eq + 1)
        .trim()
        .replace(/^["']|["']$/g, "");
    }
  } catch {
    /* optional */
  }
  return out;
}

const env = {
  ...parseEnvFile(resolve(ROOT, ".env")),
  ...process.env,
};

const APPLY = process.argv.includes("--apply");
const FORCE = process.argv.includes("--force");
const slugIdx = process.argv.indexOf("--slug");
const ONLY_SLUG = slugIdx >= 0 ? process.argv[slugIdx + 1] : null;

async function pgClient() {
  const { Client } = require(resolve(ROOT, "scripts/ops/node_modules/pg"));
  const databaseUrl = env.DATABASE_URL || "";
  if (!databaseUrl) throw new Error("DATABASE_URL required");
  const client = new Client({
    connectionString: databaseUrl,
    ssl: /supabase\.(co|com)/i.test(databaseUrl)
      ? { rejectUnauthorized: env.PG_INSECURE_SSL === "1" ? false : true }
      : undefined,
  });
  await client.connect();
  return client;
}

function absUrl(base, href) {
  try {
    return new URL(href, base).toString();
  } catch {
    return null;
  }
}

function brandRootForHomepage(homepage) {
  if (!homepage) return null;
  try {
    const host = new URL(homepage).hostname.toLowerCase();
    for (const [docsHost, root] of Object.entries(OVERRIDES.domain_roots || {})) {
      if (host === docsHost || host.endsWith(`.${docsHost}`)) return root;
    }
    const parts = host.split(".");
    if (parts.length >= 2) {
      return `https://${parts.slice(-2).join(".")}`;
    }
    return `https://${host}`;
  } catch {
    return null;
  }
}

async function fetchBytes(url, timeoutMs = 20000) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const res = await fetch(url, {
      signal: controller.signal,
      headers: { "User-Agent": "OnchainAI-logo-sync/1.0", Accept: "image/*,*/*" },
      redirect: "follow",
    });
    if (!res.ok) return null;
    const type = (res.headers.get("content-type") || "").toLowerCase();
    if (type.includes("text/html") && !url.endsWith(".ico")) return null;
    const buf = Buffer.from(await res.arrayBuffer());
    return buf.length >= MIN_BYTES ? buf : null;
  } catch {
    return null;
  } finally {
    clearTimeout(timer);
  }
}

function parseHtmlCandidates(html, pageUrl) {
  const found = [];
  const push = (href) => {
    const u = absUrl(pageUrl, href);
    if (u) found.push(u);
  };
  for (const m of html.matchAll(/<meta[^>]+property=["']og:image(?::secure_url)?["'][^>]+content=["']([^"']+)["']/gi)) {
    push(m[1]);
  }
  for (const m of html.matchAll(/<meta[^>]+name=["']twitter:image["'][^>]+content=["']([^"']+)["']/gi)) {
    push(m[1]);
  }
  for (const m of html.matchAll(/<link[^>]+rel=["'](?:apple-touch-icon|icon|shortcut icon)["'][^>]*>/gi)) {
    const tag = m[0];
    const href = tag.match(/href=["']([^"']+)["']/i)?.[1];
    if (href) push(href);
  }
  return found;
}

async function discoverLogoUrls(tool) {
  const urls = [];
  if (OVERRIDES.slugs?.[tool.slug]) urls.push(OVERRIDES.slugs[tool.slug]);

  const homepage = tool.homepage?.trim();
  if (homepage) {
    const root = brandRootForHomepage(homepage);
    if (root) {
      urls.push(`${root}/apple-touch-icon.png`);
      urls.push(`${root}/favicon.ico`);
    }
    try {
      const parsed = new URL(homepage);
      const host = parsed.hostname.toLowerCase();
      if (host !== "github.com" && !host.endsWith(".github.io")) {
        urls.push(`${parsed.origin}/apple-touch-icon.png`);
        urls.push(`${parsed.origin}/favicon.ico`);
      }
    } catch {
      /* ignore */
    }
    const page = await fetch(homepage, {
      headers: { "User-Agent": "OnchainAI-logo-sync/1.0" },
      redirect: "follow",
    }).catch(() => null);
    if (page?.ok) {
      const html = await page.text();
      urls.push(...parseHtmlCandidates(html, homepage));
    }
  }

  if (tool.repo_url) {
    try {
      const host = new URL(tool.repo_url).hostname;
      if (host === "github.com") {
        const owner = tool.repo_url.split("github.com/")[1]?.split("/")[0];
        if (owner) urls.push(`https://github.com/${owner}.png?size=128`);
      }
    } catch {
      /* ignore */
    }
  }

  return [...new Set(urls.filter(Boolean))];
}

function writePng(slug, bytes) {
  mkdirSync(BRAND_DIR, { recursive: true });
  mkdirSync(FRONTEND_BRAND_DIR, { recursive: true });
  const src = join(BRAND_DIR, `${slug}.src`);
  const out = join(BRAND_DIR, `${slug}.png`);
  const outFrontend = join(FRONTEND_BRAND_DIR, `${slug}.png`);
  writeFileSync(src, bytes);
  try {
    execFileSync("sips", ["-s", "format", "png", src, "--out", out], {
      stdio: "pipe",
    });
    execFileSync(
      "sips",
      ["-Z", "128", out, "--out", out],
      { stdio: "pipe" },
    );
  } catch (err) {
    writeFileSync(out, bytes);
  } finally {
    try {
      require("node:fs").unlinkSync(src);
    } catch {
      /* ignore */
    }
  }
  copyFileSync(out, outFrontend);
  const size = require("node:fs").statSync(out).size;
  return size >= MIN_BYTES ? out : null;
}

function needsSync(tool) {
  if (tool.slug === "onchainai") return false;
  const current = tool.logo_url || "";
  if (current.startsWith("/brand/") && !current.startsWith("/brand/tools/")) return false;
  if (current.startsWith("/brand/tools/") && !FORCE) {
    const path = join(BRAND_DIR, `${tool.slug}.png`);
    if (existsSync(path)) return false;
  }
  return true;
}

async function main() {
  const client = await pgClient();
  const where = ONLY_SLUG
    ? `slug = $1`
    : `status IN ('official', 'verified') AND approval_status = 'approved'`;
  const params = ONLY_SLUG ? [ONLY_SLUG] : [];
  const { rows } = await client.query(
    `SELECT slug, name, status, homepage, repo_url, logo_url
     FROM tools WHERE ${where} ORDER BY status DESC, slug`,
    params,
  );

  const results = [];
  for (const tool of rows) {
    if (!needsSync(tool)) {
      results.push({ slug: tool.slug, action: "skip", reason: "already_branded" });
      continue;
    }
    const candidates = await discoverLogoUrls(tool);
    let saved = null;
    let source = null;
    for (const url of candidates) {
      const bytes = await fetchBytes(url);
      if (!bytes) continue;
      saved = writePng(tool.slug, bytes);
      if (saved) {
        source = url;
        break;
      }
    }
    if (!saved) {
      results.push({ slug: tool.slug, action: "miss", candidates: candidates.slice(0, 6) });
      continue;
    }
    const logoPath = `/brand/tools/${tool.slug}.png`;
    if (APPLY) {
      await client.query(`UPDATE tools SET logo_url = $2 WHERE slug = $1`, [
        tool.slug,
        logoPath,
      ]);
    }
    results.push({
      slug: tool.slug,
      action: APPLY ? "applied" : "dry-run",
      logo_url: logoPath,
      source,
    });
  }
  await client.end();

  const applied = results.filter((r) => r.action === "applied").length;
  const dry = results.filter((r) => r.action === "dry-run").length;
  const miss = results.filter((r) => r.action === "miss").length;
  console.log(
    JSON.stringify(
      { apply: APPLY, applied, dry_run: dry, miss, skip: results.filter((r) => r.action === "skip").length, results },
      null,
      2,
    ),
  );
  if (miss > 0 && APPLY) process.exitCode = 1;
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});