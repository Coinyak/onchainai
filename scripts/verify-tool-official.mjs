#!/usr/bin/env node
// verify-tool-official.mjs — evidence-based auto verification of tools.status.
//
// Any operator or AI agent can run this when the owner asks "verify/official <tool>".
// It collects public evidence (GitHub org, npm scope, homepage domain), decides
// community | verified | official, and with --apply writes the new status plus a
// tool_review_events audit row. Deny-by-default: no evidence, no change.
//
// Usage:
//   node scripts/verify-tool-official.mjs <slug> [<slug>...] [--apply]
//   node scripts/verify-tool-official.mjs --scan [--apply --i-understand-bulk] [--limit N]
//   node scripts/verify-tool-official.mjs <slug> --expect-org <org> [--apply]
//
// Modes:
//   default   dry-run: prints one JSON report line per slug, writes nothing
//   --apply   apply qualifying status changes (never downgrades)
//   --scan    sweep public tools whose GitHub org is in the first-party map or
//             whose name/slug matches PLATFORM_KEYWORDS; report candidates
//
// Env (repo root .env, or ENV_FILE=/path/to/.env):
//   SUPABASE_URL + SUPABASE_SERVICE_KEY (REST backend) and/or DATABASE_URL
//   (direct Postgres fallback), GITHUB_API_TOKEN (optional, rate limit).
//
// DB backends: tries Supabase REST first; if PostgREST is unavailable (e.g.
// PGRST002 schema-cache 503) it falls back to direct Postgres via the `pg`
// driver. One-time bootstrap for the fallback:
//   npm install --prefix scripts/ops
// On direct connect it also fires `NOTIFY pgrst, 'reload schema'` to help the
// REST layer heal itself.
//
// Rules (see docs/OPERATOR_GUIDE.md — 자동 검증 하네스):
//   official  repo org is a curated first-party org (FIRST_PARTY_ORGS), or the
//             GitHub org is domain-verified AND its site matches tool homepage
//   verified  repo exists + three-way identity cluster (github org + npm scope +
//             homepage label) — mirrors src/trust_verification.rs
//   never     elevate tools failing the public gate (approval/relevance/risk/
//             quarantine); never downgrade; never touch x402 payment flags.

import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

// Curated first-party GitHub orgs → official team label. Extend via PR; keep
// entries defensible (vendor-controlled org, not fan/community orgs).
const FIRST_PARTY_ORGS = {
  github: "GitHub",
  xdevplatform: "X Developer Platform",
  twitterdev: "X Developer Platform",
  modelcontextprotocol: "Model Context Protocol",
  anthropics: "Anthropic",
  openai: "OpenAI",
  "google-gemini": "Google Gemini",
  coinbase: "Coinbase",
  base: "Base",
  "base-org": "Base",
  "solana-labs": "Solana Labs",
  "solana-foundation": "Solana Foundation",
  "anza-xyz": "Anza (Solana)",
  ethereum: "Ethereum Foundation",
  uniswap: "Uniswap Labs",
  aave: "Aave",
  "aave-dao": "Aave",
  smartcontractkit: "Chainlink",
  "wormhole-foundation": "Wormhole",
  "layerzero-labs": "LayerZero",
  alchemyplatform: "Alchemy",
  "thirdweb-dev": "thirdweb",
  crossmint: "Crossmint",
  "goat-sdk": "Crossmint GOAT",
  farcasterxyz: "Farcaster",
  neynarhq: "Neynar",
  neynarxyz: "Neynar",
  discord: "Discord",
  grammyjs: "grammY",
  metamask: "MetaMask",
  consensys: "Consensys",
  "safe-global": "Safe",
  walletconnect: "WalletConnect",
  "reown-com": "Reown (WalletConnect)",
  projectopensea: "OpenSea",
  "foundry-rs": "Foundry",
  paradigmxyz: "Paradigm",
  wevm: "wevm (wagmi/viem)",
  "bob-collective": "BOB",
  "ton-blockchain": "TON",
  tronprotocol: "TRON",
  "input-output-hk": "IOG (Cardano)",
  chainstacklabs: "Chainstack",
  quicknode: "QuickNode",
  graphprotocol: "The Graph",
};

// Narrow scan keywords — org hits in FIRST_PARTY_ORGS are the primary signal.
const PLATFORM_KEYWORDS = [
  "github-mcp", "farcaster", "neynar", "discord-interactions", "telegram",
  "x-api", "twitter-api", "xdevplatform",
];

function parseEnvFile(path) {
  const out = {};
  let text;
  try {
    text = readFileSync(path, "utf8");
  } catch {
    return out;
  }
  for (const raw of text.split("\n")) {
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
  return out;
}

const env = {
  ...parseEnvFile(process.env.ENV_FILE || resolve(ROOT, ".env")),
  ...process.env,
};

const SUPABASE_URL = (env.SUPABASE_URL || "").replace(/\/$/, "");
const SERVICE_KEY = env.SUPABASE_SERVICE_KEY || "";
const DATABASE_URL = env.DATABASE_URL || "";
const GITHUB_TOKEN = env.GITHUB_API_TOKEN || "";

/** Verified TLS by default; set PG_INSECURE_SSL=1 only on trusted dev networks. */
function pgSslOption(databaseUrl) {
  const mode = (env.PGSSLMODE || "").toLowerCase();
  const wantsSsl =
    mode === "require" ||
    mode === "verify-ca" ||
    mode === "verify-full" ||
    /supabase\.(co|com)/i.test(databaseUrl) ||
    databaseUrl.includes("sslmode=require");
  if (!wantsSsl) return undefined;
  if (env.PG_INSECURE_SSL === "1") {
    return { rejectUnauthorized: false };
  }
  return true;
}

const args = process.argv.slice(2);
const APPLY = args.includes("--apply");
const SCAN = args.includes("--scan");
const BULK_APPLY = args.includes("--i-understand-bulk");
const LIMIT = Number(args[args.indexOf("--limit") + 1]) || 500;
const expectOrgIdx = args.indexOf("--expect-org");
const EXPECT_ORG = expectOrgIdx >= 0 ? args[expectOrgIdx + 1] : null;
const slugs = args.filter(
  (a, i) =>
    !a.startsWith("--") &&
    args[i - 1] !== "--expect-org" &&
    args[i - 1] !== "--limit",
);

const TOOL_COLUMNS = [
  "id", "slug", "name", "status", "official_team", "repo_url", "homepage",
  "npm_package", "approval_status", "relevance_status", "install_risk_level",
  "quarantined_at", "stars",
];

// --- DB backends: Supabase REST first, direct Postgres fallback --------------

async function restFetch(path, init = {}) {
  const res = await fetch(`${SUPABASE_URL}/rest/v1/${path}`, {
    ...init,
    headers: {
      apikey: SERVICE_KEY,
      Authorization: `Bearer ${SERVICE_KEY}`,
      "Content-Type": "application/json",
      ...init.headers,
    },
  });
  if (!res.ok) {
    throw new Error(`postgrest ${res.status} ${path}: ${await res.text()}`);
  }
  return res.status === 204 ? null : res.json();
}

const restBackend = {
  name: "supabase-rest",
  async fetchTool(slug) {
    const rows = await restFetch(
      `tools?slug=eq.${encodeURIComponent(slug)}&select=${TOOL_COLUMNS.join(",")}`,
    );
    return rows[0] || null;
  },
  async scan(limit) {
    return restFetch(
      `tools?select=${TOOL_COLUMNS.join(",")}&approval_status=eq.approved` +
        `&relevance_status=eq.accepted&quarantined_at=is.null&order=stars.desc&limit=${limit}`,
    );
  },
  async apply(tool, patch, audit) {
    await restFetch(`tools?id=eq.${tool.id}`, {
      method: "PATCH",
      headers: { Prefer: "return=minimal" },
      body: JSON.stringify(patch),
    });
    await restFetch("tool_review_events", {
      method: "POST",
      headers: { Prefer: "return=minimal" },
      body: JSON.stringify(audit),
    });
  },
  async close() {},
};

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

async function makePgBackend() {
  const pg = loadPg();
  if (!pg) {
    throw new Error(
      "direct-postgres fallback needs the pg driver: run `npm install --prefix scripts/ops`",
    );
  }
  if (!DATABASE_URL) throw new Error("DATABASE_URL missing for direct-postgres fallback");
  const ssl = pgSslOption(DATABASE_URL);
  const client = new pg.Client({
    connectionString: DATABASE_URL,
    ...(ssl !== undefined ? { ssl } : {}),
    statement_timeout: 20000,
  });
  await client.connect();
  // Best-effort: ask PostgREST to rebuild its schema cache (heals PGRST002).
  await client.query("NOTIFY pgrst, 'reload schema'").catch(() => {});
  const cols = TOOL_COLUMNS.join(", ");
  return {
    name: "direct-postgres",
    async fetchTool(slug) {
      const r = await client.query(`SELECT ${cols} FROM tools WHERE slug = $1`, [slug]);
      return r.rows[0] || null;
    },
    async scan(limit) {
      const r = await client.query(
        `SELECT ${cols} FROM tools
         WHERE approval_status = 'approved' AND relevance_status = 'accepted'
           AND quarantined_at IS NULL
         ORDER BY stars DESC NULLS LAST LIMIT $1`,
        [limit],
      );
      return r.rows;
    },
    async apply(tool, patch, audit) {
      await client.query(
        `UPDATE tools SET status = $2, official_team = COALESCE($3, official_team),
                          updated_at = now()
         WHERE id = $1`,
        [tool.id, patch.status, patch.official_team ?? null],
      );
      await client.query(
        `INSERT INTO tool_review_events
           (tool_id, admin_id, action, reason, before_status, after_status)
         VALUES ($1, NULL, $2, $3, $4, $5)`,
        [tool.id, audit.action, audit.reason, audit.before_status, audit.after_status],
      );
    },
    async close() {
      await client.end().catch(() => {});
    },
  };
}

async function pickBackend() {
  if (SUPABASE_URL && SERVICE_KEY && !process.env.FORCE_PG) {
    try {
      await restFetch("tools?select=slug&limit=1");
      return restBackend;
    } catch (error) {
      console.error(`rest backend unavailable (${error.message.slice(0, 120)}…) — falling back to direct postgres`);
    }
  }
  return makePgBackend();
}

// --- external evidence sources ------------------------------------------------

async function github(path) {
  const res = await fetch(`https://api.github.com${path}`, {
    headers: {
      Accept: "application/vnd.github+json",
      "User-Agent": "onchainai-verify-harness",
      ...(GITHUB_TOKEN ? { Authorization: `Bearer ${GITHUB_TOKEN}` } : {}),
    },
  });
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`github ${res.status} ${path}`);
  return res.json();
}

// --- identity helpers (mirrors src/trust_verification.rs semantics) -----------

const normalize = (v) =>
  (v || "").trim().toLowerCase().replace(/^@/, "").replace(/[-_]/g, "");

const identityTokensRelated = (a, b) => {
  const x = normalize(a);
  const y = normalize(b);
  if (!x || !y) return false;
  return x === y || x.includes(y) || y.includes(x);
};

const identityClusterAligned = (repoUrl, homepage, npmPackage) => {
  const gh = repoUrl ? parseGithubRepo(repoUrl) : null;
  const org = gh?.org;
  const scope = npmScope(npmPackage);
  const home = homepage ? hostLabel(homepage) : null;
  const domainLabel = home?.label;
  if (!org || !scope || !domainLabel) return false;
  return (
    identityTokensRelated(org, scope) &&
    (identityTokensRelated(org, domainLabel) ||
      identityTokensRelated(scope, domainLabel))
  );
};

function parseGithubRepo(repoUrl) {
  try {
    const u = new URL(repoUrl);
    if (u.hostname !== "github.com") return null;
    const [org, repo] = u.pathname.split("/").filter(Boolean);
    return org && repo ? { org, repo: repo.replace(/\.git$/, "") } : null;
  } catch {
    return null;
  }
}

const hostLabel = (url) => {
  try {
    const host = new URL(url).hostname.replace(/^www\./, "");
    return { host, label: host.split(".")[0] };
  } catch {
    return null;
  }
};

const npmScope = (pkg) =>
  pkg && pkg.startsWith("@") && pkg.includes("/") ? pkg.slice(1).split("/")[0] : null;

// --- evidence + decision -------------------------------------------------------

async function collectEvidence(tool) {
  const evidence = { checks: [] };
  const gh = tool.repo_url ? parseGithubRepo(tool.repo_url) : null;
  evidence.github_org = gh?.org || null;

  if (gh) {
    const repo = await github(`/repos/${gh.org}/${gh.repo}`);
    evidence.repo_exists = !!repo;
    evidence.repo_archived = repo?.archived ?? null;
    evidence.repo_org_login = repo?.owner?.login || gh.org;
    if (repo) evidence.checks.push(`repo github.com/${gh.org}/${gh.repo} exists`);

    if (repo && repo.owner?.type === "Organization") {
      const org = await github(`/orgs/${repo.owner.login}`);
      evidence.org_domain_verified = org?.is_verified ?? false;
      evidence.org_site = org?.blog || null;
      if (evidence.org_domain_verified)
        evidence.checks.push(`org ${repo.owner.login} is domain-verified on GitHub`);
    }
  } else {
    evidence.repo_exists = false;
  }

  const scope = npmScope(tool.npm_package);
  evidence.npm_scope = scope;
  if (tool.npm_package) {
    try {
      const res = await fetch(
        `https://registry.npmjs.org/${encodeURIComponent(tool.npm_package)}`,
      );
      if (res.ok) {
        const meta = await res.json();
        const repoField =
          typeof meta.repository === "string"
            ? meta.repository
            : meta.repository?.url || "";
        evidence.npm_repo_matches_org =
          !!evidence.github_org &&
          repoField
            .toLowerCase()
            .includes(`github.com/${evidence.github_org.toLowerCase()}/`);
        if (evidence.npm_repo_matches_org)
          evidence.checks.push("npm package repository points at the same GitHub org");
      }
    } catch {
      /* npm evidence is optional */
    }
  }

  const home = tool.homepage ? hostLabel(tool.homepage) : null;
  evidence.homepage_label = home?.label || null;

  evidence.identity_cluster_aligned = identityClusterAligned(
    tool.repo_url,
    tool.homepage,
    tool.npm_package,
  );
  if (evidence.identity_cluster_aligned) {
    evidence.checks.push(
      `identity cluster aligned (${evidence.github_org} / ${scope ?? "—"} / ${home?.label ?? "—"})`,
    );
  }

  // Org verified-domain ↔ homepage agreement (official path for unmapped vendors).
  if (evidence.org_domain_verified && evidence.org_site && home) {
    const orgHost = hostLabel(evidence.org_site)?.host;
    evidence.org_site_matches_homepage =
      !!orgHost &&
      (orgHost === home.host ||
        identityTokensRelated(orgHost.split(".")[0], home.label));
    if (evidence.org_site_matches_homepage)
      evidence.checks.push("verified org site matches tool homepage");
  }

  return evidence;
}

function decide(tool, evidence) {
  const gates = [];
  if (tool.approval_status !== "approved") gates.push("approval_status!=approved");
  if (tool.relevance_status !== "accepted") gates.push("relevance_status!=accepted");
  if (tool.install_risk_level === "critical") gates.push("install_risk_level=critical");
  if (tool.install_risk_level === "high") gates.push("install_risk_level=high");
  if (tool.quarantined_at) gates.push("quarantined");
  if (gates.length)
    return { decision: "refuse", reason: `public gate failed: ${gates.join(", ")}` };

  const orgKey = normalize(evidence.repo_org_login || evidence.github_org);
  const firstPartyEntry = Object.entries(FIRST_PARTY_ORGS).find(
    ([org]) => normalize(org) === orgKey,
  );

  if (evidence.repo_exists && firstPartyEntry) {
    return {
      decision: "official",
      team: firstPartyEntry[1],
      reason: `first-party org github.com/${evidence.repo_org_login || evidence.github_org}`,
    };
  }
  if (
    evidence.repo_exists &&
    evidence.org_domain_verified &&
    evidence.org_site_matches_homepage
  ) {
    return {
      decision: "official",
      team: evidence.repo_org_login,
      reason: "GitHub domain-verified org, org site matches tool homepage",
    };
  }
  if (evidence.repo_exists && !evidence.repo_archived && evidence.identity_cluster_aligned) {
    return {
      decision: "verified",
      reason: "identity cluster aligned (github org + npm scope + homepage)",
    };
  }
  return {
    decision: "community",
    reason:
      "insufficient evidence (need first-party org, verified-domain org, or aligned identity cluster)",
  };
}

const RANK = { community: 0, verified: 1, official: 2 };

async function applyStatus(db, tool, decision) {
  const target = decision.decision;
  const currentRank = RANK[tool.status];
  if (currentRank === undefined) {
    return {
      applied: false,
      note: `no-op: unrecognized current status '${tool.status}' — manual review required`,
    };
  }
  const upgradesTeam = target === "official" && decision.team && !tool.official_team;
  if (RANK[target] < currentRank || (RANK[target] === currentRank && !upgradesTeam)) {
    return { applied: false, note: `no-op: current status '${tool.status}' >= '${target}'` };
  }
  const patch = { status: target };
  if (target === "official" && decision.team) patch.official_team = decision.team;
  await db.apply(tool, patch, {
    tool_id: tool.id,
    admin_id: null,
    action: "agent_auto_status",
    reason: `verify-tool-official.mjs: ${decision.reason}`,
    before_status: tool.status,
    after_status: target,
  });
  return { applied: true };
}

async function processTool(db, tool) {
  if (EXPECT_ORG) {
    const gh = tool.repo_url ? parseGithubRepo(tool.repo_url) : null;
    if (normalize(gh?.org) !== normalize(EXPECT_ORG)) {
      return {
        slug: tool.slug,
        decision: "refuse",
        reason: `--expect-org ${EXPECT_ORG} does not match repo org ${gh?.org ?? "none"}`,
      };
    }
  }
  const evidence = await collectEvidence(tool);
  const decision = decide(tool, evidence);
  const report = {
    slug: tool.slug,
    name: tool.name,
    current_status: tool.status,
    decision: decision.decision,
    team: decision.team ?? tool.official_team ?? null,
    reason: decision.reason,
    evidence: evidence.checks,
    applied: false,
  };
  if (APPLY && (decision.decision === "official" || decision.decision === "verified")) {
    const result = await applyStatus(db, tool, decision);
    report.applied = result.applied;
    if (result.note) report.note = result.note;
  }
  return report;
}

function isScanCandidate(tool) {
  if (tool.install_risk_level === "critical" || tool.install_risk_level === "high") {
    return false;
  }
  const gh = tool.repo_url ? parseGithubRepo(tool.repo_url) : null;
  const orgHit =
    gh && Object.keys(FIRST_PARTY_ORGS).some((org) => normalize(org) === normalize(gh.org));
  const text = ` ${tool.name} ${tool.slug} `.toLowerCase();
  const kwHit = PLATFORM_KEYWORDS.some((k) => text.includes(k));
  return orgHit || kwHit;
}

// --- main ----------------------------------------------------------------------

if (!SCAN && slugs.length === 0) {
  console.error(
    "usage: node scripts/verify-tool-official.mjs <slug> [--apply] | --scan [--apply --i-understand-bulk]",
  );
  process.exit(2);
}
if (SCAN && APPLY && !BULK_APPLY) {
  console.error(
    "refusing --scan --apply without --i-understand-bulk (bulk promotions need explicit ack)",
  );
  process.exit(2);
}
if (!(SUPABASE_URL && SERVICE_KEY) && !DATABASE_URL) {
  console.error(
    "config error: need SUPABASE_URL+SUPABASE_SERVICE_KEY or DATABASE_URL (set ENV_FILE=/path/to/.env)",
  );
  process.exit(2);
}

const results = [];
let db;
try {
  db = await pickBackend();
  console.error(`backend: ${db.name}${APPLY ? " (apply)" : " (dry-run)"}`);
  if (SCAN) {
    const rows = await db.scan(LIMIT);
    const candidates = rows.filter(isScanCandidate);
    console.error(
      `scan: ${candidates.length} candidate(s) of ${rows.length} public tools (first-party org or platform keyword)`,
    );
    for (const tool of candidates) results.push(await processTool(db, tool));
  } else {
    for (const slug of slugs) {
      const tool = await db.fetchTool(slug);
      if (!tool) {
        results.push({ slug, decision: "refuse", reason: "tool not found" });
        continue;
      }
      results.push(await processTool(db, tool));
    }
  }
} catch (error) {
  console.error(`harness error: ${error.message}`);
  if (db) await db.close();
  process.exit(2);
}
await db.close();

for (const r of results) console.log(JSON.stringify(r));
console.error(
  `done: ${results.length} tool(s) — ` +
    `official ${results.filter((r) => r.decision === "official").length}, ` +
    `verified ${results.filter((r) => r.decision === "verified").length}, ` +
    `community ${results.filter((r) => r.decision === "community").length}, ` +
    `refused ${results.filter((r) => r.decision === "refuse").length}` +
    `${APPLY ? " (applied where qualifying)" : " (dry-run — pass --apply to write)"}`,
);
