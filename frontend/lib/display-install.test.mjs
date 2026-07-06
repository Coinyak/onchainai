import assert from "node:assert/strict";
import test from "node:test";
import {
  displayInstallCommand,
  httpUrlFromMcpInstallCommand,
  isClientSpecificMcpCommand,
  isValidHttpMcpUrl,
  universalMcpInstallCommand,
} from "./display-install-core.mjs";

test("claude mcp add on card becomes universal add-mcp", () => {
  const cmd = displayInstallCommand({
    type: "mcp",
    install_command:
      "claude mcp add --transport http onchainai https://www.onchain-ai.xyz/mcp",
  });
  assert.equal(cmd, "npx add-mcp https://www.onchain-ai.xyz/mcp");
});

test("codex mcp add on card becomes universal add-mcp", () => {
  const cmd = displayInstallCommand({
    type: "mcp",
    install_command: "codex mcp add my-mcp --url https://api.example.com/mcp",
  });
  assert.equal(cmd, "npx add-mcp https://api.example.com/mcp");
});

test("mcp-remote on card upgrades to add-mcp", () => {
  const cmd = displayInstallCommand({
    type: "mcp",
    install_command: "npx mcp-remote 'https://docs.base.org/mcp'",
    mcp_endpoint: "https://docs.base.org/mcp",
  });
  assert.equal(cmd, "npx add-mcp https://docs.base.org/mcp");
});

test("stdio npm package install stays unchanged", () => {
  const cmd = displayInstallCommand({
    type: "mcp",
    install_command: "npx @coinbase/cdp-mcp",
  });
  assert.equal(cmd, "npx @coinbase/cdp-mcp");
});

test("cli install without mcp surface stays unchanged", () => {
  const cmd = displayInstallCommand({
    type: "cli",
    install_command: "npm i @scope/pkg",
  });
  assert.equal(cmd, "npm i @scope/pkg");
});

test("endpoint-only mcp tool uses universal command", () => {
  const cmd = displayInstallCommand({
    type: "mcp",
    mcp_endpoint: "https://api.example.com/mcp",
  });
  assert.equal(cmd, "npx add-mcp https://api.example.com/mcp");
});

test("client-specific safe_copy_command is universalized", () => {
  const cmd = displayInstallCommand({
    type: "mcp",
    safe_copy_command: "claude mcp add foo https://safe.example/mcp",
    install_command: "npx @other/pkg",
  });
  assert.equal(cmd, "npx add-mcp https://safe.example/mcp");
});

test("custom safe_copy_command is preserved even with mcp_endpoint", () => {
  const custom = "npx @operator/pkg --flag required";
  const cmd = displayInstallCommand({
    type: "mcp",
    safe_copy_command: custom,
    mcp_endpoint: "https://api.example.com/mcp",
  });
  assert.equal(cmd, custom);
});

test("universal safe_copy_command is preserved as-is", () => {
  const universal = "npx add-mcp https://api.example.com/mcp";
  const cmd = displayInstallCommand({
    type: "mcp",
    safe_copy_command: universal,
    mcp_endpoint: "https://api.example.com/mcp",
  });
  assert.equal(cmd, universal);
});

test("httpUrlFromMcpInstallCommand parses host-only legacy command", () => {
  assert.equal(
    httpUrlFromMcpInstallCommand("npx mcp-remote www.onchain-ai.xyz/mcp"),
    "https://www.onchain-ai.xyz/mcp",
  );
});

test("universalMcpInstallCommand is idempotent shape", () => {
  assert.equal(
    universalMcpInstallCommand("https://x/mcp"),
    "npx add-mcp https://x/mcp",
  );
});

test("isClientSpecificMcpCommand detects legacy and client CLIs", () => {
  assert.equal(isClientSpecificMcpCommand("claude mcp add foo https://x/mcp"), true);
  assert.equal(isClientSpecificMcpCommand("npx mcp-remote https://x/mcp"), true);
  assert.equal(isClientSpecificMcpCommand("npx add-mcp https://x/mcp"), false);
  assert.equal(isClientSpecificMcpCommand("npx @scope/pkg"), false);
});

test("isValidHttpMcpUrl rejects shell metacharacters and embedded whitespace", () => {
  assert.equal(isValidHttpMcpUrl("https://api.example.com/mcp"), true);
  assert.equal(isValidHttpMcpUrl("https://api.example.com/mcp;rm"), false);
  assert.equal(isValidHttpMcpUrl('https://api.example.com/mcp"'), false);
  assert.equal(isValidHttpMcpUrl("https://api.example.com/ mcp"), false);
  assert.equal(isValidHttpMcpUrl("https://api.example.com/\tmcp"), false);
});

test("universalMcpInstallCommand returns null for invalid URLs", () => {
  assert.equal(universalMcpInstallCommand("https://evil.com/mcp;rm"), null);
  assert.equal(universalMcpInstallCommand("not-a-url"), null);
});

test("client-specific command with invalid URL falls back to raw install", () => {
  const legacy = "claude mcp add foo https://evil.com/mcp;rm";
  const cmd = displayInstallCommand({
    type: "mcp",
    install_command: legacy,
  });
  assert.equal(cmd, legacy);
});

test("invalid mcp_endpoint does not emit universal command", () => {
  const cmd = displayInstallCommand({
    type: "mcp",
    mcp_endpoint: "https://evil.com/mcp;rm",
  });
  assert.equal(cmd, "");
});