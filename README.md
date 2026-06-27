# QuoteMakers Ops Desk

Rust + React ops dashboard for monitoring live QuoteMakers customer sites.

It checks whether each deployed site is serving the homepage, critical CSS, `robots.txt`, and `sitemap.xml`, stores the results in Postgres, and shows current failures in a small dashboard. It also includes SQL-backed quote-risk views for high-value quotes, repeated IPs, and repeated contact info.

Live app: <https://opsdesk-production-a01f.up.railway.app>

## Why This Exists

QuoteMakers manages quote websites for service businesses. When a site breaks, the failure is usually operational: a domain points wrong, static assets stop serving, a deploy regresses `robots.txt`, or a customer site quietly drops offline.

Ops Desk is a small external monitor for that risk. It gives QuoteMakers one place to see:

- which customer sites are currently enabled for monitoring
- whether the homepage and key static/SEO assets are responding correctly
- recent failed checks with status code, content type, latency, and failure reason
- quote-risk signals that could become fraud/spam review workflows

## What It Demonstrates

- Rust API service using Axum, Tokio, Reqwest, SQLx, and Postgres
- SQL-first schema, migrations, seed data, and risk queries
- React/Vite dashboard served by the Rust API in production
- Nix flake dev shell for reproducible local tooling
- Docker multi-stage build for Railway deployment
- Scheduler-style background checks controlled by environment variables
- Source-controlled ops inventory for deployed customer domains

## Architecture

```text
Railway / browser
      |
      v
Rust Axum API ----------------------+
  /api/sites                        |
  /api/checks/run                   |
  /api/health-checks/recent         |
  /api/ops/failed-health-checks     |
  /api/risk/*                       |
      |                             |
      | SQLx                        | Reqwest probes
      v                             v
Postgres                    QuoteMakers customer sites
  sites                       /
  health_checks               /static/assets/css/main.css
  quote_events                /robots.txt
  risk_cases                  /sitemap.xml
      ^
      |
React dashboard served from `dashboard/dist`
```

Production startup runs `scripts/start.sh`:

1. Apply idempotent SQL migrations.
2. Seed the current Railway/customer-site inventory.
3. Soft-prune missing Railway sites with `enabled = false`.
4. Start the Rust API.
5. Run startup/scheduled checks when env vars are enabled.

## Current Production Behavior

- Railway project: `gallant-elegance`
- Service: `OpsDesk`
- Database: Railway Postgres
- Scheduler:
  - `AUTO_RUN_CHECKS=true`
  - `RUN_CHECKS_ON_START=true`
  - `CHECK_INTERVAL_SECONDS=1800`
- Inventory source: `db/seeds/current_railway_sites.sql`
- `/api/sites` returns enabled sites only
- failed-health views filter to enabled monitored sites

The seed file is the current deployed source of truth. Updating deployed inventory means editing `db/seeds/current_railway_sites.sql` and redeploying. A future version should sync through Railway's API using a scoped token instead of relying on local Railway CLI auth.

## Health Check Targets

For each enabled site, the checker probes:

| Target | Path | Expected |
| --- | --- | --- |
| Homepage | `/` | `200` + `text/html` |
| Critical CSS | `/static/assets/css/main.css` | `200` + `text/css` |
| Robots | `/robots.txt` | `200` + `text/plain` |
| Sitemap | `/sitemap.xml` | `200` + XML content type |

Each result stores URL, status code, content type, latency, success/failure, failure reason, and timestamp.

## API Endpoints

```text
GET  /api/health
GET  /api/sites
POST /api/checks/run
GET  /api/health-checks/recent
GET  /api/ops/failed-health-checks
GET  /api/risk/high-value
GET  /api/risk/repeated-ip
GET  /api/risk/repeated-contact
```

There is also a local-only Railway inventory sync endpoint in the API code. It shells out to `railway status --json`, so it requires an authenticated Railway CLI in the runtime. The deployed Docker image does not depend on that path; production uses the checked-in seed file.

## Repo Layout

```text
api/          Rust Axum API and SQLx queries
dashboard/    React/Vite dashboard
db/           SQL migrations, current inventory seed, risk rules
docs/         architecture and implementation notes
scripts/      production startup script
Dockerfile    multi-stage dashboard + Rust API build
flake.nix     local dev shell
```

## Local Development

Enter the Nix dev shell:

```bash
nix develop
```

Create and seed the local database:

```bash
createdb quotemakers_ops_desk
psql -d quotemakers_ops_desk -f db/migrations/001_init.sql
psql -d quotemakers_ops_desk -f db/migrations/002_site_inventory_and_checks.sql
psql -d quotemakers_ops_desk -f db/seeds/current_railway_sites.sql
```

Run the API:

```bash
cd api
cargo run
```

Run the dashboard dev server in another shell:

```bash
cd dashboard
npm install
npm run dev
```

Local URLs:

- API: `http://127.0.0.1:3000/api/health`
- Dashboard dev server: `http://127.0.0.1:5173`
- Production-style static dashboard: build `dashboard/dist`, then let the Rust API serve it

## Build And Deploy

Build the dashboard:

```bash
cd dashboard
npm ci
npm run build
```

Build the API:

```bash
cd api
cargo build --release
```

The Dockerfile does both in separate build stages, then copies the compiled Rust binary, dashboard assets, SQL files, and startup script into a slim Debian runtime image.

Railway runs:

```bash
/app/scripts/start.sh
```

Required production env var:

```text
DATABASE_URL
```

Useful scheduler env vars:

```text
AUTO_RUN_CHECKS=true
RUN_CHECKS_ON_START=true
CHECK_INTERVAL_SECONDS=1800
```

## Design Choices

- `enabled=false` soft-prunes missing Railway sites instead of deleting history.
- Health checks are stored append-only so failures can be audited over time.
- SQL does the risk grouping because repeated IP/contact detection is clearer and cheaper close to the data.
- The React app stays thin: fetch JSON, compute current site state, render cards/tables.
- Nix is used for local reproducibility, not as a production dependency.

## Near-Term Improvements

- Replace seed-file inventory updates with Railway API token sync.
- Add screenshots and a small architecture image for the GitHub repo.
- Import real QuoteMakers quote submissions into `quote_events`.
- Add a lightweight review queue for risk cases: assign, clear, flag, notes.
- Add tests around check classification and SQL risk rules.
