import { useEffect, useMemo, useState } from 'react';

const API_BASE = '/api';

const endpoints = {
  sites: '/sites',
  checks: '/health-checks/recent',
  failedChecks: '/ops/failed-health-checks',
  highValue: '/risk/high-value',
  repeatedIp: '/risk/repeated-ip',
  repeatedContact: '/risk/repeated-contact',
  runChecks: '/checks/run',
  syncRailway: '/inventory/railway/sync',
};

async function fetchJson(path) {
  const response = await fetch(`${API_BASE}${path}`);

  if (!response.ok) {
    throw new Error(`${path} returned ${response.status}`);
  }

  return response.json();
}

async function postJson(path) {
  const response = await fetch(`${API_BASE}${path}`, { method: 'POST' });

  if (!response.ok) {
    throw new Error(`${path} returned ${response.status}`);
  }

  return response.json();
}

function latestForTarget(checks, targetType) {
  return checks
    .filter((check) => check.target_type === targetType)
    .sort((a, b) => new Date(b.checked_at) - new Date(a.checked_at))[0];
}

function buildSiteHealth(sites, checks) {
  return sites.map((site) => {
    const siteChecks = checks.filter((check) => check.site_id === site.id);
    const homepage = latestForTarget(siteChecks, 'homepage');
    const css = latestForTarget(siteChecks, 'css');
    const robots = latestForTarget(siteChecks, 'robots');
    const sitemap = latestForTarget(siteChecks, 'sitemap');
    const latestCheck = [...siteChecks].sort(
      (a, b) => new Date(b.checked_at) - new Date(a.checked_at),
    )[0];
    const latestTargets = [homepage, css, robots, sitemap].filter(Boolean);
    const hasFailure = latestTargets.some((check) => !check.ok);
    const hasMissingTarget = latestTargets.length < 4;
    const state = hasFailure ? 'failing' : hasMissingTarget ? 'unknown' : 'healthy';

    return {
      ...site,
      state,
      homepage,
      css,
      robots,
      sitemap,
      latestCheck,
      latencyMs: Math.max(...latestTargets.map((check) => check.latency_ms ?? 0), 0),
    };
  });
}

function formatDate(value) {
  if (!value) return 'never';

  return new Intl.DateTimeFormat(undefined, {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  }).format(new Date(value));
}

function formatMoney(cents) {
  return new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency: 'USD',
  }).format(cents / 100);
}

function stateLabel(state) {
  if (state === 'healthy') return 'Serving';
  if (state === 'failing') return 'Failing';
  return 'Unknown';
}

function App() {
  const [data, setData] = useState({
    sites: [],
    checks: [],
    failedChecks: [],
    highValue: [],
    repeatedIp: [],
    repeatedContact: [],
  });
  const [loading, setLoading] = useState(true);
  const [activeAction, setActiveAction] = useState('');
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [lastLoadedAt, setLastLoadedAt] = useState(null);

  async function loadDashboard(showLoading = true) {
    if (showLoading) setLoading(true);
    setError('');

    try {
      const [sites, checks, failedChecks, highValue, repeatedIp, repeatedContact] =
        await Promise.all([
          fetchJson(endpoints.sites),
          fetchJson(endpoints.checks),
          fetchJson(endpoints.failedChecks),
          fetchJson(endpoints.highValue),
          fetchJson(endpoints.repeatedIp),
          fetchJson(endpoints.repeatedContact),
        ]);

      setData({ sites, checks, failedChecks, highValue, repeatedIp, repeatedContact });
      setLastLoadedAt(new Date());
    } catch (err) {
      setError(err.message || 'Dashboard load failed');
    } finally {
      if (showLoading) setLoading(false);
    }
  }

  async function runChecks() {
    setActiveAction('checks');
    setError('');
    setNotice('');

    try {
      const result = await postJson(endpoints.runChecks);
      setNotice(
        `Inserted ${result.inserted_checks} checks across ${result.checked_sites} sites; ${result.failures} failures.`,
      );
      await loadDashboard(false);
    } catch (err) {
      setError(err.message || 'Check run failed');
    } finally {
      setActiveAction('');
    }
  }

  async function syncRailway() {
    setActiveAction('railway');
    setError('');
    setNotice('');

    try {
      const result = await postJson(endpoints.syncRailway);
      const skipped = result.services_without_domains?.length || 0;
      setNotice(
        `Synced ${result.upserted_sites} Railway domains; ${skipped} app services have no public domain.`,
      );
      await loadDashboard(false);
    } catch (err) {
      setError(err.message || 'Railway sync failed');
    } finally {
      setActiveAction('');
    }
  }

  useEffect(() => {
    loadDashboard();
  }, []);

  const siteHealth = useMemo(
    () => buildSiteHealth(data.sites, data.checks),
    [data.sites, data.checks],
  );
  const failingSites = siteHealth.filter((site) => site.state === 'failing').length;
  const suspiciousCount =
    data.highValue.length + data.repeatedIp.length + data.repeatedContact.length;
  const actionBusy = Boolean(activeAction);

  return (
    <main className="ops-shell">
      <section className="hero-panel">
        <div>
          <p className="eyebrow">QuoteMakers Ops Desk</p>
          <h1>Site health and quote-risk review.</h1>
          <p className="hero-copy">
            External checks for live sites, static assets, recent failures, and suspicious
            submission patterns.
          </p>
        </div>

        <div className="hero-actions">
          <div className="action-stack">
            <button className="refresh-button" onClick={runChecks} disabled={actionBusy || loading}>
              {activeAction === 'checks' ? 'Running...' : 'Run checks'}
            </button>
            <button
              className="refresh-button secondary"
              onClick={syncRailway}
              disabled={actionBusy || loading}
            >
              {activeAction === 'railway' ? 'Syncing...' : 'Sync Railway'}
            </button>
            <button
              className="refresh-button ghost"
              onClick={() => loadDashboard()}
              disabled={actionBusy || loading}
            >
              {loading ? 'Refreshing...' : 'Refresh'}
            </button>
          </div>
          <span className="timestamp">
            Last loaded {lastLoadedAt ? formatDate(lastLoadedAt) : 'not yet'}
          </span>
        </div>
      </section>

      {error && <div className="error-banner">{error}</div>}
      {notice && <div className="notice-banner">{notice}</div>}

      <section className="metric-grid" aria-label="Summary metrics">
        <MetricCard label="Monitored sites" value={data.sites.length} tone="neutral" />
        <MetricCard label="Failing sites" value={failingSites} tone={failingSites ? 'bad' : 'good'} />
        <MetricCard label="Recent errors" value={data.failedChecks.length} tone={data.failedChecks.length ? 'bad' : 'good'} />
        <MetricCard label="Risk signals" value={suspiciousCount} tone={suspiciousCount ? 'warn' : 'good'} />
      </section>

      <section className="dashboard-section">
        <div className="section-heading">
          <p className="eyebrow">Current State</p>
          <h2>Sites and static assets</h2>
        </div>

        <div className="site-grid">
          {siteHealth.map((site) => (
            <article className={`site-card ${site.state}`} key={site.id}>
              <div className="site-card-topline">
                <div>
                  <h3>{site.name}</h3>
                  <a href={site.base_url} target="_blank" rel="noreferrer">
                    {site.base_url}
                  </a>
                  <span className="site-meta">
                    {site.source} / {site.domain_type}
                  </span>
                </div>
                <span className="status-pill">{stateLabel(site.state)}</span>
              </div>

              <div className="check-list">
                <CheckRow label="Homepage" check={site.homepage} />
                <CheckRow label="CSS" check={site.css} />
                <CheckRow label="Robots" check={site.robots} />
                <CheckRow label="Sitemap" check={site.sitemap} />
              </div>

              <div className="site-card-footer">
                <span>Last check {formatDate(site.latestCheck?.checked_at)}</span>
                <span>{site.latencyMs} ms max latency</span>
              </div>
            </article>
          ))}
        </div>
      </section>

      <section className="dashboard-section split-layout">
        <Panel title="Most recent errors" eyebrow="Ops Alerts">
          {data.failedChecks.length === 0 ? (
            <EmptyState message="No failed homepage, CSS, robots, or sitemap checks in the last 24 hours." />
          ) : (
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>Site</th>
                    <th>Target</th>
                    <th>Status</th>
                    <th>Failure</th>
                  </tr>
                </thead>
                <tbody>
                  {data.failedChecks.map((check) => (
                    <tr key={check.health_check_id}>
                      <td>{check.site_name}</td>
                      <td>{check.target_type}</td>
                      <td>{check.status_code ?? 'n/a'}</td>
                      <td>{check.failure_reason || check.content_type || 'unknown failure'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Panel>

        <Panel title="Suspicious submissions" eyebrow="Risk Queue">
          <RiskList
            highValue={data.highValue}
            repeatedIp={data.repeatedIp}
            repeatedContact={data.repeatedContact}
          />
        </Panel>
      </section>
    </main>
  );
}

function MetricCard({ label, value, tone }) {
  return (
    <article className={`metric-card ${tone}`}>
      <span>{label}</span>
      <strong>{value}</strong>
    </article>
  );
}

function CheckRow({ label, check }) {
  if (!check) {
    return (
      <div className="check-row unknown">
        <span>{label}</span>
        <strong>Missing</strong>
      </div>
    );
  }

  return (
    <div className={`check-row ${check.ok ? 'ok' : 'bad'}`}>
      <span>{label}</span>
      <strong>{check.status_code ?? 'n/a'}</strong>
      <small>{check.content_type || check.failure_reason || 'no content type'}</small>
    </div>
  );
}

function Panel({ eyebrow, title, children }) {
  return (
    <article className="panel">
      <div className="section-heading compact">
        <p className="eyebrow">{eyebrow}</p>
        <h2>{title}</h2>
      </div>
      {children}
    </article>
  );
}

function EmptyState({ message }) {
  return <div className="empty-state">{message}</div>;
}

function RiskList({ highValue, repeatedIp, repeatedContact }) {
  const hasSignals = highValue.length || repeatedIp.length || repeatedContact.length;

  if (!hasSignals) {
    return <EmptyState message="No suspicious quote patterns in the current sample data." />;
  }

  return (
    <div className="risk-stack">
      {highValue.map((quote) => (
        <RiskItem
          key={`high-${quote.quote_event_id}`}
          label="High-value quote"
          score={quote.risk_score}
          detail={`${quote.site_name} / ${quote.service_name} / ${formatMoney(
            quote.quoted_price_cents,
          )}`}
          meta={quote.customer_email || 'no email'}
        />
      ))}

      {repeatedIp.map((ip) => (
        <RiskItem
          key={`ip-${ip.ip_address}`}
          label="Traffic concentration"
          score={ip.risk_score}
          detail={`${ip.quote_count} quotes from ${ip.ip_address}`}
          meta={`${formatDate(ip.first_seen)} - ${formatDate(ip.last_seen)}`}
        />
      ))}

      {repeatedContact.map((contact) => (
        <RiskItem
          key={`contact-${contact.customer_email}-${contact.customer_phone}`}
          label="Repeated contact"
          score={contact.risk_score}
          detail={`${contact.quote_count} quotes from ${contact.customer_email || 'unknown email'}`}
          meta={contact.customer_phone || 'no phone'}
        />
      ))}
    </div>
  );
}

function RiskItem({ label, score, detail, meta }) {
  return (
    <div className="risk-item">
      <div>
        <span className="risk-label">{label}</span>
        <strong>{detail}</strong>
        <small>{meta}</small>
      </div>
      <span className="risk-score">{score}</span>
    </div>
  );
}

export default App;
