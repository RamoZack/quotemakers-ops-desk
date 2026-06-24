# QuoteMakers Ops Desk

Internal ops dashboard for QuoteMakers site health, static asset checks, quote analytics, and suspicious quote review.

## Friday MVP

- Rust API + Postgres backend
- Health checks for live QuoteMakers/customer sites
- Verify homepage `200`, CSS/static files `200`, and CSS content type
- Store check history: status, latency, failure reason, timestamp
- Basic dashboard: green/red site cards, broken asset alerts, recent checks
- Quote analytics: requests by site/service/day, high-value quotes
- Suspicious quote queue using SQL rules
- Nix flake for reproducible dev/build
- README, screenshots, architecture notes, resume bullet

## Repo Layout

```text
api/          Rust API
dashboard/    dashboard UI
db/           migrations, seed data, SQL rules
docs/         screenshots and architecture notes
```

## Why

QuoteMakers needs a small external monitor that proves customer sites are up, static assets are serving, and quote activity looks healthy.
