# OnchainAI Plugin for Claude Code

Discover, vet, and install crypto tools (MCP servers, CLIs, SDKs, APIs, x402
services, AI-agent tools) from the [OnchainAI directory](https://www.onchain-ai.xyz)
without leaving your agent.

## Install

```
/plugin marketplace add Coinyak/onchainai
/plugin install onchainai@onchainai
```

## What you get

- **MCP server** — connects `https://www.onchain-ai.xyz/mcp` automatically
  (free discovery by default: `search_tools`, `get_tool_detail`, `list_categories`,
  `get_dashboard_snapshot`, `get_install_guide`, `compare_tools`, …). Never points at
  `/mcp/okx` (that package is pay-per-call only when you intentionally install it).
- **`/find-tool` command** — `/find-tool bridge USDC to Base` returns the top
  matches with trust, install risk, and x402/paid status.
- **`onchainai-crypto-tools` skill** — teaches Claude when to reach for the
  directory, how to judge install risk (`critical` is never installed), and how
  to disclose x402 pricing before a paid tool is used.

## Pricing (public `/mcp`)

Discovery stays free. OnchainAI premium tools are always paid via x402:
~$0.01 for `export_toolkit` / `recommend_verified_tool` / `gap_audit`;
~$0.001 for `check_endpoint_health`. Claude Code often cannot settle x402 —
use free search/detail instead when unpaid. Catalog tools with third-party
`pricing = "x402"` are separate; OnchainAI only surfaces their payment metadata.

## Safety model

The plugin connects OnchainAI's own MCP endpoint for discovery. It never runs
install commands on its own, never recommends `critical`-risk tools, and always
surfaces x402 payment metadata before a paid catalog tool is suggested. OnchainAI
publishes payment metadata only — it does not connect wallets or move funds for
third-party tools.
