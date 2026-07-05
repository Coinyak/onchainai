---
name: onchainai-crypto-tools
description: Find, vet, and install crypto/onchain tools such as MCP servers, CLIs, SDKs, APIs, x402 services, and AI-agent tools through the OnchainAI directory. Use when the user needs an onchain capability, asks which tool fits a chain or task, wants install steps for Claude/Cursor, or needs trust, install risk, or x402 payment checks before choosing a tool.
---

# OnchainAI Crypto Tool Finder

Use the connected `onchainai` MCP server to discover and safely install crypto tools.

## Query Workflow

1. Translate the user's goal into a search intent, including chain, protocol, type, or action when known.
2. Call `search_tools` first. Use `query` for the capability, and pass chain/category/type filters when the MCP schema exposes them.
3. For promising results, call `get_tool_detail` to inspect trust, source, supported chains, install metadata, and x402 fields.
4. When the user wants setup help, call `get_install_guide` for the target platform (`claude`, `cursor`, or `generic`).
5. If the user is browsing rather than asking for a specific capability, call `list_categories` before searching.

## Result Rules

- Prefer tools marked `official`, `verified`, or `claimed` when several results match.
- Prefer stronger trust signals, recent activity, relevant chain support, and lower install risk over raw star count.
- Do not recommend tools that do not match the user's chain, task, or agent environment.
- Do not invent tools, claims, commands, MCP config, pricing, or verification status. Only report fields returned by OnchainAI.

## Install Safety

- `critical`: do not install or run. Tell the user it is blocked pending operator review.
- `high`: warn before any install steps. Do not paste raw shell wrappers as a safe default; proceed only if the user explicitly trusts the source.
- `medium` or `low`: show the provided command or MCP config from `get_install_guide`.
- Never run an install command yourself unless the user explicitly asks and the repo task requires it.

## x402 And Paid Tools

- Do **not** call `check_endpoint_health` from Claude Code, Cursor, or other MCP clients without x402 payment support — they show `Connection closed` instead of price info. Use free `get_tool_detail` for `x402_endpoint_verified`, `price_verified`, and `payment_verified`, or document REST `GET /api/v2/premium/check-endpoint-health/{slug}` for x402-native HTTP clients.
- If `pricing = "x402"` or an `x402_price` is present, state that calls may charge on use and require a connected agent wallet.
- Surface the returned price and endpoint/payment verification flags.
- Say "operator verified" only when `payment_verified`, `x402_endpoint_verified`, and `price_verified` are all true.
- OnchainAI is discovery and trust metadata only. Do not create custody, facilitator, gateway, fund-moving, undocumented `referrer`, or `split` payment flows.

## Agent Sync (save to web toolkit)

- Saving to the user's OnchainAI toolkit requires an **Agent Sync** link (`ONCHAINAI_AGENT_TOKEN` or MCP `Authorization: Bearer` header).
- Call `link_status` when unsure whether the client is linked.
- Call `save_to_toolkit` only when the user **explicitly** asks to save, bookmark, or add to toolkit — not after every search.
- If the tool returns `link_required`, direct the user to https://www.onchain-ai.xyz/connect#agent-sync (device flow; no manual token copy required on web).
- For a confirmed multi-tool stack, `save_stack_to_blueprint` saves slugs to toolkit and appends nodes to today's `Agent session · {date}` blueprint.

## Response Shape

For recommendations, return the top 1-3 tools with:

- name and slug
- why it fits the user's task
- chains and tool type
- trust or official status
- install risk level
- x402 or paid status
- next install step, if requested
