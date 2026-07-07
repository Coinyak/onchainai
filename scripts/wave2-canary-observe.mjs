#!/usr/bin/env node
/**
 * W1 canary observation — batch-1 slugs (exa, token-price) health after approval.
 *
 * Checks (no writes):
 *   - Slugs visible in x402 catalog API
 *   - Not quarantined in DB
 *   - Optional: recent x402_probe_history for each slug
 *   - Rubric re-score (dry-run) still passes
 *
 * Usage:
 *   node scripts/wave2-canary-observe.mjs
 *   node scripts/wave2-canary-observe.mjs --slugs exa,token-price
 *
 * Env: DATABASE_URL and/or SUPABASE_* (same as bazaar-approve-rubric.mjs)
 */
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { readFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

function loadPg() {
  try {
    return require(resolve(ROOT, "scripts/ops/node_modules/pg"));
  } catch {
    try {
      return require("pg");
    } catch {
      return null;
    }
  }
}

function pgSslOption(databaseUrl) {
  if (process.env.PG_INSECURE_SSL === "1") return { rejectUnauthorized: false };
  if (/sslmode=require|supabase\.co/i.test(databaseUrl)) return { rejectUnauthorized: false };
  return undefined;
}
const API_URL = (
  process.env.ONCHAINAI_API_URL ||
  process.env.RAILWAY_API_URL ||
  "https://onchainai-production.up.railway.app"
).replace(/\/$/, "");

function loadDotEnv() {
  const envPath = resolve(ROOT, ".env");
  if (!existsSync(envPath)) return;
  for (const raw of readFileSync(envPath, "utf8").split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith("#") || !line.includes("=")) continue;
    const i = line.indexOf("=");
    const key = line.slice(0, i).trim();
    let val = line.slice(i + 1).trim();
    if (
      (val.startsWith('"') && val.endsWith('"')) ||
      (val.startsWith("'") && val.endsWith("'"))
    ) {
      val = val.slice(1, -1);
    }
    if (key && process.env[key] === undefined) process.env[key] = val;
  }
}

function parseArgs() {
  const argv = process.argv.slice(2);
  const slugsIdx = argv.indexOf("--slugs");
  if (slugsIdx === -1) {
    return { slugs: ["exa", "token-price"] };
  }
  const raw = argv[slugsIdx + 1];
  if (!raw || raw.startsWith("-")) {
    console.error("usage: node scripts/wave2-canary-observe.mjs [--slugs slug1,slug2]");
    process.exit(2);
  }
  const slugs = raw.split(",").map((s) => s.trim()).filter(Boolean);
  if (slugs.length < 1) {
    console.error("usage: --slugs requires at least one slug");
    process.exit(2);
  }
  return { slugs };
}

async function fetchX402Slugs() {
  const res = await fetch(`${API_URL}/api/v2/tools/list`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      sort: "new",
      offset: 0,
      limit: 100,
      filters: { tool_type: ["x402"] },
    }),
  });
  if (!res.ok) throw new Error(`tools/list x402 HTTP ${res.status}`);
  const items = await res.json();
  return new Set(items.map((t) => t.slug));
}

function rubricDryRun(slug) {
  const r = spawnSync("node", ["scripts/bazaar-approve-rubric.mjs", slug], {
    cwd: ROOT,
    env: { ...process.env },
    encoding: "utf8",
  });
  const out = (r.stdout || "") + (r.stderr || "");
  const pass = out.includes('"pass":true') || out.includes('"pass": true');
  const score = out.match(/"score":\s*(\d+)/)?.[1] ?? "?";
  return { pass, score, exit: r.status, snippet: out.slice(0, 400) };
}

async function dbCanaryRows(slugs) {
  if (!process.env.DATABASE_URL) return null;
  const pg = loadPg();
  if (!pg) return { error: "pg module missing — npm install --prefix scripts/ops" };
  const ssl = pgSslOption(process.env.DATABASE_URL);
  const client = new pg.Client({
    connectionString: process.env.DATABASE_URL,
    ...(ssl !== undefined ? { ssl } : {}),
  });
  await client.connect();
  const { rows } = await client.query(
    `SELECT slug, approval_status, relevance_status, referral_enabled,
            quarantined_at IS NOT NULL AS quarantined,
            x402_last_checked_at
     FROM tools WHERE slug = ANY($1::text[])`,
    [slugs],
  );
  const probeRows = await client.query(
    `SELECT t.slug, h.status, h.probed_at
     FROM x402_probe_history h
     JOIN tools t ON t.id = h.tool_id
     WHERE t.slug = ANY($1::text[])
     ORDER BY h.probed_at DESC
     LIMIT 20`,
    [slugs],
  );
  await client.end();
  return { tools: rows, probes: probeRows.rows };
}

async function main() {
  loadDotEnv();
  const { slugs } = parseArgs();
  const report = { at: new Date().toISOString(), slugs, checks: [] };
  let fail = 0;

  const catalog = await fetchX402Slugs();
  for (const slug of slugs) {
    const visible = catalog.has(slug);
    if (!visible) fail++;
    report.checks.push({ slug, catalog_visible: visible });
  }

  const db = await dbCanaryRows(slugs);
  if (db?.tools) {
    for (const row of db.tools) {
      const check = report.checks.find((c) => c.slug === row.slug);
      if (!check) continue;
      Object.assign(check, {
        approval_status: row.approval_status,
        relevance_status: row.relevance_status,
        referral_enabled: row.referral_enabled,
        quarantined: row.quarantined,
        x402_last_checked_at: row.x402_last_checked_at,
      });
      if (row.quarantined || row.referral_enabled) fail++;
      if (row.approval_status !== "approved" || row.relevance_status !== "accepted") fail++;
    }
    report.recent_probes = db.probes;
  } else if (db?.error) {
    report.db_note = db.error;
  }

  for (const slug of slugs) {
    const rubric = rubricDryRun(slug);
    const check = report.checks.find((c) => c.slug === slug);
    if (check) {
      check.rubric_pass = rubric.pass;
      check.rubric_score = rubric.score;
      if (!rubric.pass) fail++;
    }
  }

  report.ok = fail === 0;
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) {
    console.error(`WAVE2 CANARY OBSERVE FAIL (${fail} issue(s))`);
    process.exit(1);
  }
  console.error("WAVE2 CANARY OBSERVE PASS");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});