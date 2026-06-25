# Rust API Notes

Request flow:

1. Axum receives an HTTP request for a route like `GET /sites`.
2. The route calls a handler function such as `list_sites`.
3. The handler gets shared app state with the Postgres connection pool.
4. SQLx runs a SQL query against Postgres.
5. SQLx maps rows into Rust structs with `FromRow`.
6. Axum serializes those structs into JSON with `serde`.

Key pieces:

- `Router` maps URL paths to handler functions.
- `AppState` holds shared dependencies, currently the `PgPool`.
- `PgPool` reuses database connections instead of opening one per request.
- `query_as::<_, Type>()` runs SQL and maps each row into `Type`.
- `Json(value)` turns Rust data into an HTTP JSON response.
- `AppError` converts database failures into HTTP 500 responses.

Current API endpoints expose the same SQL ideas from the first chunk: site inventory, health-check history, and explainable risk rules.
