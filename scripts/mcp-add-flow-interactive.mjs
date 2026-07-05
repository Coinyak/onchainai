/**
 * MCP add flow interactive verification — full transcript + screenshots.
 * Usage: node scripts/mcp-add-flow-interactive.mjs [baseUrl] [scratchDir]
 */
import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "fs";
import { join } from "path";

const base = (process.argv[2] || "http://127.0.0.1:3000").replace(/\/$/, "");
const scratch = process.argv[3] || ".playwright-cli/mcp-add-flow";
const snapDir = join(scratch, "ui-snapshots");
mkdirSync(snapDir, { recursive: true });

const transcript = [];
const log = (step, detail) => {
  const line = `[${new Date().toISOString()}] ${step}: ${detail}`;
  transcript.push(line);
  console.log(line);
};

const wait = { waitUntil: "domcontentloaded", timeout: 45000 };
const browser = await chromium.launch({ headless: true });

try {
  log("A1", `GET ${base}/`);
  const home = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await home.goto(`${base}/`, wait);
  const card = home.locator('[data-testid="connect-onchainai-mcp-card"]');
  await card.waitFor({ timeout: 15000 });
  const copyButtons = await card.locator(
    'button[aria-label="Copy config"], button[aria-label="Copy command"]',
  ).count();
  log("A2", `home connect-card copy buttons=${copyButtons}`);
  const moreLink = card.locator('[data-testid="connect-more-clients-link"]');
  await moreLink.waitFor({ timeout: 10000 });
  log("A3", `home connect-more href=${await moreLink.getAttribute("href")}`);
  await home.screenshot({ path: join(snapDir, "01-home-connect-desktop-1280x900.png") });
  await home.setViewportSize({ width: 375, height: 812 });
  await home.screenshot({ path: join(snapDir, "02-home-connect-mobile-375x812.png") });
  await home.close();

  log("A4", `GET ${base}/connect`);
  const connect = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await connect.goto(`${base}/connect`, wait);
  await connect.waitForSelector('[data-testid="connect-page"]', { timeout: 15000 });
  await connect
    .waitForSelector('[data-testid^="connect-client-"]', { timeout: 20000 })
    .catch(() => {});
  const clientCards = await connect.locator('[data-testid^="connect-client-"]').count();
  log("A5", `connect-page client cards=${clientCards}`);
  const universalCopy = await connect
    .locator('button[aria-label="Copy command"]')
    .first()
    .count();
  log("A6", `connect-page universal copy buttons=${universalCopy}`);
  await connect.goto(`${base}/connect#agent-sync`, wait);
  await connect.waitForSelector('[data-testid="agent-link-section"]', { timeout: 15000 });
  const signInBtn = connect.locator('[data-testid="agent-link-sign-in"]');
  const approveBtn = connect.locator('[data-testid="agent-link-approve"]');
  const signInVisible = await signInBtn.isVisible().catch(() => false);
  const approveVisible = await approveBtn.isVisible().catch(() => false);
  log("A7", `agent-sync section signIn=${signInVisible} approve=${approveVisible}`);
  await connect.screenshot({ path: join(snapDir, "02b-connect-agent-sync-desktop.png") });
  await connect.close();

  log("B1", `GET ${base}/tools?type=mcp`);
  const tools = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await tools.goto(`${base}/tools?type=mcp`, wait);
  await tools.waitForSelector('[data-testid="tool-card-link"]', { timeout: 15000 });
  const cardCount = await tools.locator(".tool-card").count();
  const linkCount = await tools.locator('[data-testid="tool-card-link"]').count();
  log("B2", `tools list cards=${cardCount} links=${linkCount}`);

  const slugLinks = await tools.locator('[data-testid="tool-card-link"]').all();
  const derivedSlugs = [];
  for (const link of slugLinks) {
    const href = await link.getAttribute("href");
    if (!href) continue;
    const pathMatch = href.match(/\/tools\/([^/?]+)/);
    const queryMatch = href.match(/[?&]selected=([^&]+)/);
    const slug = queryMatch?.[1] ?? pathMatch?.[1];
    if (!slug) continue;
    const decoded = decodeURIComponent(slug);
    if (!derivedSlugs.includes(decoded)) derivedSlugs.push(decoded);
    if (derivedSlugs.length >= 2) break;
  }
  const selectedSlug = derivedSlugs[0] ?? null;
  const compareSlug = derivedSlugs[1] ?? null;
  if (!selectedSlug) {
    throw new Error("could not derive any tool slug from /tools?type=mcp tool-card links");
  }
  if (!compareSlug) {
    throw new Error("need at least two MCP tool slugs for compare install section smoke");
  }
  log("B2b", `derived slugs=${derivedSlugs.join(",")}`);

  const addUrl = `${base}/tools?type=mcp&selected=${selectedSlug}&intent=add-mcp`;
  log("B3", `GET ${addUrl}`);
  await tools.goto(addUrl, wait);
  await tools.waitForSelector(".install-guide-panel", { state: "attached", timeout: 15000 });
  const panel = tools.locator(".install-guide-panel:visible").first();
  await panel.waitFor({ state: "visible", timeout: 15000 });
  log(
    "B4",
    `add-mode markers addMode=${await tools.locator(".add-mode").count()} progress=${await tools.locator(".install-progress").count()}`,
  );
  await tools.screenshot({ path: join(snapDir, "03-add-mode-desktop-1280x900.png") });

  const platformBtn = panel.locator(".install-platform-btn").first();
  const platformLabel = (await platformBtn.innerText()).trim();
  log("B5", `click ${platformLabel} platform in install guide`);
  await platformBtn.click();
  log(
    "B6",
    `${platformLabel} selected=${await platformBtn.getAttribute("aria-selected")}`,
  );
  log(
    "B7",
    `copy buttons=${await panel.locator('[aria-label="Copy config"], [aria-label="Copy command"]').count()}`,
  );

  await tools.setViewportSize({ width: 375, height: 812 });
  await tools.screenshot({ path: join(snapDir, "04-add-mode-mobile-375x812.png") });
  await tools.close();

  const compareToolsParam = `${selectedSlug},${compareSlug}`;
  log("C1", `GET ${base}/compare?tools=${compareToolsParam}`);
  const compare = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await compare.goto(`${base}/compare?tools=${encodeURIComponent(compareToolsParam)}`, wait);
  const installSection = compare.locator(`[data-testid="compare-install-${selectedSlug}"]`);
  await installSection.waitFor({ state: "attached", timeout: 15000 });
  await installSection.locator("summary").click();
  await compare.locator(".add-mcp-inline-btn:visible").first().waitFor({ timeout: 15000 });
  const addLink = compare.locator(".add-mcp-inline-btn:visible").first();
  const href = await addLink.getAttribute("href");
  log("C2", `compare AddMcpAction href=${href}`);
  await addLink.click();
  await compare.waitForSelector(".install-guide-panel", { state: "attached", timeout: 15000 });
  await compare.locator(".install-guide-panel:visible").first().waitFor({ state: "visible", timeout: 15000 });
  log("C3", `navigated to ${compare.url()}`);
  await compare.screenshot({ path: join(snapDir, "05-compare-add-mode-desktop-1280x900.png") });
  await compare.close();

  log("D1", `GET ${base}/toolkit`);
  const toolkit = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  const tkResp = await toolkit.goto(`${base}/toolkit`, wait);
  log("D2", `toolkit status=${tkResp?.status()}`);
  await toolkit.screenshot({ path: join(snapDir, "06-toolkit-desktop-1280x900.png") });
  await toolkit.close();

  if (copyButtons < 1) throw new Error("home connect card missing copy button");
  if (clientCards < 3) throw new Error(`expected >=3 connect client cards, got ${clientCards}`);
  if (!signInVisible && !approveVisible) {
    throw new Error("agent-sync section missing sign-in or approve CTA on /connect#agent-sync");
  }
  if (!href?.includes("intent=add-mcp")) throw new Error("compare href missing intent=add-mcp");
  if (!href?.includes("compare_tools=")) throw new Error("compare href missing compare_tools");
  log("RESULT", "INTERACTIVE_PASS");
} catch (e) {
  log("RESULT", `INTERACTIVE_FAIL: ${e.message}`);
  process.exitCode = 1;
} finally {
  await browser.close();
  writeFileSync(join(scratch, "mcp-add-interactive-transcript.log"), transcript.join("\n") + "\n");
}