import assert from "node:assert/strict";
import test from "node:test";
import { displayInstallCommand } from "./display-install-core.mjs";
import {
  ONCHAINAI_CLAUDE_CODE_CMD,
  ONCHAINAI_MCP_HTTP_URL,
  ONCHAINAI_MCP_UNIVERSAL_CMD,
} from "./mcp-deeplinks-core.mjs";

test("client-tab Claude command stays client-specific", () => {
  assert.match(ONCHAINAI_CLAUDE_CODE_CMD, /^claude mcp add/);
  assert.doesNotMatch(ONCHAINAI_CLAUDE_CODE_CMD, /^npx add-mcp/);
  assert.ok(ONCHAINAI_CLAUDE_CODE_CMD.includes(ONCHAINAI_MCP_HTTP_URL));
});

test("card command universalizes while client-tab constant remains", () => {
  const card = displayInstallCommand({
    type: "mcp",
    install_command: ONCHAINAI_CLAUDE_CODE_CMD,
    mcp_endpoint: ONCHAINAI_MCP_HTTP_URL,
  });
  assert.equal(card, ONCHAINAI_MCP_UNIVERSAL_CMD);
  assert.notEqual(card, ONCHAINAI_CLAUDE_CODE_CMD);
});