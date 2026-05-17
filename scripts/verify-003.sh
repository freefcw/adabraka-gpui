#!/bin/bash
set -e

echo "=== 003 - Visual Test Platform ==="

echo "[1/5] Compile checks..."
cargo check -p adabraka-gpui
cargo check -p adabraka-gpui --no-default-features
cargo check -p adabraka-gpui --no-default-features --features wgpu

echo "[2/5] Capability detection tests..."
cargo test -p adabraka-gpui --lib --features test-support -- visual_test_capabilities

echo "[3/5] Mock visual tests..."
cargo test -p adabraka-gpui --lib --features test-support -- visual_test

echo "[4/5] Frozen tests (regression guard)..."
cargo test -p adabraka-gpui --lib --features test-support -- app::test
cargo test -p adabraka-gpui --lib --features test-support -- executor

echo "[5/5] Full lib test..."
cargo test -p adabraka-gpui --lib --features test-support

echo ""
echo "(Optional) Real renderer smoke:"
echo "  cargo test -p adabraka-gpui --lib --features test-support -- real_visual --ignored"

echo "=== 003 ALL PASSED ==="
