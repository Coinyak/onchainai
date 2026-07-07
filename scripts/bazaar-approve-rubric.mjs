#!/usr/bin/env node
// bazaar-approve-rubric.mjs — score pending Bazaar x402 tools against the 16-point rubric.
//
// Usage:
//   node scripts/bazaar-approve-rubric.mjs <slug>
//   node scripts/bazaar-approve-rubric.mjs --pending-bazaar [--limit N]
//   node scripts/bazaar-approve-rubric.mjs <slug> --apply --i-understand-canary
//   node scripts/bazaar-approve-rubric.mjs --self-test
//
// See docs/OPERATOR_GUIDE.md §5.5 and docs/superpowers/specs/2026-07-07-okx-x402-infra-waves.md §6.

import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

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

const args = process.argv.slice(2);
const APPLY = args.includes("--apply");
const CANARY_ACK = args.includes("--i-understand-canary");
const PENDING = args.includes("--pending-bazaar");
const LIMIT = Number(args[args.indexOf("--limit") + 1]) || 20;
const slugs = args.filter((a, i) => !a.startsWith("--") && args[i - 1] !== "--limit");

const TOOL_COLUMNS = [
  "id", "slug", "name", "status", "source", "repo_url", "homepage", "npm_package",
  "mcp_endpoint", "function", "chains", "stars", "approval_status", "relevance_status",
  "install_risk_level", "quarantined_at", "pricing", "x402_endpoint", "x402_price",
  "x402_endpoint_verified", "price_verified", "referral_enabled",
];

function pgSslOption(databaseUrl) {
  const mode = (env.PGSSLMODE || "").toLowerCase();
  const wantsSsl =
    mode === "require" ||
    mode === "verify-ca" ||
    mode === "verify-full" ||
    /supabase\.(co|com)/i.test(databaseUrl) ||
    databaseUrl.includes("sslmode=require");
  if (!wantsSsl) return undefined;
  if (env.PG_INSECURE_SSL === "1") return { rejectUnauthorized: false };
  return true;
}

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
  if (!res.ok) throw new Error(`postgrest ${res.status} ${path}: ${await res.text()}`);
  return res.status === 204 ? null : res.json();
}

async function makeBackend() {
  if (SUPABASE_URL && SERVICE_KEY && !process.env.FORCE_PG) {
    try {
      await restFetch("tools?select=slug&limit=1");
      const cols = TOOL_COLUMNS.join(",");
      return {
        name: "supabase-rest",
        async fetchTool(slug) {
          const rows = await restFetch(`tools?slug=eq.${encodeURIComponent(slug)}&select=${cols}`);
          return rows[0] || null;
        },
        async pendingBazaar(limit) {
          return restFetch(
            `tools?select=${cols}&source=eq.bazaar&approval_status=eq.pending&pricing=eq.x402` +
              `&quarantined_at=is.null&order=updated_at.desc&limit=${limit}`,
          );
        },
        async duplicateEndpoint(endpoint, excludeId) {
          const rows = await restFetch(
            `tools?x402_endpoint=eq.${encodeURIComponent(endpoint)}&id=neq.${excludeId}` +
              `&approval_status=eq.approved&select=slug&limit=1`,
          );
          return rows[0] || null;
        },
        async apply(tool, audit) {
          await restFetch(`tools?id=eq.${tool.id}`, {
            method: "PATCH",
            headers: { Prefer: "return=minimal" },
            body: JSON.stringify({
              approval_status: "approved",
              relevance_status: "accepted",
              referral_enabled: false,
              updated_at: new Date().toISOString(),
            }),
          });
          await restFetch("tool_review_events", {
            method: "POST",
            headers: { Prefer: "return=minimal" },
            body: JSON.stringify(audit),
          });
        },
        async close() {},
      };
    } catch (e) {
      console.error(`rest unavailable (${e.message.slice(0, 80)}) — postgres fallback`);
    }
  }

  const pg = (() => {
    try {
      return require(resolve(ROOT, "scripts/ops/node_modules/pg"));
    } catch {
      try {
        return require("pg");
      } catch {
        return null;
      }
    }
  })();
  if (!pg || !DATABASE_URL) {
    throw new Error("Need SUPABASE_URL+SERVICE_KEY or DATABASE_URL (npm install --prefix scripts/ops)");
  }
  const ssl = pgSslOption(DATABASE_URL);
  const client = new pg.Client({
    connectionString: DATABASE_URL,
    ...(ssl !== undefined ? { ssl } : {}),
  });
  await client.connect();
  const cols = TOOL_COLUMNS.join(", ");
  return {
    name: "direct-postgres",
    async fetchTool(slug) {
      const r = await client.query(`SELECT ${cols} FROM tools WHERE slug = $1`, [slug]);
      return r.rows[0] || null;
    },
    async pendingBazaar(limit) {
      const r = await client.query(
        `SELECT ${cols} FROM tools
         WHERE source = 'bazaar' AND approval_status = 'pending' AND pricing = 'x402'
           AND quarantined_at IS NULL
         ORDER BY updated_at DESC LIMIT $1`,
        [limit],
      );
      return r.rows;
    },
    async duplicateEndpoint(endpoint, excludeId) {
      const r = await client.query(
        `SELECT slug FROM tools
         WHERE x402_endpoint = $1 AND id <> $2 AND approval_status = 'approved' LIMIT 1`,
        [endpoint, excludeId],
      );
      return r.rows[0] || null;
    },
    async apply(tool, audit) {
      await client.query(
        `UPDATE tools SET approval_status = 'approved', relevance_status = 'accepted',
                          referral_enabled = false, updated_at = now()
         WHERE id = $1`,
        [tool.id],
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

function normalizeAmount(value) {
  return (value || "")
    .trim()
    .toLowerCase()
    .replace(/,/g, "")
    .replace(/[^a-z0-9.]/g, "");
}

function extractDigits(value) {
  const n = normalizeAmount(value);
  let out = "";
  for (const ch of n) {
    if ((ch >= "0" && ch <= "9") || ch === ".") out += ch;
    else break;
  }
  return out;
}

/** Strip trailing "(0x…)" asset hints Bazaar stores in x402_price. */
function advertisedAmountDigits(value) {
  const head = String(value || "").split("(")[0];
  return extractDigits(head);
}

function priceWithinTolerance(probed, advertised, pct = 0.1) {
  const p = Number(extractDigits(probed));
  const a = Number(advertisedAmountDigits(advertised));
  if (!Number.isFinite(p) || !Number.isFinite(a) || a === 0) return false;
  return Math.abs(p - a) / a <= pct;
}

/** Extract payable amount from one x402 accept object (v1 + v2 field names). */
function extractAmountFromAccept(accept) {
  if (!accept || typeof accept !== "object") return null;
  for (const key of [
    "maxAmountRequired",
    "maxAmount",
    "amount",
    "price",
    "value",
    "max_amount_required",
    "max_amount",
  ]) {
    const raw = accept[key];
    if (raw == null) continue;
    const text = String(raw).trim();
    if (text) return text;
  }
  return null;
}

/** Collect accept entries from common x402 402 JSON shapes. */
function collectAccepts(parsed) {
  const out = [];
  if (Array.isArray(parsed?.accepts)) out.push(...parsed.accepts);
  const pr = parsed?.paymentRequirements;
  if (Array.isArray(pr)) out.push(...pr);
  else if (pr && typeof pr === "object") out.push(pr);
  return out.filter((a) => a && typeof a === "object");
}

/** Prefer Base mainnet (eip155:8453) when multiple accepts are present. */
function pickAccept(accepts) {
  if (!accepts.length) return null;
  for (const accept of accepts) {
    const net = String(accept.network || "").toLowerCase();
    if (net.includes("8453") || net.includes("base")) return accept;
  }
  return accepts[0];
}

function parse402Payload(body) {
  let parsed;
  try {
    parsed = JSON.parse(body);
  } catch {
    return { ok: false, reason: "402 body not JSON" };
  }
  const accepts = collectAccepts(parsed);
  if (!accepts.length) {
    return { ok: false, reason: "402 missing accepts[]" };
  }
  const accept = pickAccept(accepts);
  const amount = extractAmountFromAccept(accept);
  if (!amount) {
    return { ok: false, reason: "402 accept missing amount fields" };
  }
  return { ok: true, amount, asset: accept.asset || null };
}

async function fetchWithTimeout(url, init, ms = 8000) {
  return fetch(url, { ...init, redirect: "manual", signal: AbortSignal.timeout(ms) });
}

async function probeX402Once(url, method) {
  try {
    const res = await fetchWithTimeout(url, { method });
    const body = await res.text();
    return { status: res.status, body, headers: res.headers };
  } catch (err) {
    return {
      status: 0,
      body: "",
      headers: new Headers(),
      error: err?.message || String(err),
    };
  }
}

async function probeX402(url) {
  const endpoint = url.trim();
  if (!endpoint.startsWith("https://")) {
    return { ok: false, reason: "only https endpoints allowed", autoReject: true };
  }
  try {
    const u = new URL(endpoint);
    if (["localhost", "127.0.0.1"].includes(u.hostname)) {
      return { ok: false, reason: "blocked host", autoReject: true };
    }
  } catch {
    return { ok: false, reason: "invalid url", autoReject: true };
  }

  const started = Date.now();
  const attempts = [];
  let currentUrl = endpoint;

  for (let hop = 0; hop < 2; hop++) {
    let probe = await probeX402Once(currentUrl, "POST");
    attempts.push({ method: "POST", status: probe.status, url: currentUrl });
    if (probe.status !== 402) {
      probe = await probeX402Once(currentUrl, "GET");
      attempts.push({ method: "GET", status: probe.status, url: currentUrl });
    }

    if ([301, 302, 307, 308].includes(probe.status)) {
      const location = probe.headers.get("location");
      if (!location) break;
      try {
        currentUrl = new URL(location, currentUrl).toString();
        continue;
      } catch {
        break;
      }
    }

    const latencyMs = Date.now() - started;
    if (probe.status === 0) {
      return {
        ok: false,
        reason: `request failed: ${probe.error || "timeout"}`,
        latencyMs,
        autoReject: false,
      };
    }
    if (probe.status !== 402) {
      const last = attempts[attempts.length - 1];
      return {
        ok: false,
        reason: `expected 402, got ${last?.status ?? probe.status}`,
        latencyMs,
        autoReject: false,
      };
    }

    const parsed = parse402Payload(probe.body);
    if (!parsed.ok) {
      return { ok: false, reason: parsed.reason, latencyMs, autoReject: false };
    }
    return { ok: true, amount: parsed.amount, asset: parsed.asset, latencyMs };
  }

  const latencyMs = Date.now() - started;
  return { ok: false, reason: "redirect loop or unresolved endpoint", latencyMs, autoReject: false };
}

async function npmWeeklyDownloads(packageName) {
  const name = (packageName || "").trim();
  if (!name) return null;
  try {
    const res = await fetch(
      `https://api.npmjs.org/downloads/point/last-week/${encodeURIComponent(name)}`,
      { signal: AbortSignal.timeout(5000) },
    );
    if (!res.ok) return null;
    const data = await res.json();
    const n = Number(data?.downloads);
    return Number.isFinite(n) ? n : null;
  } catch {
    return null;
  }
}

function trustTierScore(status) {
  if (status === "official" || status === "verified") return 2;
  return 0;
}

function registryScore(tool) {
  const hay = [tool.mcp_endpoint, tool.homepage, tool.repo_url, tool.source_url]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
  if (/mcp\.so|smithery|modelcontextprotocol|server\.json|mcp-registry/.test(hay)) return 2;
  return 0;
}

function tagScore(tool) {
  const fnOk = tool.function && tool.function !== "unknown";
  const chains = Array.isArray(tool.chains) ? tool.chains : [];
  return fnOk && chains.length > 0 ? 2 : fnOk || chains.length > 0 ? 1 : 0;
}

async function scoreTool(tool, backend) {
  const rejections = [];
  const breakdown = [];

  if (tool.quarantined_at) rejections.push("quarantined (L4)");
  if (tool.referral_enabled) rejections.push("referral_enabled must be false");
  if (tool.source !== "bazaar") rejections.push(`source=${tool.source}, expected bazaar`);
  if (tool.pricing !== "x402") rejections.push(`pricing=${tool.pricing}, expected x402`);

  const endpoint = (tool.x402_endpoint || "").trim();
  if (!endpoint) rejections.push("missing x402_endpoint");

  let probe = { ok: false, reason: "not probed" };
  if (endpoint && !rejections.some((r) => r.includes("blocked") || r.includes("https"))) {
    probe = await probeX402(endpoint);
    if (probe.autoReject) rejections.push(probe.reason);
  }

  if (probe.ok) {
    breakdown.push({ item: "402_handshake", points: 4, required: true });
  } else if (endpoint) {
    breakdown.push({ item: "402_handshake", points: 0, required: true, note: probe.reason });
    rejections.push(`402 handshake failed: ${probe.reason}`);
  }

  if (probe.ok && tool.x402_price) {
    const match = priceWithinTolerance(probe.amount, tool.x402_price, 0.1);
    breakdown.push({
      item: "price_match",
      points: match ? 3 : 0,
      required: true,
      note: match ? null : `probed=${probe.amount} advertised=${tool.x402_price}`,
    });
    if (!match) rejections.push("price mismatch >10%");
  } else if (probe.ok) {
    breakdown.push({ item: "price_match", points: 0, required: true, note: "missing x402_price" });
    rejections.push("missing x402_price for comparison");
  }

  const stars = Number(tool.stars) || 0;
  const npmWeekly = await npmWeeklyDownloads(tool.npm_package);
  const popularity =
    stars >= 50 || (npmWeekly != null && npmWeekly >= 100) ? 2 : 0;
  const popNote =
    npmWeekly != null
      ? `stars=${stars}, npm_weekly=${npmWeekly}`
      : `stars=${stars}`;
  breakdown.push({ item: "stars_or_npm", points: popularity, note: popNote });

  const reg = registryScore(tool);
  breakdown.push({ item: "registry_crosslist", points: reg });

  const tags = tagScore(tool);
  breakdown.push({ item: "function_chain_tags", points: tags });

  const risk = (tool.install_risk_level || "").toLowerCase();
  if (risk === "critical" || risk === "high") {
    rejections.push(`install_risk=${risk}`);
    breakdown.push({ item: "install_risk_trust", points: 0, note: risk });
  } else if (risk === "low" || risk === "medium") {
    const trust = trustTierScore(tool.status);
    const pts = trust >= 2 ? 2 : 1;
    breakdown.push({ item: "install_risk_trust", points: pts, note: `${risk}/${tool.status}` });
  } else {
    breakdown.push({ item: "install_risk_trust", points: 0, note: risk || "unknown" });
  }

  let duplicate = null;
  if (endpoint) {
    duplicate = await backend.duplicateEndpoint(endpoint, tool.id);
    if (duplicate) {
      rejections.push(`duplicate endpoint: ${duplicate.slug}`);
      breakdown.push({ item: "not_duplicate", points: 0, required: true });
    } else {
      breakdown.push({ item: "not_duplicate", points: 1, required: true });
    }
  }

  const requiredPass = breakdown
    .filter((b) => b.required)
    .every((b) => b.points > 0);
  const total = breakdown.reduce((s, b) => s + b.points, 0);
  const pass = rejections.length === 0 && requiredPass && total >= 12;

  return {
    slug: tool.slug,
    pass,
    total,
    max: 16,
    requiredPass,
    rejections,
    breakdown,
    probe: probe.ok
      ? { amount: probe.amount, asset: probe.asset, latencyMs: probe.latencyMs }
      : { error: probe.reason, latencyMs: probe.latencyMs },
  };
}

function runSelfTest() {
  const samples = [
    [{ amount: "1000", asset: "0xusdc" }, "1000"],
    [{ maxAmountRequired: "2500" }, "2500"],
    [{ maxAmount: "99" }, "99"],
    [{ price: "42" }, "42"],
  ];
  for (const [accept, want] of samples) {
    const got = extractAmountFromAccept(accept);
    if (got !== want) {
      console.error(`self-test extractAmountFromAccept: want ${want}, got ${got}`);
      process.exit(1);
    }
  }
  const v2 = parse402Payload(
    JSON.stringify({
      x402Version: 2,
      accepts: [{ scheme: "exact", network: "eip155:8453", amount: "7000", asset: "0x1" }],
    }),
  );
  if (!v2.ok || v2.amount !== "7000") {
    console.error("self-test parse402Payload v2 failed");
    process.exit(1);
  }
  if (!priceWithinTolerance("7000", "7000 (0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913)")) {
    console.error("self-test priceWithinTolerance failed");
    process.exit(1);
  }
  console.error("BAZAAR_RUBRIC_SELF_TEST PASS");
}

async function main() {
  if (args.includes("--self-test")) {
    runSelfTest();
    return;
  }
  if (APPLY && (!CANARY_ACK || slugs.length !== 1)) {
    console.error("Apply requires exactly one <slug> and --i-understand-canary (no bulk).");
    process.exit(2);
  }
  if (!PENDING && slugs.length === 0) {
    console.error("Usage: node scripts/bazaar-approve-rubric.mjs <slug> | --pending-bazaar");
    process.exit(2);
  }

  const backend = await makeBackend();
  try {
    const targets = PENDING
      ? await backend.pendingBazaar(LIMIT)
      : await Promise.all(slugs.map((s) => backend.fetchTool(s)));

    for (const tool of targets.filter(Boolean)) {
      let report;
      try {
        report = await scoreTool(tool, backend);
      } catch (err) {
        report = {
          slug: tool.slug,
          pass: false,
          total: 0,
          max: 16,
          requiredPass: false,
          rejections: [`score error: ${err?.message || err}`],
          breakdown: [],
          probe: { error: err?.message || String(err) },
        };
      }
      console.log(JSON.stringify(report));

      if (APPLY && report.pass) {
        await backend.apply(tool, {
          action: "bazaar_rubric_approve",
          reason: `rubric ${report.total}/16 canary apply`,
          before_status: tool.approval_status,
          after_status: "approved",
        });
        console.error(`APPLIED: ${tool.slug}`);
      } else if (APPLY && !report.pass) {
        console.error(`SKIP APPLY (failed rubric): ${tool.slug}`);
        process.exit(1);
      }
    }
  } finally {
    await backend.close();
  }
}

main().catch((e) => {
  console.error(e.message || e);
  process.exit(1);
});