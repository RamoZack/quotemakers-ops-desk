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
psql -d quotemakers_ops_desk -f db/migrations/002_site_inventory_and_checks.sql
psql -d quotemakers_ops_desk -f db/seeds/current_railway_sites.sql
nix develop
cd api
cargo run
```

The API listens on `http://127.0.0.1:3000` by default and serves the built dashboard in production.

Run the dashboard in another shell:

```bash
nix develop
cd dashboard
npm install
npm run dev
```

The dashboard listens on `http://127.0.0.1:5173` and proxies `/api/*` to the Rust API.

Useful endpoints:

```text
GET  /health
GET  /sites
POST /inventory/railway/sync
POST /checks/run
GET  /health-checks/recent
GET  /risk/high-value
GET  /risk/repeated-ip
GET  /risk/repeated-contact
GET  /ops/failed-health-checks
```

Railway inventory sync reads the locally linked Railway project and upserts app domains into `sites`. Link this repo to the QuoteMakers Railway project before syncing:

```bash
nix run nixpkgs#railway -- link \
  --project=24b0b60a-8383-4e4e-a265-cf0708fe5388 \
  --environment=f54b5dfd-5784-42aa-86dc-75324761798d \
  --service=83410389-ca19-4d46-b1d9-177ddc94245b
```

`POST /checks/run` probes each enabled site:

```text
/                           expects 200 + text/html
/static/assets/css/main.css  expects 200 + text/css
/robots.txt                 expects 200 + text/plain
/sitemap.xml                expects 200 + xml content type
```

Production runs through `scripts/start.sh`, which applies idempotent migrations, seeds current Railway domains, then starts the API. Set these Railway variables for scheduled checks:

```text
AUTO_RUN_CHECKS=true
RUN_CHECKS_ON_START=true
CHECK_INTERVAL_SECONDS=1800
```

## Why

QuoteMakers needs a small external monitor that proves customer sites are up, static assets are serving, and quote activity looks healthy.
