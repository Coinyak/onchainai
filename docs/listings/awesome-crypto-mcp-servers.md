## OnchainAI

- **MCP endpoint:** `https://www.onchain-ai.xyz/mcp` (streamable HTTP, no auth — **default free discovery**)
- **Connect:** [onchain-ai.xyz/connect](https://www.onchain-ai.xyz/connect)
- **Repo:** [github.com/Coinyak/onchainai](https://github.com/Coinyak/onchainai)
- **Chains:** Multi-chain catalog (Base, Ethereum, Solana, …)
- **x402 / billing:** Website browse + public `/mcp` discovery (`search_tools`, detail, compare, install guides, …) are free. Optional premium tools on `/mcp`: `export_toolkit` / `recommend_verified_tool` / `gap_audit` at **$0.01 USDC**; `check_endpoint_health` ~**$0.001 USDC**. OKX marketplace only uses `https://www.onchain-ai.xyz/mcp/okx` (~$0.1 every `tools/call` when gate active). Directory metadata only for third-party x402; OnchainAI never holds third-party funds.
- **Claude Code:** `/plugin marketplace add Coinyak/onchainai` → `/plugin install onchainai@onchainai`