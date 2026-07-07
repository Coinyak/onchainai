#!/usr/bin/env node
/**
 * W2 guard — enable allow_x402_registration only after L4 probe history is live.
 *
 * Preconditions (all required):
 *   - x402_probe_history has rows with tool_id (scheduled L4 writes)
 *   - At least one cron-eligible x402 tool has x402_last_checked_at set
 *   - wave2-canary-observe passes for batch-1 slugs (default exa, token-price)
 *
 * Usage:
 *   node scripts/enable-w2-guard.mjs              # dry-run
 *   node scripts/enable-w2-guard.mjs --apply --i-understand-w2
 *
 * Env: DATABASE_URL (required for apply)
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

function pgClient() {
  const pg = loadPg();
  if (!pg || !process.env.DATABASE_URL) return null;
  const ssl = pgSslOption(process.env.DATABASE_URL);
  return new pg.Client({
    connectionString: process.env.DATABASE_URL,
    ...(ssl !== undefined ? { ssl } : {}),
  });
}

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

async function l4Ready() {
  if (!process.env.DATABASE_URL) {
    return { ok: false, reason: "DATABASE_URL unset" };
  }
  const client = pgClient();
  if (!client) return { ok: false, reason: "pg module missing — npm install --prefix scripts/ops" };
  await client.connect();
  const scheduled = (
    await client.query(
      "SELECT COUNT(*)::int AS n FROM x402_probe_history WHERE tool_id IS NOT NULL",
    )
  ).rows[0].n;
  const checked = (
    await client.query(
      `SELECT COUNT(*)::int AS n FROM tools
       WHERE pricing = 'x402' AND x402_endpoint IS NOT NULL AND trim(x402_endpoint) <> ''
         AND approval_status = 'approved' AND relevance_status = 'accepted'
         AND quarantined_at IS NULL AND x402_last_checked_at IS NOT NULL`,
    )
  ).rows[0].n;
  const current = (
    await client.query(
      "SELECT allow_x402_registration FROM site_settings LIMIT 1",
    )
  ).rows[0]?.allow_x402_registration;
  await client.end();
  if (scheduled < 1) {
    return { ok: false, reason: "no scheduled probe_history rows (L4 cron not run yet)", scheduled, checked, current };
  }
  if (checked < 1) {
    return { ok: false, reason: "no x402 tools with x402_last_checked_at", scheduled, checked, current };
  }
  return { ok: true, scheduled, checked, current };
}

function canaryObserve() {
  const r = spawnSync("node", ["scripts/wave2-canary-observe.mjs"], {
    cwd: ROOT,
    env: { ...process.env },
    encoding: "utf8",
  });
  return r.status === 0;
}

async function applyW2() {
  const client = pgClient();
  if (!client) throw new Error("pg module missing");
  await client.connect();
  await client.query(
    "UPDATE site_settings SET allow_x402_registration = true, updated_at = NOW()",
  );
  const after = (
    await client.query(
      "SELECT allow_x402_registration FROM site_settings LIMIT 1",
    )
  ).rows[0].allow_x402_registration;
  await client.end();
  return after;
}

async function main() {
  loadDotEnv();
  const apply =
    process.argv.includes("--apply") && process.argv.includes("--i-understand-w2");

  const l4 = await l4Ready();
  const canary = canaryObserve();
  const report = { l4, canary_ok: canary, apply_requested: apply };

  if (!l4.ok) {
    console.log(JSON.stringify({ ...report, action: "blocked", reason: l4.reason }, null, 2));
    console.error(`W2 GUARD BLOCKED: ${l4.reason}`);
    process.exit(1);
  }
  if (!canary) {
    console.log(JSON.stringify({ ...report, action: "blocked", reason: "canary observe failed" }, null, 2));
    console.error("W2 GUARD BLOCKED: canary observe failed");
    process.exit(1);
  }

  if (l4.current === true) {
    console.log(JSON.stringify({ ...report, action: "noop", allow_x402_registration: true }, null, 2));
    console.error("W2 GUARD PASS (already enabled)");
    return;
  }

  if (!apply) {
    console.log(
      JSON.stringify(
        {
          ...report,
          action: "dry-run",
          would_set: { allow_x402_registration: true },
          hint: "node scripts/enable-w2-guard.mjs --apply --i-understand-w2",
        },
        null,
        2,
      ),
    );
    console.error("W2 GUARD READY (dry-run — L4 + canary OK, run --apply to enable)");
    return;
  }

  const after = await applyW2();
  console.log(JSON.stringify({ ...report, action: "applied", allow_x402_registration: after }, null, 2));
  console.error("W2 GUARD APPLIED: allow_x402_registration=true");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});