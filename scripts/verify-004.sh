#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=verify-common.sh
source "$SCRIPT_DIR/verify-common.sh"

echo "=== 004 - Scheduler / Queue Test Prototype ==="

echo "[1/6] Compile checks..."
check_compile_baseline

echo "[2/6] Baseline tests..."
test_core baseline_

echo "[3/6] Priority tests..."
test_core spawn_with_priority
test_core priority

echo "[4/6] Dispatcher tests..."
test_core dispatcher
test_core executor

echo "[5/6] Frozen tests (regression guard)..."
test_core app::test
test_core elements::list
test_core keymap

echo "[6/6] Full core lib test..."
test_core --test-threads=1

echo "=== 004 TEST PROTOTYPE PASSED ==="
