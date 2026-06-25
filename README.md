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

## Local Setup

```bash
createdb quotemakers_ops_desk
psql -d quotemakers_ops_desk -f db/migrations/001_init.sql
psql -d quotemakers_ops_desk -f db/seeds/001_seed.sql
nix develop
cd api
cargo run
```

The API listens on `http://127.0.0.1:3000` by default.

Useful endpoints:

```text
GET /health
GET /sites
GET /health-checks/recent
GET /risk/high-value
GET /risk/repeated-ip
GET /risk/repeated-contact
GET /ops/failed-health-checks
```

## Why

QuoteMakers needs a small external monitor that proves customer sites are up, static assets are serving, and quote activity looks healthy.
