use std::{collections::HashSet, env, time::Duration, time::Instant};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use reqwest::header::CONTENT_TYPE;
use serde::Serialize;
use serde_json::Value;
use sqlx::{FromRow, PgPool, postgres::PgPoolOptions};
use tokio::{process::Command, time};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    http: reqwest::Client,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://omarm@localhost/quotemakers_ops_desk?host=/var/run/postgresql".to_string()
    });
    let bind_addr = env::var("API_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("QuoteMakersOpsDesk/0.1")
        .build()?;

    let state = AppState { pool, http };

    if env_flag("RUN_CHECKS_ON_START") {
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = run_checks_once(&state).await {
                eprintln!("startup check run failed: {}", error.0);
            }
        });
    }

    if env_flag("AUTO_RUN_CHECKS") {
        spawn_check_scheduler(state.clone());
    }

    let static_dir = env::var("STATIC_DIR").unwrap_or_else(|_| "../dashboard/dist".to_string());
    let spa =
        ServeDir::new(&static_dir).fallback(ServeFile::new(format!("{static_dir}/index.html")));
    let app = Router::new()
        .route("/api", get(root))
        .nest("/api", api_routes())
        .merge(api_routes())
        .fallback_service(spa)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    println!("QuoteMakers Ops API listening on http://{bind_addr}");

    axum::serve(listener, app).await?;
    Ok(())
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/sites", get(list_sites))
        .route("/checks/run", post(run_checks))
        .route("/inventory/railway/sync", post(sync_railway_inventory))
        .route("/health-checks/recent", get(recent_health_checks))
        .route("/risk/high-value", get(high_value_quotes))
        .route("/risk/repeated-ip", get(repeated_ip_quotes))
        .route("/risk/repeated-contact", get(repeated_contacts))
        .route("/ops/failed-health-checks", get(failed_health_checks))
}

fn spawn_check_scheduler(state: AppState) {
    let interval_seconds = env::var("CHECK_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1_800);

    tokio::spawn(async move {
        loop {
            time::sleep(Duration::from_secs(interval_seconds)).await;

            match run_checks_once(&state).await {
                Ok(summary) => eprintln!(
                    "scheduled check run: {} sites, {} checks, {} failures",
                    summary.checked_sites, summary.inserted_checks, summary.failures
                ),
                Err(error) => eprintln!("scheduled check run failed: {}", error.0),
            }
        }
    });
}

fn env_flag(key: &str) -> bool {
    env::var(key)
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

async fn root() -> &'static str {
    "QuoteMakers Ops Desk API"
}

async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, AppError> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;

    Ok(Json(HealthResponse {
        status: "ok",
        database: "ok",
    }))
}

async fn list_sites(State(state): State<AppState>) -> Result<Json<Vec<Site>>, AppError> {
    let sites = sqlx::query_as::<_, Site>(
        r#"
        SELECT id, name, base_url, homepage_path, critical_css_url, enabled,
               source, external_service_id, service_name, domain_type,
               last_seen_at, created_at, updated_at
        FROM sites
        WHERE enabled = TRUE
        ORDER BY name
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(sites))
}

async fn run_checks(State(state): State<AppState>) -> Result<Json<RunChecksResponse>, AppError> {
    Ok(Json(run_checks_once(&state).await?))
}

async fn run_checks_once(state: &AppState) -> Result<RunChecksResponse, AppError> {
    let sites = sqlx::query_as::<_, Site>(
        r#"
        SELECT id, name, base_url, homepage_path, critical_css_url, enabled,
               source, external_service_id, service_name, domain_type,
               last_seen_at, created_at, updated_at
        FROM sites
        WHERE enabled = TRUE
        ORDER BY name
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let mut checks = Vec::new();

    for site in &sites {
        for target in check_targets(site) {
            let probe = execute_http_check(&state.http, target).await;
            let row = insert_health_check(&state.pool, site.id, &probe).await?;
            checks.push(row);
        }
    }

    let failures = checks.iter().filter(|check| !check.ok).count();

    Ok(RunChecksResponse {
        checked_sites: sites.len(),
        inserted_checks: checks.len(),
        failures,
        checks,
    })
}

async fn sync_railway_inventory(
    State(state): State<AppState>,
) -> Result<Json<RailwaySyncResponse>, AppError> {
    let output = Command::new("railway")
        .arg("status")
        .arg("--json")
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::message(format!(
            "railway status --json failed: {stderr}"
        )));
    }

    let json: Value = serde_json::from_slice(&output.stdout)?;
    let mut services = Vec::new();
    collect_railway_services(&json, &mut services);

    let mut seen_services = HashSet::new();
    let mut seen_domains = HashSet::new();
    let mut upserted_sites = 0;
    let mut skipped_without_domains = Vec::new();

    for service in services {
        let service_key = service
            .service_id
            .clone()
            .unwrap_or_else(|| service.service_name.clone());

        if !seen_services.insert(service_key) || is_infra_service(&service.service_name) {
            continue;
        }

        let domains = service.domains();

        if domains.is_empty() {
            skipped_without_domains.push(RailwayServiceSummary {
                service_name: service.service_name,
                deployment_status: service.deployment_status,
                instance_statuses: service.instance_statuses,
            });
            continue;
        }

        for (domain, domain_type) in domains {
            if !seen_domains.insert(domain.clone()) {
                continue;
            }

            upsert_site_from_railway(&state.pool, &service, &domain, domain_type).await?;
            upserted_sites += 1;
        }
    }

    Ok(Json(RailwaySyncResponse {
        upserted_sites,
        domains_seen: seen_domains.len(),
        services_without_domains: skipped_without_domains,
    }))
}

async fn recent_health_checks(
    State(state): State<AppState>,
) -> Result<Json<Vec<HealthCheck>>, AppError> {
    let checks = sqlx::query_as::<_, HealthCheck>(
        r#"
        SELECT id, site_id, target_type, url, status_code, content_type,
               latency_ms, ok, failure_reason, checked_at
        FROM health_checks
        ORDER BY checked_at DESC
        LIMIT 80
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(checks))
}

async fn high_value_quotes(
    State(state): State<AppState>,
) -> Result<Json<Vec<HighValueQuote>>, AppError> {
    let rows = sqlx::query_as::<_, HighValueQuote>(
        r#"
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
        WHERE qe.quoted_price_cents >= 100000
        ORDER BY qe.quoted_price_cents DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

async fn repeated_ip_quotes(
    State(state): State<AppState>,
) -> Result<Json<Vec<RepeatedIp>>, AppError> {
    let rows = sqlx::query_as::<_, RepeatedIp>(
        r#"
        SELECT
          qe.ip_address::text AS ip_address,
          COUNT(*) AS quote_count,
          MIN(qe.submitted_at) AS first_seen,
          MAX(qe.submitted_at) AS last_seen,
          'repeated_ip' AS rule_name,
          50 AS risk_score
        FROM quote_events qe
        WHERE qe.ip_address IS NOT NULL
        GROUP BY qe.ip_address
        HAVING COUNT(*) >= 3
        ORDER BY quote_count DESC, last_seen DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

async fn repeated_contacts(
    State(state): State<AppState>,
) -> Result<Json<Vec<RepeatedContact>>, AppError> {
    let rows = sqlx::query_as::<_, RepeatedContact>(
        r#"
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
        HAVING COUNT(*) >= 2
        ORDER BY quote_count DESC, last_seen DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

async fn failed_health_checks(
    State(state): State<AppState>,
) -> Result<Json<Vec<FailedHealthCheck>>, AppError> {
    let rows = sqlx::query_as::<_, FailedHealthCheck>(
        r#"
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
        WHERE s.enabled = TRUE
          AND hc.ok = FALSE
          AND hc.checked_at >= now() - interval '24 hours'
        ORDER BY hc.checked_at DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

fn check_targets(site: &Site) -> Vec<CheckTarget> {
    vec![
        CheckTarget {
            target_type: "homepage",
            url: join_url(&site.base_url, &site.homepage_path),
            expected_content_types: &["text/html"],
        },
        CheckTarget {
            target_type: "css",
            url: site_asset_url(site, &site.critical_css_url),
            expected_content_types: &["text/css"],
        },
        CheckTarget {
            target_type: "robots",
            url: join_url(&site.base_url, "/robots.txt"),
            expected_content_types: &["text/plain"],
        },
        CheckTarget {
            target_type: "sitemap",
            url: join_url(&site.base_url, "/sitemap.xml"),
            expected_content_types: &["application/xml", "text/xml"],
        },
    ]
}

async fn execute_http_check(client: &reqwest::Client, target: CheckTarget) -> ProbeResult {
    let started = Instant::now();
    let response = client.get(&target.url).send().await;
    let latency_ms = elapsed_ms(started);

    match response {
        Ok(response) => {
            let status_code = i32::from(response.status().as_u16());
            let content_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned);
            let status_ok = status_code == 200;
            let content_ok =
                content_type_matches(content_type.as_deref(), target.expected_content_types);
            let ok = status_ok && content_ok;
            let failure_reason = if !status_ok {
                Some(format!("expected 200, got {status_code}"))
            } else if !content_ok {
                Some(format!(
                    "expected content type {}, got {}",
                    target.expected_content_types.join(" or "),
                    content_type.as_deref().unwrap_or("missing")
                ))
            } else {
                None
            };

            ProbeResult {
                target_type: target.target_type,
                url: target.url,
                status_code: Some(status_code),
                content_type,
                latency_ms,
                ok,
                failure_reason,
            }
        }
        Err(error) => ProbeResult {
            target_type: target.target_type,
            url: target.url,
            status_code: None,
            content_type: None,
            latency_ms,
            ok: false,
            failure_reason: Some(format!("request failed: {error}")),
        },
    }
}

async fn insert_health_check(
    pool: &PgPool,
    site_id: i64,
    probe: &ProbeResult,
) -> Result<HealthCheck, sqlx::Error> {
    sqlx::query_as::<_, HealthCheck>(
        r#"
        INSERT INTO health_checks (
          site_id, target_type, url, status_code, content_type,
          latency_ms, ok, failure_reason
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, site_id, target_type, url, status_code, content_type,
                  latency_ms, ok, failure_reason, checked_at
        "#,
    )
    .bind(site_id)
    .bind(probe.target_type)
    .bind(&probe.url)
    .bind(probe.status_code)
    .bind(&probe.content_type)
    .bind(probe.latency_ms)
    .bind(probe.ok)
    .bind(&probe.failure_reason)
    .fetch_one(pool)
    .await
}

async fn upsert_site_from_railway(
    pool: &PgPool,
    service: &RailwayService,
    domain: &str,
    domain_type: &'static str,
) -> Result<i64, sqlx::Error> {
    let base_url = domain_to_base_url(domain);
    let critical_css_url = join_url(&base_url, "/static/assets/css/main.css");
    let name = if domain_type == "custom" {
        service.service_name.clone()
    } else {
        format!("{} (Railway)", service.service_name)
    };

    sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO sites (
          name, base_url, homepage_path, critical_css_url, enabled,
          source, external_service_id, service_name, domain_type,
          last_seen_at, updated_at
        )
        VALUES ($1, $2, '/', $3, TRUE, 'railway', $4, $5, $6, now(), now())
        ON CONFLICT (base_url) DO UPDATE SET
          name = EXCLUDED.name,
          critical_css_url = EXCLUDED.critical_css_url,
          source = EXCLUDED.source,
          external_service_id = EXCLUDED.external_service_id,
          service_name = EXCLUDED.service_name,
          domain_type = EXCLUDED.domain_type,
          last_seen_at = now(),
          updated_at = now()
        RETURNING id
        "#,
    )
    .bind(name)
    .bind(base_url)
    .bind(critical_css_url)
    .bind(&service.service_id)
    .bind(&service.service_name)
    .bind(domain_type)
    .fetch_one(pool)
    .await
}

fn collect_railway_services(value: &Value, services: &mut Vec<RailwayService>) {
    match value {
        Value::Object(map) => {
            if map.contains_key("serviceName") && map.contains_key("domains") {
                if let Some(service) = parse_railway_service(value) {
                    services.push(service);
                }
            }

            for child in map.values() {
                collect_railway_services(child, services);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_railway_services(child, services);
            }
        }
        _ => {}
    }
}

fn parse_railway_service(value: &Value) -> Option<RailwayService> {
    let service_name = value.get("serviceName")?.as_str()?.to_string();
    let service_id = value
        .get("serviceId")
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let domains = value.get("domains")?;
    let custom_domains = domain_values(domains, "customDomains");
    let service_domains = domain_values(domains, "serviceDomains");
    let latest_deployment = value.get("latestDeployment");
    let deployment_status = latest_deployment
        .and_then(|deployment| deployment.get("status"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let instance_statuses = latest_deployment
        .and_then(|deployment| deployment.get("instances"))
        .and_then(Value::as_array)
        .map(|instances| {
            instances
                .iter()
                .filter_map(|instance| instance.get("status").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();

    Some(RailwayService {
        service_id,
        service_name,
        custom_domains,
        service_domains,
        deployment_status,
        instance_statuses,
    })
}

fn domain_values(domains: &Value, key: &str) -> Vec<String> {
    domains
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("domain").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn is_infra_service(service_name: &str) -> bool {
    let normalized = service_name.trim().to_ascii_lowercase();
    normalized == "redis" || normalized.ends_with(" db")
}

fn join_url(base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };

    format!("{base}{path}")
}

fn site_asset_url(site: &Site, value: &str) -> String {
    if value.starts_with("http://") || value.starts_with("https://") {
        value.to_string()
    } else {
        join_url(&site.base_url, value)
    }
}

fn domain_to_base_url(domain: &str) -> String {
    let cleaned = domain
        .trim()
        .trim_end_matches('/')
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    format!("https://{cleaned}")
}

fn content_type_matches(content_type: Option<&str>, expected: &[&str]) -> bool {
    let Some(content_type) = content_type else {
        return false;
    };
    let normalized = content_type.to_ascii_lowercase();

    expected.iter().any(|item| normalized.contains(item))
}

fn elapsed_ms(started: Instant) -> i32 {
    i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX)
}

#[derive(Debug)]
struct CheckTarget {
    target_type: &'static str,
    url: String,
    expected_content_types: &'static [&'static str],
}

#[derive(Debug)]
struct ProbeResult {
    target_type: &'static str,
    url: String,
    status_code: Option<i32>,
    content_type: Option<String>,
    latency_ms: i32,
    ok: bool,
    failure_reason: Option<String>,
}

#[derive(Clone, Debug)]
struct RailwayService {
    service_id: Option<String>,
    service_name: String,
    custom_domains: Vec<String>,
    service_domains: Vec<String>,
    deployment_status: Option<String>,
    instance_statuses: Vec<String>,
}

impl RailwayService {
    fn domains(&self) -> Vec<(String, &'static str)> {
        self.custom_domains
            .iter()
            .map(|domain| (domain.clone(), "custom"))
            .chain(
                self.service_domains
                    .iter()
                    .map(|domain| (domain.clone(), "railway_service")),
            )
            .collect()
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    database: &'static str,
}

#[derive(Serialize)]
struct RunChecksResponse {
    checked_sites: usize,
    inserted_checks: usize,
    failures: usize,
    checks: Vec<HealthCheck>,
}

#[derive(Serialize)]
struct RailwaySyncResponse {
    upserted_sites: usize,
    domains_seen: usize,
    services_without_domains: Vec<RailwayServiceSummary>,
}

#[derive(Serialize)]
struct RailwayServiceSummary {
    service_name: String,
    deployment_status: Option<String>,
    instance_statuses: Vec<String>,
}

#[derive(FromRow, Serialize)]
struct Site {
    id: i64,
    name: String,
    base_url: String,
    homepage_path: String,
    critical_css_url: String,
    enabled: bool,
    source: String,
    external_service_id: Option<String>,
    service_name: Option<String>,
    domain_type: String,
    last_seen_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow, Serialize)]
struct HealthCheck {
    id: i64,
    site_id: i64,
    target_type: String,
    url: String,
    status_code: Option<i32>,
    content_type: Option<String>,
    latency_ms: Option<i32>,
    ok: bool,
    failure_reason: Option<String>,
    checked_at: DateTime<Utc>,
}

#[derive(FromRow, Serialize)]
struct HighValueQuote {
    quote_event_id: i64,
    site_name: String,
    service_name: String,
    customer_email: Option<String>,
    quoted_price_cents: i32,
    rule_name: String,
    risk_score: i32,
}

#[derive(FromRow, Serialize)]
struct RepeatedIp {
    ip_address: String,
    quote_count: i64,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    rule_name: String,
    risk_score: i32,
}

#[derive(FromRow, Serialize)]
struct RepeatedContact {
    customer_email: Option<String>,
    customer_phone: Option<String>,
    quote_count: i64,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    rule_name: String,
    risk_score: i32,
}

#[derive(FromRow, Serialize)]
struct FailedHealthCheck {
    health_check_id: i64,
    site_name: String,
    target_type: String,
    url: String,
    status_code: Option<i32>,
    content_type: Option<String>,
    failure_reason: Option<String>,
    rule_name: String,
    risk_score: i32,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

struct AppError(anyhow::Error);

impl AppError {
    fn message(message: impl std::fmt::Display) -> Self {
        Self(anyhow::anyhow!("{message}"))
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self(error.into())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        Self(error.into())
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self(error.into())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self(error.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        eprintln!("API error: {}", self.0);

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "request failed",
            }),
        )
            .into_response()
    }
}
