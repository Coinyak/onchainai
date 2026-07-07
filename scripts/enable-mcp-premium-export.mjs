#!/usr/bin/env node
/**
 * Enable Axis B MCP premium for export_toolkit (site_settings).
 *
 * Sets mcp_premium_enabled + pay_to + $0.01/call on Base mainnet, aligned with
 * X402_PAY_TO_ADDRESS / default_referral_payout_address.
 *
 * Usage:
 *   node scripts/enable-mcp-premium-export.mjs
 *   node scripts/enable-mcp-premium-export.mjs --apply
 *
 * Env: DATABASE_URL (required for --apply)
 */
import { createRequire } from "node:module";
import { readFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

const DEFAULT_PAY_TO = "0x2af05c1661da38a2919dc27b4c8b71cb91c30017";
const PREMIUM_PRICE = process.env.MCP_PREMIUM_PRICE ?? "$0.01";
const PREMIUM_DISPLAY = process.env.MCP_PREMIUM_DISPLAY_PRICE ?? "$0.01/call";
const PREMIUM_NETWORK = process.env.MCP_PREMIUM_NETWORK ?? "eip155:8453";

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

async function main() {
  loadDotEnv();
  const apply = process.argv.includes("--apply");
  // Prefer explicit MCP_PREMIUM_PAY_TO; avoid stale local X402_PAY_TO_ADDRESS in .env.
  const payTo = process.env.MCP_PREMIUM_PAY_TO?.trim() || DEFAULT_PAY_TO;

  const pg = loadPg();
  if (!pg) {
    console.error("pg module not found (scripts/ops/node_modules/pg)");
    process.exit(1);
  }
  if (!apply) {
    console.log(
      JSON.stringify(
        {
          mode: "dry-run",
          planned: {
            mcp_premium_enabled: true,
            mcp_premium_pay_to_address: payTo,
            mcp_premium_price: PREMIUM_PRICE,
            mcp_premium_network: PREMIUM_NETWORK,
            mcp_premium_display_price: PREMIUM_DISPLAY,
          },
        },
        null,
        2,
      ),
    );
    console.log("\nDry-run only. Re-run with --apply to write (requires DATABASE_URL).");
    return;
  }

  if (!process.env.DATABASE_URL) {
    console.error("DATABASE_URL required for --apply");
    process.exit(1);
  }

  const ssl = pgSslOption(process.env.DATABASE_URL);
  const client = new pg.Client({
    connectionString: process.env.DATABASE_URL,
    ...(ssl !== undefined ? { ssl } : {}),
  });
  await client.connect();

  const before = await client.query(
    `SELECT mcp_premium_enabled, mcp_premium_pay_to_address, mcp_premium_price,
            mcp_premium_network, mcp_premium_display_price
     FROM site_settings WHERE id = 1`,
  );
  const row = before.rows[0];
  if (!row) {
    console.error("site_settings id=1 missing");
    process.exit(1);
  }

  const planned = {
    mcp_premium_enabled: true,
    mcp_premium_pay_to_address: payTo,
    mcp_premium_price: PREMIUM_PRICE,
    mcp_premium_network: PREMIUM_NETWORK,
    mcp_premium_display_price: PREMIUM_DISPLAY,
  };

  console.log(JSON.stringify({ mode: "apply", before: row, planned }, null, 2));

  const after = await client.query(
    `UPDATE site_settings SET
       mcp_premium_enabled = true,
       mcp_premium_pay_to_address = $1,
       mcp_premium_price = $2,
       mcp_premium_network = $3,
       mcp_premium_display_price = $4,
       updated_at = now()
     WHERE id = 1
     RETURNING mcp_premium_enabled, mcp_premium_pay_to_address, mcp_premium_price,
               mcp_premium_network, mcp_premium_display_price`,
    [payTo, PREMIUM_PRICE, PREMIUM_NETWORK, PREMIUM_DISPLAY],
  );
  console.log(JSON.stringify({ applied: after.rows[0] }, null, 2));
  await client.end();
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});