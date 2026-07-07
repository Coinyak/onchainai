#!/usr/bin/env node
// Apply verify-tool-official.mjs --apply for gate-passing first-party agent-surface tools.
//
// Default: dry-run. Apply requires explicit flags + prod-curate env.
//
// Usage:
//   node scripts/bulk-verify-first-party-apply.mjs
//   node scripts/bulk-verify-first-party-apply.mjs --apply --i-understand-bulk --batch 40
//   ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 \
//     node scripts/bulk-verify-first-party-apply.mjs --apply --i-understand-bulk
//   node scripts/bulk-verify-first-party-apply.mjs --include-non-agent   # widen SQL filter

import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { loadEnv } from "./seed-tool-lib.mjs";
import { loadFirstPartyOrgs } from "./vendor-orgs-lib.mjs";
import { AGENT_SURFACE_SQL } from "./agent-surface-lib.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const FIRST_PARTY_ORGS = Object.keys(loadFirstPartyOrgs()).map((o) => o.toLowerCase());
const args = process.argv.slice(2);
const APPLY = args.includes("--apply");
const BULK_APPLY = args.includes("--i-understand-bulk");
const INCLUDE_NON_AGENT = args.includes("--include-non-agent");
const SPAWN_TIMEOUT_MS = 5 * 60 * 1000;

function optionValue(flag) {
  const idx = args.indexOf(flag);
  if (idx < 0) return null;
  const value = args[idx + 1];
  if (!value || value.startsWith("--")) {
    console.error(`missing value for ${flag}`);
    process.exit(2);
  }
  return value;
}

const batchValue = optionValue("--batch");
const rawBatch = batchValue === null ? NaN : Number.parseInt(batchValue, 10);
const BATCH = Number.isInteger(rawBatch) && rawBatch > 0 ? rawBatch : 40;

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

const GITHUB_ORG_SQL = `lower(split_part(regexp_replace(repo_url, '^https?://(www\\.)?github\\.com/', ''), '/', 1))`;

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
  try {
    const agentFilter = INCLUDE_NON_AGENT ? "TRUE" : AGENT_SURFACE_SQL;
    const r = await client.query(
      `SELECT slug FROM tools
       WHERE status = 'community'
         AND approval_status = 'approved' AND relevance_status = 'accepted'
         AND quarantined_at IS NULL
         AND (install_risk_level IS NULL OR install_risk_level NOT IN ('critical', 'high'))
         AND repo_url IS NOT NULL
         AND ${GITHUB_ORG_SQL} = ANY($1::text[])
         AND (${agentFilter})
       ORDER BY stars DESC NULLS LAST`,
      [FIRST_PARTY_ORGS],
    );
    return r.rows.map((row) => row.slug);
  } finally {
    await client.end();
  }
}

async function main() {
  const env = loadEnv();
  const prodCurate =
    env.SEED_ENV === "prod-curate" || env.DEMOTE_ENV === "prod-curate";

  if (APPLY && !prodCurate) {
    console.error(
      "refusing --apply without SEED_ENV=prod-curate or DEMOTE_ENV=prod-curate",
    );
    process.exit(2);
  }
  if (APPLY && !BULK_APPLY) {
    console.error("refusing --apply without --i-understand-bulk");
    process.exit(2);
  }
  if (APPLY && process.env.PG_INSECURE_SSL === "1") {
    console.error(
      "warning: PG_INSECURE_SSL=1 disables Postgres TLS certificate verification",
    );
  }

  const slugs = await fetchSlugs();
  const mode = APPLY ? "apply" : "dry-run";
  console.error(
    `first-party agent-surface slugs: ${slugs.length} (${mode}, batch=${BATCH}` +
      `${INCLUDE_NON_AGENT ? ", include-non-agent" : ""})`,
  );

  let appliedOfficial = 0;
  let appliedVerified = 0;
  let noops = 0;
  let community = 0;

  for (let i = 0; i < slugs.length; i += BATCH) {
    const chunk = slugs.slice(i, i + BATCH);
    const verifyArgs = [
      resolve(ROOT, "scripts/verify-tool-official.mjs"),
      ...chunk,
      ...(APPLY ? ["--apply", "--i-understand-bulk"] : []),
    ];
    const run = spawnSync(process.execPath, verifyArgs, {
      env: process.env,
      encoding: "utf8",
      maxBuffer: 50 * 1024 * 1024,
      timeout: SPAWN_TIMEOUT_MS,
    });
    if (run.error) {
      console.error(
        `batch ${Math.floor(i / BATCH) + 1} failed: ${run.error.message}`,
      );
      process.exit(1);
    }
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
        mode,
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
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});