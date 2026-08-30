#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
# shellcheck source=verify-common.sh
source "$SCRIPT_DIR/verify-common.sh"

echo "=== 003 - Visual Test Platform ==="

echo "[1/5] Compile checks..."
check_compile_baseline

echo "[2/5] Capability detection tests..."
test_core visual_test_capabilities

echo "[3/5] Mock visual tests..."
test_core visual_test

echo "[4/5] Frozen tests (regression guard)..."
test_core app::test
test_core executor

echo "[5/5] Full core lib test..."
test_core --test-threads=1

echo
echo "Optional real renderer smoke:"
echo "  cargo test -p fc-gpui --test real_visual_smoke --features test-support -- --ignored"

echo "=== 003 ALL PASSED ==="
