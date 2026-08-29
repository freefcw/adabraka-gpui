#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
# shellcheck source=verify-common.sh
source "$SCRIPT_DIR/verify-common.sh"

echo "=== 001 - Profiler / Observability ==="

echo "[1/5] Compile checks..."
check_compile_baseline

echo "[2/5] Profiler tests..."
test_core profiler

echo "[3/5] Executor regression tests..."
test_core executor

echo "[4/5] Frozen tests (regression guard)..."
test_core app::test
test_core elements::list

echo "[5/5] Full core lib test..."
test_core --test-threads=1

echo "=== 001 ALL PASSED ==="
