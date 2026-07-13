---
description: Find a crypto MCP/CLI/SDK/x402 tool via OnchainAI
argument-hint: <what you need, e.g. "bridge USDC to Base">
---

Use the free `onchainai` MCP `search_tools` tool (public
`https://www.onchain-ai.xyz/mcp` — not `/mcp/okx`) to find tools matching: $ARGUMENTS

Summarize the top 3 results with name, purpose, chains, type, install risk,
and x402/paid status. Prefer free `get_tool_detail` for trust and payment
metadata. Do not call premium tools (`export_toolkit`, `recommend_verified_tool`,
`gap_audit` ~$0.01; `check_endpoint_health` ~$0.001) unless the user asks and
the client can settle x402 — Claude Code often cannot. Then offer install steps
via `get_install_guide` for the user's agent. Never recommend a critical-risk tool.

If the user asks to save or add a tool to their OnchainAI toolkit, call
`save_to_toolkit` with the slug (only after explicit consent). If the MCP
response includes `link_required`, tell them to link at
https://www.onchain-ai.xyz/connect#agent-sync and set `ONCHAINAI_AGENT_TOKEN`
in their MCP config (see plugin `.mcp.json`).

For multi-tool workflows, offer `save_stack_to_blueprint` after the user
confirms — it saves to toolkit and today's agent session blueprint.
