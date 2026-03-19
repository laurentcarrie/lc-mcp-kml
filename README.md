# lc-kml-utils

KML processing and map visualization platform for Ile-de-France transportation and geographic data.

## What it does

- Generate custom KML maps from YAML configuration files
- Visualize transportation networks (RER, Metro, buses) on interactive maps
- Perform geometric operations: concentric circles, polygon unions, route computation, triangle bisectors
- Serve a React frontend with Leaflet-based map rendering

## Architecture

**Backend** (Rust)

| Binary | Description |
|--------|-------------|
| `lc-kml-utils` | CLI: converts YAML config to KML output |
| `server` | HTTP API + serves the React frontend |
| `mcp-server` | MCP server exposing geospatial tools to Claude |

**Frontend** (React + Vite)

- Leaflet map with multi-layer KML overlay support
- YAML configuration editor
- Located in `frontend/`

## Prerequisites

- Rust 1.91+
- Node.js 22+
- AWS credentials with S3 access

## Environment variables

| Variable | Required | Description |
|----------|----------|-------------|
| `S3_BUCKET` | yes | S3 bucket name for KML library |
| `S3_REGION` | no | AWS region (defaults to `eu-west-3`) |
| `ANTHROPIC_API_KEY` | no | For AI features |
| `GOOGLE_API_KEY` | no | For Google Maps integration |
| `ORS_API_KEY` | no | For OpenRouteService routing |

## Build & run

```bash
# Build the Rust binaries
cargo build --release

# Build the frontend
cd frontend && npm install && npm run build && cd ..

# Run the server
S3_BUCKET=my-bucket cargo run --bin server

# CLI usage
S3_BUCKET=my-bucket cargo run -- input.yml output.kml
```

## Demo

A live instance is available at: https://39cty44qnv.eu-west-3.awsapprunner.com

## Deploy

Deployed to AWS App Runner via GitHub Actions on push to `main`. The workflow builds a Docker image, pushes to ECR, and updates the App Runner service.

## Project structure

```
src/
  main.rs          # CLI entrypoint
  model.rs         # Data model (EChoice, PointDefinition, etc.)
  processing.rs    # KML generation and geometric operations
  bin/
    server.rs      # HTTP API server
    mcp_server.rs  # MCP server for Claude
frontend/          # React + Vite app
idf/               # KML data (RER, metro, bus, communes)
```
