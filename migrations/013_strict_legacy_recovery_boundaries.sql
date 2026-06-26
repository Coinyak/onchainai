-- Correct the legacy recovery pass with strict word boundaries.
--
-- Migration 012 used substring regexes, so examples like "indexes" matched
-- "dex", "define" matched "defi", and "forwarding" matched "rwa".
-- Keep recovered rows public only when the strong signal appears as a real
-- term or phrase.

UPDATE tools
SET relevance_status = 'needs_review',
    crypto_relevance_score = 0,
    crypto_relevance_reasons = ARRAY[
      'quality-gate: strict legacy keyword boundary review required'
    ]
WHERE approval_status = 'approved'
  AND relevance_status = 'accepted'
  AND crypto_relevance_reasons = ARRAY[
    'quality-gate: strong onchain keyword after legacy backfill review'
  ]::TEXT[]
  AND NOT (
    lower(coalesce(name, '') || ' ' || coalesce(description, '') || ' ' || coalesce(repo_url, '') || ' ' || coalesce(npm_package, '')) ~
    E'\\\\m(bitcoin|ethereum|solana|polygon|arbitrum|optimism|avalanche|defi|uniswap|aave|compound|metamask|usdc|usdt|erc-?20|erc-?721|evm|mainnet|testnet|onchain|x402|rwa|nft|staking|dex|wormhole|layerzero|web3|blockchain|crypto|mempool|validator|liquidity|amm)\\\\M|\\\\m(base network|base chain|on-chain|smart.?contract|cross-chain|bob gateway)\\\\M'
  );
