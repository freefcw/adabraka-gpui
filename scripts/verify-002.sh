#!/bin/bash
set -e

echo "=== 002 - TestApp / Headless ==="

echo "[1/5] Compile checks..."
cargo check -p adabraka-gpui
cargo check -p adabraka-gpui --no-default-features
cargo check -p adabraka-gpui --no-default-features --features wgpu

echo "[2/5] TestApp tests..."
cargo test -p adabraka-gpui --lib --features test-support -- test_app

echo "[3/5] Headless tests..."
cargo test -p adabraka-gpui --lib --features test-support -- headless

echo "[4/5] Frozen tests (regression guard)..."
cargo test -p adabraka-gpui --lib --features test-support -- app::test
cargo test -p adabraka-gpui --lib --features test-support -- executor
cargo test -p adabraka-gpui --lib --features test-support -- text_system

echo "[5/5] Full lib test..."
cargo test -p adabraka-gpui --lib --features test-support -- --test-threads=1

echo "=== 002 ALL PASSED ==="
