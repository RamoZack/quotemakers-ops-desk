-- High-value quotes may need manual review.
SELECT
  qe.id AS quote_event_id,
  s.name AS site_name,
  qe.service_name,
  qe.customer_email,
  qe.quoted_price_cents,
  'high_quote_amount' AS rule_name,
  40 AS risk_score
FROM quote_events qe
JOIN sites s ON s.id = qe.site_id
WHERE qe.quoted_price_cents >= 100000;

-- Same IP creating multiple quotes can indicate spam or testing abuse.
SELECT
  qe.ip_address,
  COUNT(*) AS quote_count,
  MIN(qe.submitted_at) AS first_seen,
  MAX(qe.submitted_at) AS last_seen,
  'repeated_ip' AS rule_name,
  50 AS risk_score
FROM quote_events qe
WHERE qe.ip_address IS NOT NULL
GROUP BY qe.ip_address
HAVING COUNT(*) >= 3;

-- Same email or phone reused across quotes can indicate duplicate/fake leads.
SELECT
  qe.customer_email,
  qe.customer_phone,
  COUNT(*) AS quote_count,
  MIN(qe.submitted_at) AS first_seen,
  MAX(qe.submitted_at) AS last_seen,
  'repeated_contact' AS rule_name,
  60 AS risk_score
FROM quote_events qe
WHERE qe.customer_email IS NOT NULL
   OR qe.customer_phone IS NOT NULL
GROUP BY qe.customer_email, qe.customer_phone
HAVING COUNT(*) >= 2;

-- Recent failed health checks surface broken customer sites or assets.
SELECT
  hc.id AS health_check_id,
  s.name AS site_name,
  hc.target_type,
  hc.url,
  hc.status_code,
  hc.content_type,
  hc.failure_reason,
  'failed_health_check' AS rule_name,
  70 AS risk_score
FROM health_checks hc
JOIN sites s ON s.id = hc.site_id
WHERE hc.ok = FALSE
  AND hc.checked_at >= now() - interval '24 hours';
