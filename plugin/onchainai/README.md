# OnchainAI Plugin for Claude Code

Discover, vet, and install crypto tools (MCP servers, CLIs, SDKs, APIs, x402
services, AI-agent tools) from the [OnchainAI directory](https://www.onchain-ai.xyz)
without leaving your agent.

## Install

```
/plugin marketplace add hoyeon4315-cpu/onchainai
/plugin install onchainai@onchainai
```

## What you get

- **MCP server** — connects `https://www.onchain-ai.xyz/mcp` automatically
  (read-only search: `search_tools`, `get_tool_detail`, `list_categories`,
  `get_dashboard_snapshot`, `get_install_guide`).
- **`/find-tool` command** — `/find-tool bridge USDC to Base` returns the top
  matches with trust, install risk, and x402/paid status.
- **`onchainai-crypto-tools` skill** — teaches Claude when to reach for the
  directory, how to judge install risk (`critical` is never installed), and how
  to disclose x402 pricing before a paid tool is used.

## Safety model

The plugin only connects OnchainAI's own read-only endpoint. It never runs
install commands on its own, never recommends `critical`-risk tools, and always
surfaces x402 payment metadata before a paid tool is suggested. OnchainAI
publishes payment metadata only — it does not connect wallets or move funds.
