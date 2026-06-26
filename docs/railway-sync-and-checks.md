# Railway Sync And Health Checks

Source of truth for live site inventory is Railway domains, not QuoteMakers application data.

Why:

- `WebsiteSetup` stores business/site content but does not store deployed domains.
- `ALLOWED_HOSTS` is app config and can drift from actual Railway domains.
- Railway knows app services, custom domains, service domains, and deploy/instance state.

Flow:

1. `POST /inventory/railway/sync` runs `railway status --json` in the locally linked project.
2. The API parses app services and public domains from the JSON.
3. Each domain is upserted into `sites` with Railway metadata.
4. App services without public domains are returned in the sync summary.
5. `POST /checks/run` probes enabled sites and writes rows to `health_checks`.
6. The dashboard reads recent checks and failed checks from the API.

Current probe suite:

- Homepage: `/`, expects `200` and `text/html`.
- Critical CSS: `/static/assets/css/main.css`, expects `200` and `text/css`.
- Robots: `/robots.txt`, expects `200` and `text/plain`.
- Sitemap: `/sitemap.xml`, expects `200` and XML content type.

Recommended schedule:

- Railway inventory sync: hourly and manually after adding/changing domains.
- Health checks: every 5 minutes.
- Post-deploy checks: immediately after deploy.
- Failure handling: retry once before paging or sending a high-priority alert.
