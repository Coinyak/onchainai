-- Re-apply strict legacy recovery with a plain Postgres word-boundary regex.

UPDATE tools
SET relevance_status = 'accepted',
    crypto_relevance_score = GREATEST(crypto_relevance_score, 60),
    crypto_relevance_reasons = ARRAY[
      'quality-gate: strict strong onchain keyword after legacy review'
    ]
WHERE approval_status = 'approved'
  AND relevance_status = 'needs_review'
  AND crypto_relevance_score = 0
  AND crypto_relevance_reasons = ARRAY[
    'quality-gate: strict legacy keyword boundary review required'
  ]::TEXT[]
  AND lower(coalesce(name, '') || ' ' || coalesce(description, '') || ' ' || coalesce(repo_url, '') || ' ' || coalesce(npm_package, '')) ~
    '\m(bitcoin|ethereum|solana|polygon|arbitrum|optimism|avalanche|defi|uniswap|aave|compound|metamask|usdc|usdt|erc-?20|erc-?721|evm|mainnet|testnet|onchain|x402|rwa|nft|staking|dex|wormhole|layerzero|web3|blockchain|crypto|mempool|validator|liquidity|amm)\M|\m(base network|base chain|on-chain|smart.?contract|cross-chain|bob gateway)\M';
