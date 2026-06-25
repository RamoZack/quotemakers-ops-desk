use std::env;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool, postgres::PgPoolOptions};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
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

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/sites", get(list_sites))
        .route("/health-checks/recent", get(recent_health_checks))
        .route("/risk/high-value", get(high_value_quotes))
        .route("/risk/repeated-ip", get(repeated_ip_quotes))
        .route("/risk/repeated-contact", get(repeated_contacts))
        .route("/ops/failed-health-checks", get(failed_health_checks))
        .with_state(AppState { pool });

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    println!("QuoteMakers Ops API listening on http://{bind_addr}");

    axum::serve(listener, app).await?;
    Ok(())
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
        SELECT id, name, base_url, homepage_path, critical_css_url, enabled, created_at
        FROM sites
        ORDER BY name
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(sites))
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
        LIMIT 20
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
        WHERE hc.ok = FALSE
          AND hc.checked_at >= now() - interval '24 hours'
        ORDER BY hc.checked_at DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    database: &'static str,
}

#[derive(FromRow, Serialize)]
struct Site {
    id: i64,
    name: String,
    base_url: String,
    homepage_path: String,
    critical_css_url: String,
    enabled: bool,
    created_at: DateTime<Utc>,
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

struct AppError(sqlx::Error);

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self(error)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        eprintln!("API error: {}", self.0);

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "database query failed",
            }),
        )
            .into_response()
    }
}
