-- 034_onchainai_brand_logo.sql — first-party OnchainAI listing uses bundled brand asset.
UPDATE tools
SET logo_url = '/brand/onchainai-logo.png',
    updated_at = now()
WHERE slug = 'onchainai'
  AND (logo_url IS NULL OR logo_url <> '/brand/onchainai-logo.png');