-- D1: Content management fields on site_settings singleton.
ALTER TABLE site_settings
  ADD COLUMN IF NOT EXISTS hero_title TEXT,
  ADD COLUMN IF NOT EXISTS hero_subtitle TEXT,
  ADD COLUMN IF NOT EXISTS about_content TEXT,
  ADD COLUMN IF NOT EXISTS footer_links JSONB NOT NULL DEFAULT '[]';