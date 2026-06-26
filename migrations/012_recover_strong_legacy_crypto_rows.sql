-- Recover legacy rows that have strong onchain/crypto evidence after the
-- public quality gate moved broad migration backfill rows back to review.
--
-- Keep this stricter than migration 006: generic "mcp", "api", "agent",
-- "chain", "bridge", "token", and "governance" alone are not enough.

UPDATE tools
SET relevance_status = 'accepted',
    crypto_relevance_score = GREATEST(crypto_relevance_score, 60),
    crypto_relevance_reasons = ARRAY[
      'quality-gate: strong onchain keyword after legacy backfill review'
    ]
WHERE approval_status = 'approved'
  AND relevance_status = 'needs_review'
  AND crypto_relevance_score = 0
  AND (
    crypto_relevance_reasons = ARRAY[
      'quality-gate: legacy migration backfill requires operator review'
    ]::TEXT[]
    OR crypto_relevance_reasons = ARRAY[
      'migration-backfill: crypto keyword in name or description'
    ]::TEXT[]
  )
  AND lower(coalesce(name, '') || ' ' || coalesce(description, '') || ' ' || coalesce(repo_url, '') || ' ' || coalesce(npm_package, '')) ~
    '(bitcoin|ethereum|solana|polygon|arbitrum|optimism|avalanche|base network|base chain|defi|uniswap|aave|compound|metamask|usdc|usdt|erc-?20|erc-?721|evm|mainnet|testnet|onchain|on-chain|smart.?contract|x402|rwa|nft|staking|dex|cross-chain|wormhole|layerzero|bob gateway|web3|blockchain|crypto|mempool|validator|liquidity|amm)';
