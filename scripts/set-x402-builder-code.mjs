#!/usr/bin/env node
// Set site_settings.x402_builder_code (Base Builder Code from dashboard.base.org).
//
// Usage:
//   BUILDER_CODE=bc_ljttbnhv node scripts/set-x402-builder-code.mjs
//   ENV_FILE=.env SEED_ENV=prod-curate BUILDER_CODE=bc_ljttbnhv node scripts/set-x402-builder-code.mjs

import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { loadEnv } from "./seed-tool-lib.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const CODE = (process.env.BUILDER_CODE || "").trim();

if (!/^bc_[a-z0-9_]{1,28}$/.test(CODE)) {
  console.error("BUILDER_CODE must match bc_[a-z0-9_]{1,28} (Base Builder Code)");
  process.exit(2);
}

const env = loadEnv();
if (env.SEED_ENV !== "prod-curate") {
  console.log(
    JSON.stringify(
      {
        ok: true,
        mode: "dry-run",
        builder_code: CODE,
        apply_hint: `ENV_FILE=.env SEED_ENV=prod-curate BUILDER_CODE=${CODE} node scripts/set-x402-builder-code.mjs`,
      },
      null,
      2,
    ),
  );
  process.exit(0);
}

const DATABASE_URL = env.DATABASE_URL || "";
if (!DATABASE_URL) {
  console.error("DATABASE_URL missing");
  process.exit(2);
}

function pgSslOption(env, databaseUrl) {
  const wantsSsl =
    /supabase\.(co|com)/i.test(databaseUrl) || databaseUrl.includes("sslmode=require");
  if (!wantsSsl) return undefined;
  if (env.PG_INSECURE_SSL === "1") return { rejectUnauthorized: false };
  return true;
}

const pg = require(resolve(ROOT, "scripts/ops/node_modules/pg"));
const client = new pg.Client({
  connectionString: DATABASE_URL,
  ...(pgSslOption(env, DATABASE_URL) !== undefined
    ? { ssl: pgSslOption(env, DATABASE_URL) }
    : {}),
});
await client.connect();
const r = await client.query(
  `UPDATE site_settings SET x402_builder_code = $1, updated_at = now() WHERE id = 1 RETURNING x402_builder_code`,
  [CODE],
);
const tool = await client.query(
  `UPDATE tools SET x402_builder_code = $1, updated_at = now() WHERE slug = 'onchainai' RETURNING slug`,
  [CODE],
);
await client.end();
console.log(
  JSON.stringify(
    {
      ok: true,
      site_settings: r.rows[0]?.x402_builder_code,
      tool_slug: tool.rows[0]?.slug ?? null,
    },
    null,
    2,
  ),
);