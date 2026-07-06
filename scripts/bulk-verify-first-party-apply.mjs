#!/usr/bin/env node
// Apply verify-tool-official.mjs --apply for all gate-passing first-party community tools.
//
// Usage:
//   PG_INSECURE_SSL=1 node scripts/bulk-verify-first-party-apply.mjs [--dry-run] [--batch N]

import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { loadEnv } from "./seed-tool-lib.mjs";
import { loadFirstPartyOrgs } from "./vendor-orgs-lib.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const FIRST_PARTY_ORGS = Object.keys(loadFirstPartyOrgs()).map((o) => o.toLowerCase());
const DRY = process.argv.includes("--dry-run");
const batchIdx = process.argv.indexOf("--batch");
const BATCH = batchIdx >= 0 ? Number(process.argv[batchIdx + 1]) || 40 : 40;

function pgSslOption(env, databaseUrl) {
  const mode = (env.PGSSLMODE || "").toLowerCase();
  const wantsSsl =
    mode === "require" ||
    /supabase\.(co|com)/i.test(databaseUrl) ||
    databaseUrl.includes("sslmode=require");
  if (!wantsSsl) return undefined;
  if (env.PG_INSECURE_SSL === "1") return { rejectUnauthorized: false };
  return true;
}

async function fetchSlugs() {
  const env = loadEnv();
  const DATABASE_URL = env.DATABASE_URL || "";
  const pg = require(resolve(ROOT, "scripts/ops/node_modules/pg"));
  const client = new pg.Client({
    connectionString: DATABASE_URL,
    ...(pgSslOption(env, DATABASE_URL) !== undefined
      ? { ssl: pgSslOption(env, DATABASE_URL) }
      : {}),
  });
  await client.connect();
  const r = await client.query(
    `SELECT slug FROM tools
     WHERE status = 'community'
       AND approval_status = 'approved' AND relevance_status = 'accepted'
       AND quarantined_at IS NULL
       AND install_risk_level NOT IN ('critical', 'high')
       AND repo_url IS NOT NULL
       AND lower(split_part(replace(repo_url, 'https://github.com/', ''), '/', 1)) = ANY($1::text[])
     ORDER BY stars DESC NULLS LAST`,
    [FIRST_PARTY_ORGS],
  );
  await client.end();
  return r.rows.map((row) => row.slug);
}

const slugs = await fetchSlugs();
console.error(`first-party community slugs: ${slugs.length} (${DRY ? "dry-run" : "apply"}, batch=${BATCH})`);

let appliedOfficial = 0;
let appliedVerified = 0;
let noops = 0;
let community = 0;

for (let i = 0; i < slugs.length; i += BATCH) {
  const chunk = slugs.slice(i, i + BATCH);
  const args = [
    resolve(ROOT, "scripts/verify-tool-official.mjs"),
    ...chunk,
    ...(DRY ? [] : ["--apply"]),
  ];
  const env = { ...process.env, PG_INSECURE_SSL: process.env.PG_INSECURE_SSL || "1" };
  const run = spawnSync(process.execPath, args, { env, encoding: "utf8", maxBuffer: 50 * 1024 * 1024 });
  if (run.status !== 0) {
    console.error(run.stderr || run.stdout);
    process.exit(run.status || 1);
  }
  for (const line of (run.stdout || "").split("\n").filter(Boolean)) {
    try {
      const row = JSON.parse(line);
      if (row.applied && row.decision === "official") appliedOfficial++;
      else if (row.applied && row.decision === "verified") appliedVerified++;
      else if (row.decision === "official" || row.decision === "verified") noops++;
      else if (row.decision === "community") community++;
    } catch {
      /* stderr summary lines */
    }
  }
  console.error(`batch ${Math.floor(i / BATCH) + 1}/${Math.ceil(slugs.length / BATCH)} done`);
}

console.log(
  JSON.stringify(
    {
      ok: true,
      mode: DRY ? "dry-run" : "apply",
      slug_count: slugs.length,
      applied_official: appliedOfficial,
      applied_verified: appliedVerified,
      noops_already_elevated: noops,
      remained_community: community,
    },
    null,
    2,
  ),
);