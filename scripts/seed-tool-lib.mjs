import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

export function parseEnvFile(path) {
  const out = {};
  try {
    for (const raw of readFileSync(path, "utf8").split("\n")) {
      const line = raw.trim();
      if (!line || line.startsWith("#")) continue;
      const eq = line.indexOf("=");
      if (eq <= 0) continue;
      const key = line.slice(0, eq).trim();
      let value = line.slice(eq + 1);
      const hash = value.search(/\s+#/);
      if (hash >= 0) value = value.slice(0, hash);
      value = value.trim().replace(/^["']|["']$/g, "");
      if (key) out[key] = value;
    }
  } catch {
    /* optional */
  }
  return out;
}

export function loadEnv() {
  return {
    ...parseEnvFile(process.env.ENV_FILE || resolve(ROOT, ".env")),
    ...process.env,
  };
}

export function tool(row) {
  return {
    asset_class: "crypto",
    actor: row.actor ?? "ai-agent",
    repo_url: row.repo_url ?? null,
    npm_package: row.npm_package ?? null,
    install_command: row.install_command ?? null,
    mcp_endpoint: row.mcp_endpoint ?? null,
    chains: row.chains ?? [],
    stars: row.stars ?? 0,
    license: row.license ?? null,
    source: row.source ?? "aggregator",
    crypto_relevance_score: row.crypto_relevance_score ?? 80,
    crypto_relevance_reasons: row.crypto_relevance_reasons ?? [
      "aggregator-discovery: official exchange/ecosystem evidence",
      "operator-curated tool for onchain agents",
    ],
    relevance_status: "accepted",
    install_risk_level: row.install_risk_level ?? "low",
    install_risk_reasons: row.install_risk_reasons ?? [
      row.install_command ? "documented package manager install" : "HTTP API surface",
    ],
    requires_secret: row.requires_secret ?? false,
    ...row,
  };
}

const UPSERT_SQL = `
INSERT INTO tools (
  name, slug, description, function, asset_class, actor, type,
  repo_url, homepage, npm_package, install_command, mcp_endpoint,
  chains, status, approval_status, rejection_reason,
  crypto_relevance_score, crypto_relevance_reasons, relevance_status,
  install_risk_level, install_risk_reasons, requires_secret,
  license, pricing, stars, source, review_policy_version,
  created_at, updated_at
) VALUES (
  $1, $2, $3, $4, $5, $6, $7,
  $8, $9, $10, $11, $12,
  $13, 'community', 'approved', NULL,
  $14, $15, $16,
  $17, $18, $19,
  $20, 'free', $21, $22, 'operator-aggregator-curate-v1',
  now(), now()
)
ON CONFLICT (slug) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  function = EXCLUDED.function,
  asset_class = EXCLUDED.asset_class,
  actor = EXCLUDED.actor,
  type = EXCLUDED.type,
  repo_url = EXCLUDED.repo_url,
  homepage = EXCLUDED.homepage,
  npm_package = EXCLUDED.npm_package,
  install_command = EXCLUDED.install_command,
  mcp_endpoint = EXCLUDED.mcp_endpoint,
  chains = EXCLUDED.chains,
  crypto_relevance_score = EXCLUDED.crypto_relevance_score,
  crypto_relevance_reasons = EXCLUDED.crypto_relevance_reasons,
  relevance_status = EXCLUDED.relevance_status,
  install_risk_level = EXCLUDED.install_risk_level,
  install_risk_reasons = EXCLUDED.install_risk_reasons,
  requires_secret = EXCLUDED.requires_secret,
  license = EXCLUDED.license,
  stars = GREATEST(tools.stars, EXCLUDED.stars),
  source = EXCLUDED.source,
  updated_at = now()
RETURNING slug, (xmax = 0) AS inserted;
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

export async function runSeed(tools, scriptName) {
  const env = loadEnv();
  const apply = env.SEED_ENV === "prod-curate";
  if (!apply) {
    console.log(
      JSON.stringify(
        {
          ok: true,
          mode: "dry-run",
          script: scriptName,
          tool_count: tools.length,
          slugs: tools.map((t) => t.slug),
          apply_hint: `ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/${scriptName}`,
        },
        null,
        2,
      ),
    );
    return;
  }
  const DATABASE_URL = env.DATABASE_URL || "";
  if (!DATABASE_URL) {
    console.error("DATABASE_URL missing");
    process.exit(2);
  }
  const require = createRequire(import.meta.url);
  const pg = require(resolve(ROOT, "scripts/ops/node_modules/pg"));
  const ssl = pgSslOption(env, DATABASE_URL);
  const client = new pg.Client({
    connectionString: DATABASE_URL,
    ...(ssl !== undefined ? { ssl } : {}),
  });
  await client.connect();
  const results = [];
  for (const t of tools) {
    const r = await client.query(UPSERT_SQL, [
      t.name,
      t.slug,
      t.description,
      t.function,
      t.asset_class,
      t.actor,
      t.type,
      t.repo_url,
      t.homepage,
      t.npm_package,
      t.install_command,
      t.mcp_endpoint,
      t.chains,
      t.crypto_relevance_score,
      t.crypto_relevance_reasons,
      t.relevance_status,
      t.install_risk_level,
      t.install_risk_reasons,
      t.requires_secret,
      t.license,
      t.stars,
      t.source,
    ]);
    results.push({
      slug: t.slug,
      action: r.rows[0].inserted ? "inserted" : "updated",
    });
  }
  await client.end();
  console.log(JSON.stringify({ ok: true, mode: "apply", script: scriptName, tools: results }, null, 2));
}