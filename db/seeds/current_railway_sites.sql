INSERT INTO sites (
  name, base_url, homepage_path, critical_css_url, enabled,
  source, external_service_id, service_name, domain_type,
  last_seen_at, updated_at
) VALUES
  ('Big A Towing', 'https://bigatowing.thequotemakers.com', '/', 'https://bigatowing.thequotemakers.com/static/assets/css/main.css', TRUE, 'railway', '596b238f-263a-4633-bfa6-33ae6341cf0b', 'Big A Towing', 'custom', now(), now()),
  ('Demo Site', 'https://demo.thequotemakers.com', '/', 'https://demo.thequotemakers.com/static/assets/css/main.css', TRUE, 'railway', '83410389-ca19-4d46-b1d9-177ddc94245b', 'Demo Site', 'custom', now(), now()),
  ('Demo Site (Railway)', 'https://demo-site-production-93e1.up.railway.app', '/', 'https://demo-site-production-93e1.up.railway.app/static/assets/css/main.css', TRUE, 'railway', '83410389-ca19-4d46-b1d9-177ddc94245b', 'Demo Site', 'railway_service', now(), now()),
  ('Georgia Valley Tow', 'https://georgiavalleytow.thequotemakers.com', '/', 'https://georgiavalleytow.thequotemakers.com/static/assets/css/main.css', TRUE, 'railway', 'fb0ce776-2fe0-4411-aca1-55806a50031f', 'Georgia Valley Tow', 'custom', now(), now()),
  ('Pelican Plumbing', 'https://coralpelican.thequotemakers.com', '/', 'https://coralpelican.thequotemakers.com/static/assets/css/main.css', TRUE, 'railway', 'c25bd743-38c3-49bb-ab61-bc91a169d189', 'Pelican Plumbing', 'custom', now(), now()),
  ('Quotemakers', 'https://thequotemakers.com', '/', 'https://thequotemakers.com/static/assets/css/main.css', TRUE, 'railway', 'a4016b70-2e20-464f-9ae7-2423d571d5b5', 'Quotemakers', 'custom', now(), now()),
  ('RootWrecker', 'https://rootwrecker.thequotemakers.com', '/', 'https://rootwrecker.thequotemakers.com/static/assets/css/main.css', TRUE, 'railway', 'fbd51acc-d693-4eef-ad3b-581bf95f16d7', 'RootWrecker', 'custom', now(), now()),
  ('RootWrecker (Railway)', 'https://rootwrecker.up.railway.app', '/', 'https://rootwrecker.up.railway.app/static/assets/css/main.css', TRUE, 'railway', 'fbd51acc-d693-4eef-ad3b-581bf95f16d7', 'RootWrecker', 'railway_service', now(), now()),
  ('Southern Pressure (Railway)', 'https://southern-pressure-production.up.railway.app', '/', 'https://southern-pressure-production.up.railway.app/static/assets/css/main.css', TRUE, 'railway', '666f7df5-deff-4347-86d0-3473a6245b0b', 'Southern Pressure', 'railway_service', now(), now()),
  ('SunCrafted Homes', 'https://suncraftedhomes.thequotemakers.com', '/', 'https://suncraftedhomes.thequotemakers.com/static/assets/css/main.css', TRUE, 'railway', '52bd38fe-4c73-4145-8a6a-f8321354034d', 'SunCrafted Homes', 'custom', now(), now()),
  ('Triple T On Time Tow', 'https://tripletow.thequotemakers.com', '/', 'https://tripletow.thequotemakers.com/static/assets/css/main.css', TRUE, 'railway', '2a5e7046-e8cd-4386-a6f2-204de235bcb9', 'Triple T On Time Tow', 'custom', now(), now())
ON CONFLICT (base_url) DO UPDATE SET
  name = EXCLUDED.name,
  critical_css_url = EXCLUDED.critical_css_url,
  enabled = TRUE,
  source = EXCLUDED.source,
  external_service_id = EXCLUDED.external_service_id,
  service_name = EXCLUDED.service_name,
  domain_type = EXCLUDED.domain_type,
  last_seen_at = now(),
  updated_at = now();
