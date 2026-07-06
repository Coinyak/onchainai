#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const root = process.cwd();
const args = new Set(process.argv.slice(2));
const jsonMode = args.has("--json");
const strictMode = args.has("--strict");
const strictCiMode = args.has("--strict-ci");
const inCi = process.env.CI === "true" || process.env.GITHUB_ACTIONS === "true";

const levels = [
  { id: 1, name: "Functional Foundations" },
  { id: 2, name: "Documented Workflow" },
  { id: 3, name: "Reliable Automation" },
  { id: 4, name: "Operational Safety" },
  { id: 5, name: "Autonomous Scale" },
];

const criteria = [];
const envChecks = [];

function run(cmd, commandArgs = [], options = {}) {
  const result = spawnSync(cmd, commandArgs, {
    cwd: root,
    encoding: "utf8",
    timeout: options.timeout ?? 8000,
    shell: false,
  });
  return {
    status: result.status,
    ok: result.status === 0,
    stdout: (result.stdout ?? "").trim(),
    stderr: (result.stderr ?? "").trim(),
    error: result.error?.message ?? "",
  };
}

function commandPath(command) {
  const result = run("bash", ["-lc", `command -v ${shellQuote(command)}`], {
    timeout: 3000,
  });
  return result.ok ? result.stdout.split("\n")[0] : "";
}

function shellQuote(value) {
  return `'${String(value).replace(/'/g, `'\\''`)}'`;
}

function text(file) {
  try {
    return readFileSync(file, "utf8");
  } catch {
    return "";
  }
}

function has(file) {
  return existsSync(path.join(root, file));
}

function executable(file) {
  try {
    return (statSync(path.join(root, file)).mode & 0o111) !== 0;
  } catch {
    return false;
  }
}

function contains(file, pattern) {
  const body = text(path.join(root, file));
  return pattern instanceof RegExp ? pattern.test(body) : body.includes(pattern);
}

function anyContains(files, pattern) {
  return files.some((file) => contains(file, pattern));
}

function listFiles(dir) {
  const start = path.join(root, dir);
  if (!existsSync(start)) return [];
  const out = [];
  const walk = (current) => {
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const full = path.join(current, entry.name);
      const rel = path.relative(root, full);
      if (entry.isDirectory()) {
        if (["target", "node_modules", ".git"].includes(entry.name)) continue;
        walk(full);
      } else {
        out.push(rel);
      }
    }
  };
  walk(start);
  return out;
}

function addCriterion(level, pillar, name, passed, evidence, action) {
  criteria.push({
    level,
    pillar,
    name,
    passed: Boolean(passed),
    evidence,
    action,
  });
}

function addEnv(status, area, detail, action = "") {
  envChecks.push({ status, area, detail, action });
}

const agentsText = text(path.join(root, "AGENTS.md"));
const buildDocs = text(path.join(root, "docs/BUILD_DEPLOY_RULES.md"));
const harnessDocs = text(path.join(root, "docs/AGENT_HARNESS.md"));
const gitignore = text(path.join(root, ".gitignore"));
const cargoToml = text(path.join(root, "Cargo.toml"));
const sourceFiles = listFiles("src");
const docsFiles = listFiles("docs");
const scriptsFiles = listFiles("scripts");
const migrationsFiles = listFiles("migrations");

const agentsLines = agentsText ? (agentsText.match(/\n/g) ?? []).length : 0;
const trackedEnv = run("git", ["ls-files", ".env"]).stdout;
const dirtyCount = run("git", ["status", "--short"]).stdout
  .split("\n")
  .filter(Boolean).length;

const cargo = commandPath("cargo");
const node = commandPath("node");
const rg = commandPath("rg");
const curl = commandPath("curl");
const rustup = commandPath("rustup");
const cargoLeptos = run("cargo", ["leptos", "--version"], { timeout: 8000 });
const wasmTarget = rustup
  ? run("rustup", ["target", "list", "--installed"], { timeout: 8000 }).stdout
      .split("\n")
      .includes("wasm32-unknown-unknown")
  : false;
const playwrightImport = run(
  "node",
  [
    "--input-type=module",
    "-e",
    'try { await import("playwright"); } catch { process.exit(1); }',
  ],
  { timeout: 8000 },
);

const df = run("df", ["-Pk", "."], { timeout: 5000 }).stdout.split("\n")[1] ?? "";
const freeKb = Number(df.trim().split(/\s+/)[3] ?? 0);
const freeGb = Math.floor(freeKb / 1024 / 1024);
const targetSizeKb = has("target")
  ? Number(run("du", ["-sk", "target"], { timeout: 10000 }).stdout.split(/\s+/)[0] ?? 0)
  : 0;
const targetGb = Math.floor(targetSizeKb / 1024 / 1024);

const hasReleaseBundle =
  has("target/release/onchainai") &&
  has("target/site/pkg/onchainai.js") &&
  has("target/site/pkg/onchainai.wasm") &&
  has("style/output.css");
const bundleVerify = hasReleaseBundle
  ? run("./scripts/verify-bundle.sh", [], { timeout: 15000 })
  : { ok: false, stdout: "", stderr: "No complete release bundle" };

addCriterion(1, "Build System", "Rust crate manifest exists", has("Cargo.toml"), "Cargo.toml present", "Restore Cargo.toml.");
addCriterion(1, "Build System", "Dependency lockfile exists", has("Cargo.lock"), "Cargo.lock present", "Commit Cargo.lock for reproducible builds.");
addCriterion(1, "Build System", "Container deploy path exists", has("Dockerfile") && has("railway.json"), "Dockerfile and railway.json present", "Add/restore Dockerfile and railway.json.");
addCriterion(1, "Development Environment", "Required cargo command is available", Boolean(cargo), cargo || "cargo missing", "Install Rust/cargo.");
addCriterion(1, "Development Environment", "Required node command is available", Boolean(node), node || "node missing", "Install Node.js for browser smoke scripts.");
addCriterion(1, "Style & Validation", "Formatter command is routed", agentsText.includes("cargo fmt --check"), "AGENTS.md mentions cargo fmt --check", "Route formatter command from AGENTS.md.");
addCriterion(1, "Style & Validation", "Clippy command is routed", agentsText.includes("cargo clippy --features ssr"), "AGENTS.md mentions clippy", "Route clippy command from AGENTS.md.");
addCriterion(1, "Testing", "Rust test command is routed", agentsText.includes("cargo test --features ssr"), "AGENTS.md mentions cargo test --features ssr", "Route test command from AGENTS.md.");
addCriterion(1, "Testing", "Test directory exists", has("tests") || sourceFiles.some((f) => f.endsWith(".rs") && contains(f, "#[test]")), "tests/ or Rust unit tests present", "Add focused tests for changed behavior.");
addCriterion(1, "Documentation", "README exists", has("README.md"), "README.md present", "Add README.md.");
addCriterion(1, "Security", ".env is ignored and untracked", gitignore.includes(".env") && !trackedEnv, ".env ignored; git ls-files .env empty", "Ignore and untrack .env.");

addCriterion(2, "Agent Harness", "AGENTS.md stays below 70 lines", agentsLines > 0 && agentsLines < 70, `${agentsLines} lines`, "Keep AGENTS.md as a short router.");
addCriterion(2, "Agent Harness", "Agent harness doc exists", has("docs/AGENT_HARNESS.md"), "docs/AGENT_HARNESS.md present", "Add docs/AGENT_HARNESS.md.");
addCriterion(2, "Agent Harness", "AGENTS routes to agent harness", agentsText.includes("docs/AGENT_HARNESS.md"), "AGENTS.md links docs/AGENT_HARNESS.md", "Add route link to AGENTS.md.");
addCriterion(2, "Documentation", "Design docs exist", has("DESIGN.md") && has("docs/UI_UX_DESIGN.md"), "DESIGN.md and UI_UX_DESIGN.md present", "Restore design docs.");
addCriterion(2, "Documentation", "Build/deploy rules exist", has("docs/BUILD_DEPLOY_RULES.md"), "BUILD_DEPLOY_RULES.md present", "Restore build/deploy rules.");
addCriterion(2, "Documentation", "Security doc exists", has("docs/SECURITY.md"), "SECURITY.md present", "Restore security doc.");
addCriterion(2, "Documentation", "Architecture doc exists", has("docs/MVP_DESIGN.md"), "MVP_DESIGN.md present", "Restore architecture doc.");
addCriterion(2, "Documentation", "Docs index routes agent readiness", contains("docs/INDEX.md", "AGENT_READINESS_REPORT"), "docs/INDEX.md links readiness report", "Add readiness report to docs index.");
addCriterion(2, "Development Environment", ".env template exists", has(".env.example"), ".env.example present", "Add .env.example with non-secret placeholders.");
addCriterion(2, "Security", "x402 guardrails are documented", has("docs/X402_REFERRAL_SPEC.md") && agentsText.includes("attribution only"), "x402 docs and AGENTS hard rule present", "Restore x402 referral/trust guardrails.");

addCriterion(3, "Agent Harness", "Agent harness check is executable", executable("scripts/agent-harness-check.sh"), "scripts/agent-harness-check.sh executable", "chmod +x scripts/agent-harness-check.sh.");
addCriterion(
  3,
  "Agent Harness",
  "UI staleness self-test exists",
  executable("scripts/test-ui-staleness-check.sh"),
  "scripts/test-ui-staleness-check.sh executable",
  "chmod +x scripts/test-ui-staleness-check.sh.",
);
addCriterion(
  3,
  "Agent Harness",
  "Harness check references staleness self-test",
  contains("scripts/agent-harness-check.sh", "test-ui-staleness-check.sh"),
  "agent-harness-check.sh runs test-ui-staleness-check.sh",
  "Wire test-ui-staleness-check.sh into agent-harness-check.sh.",
);
addCriterion(3, "Agent Harness", "UI change gate is executable", executable("scripts/ui-change-gate.sh"), "scripts/ui-change-gate.sh executable", "chmod +x scripts/ui-change-gate.sh.");
addCriterion(3, "Agent Harness", "Fast UI watch loop exists", executable("scripts/dev-watch.sh"), "scripts/dev-watch.sh executable", "chmod +x scripts/dev-watch.sh.");
addCriterion(3, "Agent Harness", "UI staleness checker exists", executable("scripts/ui-staleness-check.sh"), "scripts/ui-staleness-check.sh executable", "chmod +x scripts/ui-staleness-check.sh.");
addCriterion(3, "Agent Harness", "Universal git pre-commit hook exists", executable("scripts/git-hooks/pre-commit") && contains("scripts/git-hooks/pre-commit", "ui-staleness-check.sh"), "scripts/git-hooks/pre-commit present", "Add git pre-commit hook for tool-agnostic UI staleness.");
addCriterion(3, "Agent Harness", "Optional IDE stop hooks are committed", has(".cursor/hooks.json") && has(".claude/settings.json") && contains(".cursor/hooks.json", "ui-staleness-stop.sh") && contains(".claude/settings.json", "ui-staleness-check.sh"), "Cursor + Claude hook configs present", "Commit .cursor/hooks.json and .claude/settings.json.");
addCriterion(3, "Agent Harness", "Agent hook installer exists", executable("scripts/install-agent-hooks.sh"), "scripts/install-agent-hooks.sh executable", "chmod +x scripts/install-agent-hooks.sh.");
addCriterion(3, "Agent Harness", "Readiness report is executable", executable("scripts/agent-readiness-report.sh"), "scripts/agent-readiness-report.sh executable", "chmod +x scripts/agent-readiness-report.sh.");
addCriterion(3, "Build System", "Coherent release build script exists", executable("scripts/release-build.sh"), "release-build.sh executable", "Restore release-build.sh.");
addCriterion(3, "Build System", "Bundle verifier exists", executable("scripts/verify-bundle.sh"), "verify-bundle.sh executable", "Restore verify-bundle.sh.");
addCriterion(3, "Build System", "Restart helper exists", executable("scripts/restart-dev.sh"), "restart-dev.sh executable", "Restore restart-dev.sh.");
addCriterion(3, "Testing", "Curl smoke test exists", executable("scripts/smoke-test.sh"), "smoke-test.sh executable", "Restore smoke-test.sh.");
addCriterion(3, "Testing", "Browser smoke test exists", has("scripts/browser-smoke.mjs"), "browser-smoke.mjs present", "Restore browser smoke.");
addCriterion(3, "Testing", "Visual snapshot script exists", has("scripts/visual-snapshots.mjs"), "visual-snapshots.mjs present", "Restore visual snapshots.");
addCriterion(3, "Testing", "Local auth smoke exists", has("scripts/local-auth-smoke.mjs"), "local-auth-smoke.mjs present", "Restore local auth smoke.");
addCriterion(
  3,
  "Development Environment",
  "cargo-leptos is runnable",
  strictCiMode ? true : cargoLeptos.ok,
  strictCiMode ? "skipped in strict-ci (ui-coherence covers leptos build)" : cargoLeptos.stdout || cargoLeptos.stderr || "cargo leptos missing",
  "Install cargo-leptos.",
);
addCriterion(3, "Development Environment", "Rust wasm target is installed", wasmTarget, wasmTarget ? "wasm32-unknown-unknown installed" : "missing", "rustup target add wasm32-unknown-unknown.");
addCriterion(3, "Testing", "Playwright package is importable", playwrightImport.ok, playwrightImport.ok ? "node can import playwright" : "playwright import failed", "Install Playwright for browser and visual QA.");
addCriterion(3, "Build System", "Release bundle is coherent when present", !hasReleaseBundle || bundleVerify.ok, hasReleaseBundle ? (bundleVerify.ok ? "verify-bundle passes" : bundleVerify.stderr || bundleVerify.stdout) : "no complete release bundle yet", "Run ./scripts/release-build.sh then ./scripts/verify-bundle.sh.");

addCriterion(4, "Security", "RLS policy documentation exists", contains("docs/SECURITY.md", "RLS"), "SECURITY.md documents RLS", "Document RLS policy expectations.");
addCriterion(4, "Security", "Database migrations exist", migrationsFiles.length > 0, `${migrationsFiles.length} migration files`, "Add migrations for DB changes.");
addCriterion(4, "Security", "Secret redaction module exists", has("src/server/secret_redaction.rs"), "secret_redaction.rs present", "Add server-side secret redaction.");
addCriterion(4, "Security", "Secret redaction has tests", contains("src/server/secret_redaction.rs", "#[test]") || contains("src/server/functions.rs", "assert_json_has_no_secrets"), "secret redaction tests found", "Add tests proving secrets do not leave server responses.");
addCriterion(4, "Debugging & Observability", "Tracing dependencies are present", /tracing/.test(cargoToml), "Cargo.toml includes tracing", "Add structured tracing/logging.");
addCriterion(4, "Debugging & Observability", "Post-deploy verification exists", executable("scripts/post-deploy-verify.sh"), "post-deploy-verify.sh executable", "Restore post-deploy verification.");
addCriterion(4, "Development Environment", "Disk guard exists", executable("scripts/disk-guard.sh"), "disk-guard.sh executable", "Restore disk guard.");
addCriterion(4, "Build System", "Deploy script exists", executable("scripts/deploy-railway.sh"), "deploy-railway.sh executable", "Restore deploy script.");
addCriterion(4, "Security", "Admin server-side guard code exists", has("src/auth/guard.rs") && contains("AGENTS.md", "server-side"), "auth guard and AGENTS rule present", "Keep admin checks server-side.");
addCriterion(4, "Testing", "Verification plan script exists", executable("scripts/run-verification-plan.sh"), "run-verification-plan.sh executable", "Restore verification plan script.");
const workflowFiles = listFiles(".github/workflows");
const ciWorkflowContent = workflowFiles.map((f) => text(path.join(root, f))).join("\n");
const ciHasRustGates =
  /cargo fmt/.test(ciWorkflowContent) &&
  /clippy/.test(ciWorkflowContent) &&
  /agent-harness-check/.test(ciWorkflowContent);
addCriterion(
  4,
  "Style & Validation",
  "CI workflow exists",
  workflowFiles.length > 0 && ciHasRustGates,
  workflowFiles.length > 0
    ? ciHasRustGates
      ? "workflows present with fmt/clippy/harness"
      : "workflows present but missing fmt/clippy/harness references"
    : ".github/workflows missing",
  "Add .github/workflows/ci.yml with fmt, clippy, tests, and agent-harness-check.",
);

addCriterion(5, "Task Discovery", "Docs contain implementation specs/plans", docsFiles.some((f) => f.includes("superpowers/specs")) || docsFiles.some((f) => f.includes("superpowers/plans")), "superpowers specs/plans present", "Create durable specs/plans for agent task discovery.");
addCriterion(5, "Task Discovery", "Operator guide exists", has("docs/OPERATOR_GUIDE.md"), "OPERATOR_GUIDE.md present", "Add operator guide.");
addCriterion(5, "Task Discovery", "Agent review harness spec exists", docsFiles.some((f) => f.includes("agent-review-harness")), "agent review harness spec present", "Add external-agent review harness spec.");
addCriterion(5, "Product & Experimentation", "Dashboard metrics surface exists", has("src/pages/dashboard.rs") && contains("src/server/functions.rs", "DashboardMetrics"), "dashboard metrics code present", "Add product/operator metrics surface.");
addCriterion(5, "Product & Experimentation", "Toolkit surface exists", has("src/pages/toolkit.rs"), "toolkit page present", "Add agent-ready toolkit surface.");
addCriterion(5, "Agent Harness", "Readiness report model exists", has("docs/AGENT_READINESS_REPORT.md") && has("scripts/agent-readiness-report.mjs"), "readiness docs and model present", "Keep readiness report model in docs and scripts.");
addCriterion(5, "Style & Validation", "CODEOWNERS exists", has(".github/CODEOWNERS") || has("CODEOWNERS"), has(".github/CODEOWNERS") || has("CODEOWNERS") ? "CODEOWNERS present" : "CODEOWNERS missing", "Add CODEOWNERS for review routing.");
addCriterion(5, "Task Discovery", "Issue or PR templates exist", has(".github/ISSUE_TEMPLATE") || has(".github/pull_request_template.md") || has(".github/PULL_REQUEST_TEMPLATE.md"), has(".github/ISSUE_TEMPLATE") || has(".github/pull_request_template.md") || has(".github/PULL_REQUEST_TEMPLATE.md") ? "GitHub templates present" : "GitHub issue/PR templates missing", "Add issue/PR templates for agent task intake.");
addCriterion(
  5,
  "Testing",
  "CI invokes agent readiness or UI gate",
  anyContains(workflowFiles, "agent-readiness-report") ||
    anyContains(workflowFiles, "ui-change-gate") ||
    anyContains(workflowFiles, "agent-harness-check") ||
    anyContains(workflowFiles, "test-ui-staleness-check"),
  anyContains(workflowFiles, "agent-readiness-report") ||
    anyContains(workflowFiles, "ui-change-gate") ||
    anyContains(workflowFiles, "agent-harness-check") ||
    anyContains(workflowFiles, "test-ui-staleness-check")
    ? "CI workflow gate references present"
    : "CI workflows do not reference readiness/UI/harness gates",
  "Wire readiness/gates into .github/workflows CI.",
);

addEnv(cargo ? "PASS" : "FAIL", "Tool: cargo", cargo || "missing", cargo ? "" : "Install Rust/cargo.");
addEnv(node ? "PASS" : "FAIL", "Tool: node", node || "missing", node ? "" : "Install Node.js.");
addEnv(rg ? "PASS" : "WARN", "Tool: rg", rg || "missing", rg ? "" : "Install ripgrep for fast code search.");
addEnv(curl ? "PASS" : "FAIL", "Tool: curl", curl || "missing", curl ? "" : "Install curl.");
addEnv(rustup ? "PASS" : "WARN", "Tool: rustup", rustup || "missing", rustup ? "" : "Install rustup for target management.");
addEnv(
  inCi ? "PASS" : freeGb >= 25 ? "PASS" : "FAIL",
  "Disk free",
  `${freeGb}GB available`,
  inCi || freeGb >= 25 ? "" : "Free disk or run cleanup before release builds.",
);
addEnv(
  inCi ? "PASS" : targetGb <= 35 ? "PASS" : "WARN",
  "target size",
  has("target") ? `${targetGb}GB` : "target/ missing",
  inCi || targetGb <= 35 ? "" : "./scripts/clean-build-artifacts.sh --incremental-only",
);
addEnv(
  inCi ? "PASS" : dirtyCount === 0 ? "PASS" : "WARN",
  "Git worktree",
  dirtyCount === 0 ? "clean" : `${dirtyCount} changed paths`,
  inCi || dirtyCount === 0 ? "" : "Protect unrelated changes; inspect diffs before editing.",
);
addEnv(
  inCi ? "PASS" : has(".env") ? "PASS" : "WARN",
  "Local env",
  has(".env") ? ".env present" : ".env missing",
  inCi || has(".env") ? "" : "Create local .env from .env.example when DB/auth checks are needed.",
);

function summarizeByLevel() {
  return levels.map((level) => {
    const rows = criteria.filter((item) => item.level === level.id);
    const passed = rows.filter((item) => item.passed).length;
    const total = rows.length;
    const rate = total ? Math.round((passed / total) * 100) : 0;
    return { ...level, passed, total, rate, unlocked: rate >= 80 };
  });
}

function summarizeByPillar() {
  const map = new Map();
  for (const item of criteria) {
    const bucket = map.get(item.pillar) ?? { pillar: item.pillar, passed: 0, total: 0 };
    bucket.total += 1;
    if (item.passed) bucket.passed += 1;
    map.set(item.pillar, bucket);
  }
  return [...map.values()]
    .map((bucket) => ({
      ...bucket,
      rate: Math.round((bucket.passed / bucket.total) * 100),
    }))
    .sort((a, b) => a.pillar.localeCompare(b.pillar));
}

const levelSummary = summarizeByLevel();
let achievedLevel = 1;
for (const level of levelSummary) {
  if (level.unlocked) achievedLevel = level.id;
  else break;
}

const envFailures = envChecks.filter((item) => item.status === "FAIL").length;
const envWarnings = envChecks.filter((item) => item.status === "WARN").length;
const levelOneReady = levelSummary[0]?.rate >= 80;
const levelTwoReady = levelSummary[1]?.rate >= 80;
const levelThreeReady = levelSummary[2]?.rate >= 80;

let overall = "READY";
if (!levelOneReady || !levelTwoReady || envFailures > 0) {
  overall = "NOT READY";
} else if (!levelThreeReady || envWarnings > 0 || levelSummary.some((level) => !level.unlocked)) {
  overall = "READY WITH WARNINGS";
}

const failedCriteria = criteria.filter((item) => !item.passed);
const actionItems = failedCriteria
  .sort((a, b) => a.level - b.level || a.pillar.localeCompare(b.pillar))
  .slice(0, 12);

const applications = [];
if (has("Cargo.toml") && cargoToml.includes("leptos")) {
  applications.push({
    name: "OnchainAI web app",
    type: "Rust Leptos SSR + Axum single binary",
    evidence: "Cargo.toml includes Leptos/Axum stack",
  });
}
if (has("src/server/mcp.rs")) {
  applications.push({
    name: "OnchainAI MCP server",
    type: "rmcp server surface",
    evidence: "src/server/mcp.rs present",
  });
}

function mdEscape(value) {
  return String(value ?? "").replace(/\n/g, " ").replace(/\|/g, "\\|");
}

function mdTable(rows, columns) {
  const lines = [];
  lines.push(`| ${columns.map((col) => col.label).join(" | ")} |`);
  lines.push(`| ${columns.map(() => "---").join(" | ")} |`);
  for (const row of rows) {
    lines.push(`| ${columns.map((col) => mdEscape(col.value(row))).join(" | ")} |`);
  }
  return lines.join("\n");
}

const report = {
  generatedAt: new Date().toISOString(),
  root,
  overall,
  achievedLevel,
  achievedLevelName: levels.find((level) => level.id === achievedLevel)?.name,
  applications,
  levelSummary,
  pillarSummary: summarizeByPillar(),
  environment: envChecks,
  criteria,
  actionItems,
};

if (jsonMode) {
  console.log(JSON.stringify(report, null, 2));
} else {
  console.log("# OnchainAI Agent Readiness Report");
  console.log("");
  console.log(`- Generated: ${report.generatedAt}`);
  console.log(`- Root: ${root}`);
  console.log(`- Overall: ${overall}`);
  console.log(`- Level Achieved: Level ${achievedLevel} - ${report.achievedLevelName}`);
  console.log(`- Criteria Passed: ${criteria.filter((item) => item.passed).length}/${criteria.length}`);
  console.log(`- Environment Failures: ${envFailures}`);
  console.log(`- Environment Warnings: ${envWarnings}`);
  console.log("");

  console.log("## Applications Discovered");
  console.log("");
  console.log(
    applications.length
      ? mdTable(applications, [
          { label: "Application", value: (row) => row.name },
          { label: "Type", value: (row) => row.type },
          { label: "Evidence", value: (row) => row.evidence },
        ])
      : "No application surfaces discovered.",
  );
  console.log("");

  console.log("## Level Progression");
  console.log("");
  console.log(
    mdTable(levelSummary, [
      { label: "Level", value: (row) => `L${row.id}` },
      { label: "Name", value: (row) => row.name },
      { label: "Score", value: (row) => `${row.passed}/${row.total}` },
      { label: "Rate", value: (row) => `${row.rate}%` },
      { label: "Unlocked", value: (row) => (row.unlocked ? "yes" : "no") },
    ]),
  );
  console.log("");

  console.log("## Pillar Breakdown");
  console.log("");
  console.log(
    mdTable(report.pillarSummary, [
      { label: "Pillar", value: (row) => row.pillar },
      { label: "Score", value: (row) => `${row.passed}/${row.total}` },
      { label: "Rate", value: (row) => `${row.rate}%` },
    ]),
  );
  console.log("");

  console.log("## Environment");
  console.log("");
  console.log(
    mdTable(envChecks, [
      { label: "Status", value: (row) => row.status },
      { label: "Area", value: (row) => row.area },
      { label: "Detail", value: (row) => row.detail },
      { label: "Action", value: (row) => row.action },
    ]),
  );
  console.log("");

  console.log("## Action Items");
  console.log("");
  if (actionItems.length) {
    console.log(
      mdTable(actionItems, [
        { label: "Level", value: (row) => `L${row.level}` },
        { label: "Pillar", value: (row) => row.pillar },
        { label: "Criterion", value: (row) => row.name },
        { label: "Action", value: (row) => row.action },
      ]),
    );
  } else {
    console.log("No failed criteria.");
  }
  console.log("");

  console.log("## Criteria Results");
  console.log("");
  console.log(
    mdTable(criteria, [
      { label: "Status", value: (row) => (row.passed ? "PASS" : "FAIL") },
      { label: "Level", value: (row) => `L${row.level}` },
      { label: "Pillar", value: (row) => row.pillar },
      { label: "Criterion", value: (row) => row.name },
      { label: "Evidence", value: (row) => row.evidence },
    ]),
  );
  console.log("");
}

if (strictMode && overall === "NOT READY") {
  process.exit(1);
}

if (strictCiMode) {
  const blocking = criteria.filter((item) => item.level <= 4 && !item.passed);
  if (blocking.length > 0) {
    console.error(`READINESS STRICT-CI FAIL: ${blocking.length} L1-L4 criteria failed`);
    process.exit(1);
  }
}
