CREATE TABLE sites (
  id BIGSERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL UNIQUE,
  homepage_path TEXT NOT NULL DEFAULT '/',
  critical_css_url TEXT NOT NULL,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE health_checks (
  id BIGSERIAL PRIMARY KEY,
  site_id BIGINT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  target_type TEXT NOT NULL CHECK (target_type IN ('homepage', 'css')),
  url TEXT NOT NULL,
  status_code INTEGER,
  content_type TEXT,
  latency_ms INTEGER,
  ok BOOLEAN NOT NULL,
  failure_reason TEXT,
  checked_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE quote_events (
  id BIGSERIAL PRIMARY KEY,
  site_id BIGINT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  service_name TEXT NOT NULL,
  customer_email TEXT,
  customer_phone TEXT,
  ip_address INET,
  quoted_price_cents INTEGER NOT NULL,
  city TEXT,
  state TEXT,
  submitted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE risk_cases (
  id BIGSERIAL PRIMARY KEY,
  quote_event_id BIGINT NOT NULL REFERENCES quote_events(id) ON DELETE CASCADE,
  risk_score INTEGER NOT NULL,
  reason TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'new'
    CHECK (status IN ('new', 'reviewing', 'cleared', 'flagged')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE risk_case_notes (
  id BIGSERIAL PRIMARY KEY,
  risk_case_id BIGINT NOT NULL REFERENCES risk_cases(id) ON DELETE CASCADE,
  note TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_health_checks_site_time ON health_checks(site_id, checked_at DESC);
CREATE INDEX idx_quote_events_site_time ON quote_events(site_id, submitted_at DESC);
CREATE INDEX idx_quote_events_email ON quote_events(customer_email);
CREATE INDEX idx_quote_events_phone ON quote_events(customer_phone);
CREATE INDEX idx_quote_events_ip ON quote_events(ip_address);
CREATE INDEX idx_risk_cases_status ON risk_cases(status);
