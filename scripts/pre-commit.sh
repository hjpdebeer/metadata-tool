#!/bin/bash
set -e

echo "=== Pre-commit verification ==="

echo "[1/5] Checking formatting..."
cargo fmt --all -- --check

echo "[2/5] Running clippy..."
cargo clippy --workspace --all-targets -- -D warnings

echo "[3/5] Running tests..."
cargo test --workspace

echo "[4/5] Building docs..."
cargo doc --workspace --no-deps 2>&1 | grep -v "Documenting" || true

echo "[5/5] Building frontend..."
cd frontend && npm run build

echo "=== All checks passed ==="
