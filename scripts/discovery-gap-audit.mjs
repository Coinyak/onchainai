#!/usr/bin/env node
// discovery-gap-audit.mjs — measure catalog recall against ground-truth tools.
//
// Usage:
//   node scripts/discovery-gap-audit.mjs [--json] [--live-probe]
//   node scripts/discovery-gap-audit.mjs --min-catalog-recall 0.5
//
// Always read-only. Compares fixtures/discovery-ground-truth.json against the
// public catalog API (and optional live source probes).

import {
  buildReport,
  catalogProbePublic,
  liveProbeClawhub,
  liveProbeGithubRepo,
  loadGroundTruth,
} from "./discovery-gap-lib.mjs";

const args = process.argv.slice(2);
const JSON_OUT = args.includes("--json");
const LIVE_PROBE = args.includes("--live-probe");

function parseMinCatalogRecall(argv) {
  const idx = argv.indexOf("--min-catalog-recall");
  if (idx < 0) return null;
  const raw = argv[idx + 1];
  if (raw == null || raw.startsWith("--")) {
    console.error("discovery-gap-audit: --min-catalog-recall requires a numeric value in [0, 1]");
    process.exit(2);
  }
  const value = Number(raw);
  if (!Number.isFinite(value) || value < 0 || value > 1) {
    console.error(`discovery-gap-audit: invalid --min-catalog-recall value: ${raw}`);
    process.exit(2);
  }
  return value;
}

const MIN_RECALL = parseMinCatalogRecall(args);

async function liveProbeForTool(tool) {
  const probes = [];

  if (tool.id === "tiny-place") {
    probes.push({ source: "clawhub", ...(await liveProbeClawhub("tinyplace")) });
  }

  if (tool.match?.repo_url) {
    probes.push({
      source: "github",
      ...(await liveProbeGithubRepo(tool.match.repo_url)),
    });
  }

  const found = probes.some((p) => p.found);
  return { found, probes };
}

async function main() {
  const groundTruth = loadGroundTruth();
  const results = [];

  for (const tool of groundTruth.tools) {
    const catalog = await catalogProbePublic(tool);
    const entry = {
      id: tool.id,
      slug: tool.slug,
      catalog,
      notes: tool.notes,
      expected_sources: tool.expected_sources || [],
    };

    if (LIVE_PROBE) {
      entry.live = await liveProbeForTool(tool);
    }

    results.push(entry);

    if (!JSON_OUT) {
      const status = catalog.found ? "IN_CATALOG" : "MISSING";
      const via = catalog.found
        ? ` via slug=${catalog.slug}${catalog.matched_via ? ` (${catalog.matched_via})` : ""}`
        : "";
      console.log(`${status} ${tool.id}${via}`);
      if (LIVE_PROBE && entry.live) {
        const liveStatus = entry.live.found ? "LIVE_OK" : "LIVE_MISS";
        console.log(`  ${liveStatus} ${tool.id}`);
      }
    }
  }

  const report = buildReport(results);

  if (JSON_OUT) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    const { catalog_recall, catalog_hits, total } = report.metrics;
    console.log("");
    console.log(
      `catalog_recall: ${catalog_hits}/${total} (${(catalog_recall * 100).toFixed(1)}%)`,
    );
  }

  if (MIN_RECALL != null && report.metrics.catalog_recall < MIN_RECALL) {
    process.exitCode = 1;
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});