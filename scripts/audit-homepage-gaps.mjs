#!/usr/bin/env node
// audit-homepage-gaps.mjs — find catalog tools with weak or broken homepage/repo metadata.
//
// Usage:
//   node scripts/audit-homepage-gaps.mjs [--limit N] [--check-github]
//
// Env: repo root .env (DATABASE_URL). Optional: GITHUB_API_TOKEN for rate limits.
// Direct Postgres fallback uses PG_INSECURE_SSL=1 on Supabase pooler when needed.

import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { loadFirstPartyOrgs } from "./vendor-orgs-lib.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const FIRST_PARTY_ORGS = loadFirstPartyOrgs();

function parseEnvFile(path) {
  const out = {};
  try {
    for (const raw of readFileSync(path, "utf8").split("\n")) {
      const line = raw.trim();
      if (!line || line.startsWith("#")) continue;
      const eq = line.indexOf("=");
      if (eq <= 0) continue;
      const key = line.slice(0, eq).trim();
      let value = line.slice(eq + 1).trim().replace(/^["']|["']$/g, "");
      if (key) out[key] = value;
    }
  } catch {
    /* optional */
  }
  return out;
}

const env = {
  ...parseEnvFile(process.env.ENV_FILE || resolve(ROOT, ".env")),
  ...process.env,
};

const args = process.argv.slice(2);
const LIMIT = Number(args[args.indexOf("--limit") + 1]) || 200;
const CHECK_GITHUB = args.includes("--check-github");
const GITHUB_TOKEN = env.GITHUB_API_TOKEN || "";

function parseGithubRepo(url) {
  if (!url) return null;
  const m = url.match(/github\.com\/([^/]+)\/([^/#?]+)/i);
  if (!m) return null;
  return { org: m[1], repo: m[2].replace(/\.git$/, "") };
}

function hostLabel(url) {
  try {
    const host = new URL(url).hostname.replace(/^www\./, "");
    return host.split(".")[0];
  } catch {
    return null;
  }
}

async function connectPg() {
  const { Client } = require(
    (() => {
      try {
        require.resolve("pg", { paths: [resolve(ROOT, "scripts/ops")] });
        return resolve(ROOT, "scripts/ops/node_modules/pg");
      } catch {
        return "pg";
      }
    })(),
  );
  const ssl =
    env.PG_INSECURE_SSL === "1" ? { rejectUnauthorized: false } : undefined;
  const client = new Client({ connectionString: env.DATABASE_URL, ssl });
  await client.connect();
  return client;
}

async function githubStatus(org, repo) {
  const headers = {
    Accept: "application/vnd.github+json",
    "User-Agent": "onchainai-homepage-audit",
  };
  if (GITHUB_TOKEN) headers.Authorization = `Bearer ${GITHUB_TOKEN}`;
  const res = await fetch(`https://api.github.com/repos/${org}/${repo}`, {
    headers,
  });
  return res.status;
}

function classifyRow(tool) {
  const issues = [];
  const gh = parseGithubRepo(tool.repo_url);
  const homeGh = parseGithubRepo(tool.homepage);
  const firstParty = gh && FIRST_PARTY_ORGS[gh.org.toLowerCase()];

  if (!tool.homepage && !tool.repo_url) {
    issues.push("no_homepage_or_repo");
  }
  if (tool.homepage && tool.homepage === tool.repo_url && gh) {
    issues.push("homepage_equals_github_repo");
  }
  if (firstParty && tool.homepage?.includes("github.com")) {
    issues.push("first_party_github_only_homepage");
  }
  if (
    tool.homepage?.includes("github.com") &&
    tool.repo_url &&
    tool.homepage === tool.repo_url
  ) {
    issues.push("likely_missing_product_site");
  }
  if (gh && !tool.official_links?.some((l) => l.display_label === "Documentation")) {
    if (firstParty || tool.source === "pypi") {
      issues.push("missing_docs_link");
    }
  }

  return {
    slug: tool.slug,
    name: tool.name,
    status: tool.status,
    source: tool.source,
    homepage: tool.homepage,
    repo_url: tool.repo_url,
    github: gh ? `${gh.org}/${gh.repo}` : null,
    first_party: !!firstParty,
    issues,
  };
}

async function main() {
  if (!env.DATABASE_URL) {
    console.error("DATABASE_URL required");
    process.exit(1);
  }

  const client = await connectPg();
  const { rows: tools } = await client.query(
    `SELECT t.id, t.slug, t.name, t.status, t.source, t.homepage, t.repo_url,
            t.official_team, t.approval_status, t.relevance_status
     FROM tools t
     WHERE t.approval_status = 'approved'
       AND t.relevance_status = 'accepted'
       AND t.install_risk_level <> 'critical'
       AND t.quarantined_at IS NULL
     ORDER BY t.updated_at DESC
     LIMIT $1`,
    [LIMIT],
  );

  const ids = tools.map((t) => t.id);
  let linkRows = [];
  if (ids.length) {
    const res = await client.query(
      `SELECT tool_id, link_type, url, display_label, verification_status
       FROM tool_official_links
       WHERE tool_id = ANY($1::uuid[])`,
      [ids],
    );
    linkRows = res.rows;
  }
  await client.end();

  const linksByTool = new Map();
  for (const link of linkRows) {
    const list = linksByTool.get(link.tool_id) || [];
    list.push(link);
    linksByTool.set(link.tool_id, list);
  }

  const flagged = [];
  for (const tool of tools) {
    tool.official_links = linksByTool.get(tool.id) || [];
    const row = classifyRow(tool);
    if (row.issues.length) flagged.push(row);
  }

  if (CHECK_GITHUB) {
    for (const row of flagged) {
      if (!row.github) continue;
      const [org, repo] = row.github.split("/");
      row.github_http = await githubStatus(org, repo);
      if (row.github_http === 404) row.issues.push("github_repo_404");
      await new Promise((r) => setTimeout(r, 120));
    }
  }

  const summary = {
    scanned: tools.length,
    flagged: flagged.length,
    check_github: CHECK_GITHUB,
    buckets: {},
  };
  for (const row of flagged) {
    for (const issue of row.issues) {
      summary.buckets[issue] = (summary.buckets[issue] || 0) + 1;
    }
  }

  console.log(JSON.stringify({ summary, flagged }, null, 2));
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});