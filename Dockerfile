FROM node:22-bookworm-slim AS dashboard-builder
WORKDIR /app/dashboard
COPY dashboard/package*.json ./
RUN npm ci
COPY dashboard ./
RUN npm run build

FROM rust:1-bookworm AS api-builder
WORKDIR /app
COPY api ./api
RUN cd api && cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates postgresql-client \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=api-builder /app/api/target/release/quotemakers-ops-api /app/quotemakers-ops-api
COPY --from=dashboard-builder /app/dashboard/dist /app/dashboard/dist
COPY db /app/db
COPY scripts/start.sh /app/scripts/start.sh
RUN chmod +x /app/scripts/start.sh
ENV STATIC_DIR=/app/dashboard/dist
CMD ["/app/scripts/start.sh"]
