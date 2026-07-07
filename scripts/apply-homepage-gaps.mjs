#!/usr/bin/env node
// apply-homepage-gaps.mjs — bulk homepage + documentation link curation for audit gaps.
//
//   node scripts/apply-homepage-gaps.mjs              # dry-run summary
//   ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/apply-homepage-gaps.mjs
//
// Updates tools.homepage (never repo_url) and upserts tool_official_links (Documentation/Product).

import { loadEnv, connectPg, runInTransaction } from "./seed-tool-lib.mjs";
import {
  FIRST_PARTY_ORGS,
  ORG_BRANDS,
  isGithubUrl,
  npmScope,
  parseGithubRepo,
  resolvePatch,
} from "./homepage-curate-lib.mjs";

const APPLY = process.env.SEED_ENV === "prod-curate";

/** npm scoped packages: keep leading @, encode only the scope/name slash. */
function npmRegistryPackagePath(pkg) {
  const name = pkg.startsWith("@") ? pkg : pkg.split("/")[0];
  if (name.startsWith("@") && name.includes("/")) {
    return name.replace("/", "%2F");
  }
  return encodeURIComponent(name);
}

async function fetchNpmHomepage(pkg) {
  try {
    const res = await fetch(
      `https://registry.npmjs.org/${npmRegistryPackagePath(pkg)}`,
      { headers: { Accept: "application/json" } },
    );
    if (!res.ok) return null;
    const meta = await res.json();
    const home =
      meta.homepage ||
      (typeof meta.repository === "object" ? meta.repository.url : meta.repository);
    if (!home || isGithubUrl(home)) return null;
    return home.replace(/^git\+/, "").replace(/\.git$/, "");
  } catch {
    return null;
  }
}

function classifyIssues(tool, links) {
  const issues = [];
  const gh = parseGithubRepo(tool.repo_url);
  const firstParty = gh && FIRST_PARTY_ORGS[gh.org.toLowerCase()];
  if (tool.homepage && tool.homepage === tool.repo_url && gh) {
    issues.push("homepage_equals_github_repo");
  }
  if (firstParty && isGithubUrl(tool.homepage)) {
    issues.push("first_party_github_only_homepage");
  }
  if (!links.some((l) => l.display_label === "Documentation")) {
    issues.push("missing_docs_link");
  }
  if (isGithubUrl(tool.homepage)) {
    issues.push("github_homepage");
  }
  if (!tool.homepage && !tool.repo_url) {
    issues.push("no_homepage_or_repo");
  }
  return issues;
}

async function buildPatch(tool, existingLinks) {
  const base = resolvePatch(tool);
  let homepage = base?.homepage ?? null;
  const links = [...(base?.links ?? [])];

  if (!homepage && isGithubUrl(tool.homepage) && tool.npm_package) {
    const npmHome = await fetchNpmHomepage(tool.npm_package);
    if (npmHome) homepage = npmHome;
  }

  const gh = parseGithubRepo(tool.repo_url);
  const orgKey = gh?.org?.toLowerCase();
  const brand = orgKey ? ORG_BRANDS[orgKey] : null;

  if (!homepage && isGithubUrl(tool.homepage) && brand?.homepage) {
    homepage = brand.homepage;
  }

  if (
    links.length === 0 &&
    brand?.docs &&
    !existingLinks.some((l) => l.display_label === "Documentation")
  ) {
    links.push({ label: "Documentation", url: brand.docs });
  }

  const existingDoc = existingLinks.find((l) => l.display_label === "Documentation");
  const newDoc = links.find((l) => l.label === "Documentation");

  if (!homepage && !newDoc && !existingDoc) return null;

  if (!homepage && newDoc && tool.homepage && !isGithubUrl(tool.homepage)) {
    return {
      slug: tool.slug,
      homepage: null,
      links,
      reason: base?.reason ?? (brand ? `org:${orgKey}` : "docs-only"),
    };
  }

  if (!homepage && !newDoc) return null;

  return {
    slug: tool.slug,
    homepage,
    links,
    reason: base?.reason ?? (brand ? `org:${orgKey}` : "npm/brand-infer"),
  };
}

const UPDATE_HOME = `
UPDATE tools SET homepage = $1, updated_at = now()
WHERE slug = $2 AND (homepage IS DISTINCT FROM $1)
RETURNING slug;
`;

const UPSERT_LINK = `
INSERT INTO tool_official_links (
  tool_id, link_type, url, display_label, verification_status,
  official_badge_allowed, evidence_strength, discovered_from, verified_at
)
VALUES ($1, 'website', $2, $3, 'verified', true, 'strong', 'homepage-gap-audit', now())
ON CONFLICT (tool_id, link_type, url) DO UPDATE SET
  display_label = EXCLUDED.display_label,
  verification_status = EXCLUDED.verification_status,
  official_badge_allowed = EXCLUDED.official_badge_allowed,
  evidence_strength = EXCLUDED.evidence_strength,
  discovered_from = EXCLUDED.discovered_from,
  verified_at = now(),
  updated_at = now();
`;

async function main() {
  const env = loadEnv();
  const client = await connectPg(env);

  try {
    const { rows: tools } = await client.query(`
      SELECT id, slug, name, homepage, repo_url, npm_package, source, source_url, status
      FROM tools
      WHERE approval_status = 'approved'
        AND relevance_status = 'accepted'
        AND install_risk_level <> 'critical'
        AND quarantined_at IS NULL
      ORDER BY slug
    `);

    const toolIds = tools.map((t) => t.id);
    const { rows: allLinks } = toolIds.length
      ? await client.query(
          `SELECT tool_id, display_label, url FROM tool_official_links WHERE tool_id = ANY($1::uuid[])`,
          [toolIds],
        )
      : { rows: [] };
    const linksByTool = new Map();
    for (const l of allLinks) {
      const arr = linksByTool.get(l.tool_id) || [];
      arr.push(l);
      linksByTool.set(l.tool_id, arr);
    }

    const patches = [];
    for (const tool of tools) {
      const existing = linksByTool.get(tool.id) || [];
      const issues = classifyIssues(tool, existing);
      if (!issues.length) continue;
      const patch = await buildPatch(tool, existing);
      if (!patch) continue;
      patches.push({ ...patch, tool_id: tool.id, issues });
      if (patches.length % 25 === 0) {
        await new Promise((r) => setTimeout(r, 50));
      }
    }

    if (!APPLY) {
      console.log(
        JSON.stringify(
          {
            ok: true,
            mode: "dry-run",
            candidates: patches.length,
            sample: patches.slice(0, 20),
            apply_hint:
              "ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/apply-homepage-gaps.mjs",
          },
          null,
          2,
        ),
      );
      return;
    }

    const results = await runInTransaction(client, async () => {
      let homeUpdates = 0;
      let linkUpserts = 0;
      for (const patch of patches) {
        if (patch.homepage) {
          const r = await client.query(UPDATE_HOME, [patch.homepage, patch.slug]);
          homeUpdates += r.rowCount;
        }
        for (const link of patch.links) {
          await client.query(UPSERT_LINK, [
            patch.tool_id,
            link.url,
            link.label,
          ]);
          linkUpserts += 1;
        }
        await client.query(
          `INSERT INTO tool_review_events (tool_id, admin_id, action, reason)
           VALUES ($1, NULL, 'metadata_curate', $2)`,
          [
            patch.tool_id,
            `apply-homepage-gaps: ${patch.reason}; issues=${patch.issues.join(",")}`,
          ],
        );
      }
      return { homeUpdates, linkUpserts, tools: patches.length };
    });

    console.log(JSON.stringify({ ok: true, mode: "apply", ...results }, null, 2));
  } finally {
    await client.end();
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});