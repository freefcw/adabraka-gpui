#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=verify-common.sh
source "$SCRIPT_DIR/verify-common.sh"

echo "=== 002 - TestApp / Headless ==="

echo "[1/5] Compile checks..."
check_compile_baseline

echo "[2/5] TestApp tests..."
test_core test_app

echo "[3/5] Headless tests..."
test_core headless

echo "[4/5] Frozen tests (regression guard)..."
test_core app::test
test_core executor
test_core text_system

echo "[5/5] Full core lib test..."
test_core --test-threads=1

echo "=== 002 ALL PASSED ==="
