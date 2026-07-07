#!/usr/bin/env node
/**
 * Write migrations/039_canonicalize_chain_aliases_full.sql from src/chains.rs CHAIN_CATALOG.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const src = readFileSync(resolve(ROOT, "src/chains.rs"), "utf8");

const entries = [];
const blockRe = /ChainMeta\s*\{[^}]*id:\s*"([^"]+)"[^}]*aliases:\s*&\[([^\]]*)\]/gs;
let m;
while ((m = blockRe.exec(src)) !== null) {
  const id = m[1];
  const aliasRaw = m[2];
  const aliases = [...aliasRaw.matchAll(/"([^"]+)"/g)].map((x) => x[1]);
  for (const alias of aliases) {
    entries.push([alias.toLowerCase(), id]);
  }
}

entries.sort((a, b) => a[0].localeCompare(b[0]) || a[1].localeCompare(b[1]));

const valueLines = entries
  .map(([alias, canonical]) => `        ('${alias.replace(/'/g, "''")}', '${canonical}')`)
  .join(",\n");

const sql = `-- Full tools.chains[] canonicalization from CHAIN_CATALOG aliases.
-- Regenerate: node scripts/gen-chain-canonical-migration.mjs
-- Complements 038 (explicit family rewrites); catalog-complete + lowercase trim.

UPDATE tools
SET chains = ARRAY(
    SELECT DISTINCT COALESCE(m.canonical, lower(trim(elem)))
    FROM unnest(chains) AS t(elem)
    LEFT JOIN (
      VALUES
${valueLines}
    ) AS m(alias, canonical) ON lower(trim(elem)) = m.alias
),
    updated_at = now()
WHERE chains IS NOT NULL AND cardinality(chains) > 0;
`;

const outPath = resolve(ROOT, "migrations/039_canonicalize_chain_aliases_full.sql");
writeFileSync(outPath, sql);
console.log(`Wrote ${outPath} (${entries.length} alias rows)`);