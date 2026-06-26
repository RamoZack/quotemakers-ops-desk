ALTER TABLE sites
  ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'manual',
  ADD COLUMN IF NOT EXISTS external_service_id TEXT,
  ADD COLUMN IF NOT EXISTS service_name TEXT,
  ADD COLUMN IF NOT EXISTS domain_type TEXT NOT NULL DEFAULT 'manual',
  ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

ALTER TABLE health_checks DROP CONSTRAINT IF EXISTS health_checks_target_type_check;
ALTER TABLE health_checks
  ADD CONSTRAINT health_checks_target_type_check
  CHECK (target_type IN ('homepage', 'css', 'robots', 'sitemap'));

CREATE INDEX IF NOT EXISTS idx_sites_source ON sites(source);
CREATE INDEX IF NOT EXISTS idx_sites_external_service_id ON sites(external_service_id);
CREATE INDEX IF NOT EXISTS idx_sites_last_seen ON sites(last_seen_at DESC);
