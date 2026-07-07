-- Backfill tools.chains[] to canonical ids after chain synonym unification.
--
-- The chain catalog now treats rebranded/synonym chains as aliases:
--   fantom / ftm / fantom-mainnet  →  sonic
--   bnb / binance / binance-smart-chain / bnb-chain / bnb-smart-chain / binance-chain  →  bsc
--   gram / gram-token / toncoin / the-open-network  →  ton
--   eth / eth-mainnet / ethereum-mainnet  →  ethereum
--   btc / btc-mainnet / xbt  →  bitcoin
--   ... (full alias list in src/chains.rs CHAIN_CATALOG)
--
-- Without this backfill, existing rows with alias chain strings are invisible
-- to canonical chain filters (chains @> ['sonic'] won't match ['fantom']).
-- The normalizer now canonicalizes on ingestion, but historical data needs
-- this one-time rewrite.

-- Replace each known alias with its canonical id across all tools.chains[].
-- We do this per-alias-pair to keep the migration explicit and auditable.

-- Fantom → Sonic (2025 rebrand)
UPDATE tools
SET chains = ARRAY(
    SELECT CASE
        WHEN elem IN ('fantom', 'ftm', 'fantom-mainnet', 'fantom-testnet') THEN 'sonic'
        ELSE elem
    END
    FROM unnest(chains) AS t(elem)
),
    updated_at = now()
WHERE chains && ARRAY['fantom', 'ftm', 'fantom-mainnet', 'fantom-testnet']::text[];

-- Deduplicate after replacement (sonic may have appeared alongside fantom).
UPDATE tools
SET chains = ARRAY(SELECT DISTINCT unnest(chains)),
    updated_at = now()
WHERE chains && ARRAY['sonic']::text[]
  AND array_length(chains, 1) > 1;

-- BNB Chain synonyms → bsc
UPDATE tools
SET chains = ARRAY(
    SELECT CASE
        WHEN elem IN ('bnb', 'binance', 'binance-smart-chain', 'bnb-chain',
                      'bnb-smart-chain', 'binance-chain') THEN 'bsc'
        ELSE elem
    END
    FROM unnest(chains) AS t(elem)
),
    updated_at = now()
WHERE chains && ARRAY['bnb', 'binance', 'binance-smart-chain', 'bnb-chain',
                      'bnb-smart-chain', 'binance-chain']::text[];

UPDATE tools
SET chains = ARRAY(SELECT DISTINCT unnest(chains)),
    updated_at = now()
WHERE chains && ARRAY['bsc']::text[]
  AND array_length(chains, 1) > 1;

-- TON token synonyms → ton
UPDATE tools
SET chains = ARRAY(
    SELECT CASE
        WHEN elem IN ('gram', 'gram-token', 'toncoin', 'the-open-network') THEN 'ton'
        ELSE elem
    END
    FROM unnest(chains) AS t(elem)
),
    updated_at = now()
WHERE chains && ARRAY['gram', 'gram-token', 'toncoin', 'the-open-network']::text[];

UPDATE tools
SET chains = ARRAY(SELECT DISTINCT unnest(chains)),
    updated_at = now()
WHERE chains && ARRAY['ton']::text[]
  AND array_length(chains, 1) > 1;

-- Ethereum synonyms → ethereum
UPDATE tools
SET chains = ARRAY(
    SELECT CASE
        WHEN elem IN ('eth', 'eth-mainnet', 'ethereum-mainnet') THEN 'ethereum'
        ELSE elem
    END
    FROM unnest(chains) AS t(elem)
),
    updated_at = now()
WHERE chains && ARRAY['eth', 'eth-mainnet', 'ethereum-mainnet']::text[];

UPDATE tools
SET chains = ARRAY(SELECT DISTINCT unnest(chains)),
    updated_at = now()
WHERE chains && ARRAY['ethereum']::text[]
  AND array_length(chains, 1) > 1;

-- Bitcoin synonyms → bitcoin
UPDATE tools
SET chains = ARRAY(
    SELECT CASE
        WHEN elem IN ('btc', 'btc-mainnet', 'xbt') THEN 'bitcoin'
        ELSE elem
    END
    FROM unnest(chains) AS t(elem)
),
    updated_at = now()
WHERE chains && ARRAY['btc', 'btc-mainnet', 'xbt']::text[];

UPDATE tools
SET chains = ARRAY(SELECT DISTINCT unnest(chains)),
    updated_at = now()
WHERE chains && ARRAY['bitcoin']::text[]
  AND array_length(chains, 1) > 1;

-- Solana synonyms → solana
UPDATE tools
SET chains = ARRAY(
    SELECT CASE
        WHEN elem IN ('sol', 'solana-mainnet') THEN 'solana'
        ELSE elem
    END
    FROM unnest(chains) AS t(elem)
),
    updated_at = now()
WHERE chains && ARRAY['sol', 'solana-mainnet']::text[];

UPDATE tools
SET chains = ARRAY(SELECT DISTINCT unnest(chains)),
    updated_at = now()
WHERE chains && ARRAY['solana']::text[]
  AND array_length(chains, 1) > 1;

-- Polygon synonyms → polygon
UPDATE tools
SET chains = ARRAY(
    SELECT CASE
        WHEN elem IN ('matic', 'polygon-pos', 'polygon-mainnet', 'matic-mainnet') THEN 'polygon'
        ELSE elem
    END
    FROM unnest(chains) AS t(elem)
),
    updated_at = now()
WHERE chains && ARRAY['matic', 'polygon-pos', 'polygon-mainnet', 'matic-mainnet']::text[];

UPDATE tools
SET chains = ARRAY(SELECT DISTINCT unnest(chains)),
    updated_at = now()
WHERE chains && ARRAY['polygon']::text[]
  AND array_length(chains, 1) > 1;

-- Avalanche synonyms → avalanche
UPDATE tools
SET chains = ARRAY(
    SELECT CASE
        WHEN elem IN ('avax', 'avalanche-c', 'avax-c', 'c-chain') THEN 'avalanche'
        ELSE elem
    END
    FROM unnest(chains) AS t(elem)
),
    updated_at = now()
WHERE chains && ARRAY['avax', 'avalanche-c', 'avax-c', 'c-chain']::text[];

UPDATE tools
SET chains = ARRAY(SELECT DISTINCT unnest(chains)),
    updated_at = now()
WHERE chains && ARRAY['avalanche']::text[]
  AND array_length(chains, 1) > 1;
