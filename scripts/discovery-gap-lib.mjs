import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const GROUND_TRUTH_PATH = resolve(ROOT, "fixtures/discovery-ground-truth.json");
const PUBLIC_API = "https://www.onchain-ai.xyz/api/v2/tools/search";

export function loadGroundTruth() {
  return JSON.parse(readFileSync(GROUND_TRUTH_PATH, "utf8"));
}

export function normalizeUrl(value) {
  if (!value || typeof value !== "string") return null;
  try {
    const url = new URL(value.trim());
    return `${url.protocol}//${url.host}${url.pathname.replace(/\/$/, "")}`.toLowerCase();
  } catch {
    return value.trim().toLowerCase();
  }
}

function rowMatchesTool(row, tool) {
  const match = tool.match || {};
  if (match.slug && row.slug === match.slug) return true;
  if (tool.slug && row.slug === tool.slug) return true;
  if (match.repo_url && normalizeUrl(row.repo_url) === normalizeUrl(match.repo_url)) {
    return true;
  }
  if (match.homepage && normalizeUrl(row.homepage) === normalizeUrl(match.homepage)) {
    return true;
  }
  if (match.npm_package && row.npm_package === match.npm_package) return true;
  return false;
}

export async function searchPublicCatalog(query) {
  const url = `${PUBLIC_API}?query=${encodeURIComponent(query)}&limit=10`;
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`catalog search ${res.status} for ${query}`);
  }
  return res.json();
}

export async function catalogProbePublic(tool) {
  const queries = [
    ...(tool.match?.search_queries || []),
    tool.slug,
    tool.id,
  ].filter(Boolean);

  const seen = new Set();
  for (const query of queries) {
    if (seen.has(query)) continue;
    seen.add(query);
    const rows = await searchPublicCatalog(query);
    const hit = rows.find((row) => rowMatchesTool(row, tool));
    if (hit) {
      return { found: true, slug: hit.slug, source: hit.source, query };
    }
  }
  return { found: false, slug: null, source: null, query: null };
}

export async function liveProbeClawhub(slug) {
  const res = await fetch(`https://clawhub.ai/api/v1/skills/${encodeURIComponent(slug)}`);
  if (res.status === 404) return { found: false };
  if (!res.ok) throw new Error(`clawhub ${res.status} for ${slug}`);
  const body = await res.json();
  return {
    found: true,
    slug: body.skill?.slug || slug,
    displayName: body.skill?.displayName,
  };
}

export async function liveProbeGithubRepo(repoUrl) {
  if (!repoUrl) return { found: false };
  const res = await fetch(repoUrl, {
    headers: { Accept: "application/vnd.github+json", "User-Agent": "onchainai-discovery-audit" },
  });
  return { found: res.ok, status: res.status };
}

export function buildReport(results) {
  const total = results.length;
  const catalogHits = results.filter((r) => r.catalog.found).length;
  const liveHits = results.filter((r) => r.live?.found).length;
  return {
    version: 1,
    generated_at: new Date().toISOString(),
    metrics: {
      catalog_recall: total ? catalogHits / total : 0,
      catalog_hits: catalogHits,
      total,
      live_probe_hits: liveHits,
    },
    results,
  };
}