#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL is required}"

export API_BIND_ADDR="${API_BIND_ADDR:-0.0.0.0:${PORT:-3000}}"
export STATIC_DIR="${STATIC_DIR:-/app/dashboard/dist}"

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f /app/db/migrations/001_init.sql
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f /app/db/migrations/002_site_inventory_and_checks.sql
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f /app/db/seeds/current_railway_sites.sql

exec /app/quotemakers-ops-api
