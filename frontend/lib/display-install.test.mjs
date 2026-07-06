import assert from "node:assert/strict";
import test from "node:test";
import {
  displayInstallCommand,
  httpUrlFromMcpInstallCommand,
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

test("safe_copy_command takes precedence for parsing", () => {
  const cmd = displayInstallCommand({
    type: "mcp",
    safe_copy_command: "claude mcp add foo https://safe.example/mcp",
    install_command: "npx @other/pkg",
  });
  assert.equal(cmd, "npx add-mcp https://safe.example/mcp");
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