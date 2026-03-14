FROM node:22-slim AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

FROM rust:1.91 AS builder
RUN apt-get update && apt-get install -y cmake && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release --bin server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libgcc-s1 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/server /usr/local/bin/server
RUN ldd /usr/local/bin/server || true
COPY --from=frontend /app/frontend/dist /app/frontend/dist
WORKDIR /app
ENV PORT=8080
ENV FRONTEND_DIR=/app/frontend/dist
EXPOSE 8080
CMD ["server"]
