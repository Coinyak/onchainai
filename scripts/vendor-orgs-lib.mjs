import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const VENDOR_ORGS_PATH = resolve(ROOT, "scripts/vendor-orgs.json");

/** Parse scripts/vendor-orgs.json (shared by verify harness and PR-4 crawler). */
export function loadVendorOrgsManifest() {
  return JSON.parse(readFileSync(VENDOR_ORGS_PATH, "utf8"));
}

/** GitHub org login → official team label (verify-tool-official FIRST_PARTY_ORGS shape). */
export function loadFirstPartyOrgs() {
  const manifest = loadVendorOrgsManifest();
  return Object.fromEntries(
    manifest.orgs.map((entry) => [entry.github.toLowerCase(), entry.team]),
  );
}