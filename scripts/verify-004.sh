#!/bin/bash
set -e

echo "=== 004 - Scheduler / Queue ==="

echo "[1/6] Compile checks..."
cargo check -p adabraka-gpui
cargo check -p adabraka-gpui --no-default-features
cargo check -p adabraka-gpui --no-default-features --features wgpu

echo "[2/6] Baseline tests..."
cargo test -p adabraka-gpui --lib --features test-support -- baseline_

echo "[3/6] Priority tests..."
cargo test -p adabraka-gpui --lib --features test-support -- spawn_with_priority
cargo test -p adabraka-gpui --lib --features test-support -- priority

echo "[4/6] Dispatcher tests..."
cargo test -p adabraka-gpui --lib --features test-support -- dispatcher
cargo test -p adabraka-gpui --lib --features test-support -- executor

echo "[5/6] Frozen tests (regression guard)..."
cargo test -p adabraka-gpui --lib --features test-support -- app::test
cargo test -p adabraka-gpui --lib --features test-support -- elements::list
cargo test -p adabraka-gpui --lib --features test-support -- keymap

echo "[6/6] Full lib test..."
cargo test -p adabraka-gpui --lib --features test-support -- --test-threads=1

echo "=== 004 ALL PASSED ==="
