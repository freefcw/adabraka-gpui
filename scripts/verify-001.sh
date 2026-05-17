#!/bin/bash
set -e

echo "=== 001 - Profiler / Observability ==="

echo "[1/5] Compile checks..."
cargo check -p adabraka-gpui
cargo check -p adabraka-gpui --no-default-features
cargo check -p adabraka-gpui --no-default-features --features wgpu

echo "[2/5] Profiler tests..."
cargo test -p adabraka-gpui profiler
cargo test -p adabraka-gpui --lib --features test-support -- profiler

echo "[3/5] Executor regression tests..."
cargo test -p adabraka-gpui --lib --features test-support -- executor

echo "[4/5] Frozen tests (regression guard)..."
cargo test -p adabraka-gpui --lib --features test-support -- app::test
cargo test -p adabraka-gpui --lib --features test-support -- elements::list

echo "[5/5] Full lib test..."
cargo test -p adabraka-gpui --lib --features test-support

echo "=== 001 ALL PASSED ==="
