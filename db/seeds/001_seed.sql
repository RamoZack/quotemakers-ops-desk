INSERT INTO sites (name, base_url, homepage_path, critical_css_url) VALUES
  ('QuoteMakers Main', 'https://thequotemakers.com', '/', 'https://thequotemakers.com/static/assets/css/main.css'),
  ('Demo Site', 'https://demo-site-production-93e1.up.railway.app', '/', 'https://demo-site-production-93e1.up.railway.app/static/assets/css/main.css'),
  ('Fake Broken Client', 'https://broken-client.example.com', '/', 'https://broken-client.example.com/static/assets/css/main.css');

INSERT INTO health_checks (site_id, target_type, url, status_code, content_type, latency_ms, ok, failure_reason, checked_at) VALUES
  (1, 'homepage', 'https://thequotemakers.com/', 200, 'text/html; charset=utf-8', 142, TRUE, NULL, now() - interval '30 minutes'),
  (1, 'css', 'https://thequotemakers.com/static/assets/css/main.css', 200, 'text/css', 88, TRUE, NULL, now() - interval '29 minutes'),
  (2, 'homepage', 'https://demo-site-production-93e1.up.railway.app/', 200, 'text/html; charset=utf-8', 210, TRUE, NULL, now() - interval '25 minutes'),
  (2, 'css', 'https://demo-site-production-93e1.up.railway.app/static/assets/css/main.css', 200, 'text/css', 96, TRUE, NULL, now() - interval '24 minutes'),
  (3, 'homepage', 'https://broken-client.example.com/', 500, 'text/html', 1200, FALSE, 'homepage returned 500', now() - interval '20 minutes'),
  (3, 'css', 'https://broken-client.example.com/static/assets/css/main.css', 404, 'text/html', 340, FALSE, 'css returned 404 instead of 200', now() - interval '19 minutes');

INSERT INTO quote_events (site_id, service_name, customer_email, customer_phone, ip_address, quoted_price_cents, city, state, submitted_at) VALUES
  (1, 'Pressure Washing', 'normal.customer@example.com', '727-555-0101', '203.0.113.10', 25000, 'St. Petersburg', 'FL', now() - interval '2 days'),
  (1, 'Roof Wash', 'homeowner@example.com', '727-555-0102', '203.0.113.11', 45000, 'Tampa', 'FL', now() - interval '1 day'),
  (2, 'House Wash', 'lead1@example.com', '813-555-0201', '198.51.100.20', 30000, 'Clearwater', 'FL', now() - interval '6 hours'),
  (2, 'House Wash', 'lead2@example.com', '813-555-0202', '198.51.100.20', 31000, 'Clearwater', 'FL', now() - interval '5 hours'),
  (2, 'House Wash', 'lead3@example.com', '813-555-0203', '198.51.100.20', 32000, 'Clearwater', 'FL', now() - interval '4 hours'),
  (1, 'Commercial Cleaning', 'big.job@example.com', '727-555-9999', '203.0.113.40', 250000, 'Miami', 'FL', now() - interval '3 hours'),
  (2, 'Roof Wash', 'repeat@example.com', '813-555-0300', '198.51.100.30', 40000, 'Tampa', 'FL', now() - interval '90 minutes'),
  (2, 'Driveway Cleaning', 'repeat@example.com', '813-555-0300', '198.51.100.31', 15000, 'Tampa', 'FL', now() - interval '80 minutes');
