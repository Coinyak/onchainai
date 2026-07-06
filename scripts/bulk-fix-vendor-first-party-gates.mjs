#!/usr/bin/env node
// Bulk public-gate fix for first-party vendor_orgs agent/MCP/skill surfaces.
// Sets approval_status + relevance_status only — never tools.status.
//
// Usage:
//   node scripts/bulk-fix-vendor-first-party-gates.mjs
//   ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/bulk-fix-vendor-first-party-gates.mjs

import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { loadEnv } from "./seed-tool-lib.mjs";
import { loadFirstPartyOrgs } from "./vendor-orgs-lib.mjs";
import { AGENT_SURFACE_SQL } from "./agent-surface-lib.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const FIRST_PARTY_ORGS = Object.keys(loadFirstPartyOrgs()).map((o) => o.toLowerCase());

const SELECT_SQL = `
SELECT slug, name, status, approval_status, relevance_status, repo_url, stars
FROM tools
WHERE status = 'community'
  AND repo_url IS NOT NULL AND repo_url <> ''
  AND lower(split_part(replace(repo_url, 'https://github.com/', ''), '/', 1)) = ANY($1::text[])
  AND ${AGENT_SURFACE_SQL}
  AND (approval_status <> 'approved' OR relevance_status <> 'accepted' OR quarantined_at IS NOT NULL)
ORDER BY stars DESC NULLS LAST;
`;

const UPDATE_SQL = `
UPDATE tools
SET approval_status = 'approved',
    rejection_reason = NULL,
    relevance_status = 'accepted',
    quarantined_at = NULL,
    crypto_relevance_score = GREATEST(crypto_relevance_score, 75),
    crypto_relevance_reasons = CASE
      WHEN crypto_relevance_reasons IS NULL OR cardinality(crypto_relevance_reasons) = 0
      THEN ARRAY['operator-curated: first-party vendor agent surface']::text[]
      ELSE crypto_relevance_reasons || ARRAY['operator-curated: first-party vendor agent surface']::text[]
    END,
    review_policy_version = 'operator-vendor-first-party-gates-v1',
    updated_at = now()
WHERE status = 'community'
  AND repo_url IS NOT NULL AND repo_url <> ''
  AND lower(split_part(replace(repo_url, 'https://github.com/', ''), '/', 1)) = ANY($1::text[])
  AND ${AGENT_SURFACE_SQL}
  AND (approval_status <> 'approved' OR relevance_status <> 'accepted' OR quarantined_at IS NOT NULL)
RETURNING slug, approval_status, relevance_status;
`;

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

async function main() {
  const env = loadEnv();
  const apply = env.SEED_ENV === "prod-curate";
  const DATABASE_URL = env.DATABASE_URL || "";
  if (!DATABASE_URL) {
    console.error("DATABASE_URL missing");
    process.exit(2);
  }

  const pg = require(resolve(ROOT, "scripts/ops/node_modules/pg"));
  const client = new pg.Client({
    connectionString: DATABASE_URL,
    ...(pgSslOption(env, DATABASE_URL) !== undefined
      ? { ssl: pgSslOption(env, DATABASE_URL) }
      : {}),
  });
  await client.connect();

  const preview = await client.query(SELECT_SQL, [FIRST_PARTY_ORGS]);
  if (!apply) {
    console.log(
      JSON.stringify(
        {
          ok: true,
          mode: "dry-run",
          script: "bulk-fix-vendor-first-party-gates.mjs",
          candidate_count: preview.rows.length,
          slugs: preview.rows.map((r) => r.slug),
          apply_hint:
            "ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/bulk-fix-vendor-first-party-gates.mjs",
        },
        null,
        2,
      ),
    );
    await client.end();
    return;
  }

  const updated = await client.query(UPDATE_SQL, [FIRST_PARTY_ORGS]);
  await client.end();
  console.log(
    JSON.stringify(
      {
        ok: true,
        mode: "apply",
        script: "bulk-fix-vendor-first-party-gates.mjs",
        updated_count: updated.rows.length,
        slugs: updated.rows.map((r) => r.slug),
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