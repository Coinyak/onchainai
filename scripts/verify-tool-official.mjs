#!/usr/bin/env node
// verify-tool-official.mjs — evidence-based auto verification of tools.status.
//
// Any operator or AI agent can run this when the owner asks "verify/official <tool>".
// It collects public evidence (GitHub org, npm scope, homepage domain), decides
// community | verified | official, and with --apply writes the new status plus a
// tool_review_events audit row. Deny-by-default for elevation: no evidence →
// community (and --apply demotes any higher badge to match).
//
// Usage:
//   node scripts/verify-tool-official.mjs <slug> [--apply]
//   node scripts/verify-tool-official.mjs <slug>... [--apply --i-understand-bulk]
//   node scripts/verify-tool-official.mjs --scan [--apply --i-understand-bulk] [--limit N]
//   node scripts/verify-tool-official.mjs <slug> --expect-org <org> [--apply]
//   node scripts/verify-tool-official.mjs --self-test
//
// Modes:
//   default   dry-run: prints one JSON report line per slug, writes nothing
//   --apply   write status to match the decision (upgrade OR downgrade)
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
//   community insufficient evidence, non-tool repo, or failed identity checks
//   never     elevate tools failing the public gate (approval/relevance/risk/
//             quarantine); never touch x402 payment flags.
//   apply     sets tools.status to the decision even when that is a downgrade
//             (official→verified/community, verified→community). Clears
//             official_team when leaving official. Bulk --apply still needs
//             --i-understand-bulk.

import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { loadFirstPartyOrgs } from "./vendor-orgs-lib.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

// Curated first-party GitHub orgs → official team label (scripts/vendor-orgs.json).
const FIRST_PARTY_ORGS = loadFirstPartyOrgs();

// Narrow scan keywords — org hits in FIRST_PARTY_ORGS are the primary signal.
const PLATFORM_KEYWORDS = [
  "github-mcp", "farcaster", "neynar", "discord-interactions", "telegram",
  "x-api", "twitter-api", "xdevplatform",
];

// Repo name / slug patterns that are NOT developer tools (docs, specs, demos,
// examples, infra automation, org profile repos, audits, etc.).
// Matching repos stay/return to community — no official/verified badge —
// even when they belong to a first-party org. Also used so --apply can demote
// tools that were previously elevated by org membership alone.
const NON_TOOL_PATTERNS = [
  /^\.github$/i,
  /^(docs|documentation|documentation-en|documentation-zh|doc-|safe-docs|uniswap-docs|graphprotocol-docs|ton-blockchain-docs|wormhole-docs)/i,
  /(website|esp-website|ethereum-org-website|zkvm-website|steel-website|ton-blockchain-github-io)$/i,
  /^(consensus-specs|execution-specs|execution-apis|cryptography-specs|devp2p|walletconnect-specs|teps)$/i,
  /^(program-examples|crypto-primitives-examples|query-examples|graphprotocol-examples|haskell-nix-example|layerzero-solana-frontend-examples)/i,
  /^(demo-|.*-demo$|demos$|onramp-demo|onramp-v2-mobile-demo|wirex-wallets-demo|buy-sell-opensea-sdk-demo)/i,
  /^(example-|.*-examples$)/i,
  /^(ansible-role-)/i,
  /(helm-charts?)$/i,
  /^(eth-phishing-detect|phishing-detect)$/i,
  /(audits?|security-audits|wormhole-audits)$/i,
  /(essential-cardano-content)$/i,
  /^(errorprone-checks)$/i,
  /^(docs-template)$/i,
  /^(safe-apps-list)$/i,
  /^(safe-transaction-service)$/i,
  /^(sun-network)$/i,
  /^(x402-chat|x402\.chat)$/i,
  /^(pay-skills)$/i,
  /^(base-contracts|contract-deployments|uerc20-factory|uniroute-public|v4-hooks-public|uniswapx-parameterization-api)$/i,
  /^(lz-address-book)$/i,
  /^(ouroboros-leios-formal-spec)$/i,
  /^(adnl-tunnel)$/i,
  /^(account-policies)$/i,
  /^(action-is-release|action-publish-release)$/i,
  /^(contributor-docs)$/i,
  // CI / packaging / lint / legal / ops — not catalog products
  /^(github-actions|github-tools|ci-workflows|gh-actions-runners|actions)$/i,
  /^homebrew[-_]/i,
  /^(eslint-config|eslint-plugin)/i,
  /^(cla-signatures|cla)$/i,
  /^(grafonnet|grafonnet-lib)$/i,
  /^(system-test|system-tests)$/i,
  // Specs, papers, archives, lists, research fluff
  /(whitepaper|technical-whitepaper)$/i,
  /(-archive$|^eth-rnd-archive$)/i,
  /^(awesome[-_]|bug-bounty)$/i,
  /(specification|tvm-specification)$/i,
  /(-improvement-proposals$|projectopensea-sips$)/i,
  // Templates, starters, quickstarts, playgrounds, scaffolds
  /(template|templates|starter|starters|quickstart|scaffolding|playground)$/i,
  /^(shipyard|shipyard-core)$/i,
  /^(create-thirdweb-app|expo-app-template|fintech-starter-app|wallets-quickstart)$/i,
  /^(payments-sample-app|circle-cctp-crosschain-transfer)$/i,
  /^(hello-token|hello-wormhole)$/i,
  /^(test-dapp|test-dapp-multichain|snap-simple-keyring)$/i,
  // Internal ops / pure packaging / meeting notes
  /^(packages|tron-deployment|tronprotocol-pm|discv4-dns-lists)$/i,
  /^(task-signing-tool|claiming-app-data|solana-data-aggregator)$/i,
  /^(protocol-prototyping-site|extension-benchmark-stats)$/i,
  /^(benchmark|benchmarks)$/i,
];

/**
 * Return true if the repo URL or slug matches a non-tool pattern.
 * Used to block official/verified elevation of docs, specs, demos, examples,
 * infra automation, and org-profile repos.
 */
function isNonToolRepo(repoUrl, slug) {
  const gh = repoUrl ? parseGithubRepo(repoUrl) : null;
  const repoName = gh?.repo || "";
  // Test repo name and slug individually against each pattern.
  return NON_TOOL_PATTERNS.some(
    (p) => p.test(repoName) || (slug && p.test(slug)),
  );
}

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
      // Always set official_team (including NULL on demotion). COALESCE would
      // trap demotions with a stale team label.
      await client.query(
        `UPDATE tools SET status = $2, official_team = $3, updated_at = now()
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
  const repo = gh?.repo;
  const scope = npmScope(npmPackage);
  const home = homepage ? hostLabel(homepage) : null;
  const domainLabel = home?.label;
  if (!org || !scope || !domainLabel) return false;
  if (!identityTokensRelated(org, scope)) return false;
  return (
    identityTokensRelated(org, domainLabel) ||
    identityTokensRelated(scope, domainLabel) ||
    (repo ? identityTokensRelated(repo, domainLabel) : false)
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
  evidence.npm_registry_exists = false;
  if (tool.npm_package) {
    try {
      const res = await fetch(
        `https://registry.npmjs.org/${encodeURIComponent(tool.npm_package)}`,
      );
      if (res.ok) {
        evidence.npm_registry_exists = true;
        evidence.checks.push("npm package exists on registry");
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

  // Block elevation of non-tool repos (docs, specs, demos, examples, infra
  // automation, org-profile repos, audits, etc.) even from first-party orgs.
  if (isNonToolRepo(tool.repo_url, tool.slug)) {
    return {
      decision: "community",
      reason: "non-tool repo (docs/specs/demo/example/infra/profile/audit)",
    };
  }

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
  if (
    !evidence.repo_exists &&
    evidence.identity_cluster_aligned &&
    evidence.npm_registry_exists &&
    evidence.npm_scope &&
    evidence.github_org &&
    identityTokensRelated(evidence.github_org, evidence.npm_scope)
  ) {
    return {
      decision: "verified",
      reason:
        "identity cluster aligned (scoped npm + homepage; github repo unavailable)",
    };
  }
  return {
    decision: "community",
    reason:
      "insufficient evidence (need first-party org, verified-domain org, or aligned identity cluster)",
  };
}

const RANK = { community: 0, verified: 1, official: 2 };

function directionOf(fromStatus, toStatus) {
  const from = RANK[fromStatus] ?? 0;
  const to = RANK[toStatus] ?? 0;
  if (to > from) return "upgrade";
  if (to < from) return "downgrade";
  return "same";
}

/**
 * Apply decision to tools.status (upgrade, downgrade, or team fill-in).
 * Leaving official always clears official_team unless the new status is still
 * official (then set decision.team when present).
 */
async function applyStatus(db, tool, decision) {
  const target = decision.decision;
  if (!(target in RANK)) {
    return { applied: false, note: `no-op: non-status decision '${target}'` };
  }

  const newTeam =
    target === "official" ? decision.team || tool.official_team || null : null;
  const statusChanged = tool.status !== target;
  const teamChanged =
    target === "official"
      ? Boolean(decision.team && decision.team !== tool.official_team)
      : tool.official_team != null; // demotion clears team

  if (!statusChanged && !teamChanged) {
    return {
      applied: false,
      note: `no-op: already '${target}'`,
      direction: "same",
    };
  }

  const direction = statusChanged
    ? directionOf(tool.status, target)
    : "team";
  const patch = {
    status: target,
    // Always send official_team so demotion nulls a stale label (REST + PG).
    official_team: newTeam,
  };
  const verb =
    direction === "downgrade"
      ? "downgrade"
      : direction === "upgrade"
        ? "upgrade"
        : "team-update";
  await db.apply(tool, patch, {
    tool_id: tool.id,
    admin_id: null,
    action: "agent_auto_status",
    reason: `verify-tool-official.mjs ${verb}: ${decision.reason}`,
    before_status: tool.status,
    after_status: target,
  });
  return { applied: true, direction, team: newTeam };
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
  const isStatusDecision = decision.decision in RANK;
  const report = {
    slug: tool.slug,
    name: tool.name,
    current_status: tool.status,
    decision: decision.decision,
    // Only official keeps a team label in the report (and in DB after apply).
    team:
      decision.decision === "official"
        ? (decision.team ?? tool.official_team ?? null)
        : null,
    reason: decision.reason,
    evidence: evidence.checks,
    direction: isStatusDecision
      ? directionOf(tool.status, decision.decision)
      : null,
    applied: false,
  };
  // Apply any concrete status decision (including community demotions).
  if (APPLY && isStatusDecision) {
    const result = await applyStatus(db, tool, decision);
    report.applied = result.applied;
    if (result.note) report.note = result.note;
    if (result.direction) report.direction = result.direction;
    if (result.applied && Object.hasOwn(result, "team")) report.team = result.team;
  }
  return report;
}

function isScanCandidate(tool) {
  if (tool.install_risk_level === "critical" || tool.install_risk_level === "high") {
    return false;
  }
  const elevated = tool.status === "official" || tool.status === "verified";
  // Skip non-tool repos for elevation candidates, but keep elevated ones so
  // --scan --apply can demote docs/demos/CI that already hold a trust badge.
  if (!elevated && isNonToolRepo(tool.repo_url, tool.slug)) {
    return false;
  }
  const gh = tool.repo_url ? parseGithubRepo(tool.repo_url) : null;
  const orgHit =
    gh && Object.keys(FIRST_PARTY_ORGS).some((org) => normalize(org) === normalize(gh.org));
  const text = ` ${tool.name} ${tool.slug} `.toLowerCase();
  const kwHit = PLATFORM_KEYWORDS.some((k) => text.includes(k));
  // Include already-elevated tools without org/keyword so --scan can demote
  // hand-promoted or pattern-blocked repos that still hold a badge.
  return orgHit || kwHit || elevated;
}

// --- self-test (pure helpers; no DB/network) ---------------------------------

function runSelfTest() {
  const fails = [];
  const assert = (cond, msg) => {
    if (!cond) fails.push(msg);
  };

  assert(directionOf("community", "official") === "upgrade", "upgrade direction");
  assert(directionOf("official", "community") === "downgrade", "downgrade direction");
  assert(directionOf("verified", "verified") === "same", "same direction");
  assert(
    isNonToolRepo("https://github.com/ton-blockchain/homebrew-ton", "homebrew-ton"),
    "homebrew-ton is non-tool",
  );
  assert(
    isNonToolRepo("https://github.com/Consensys/github-actions", "github-actions"),
    "github-actions is non-tool",
  );
  assert(
    !isNonToolRepo("https://github.com/ProjectOpenSea/seaport-js", "seaport-js"),
    "seaport-js is a tool",
  );
  assert(
    isNonToolRepo("https://github.com/WalletConnect/wcn-technical-whitepaper", "wcn-technical-whitepaper"),
    "whitepaper is non-tool",
  );
  // Elevated non-tool without first-party org still scanned (demotion path).
  assert(
    isScanCandidate({
      status: "official",
      install_risk_level: "low",
      repo_url: "https://github.com/not-a-first-party/homebrew-ton",
      slug: "homebrew-ton",
      name: "homebrew-ton",
    }),
    "elevated non-tool remains scan candidate for demotion",
  );
  assert(
    !isScanCandidate({
      status: "community",
      install_risk_level: "low",
      repo_url: "https://github.com/not-a-first-party/homebrew-ton",
      slug: "homebrew-ton",
      name: "homebrew-ton",
    }),
    "community non-tool without first-party org is not a scan candidate",
  );

  if (fails.length) {
    console.error("self-test FAILED:");
    for (const f of fails) console.error(`  - ${f}`);
    process.exit(1);
  }
  console.log(JSON.stringify({ ok: true, tests: 9 }));
  process.exit(0);
}

// --- main ----------------------------------------------------------------------

if (args.includes("--self-test")) {
  runSelfTest();
}

if (!SCAN && slugs.length === 0) {
  console.error(
    "usage: node scripts/verify-tool-official.mjs <slug> [--apply] | <slug>... [--apply --i-understand-bulk] | --scan [--apply --i-understand-bulk] | --self-test",
  );
  process.exit(2);
}
if (APPLY && !BULK_APPLY && (SCAN || slugs.length > 1)) {
  console.error(
    "refusing bulk --apply without --i-understand-bulk (bulk upgrades/downgrades need explicit ack)",
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
const applied = results.filter((r) => r.applied);
const nUp = applied.filter((r) => r.direction === "upgrade").length;
const nDown = applied.filter((r) => r.direction === "downgrade").length;
const nTeam = applied.filter((r) => r.direction === "team").length;
console.error(
  `done: ${results.length} tool(s) — ` +
    `official ${results.filter((r) => r.decision === "official").length}, ` +
    `verified ${results.filter((r) => r.decision === "verified").length}, ` +
    `community ${results.filter((r) => r.decision === "community").length}, ` +
    `refused ${results.filter((r) => r.decision === "refuse").length}` +
    (APPLY
      ? ` (applied ${applied.length}: ↑${nUp} ↓${nDown} team${nTeam})`
      : " (dry-run — pass --apply to write)"),
);
