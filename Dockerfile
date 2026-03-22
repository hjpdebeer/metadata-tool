# Stage 1: Build the frontend
FROM node:22-slim AS frontend-builder

WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm install --legacy-peer-deps
COPY frontend/ ./
RUN npm run build

# Stage 2: Build the Rust binary
FROM rust:1.88-slim-bookworm AS backend-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY backend/ backend/

# Build release binary
RUN cargo build --release --bin metadata-tool

# Stage 3: Minimal runtime image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=backend-builder /app/target/release/metadata-tool /app/metadata-tool

# Copy migrations (auto-run on startup by SQLx)
COPY backend/migrations /app/migrations

# Copy frontend build
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

# Non-root user for security
RUN useradd -r -s /bin/false appuser
USER appuser

ENV FRONTEND_DIR=/app/frontend/dist

EXPOSE 8080

CMD ["/app/metadata-tool"]
