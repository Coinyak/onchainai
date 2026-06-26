-- Tool official logos: remote URL + optional monogram override.
ALTER TABLE tools ADD COLUMN IF NOT EXISTS logo_url TEXT;
ALTER TABLE tools ADD COLUMN IF NOT EXISTS logo_monogram TEXT;