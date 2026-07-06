#!/usr/bin/env node
// demote-non-agent-official.mjs — demote official listings lacking agent/MCP surface.
//
// Uses admin demote_official semantics: status official → community + audit row.
// Does NOT clear official_team (parity with review_persistence.rs).
//
// Usage:
//   node scripts/demote-non-agent-official.mjs [--limit N]
//   node scripts/demote-non-agent-official.mjs --slug <slug> [--reason "..."]
//   ENV_FILE=.env DEMOTE_ENV=prod-curate PG_INSECURE_SSL=1 \
//     node scripts/demote-non-agent-official.mjs --apply --i-understand-bulk \
//     --reason "bulk cleanup: non-agent official from vendor_orgs over-promotion"
//
// Modes:
//   default     dry-run JSON lines per slug
//   --apply     write demotions (requires DEMOTE_ENV=prod-curate or SEED_ENV=prod-curate)
//   --slug      single-slug mode (--apply does not need --i-understand-bulk)
//   bulk --apply requires --i-understand-bulk and non-empty --reason

import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { writeFileSync } from "node:fs";
import { loadEnv } from "./seed-tool-lib.mjs";
import { AGENT_SURFACE_SQL, allowlistLower } from "./agent-surface-lib.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const DEFAULT_LIMIT = 2000;
const AUDIT_PREFIX = "demote-non-agent-official.mjs";

const args = process.argv.slice(2);
const APPLY = args.includes("--apply");
const BULK_APPLY = args.includes("--i-understand-bulk");
const EXPORT = args.includes("--export");

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

const limitValue = optionValue("--limit");
const LIMIT =
  limitValue === null
    ? DEFAULT_LIMIT
    : Number.parseInt(limitValue, 10);
if (!Number.isInteger(LIMIT) || LIMIT <= 0) {
  console.error("--limit must be a positive integer");
  process.exit(2);
}
const SLUG = optionValue("--slug");
const REASON = optionValue("--reason") ?? "";

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

function buildCandidateSql(slugFilter) {
  const allow = allowlistLower();
  const base = `
    SELECT id, slug, name, status, official_team, type, function, actor, stars, source
    FROM tools
    WHERE status = 'official'
      AND NOT (${AGENT_SURFACE_SQL})
      AND lower(slug) <> ALL($1::text[])
  `;
  if (slugFilter) {
    return {
      text: `${base} AND lower(slug) = lower($2) ORDER BY stars DESC NULLS LAST`,
      params: [allow, slugFilter],
    };
  }
  return {
    text: `${base} ORDER BY stars DESC NULLS LAST LIMIT $2`,
    params: [allow, LIMIT],
  };
}

function classify(tool, { probeAgentSurface = false } = {}) {
  if (!tool) {
    return { decision: "refuse", reason: "tool not found" };
  }
  if (tool.status !== "official") {
    return {
      decision: "skip_not_official",
      reason: `current status is ${tool.status}, not official`,
    };
  }
  if (allowlistLower().includes(tool.slug.toLowerCase())) {
    return { decision: "skip_allowlist", reason: "operator allowlist" };
  }
  if (probeAgentSurface && tool.is_agent_surface) {
    return {
      decision: "skip_agent_surface",
      reason: "matches agent/MCP/skill surface heuristic",
    };
  }
  return {
    decision: "demote_official",
    reason: `official listing lacks agent/MCP/skill surface (type=${tool.type}, function=${tool.function}, actor=${tool.actor})`,
  };
}

function auditReason(classification) {
  return `${AUDIT_PREFIX}: ${REASON.trim() || classification.reason}`;
}

async function main() {
  const env = loadEnv();
  const prodCurate =
    env.DEMOTE_ENV === "prod-curate" || env.SEED_ENV === "prod-curate";

  if (APPLY && !prodCurate) {
    console.error(
      "refusing --apply without DEMOTE_ENV=prod-curate or SEED_ENV=prod-curate",
    );
    process.exit(2);
  }
  if (APPLY && !SLUG && !BULK_APPLY) {
    console.error(
      "refusing bulk --apply without --i-understand-bulk (use --slug for single apply)",
    );
    process.exit(2);
  }
  if (APPLY && !REASON.trim()) {
    console.error("refusing --apply without non-empty --reason");
    process.exit(2);
  }

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

  let rows;
  if (SLUG) {
    const r = await client.query(
      `SELECT id, slug, name, status, official_team, type, function, actor, stars, source,
              (${AGENT_SURFACE_SQL}) AS is_agent_surface
       FROM tools WHERE lower(slug) = lower($1)`,
      [SLUG],
    );
    rows = r.rows;
  } else {
    const { text, params } = buildCandidateSql(null);
    const result = await client.query(text, params);
    rows = result.rows;
  }

  const stats = { scanned: 0, demote: 0, applied: 0, skipped: 0, refused: 0 };
  const exportSlugs = [];

  for (const tool of rows) {
    stats.scanned++;
    const classification = classify(tool, { probeAgentSurface: Boolean(SLUG) });
    const willDemote = classification.decision === "demote_official";
    const report = {
      slug: tool.slug,
      name: tool.name,
      current_status: tool.status,
      decision: classification.decision,
      before_status: tool.status,
      after_status: willDemote ? "community" : tool.status,
      reason: classification.reason,
      official_team: tool.official_team,
      stars: tool.stars,
      applied: false,
    };

    if (classification.decision === "demote_official") {
      stats.demote++;
      exportSlugs.push(tool.slug);
      if (APPLY) {
        await client.query("BEGIN");
        try {
          const upd = await client.query(
            `UPDATE tools
             SET status = 'community',
                 last_reviewed_at = now(),
                 updated_at = now()
             WHERE id = $1 AND status = 'official'
             RETURNING slug`,
            [tool.id],
          );
          if (upd.rowCount === 1) {
            await client.query(
              `INSERT INTO tool_review_events
                 (tool_id, admin_id, action, reason, before_status, after_status)
               VALUES ($1, NULL, 'demote_official', $2, 'official', 'community')`,
              [tool.id, auditReason(classification)],
            );
            report.applied = true;
            stats.applied++;
          }
          await client.query("COMMIT");
        } catch (e) {
          await client.query("ROLLBACK");
          throw e;
        }
      }
    } else if (classification.decision === "refuse") {
      stats.refused++;
    } else {
      stats.skipped++;
    }

    console.log(JSON.stringify(report));
  }

  if (SLUG && rows.length === 0) {
    const missing = classify(null);
    stats.refused++;
    console.log(
      JSON.stringify({
        slug: SLUG,
        decision: missing.decision,
        reason: missing.reason,
        applied: false,
      }),
    );
  }

  if (EXPORT && exportSlugs.length) {
    const outPath = resolve(ROOT, "fixtures/demote-non-agent-official.json");
    writeFileSync(
      outPath,
      JSON.stringify(
        {
          version: 1,
          generated_at: new Date().toISOString(),
          criteria: "scripts/agent-surface-lib.mjs AGENT_SURFACE_SQL",
          allowlist: allowlistLower(),
          slug_count: exportSlugs.length,
          slugs: exportSlugs.sort(),
        },
        null,
        2,
      ) + "\n",
    );
    console.error(`exported ${exportSlugs.length} slugs → fixtures/demote-non-agent-official.json`);
  }

  await client.end();
  console.error(
    `backend: direct-postgres${APPLY ? " (apply)" : " (dry-run)"} | ` +
      `scanned ${stats.scanned}, demote ${stats.demote}, applied ${stats.applied}, ` +
      `skipped ${stats.skipped}, refused ${stats.refused}`,
  );
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});