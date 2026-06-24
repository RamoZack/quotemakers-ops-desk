# Architecture Notes

Initial data flow:

1. Checker requests configured site URLs and critical CSS/static assets.
2. Rust API stores check result, latency, status code, content type, and failure reason in Postgres.
3. Quote events are seeded first, then later can come from QuoteMakers export/webhook.
4. SQL rules flag suspicious or operationally important records.
5. Dashboard shows current health, recent failures, quote volume, and review cases.
