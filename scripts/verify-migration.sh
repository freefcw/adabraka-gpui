#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=verify-common.sh
source "$SCRIPT_DIR/verify-common.sh"

readonly MIGRATION_PACKAGES=(
    adabraka-gpui-macros
    adabraka-gpui-core
    adabraka-gpui-wgpu
    adabraka-gpui-linux
    adabraka-gpui-macos
    adabraka-gpui-windows
    adabraka-gpui-platform
    adabraka-gpui
)

# Only the facade currently has a published library baseline. The macro package is
# proc-macro-only, which cargo-semver-checks does not support, and the extracted
# internal packages have not been published yet.
readonly SEMVER_PACKAGES=(
    adabraka-gpui
)

case "${1:-}" in
    "") ;;
    --semver)
        if ! command -v cargo-semver-checks >/dev/null 2>&1; then
            echo "cargo-semver-checks is required for --semver" >&2
            echo "Install it with: cargo install cargo-semver-checks --locked" >&2
            exit 1
        fi
        ;;
    *)
        echo "Usage: $0 [--semver]" >&2
        exit 2
        ;;
esac

echo "=== Migration and Release Verification ==="

echo "[1/5] Compile public surface and renderer..."
check_compile_baseline

echo "[2/5] Run core and compatibility tests..."
test_core --test-threads=1
test_public_contracts

echo "[3/5] Run workspace tests..."
cargo test --locked --workspace --lib --tests -- --test-threads=1

echo "[4/5] Verify macro archive and package inventories..."
cargo package --locked --allow-dirty -p adabraka-gpui-macros >/dev/null
echo "  verified archive: adabraka-gpui-macros"
for package in "${MIGRATION_PACKAGES[@]}"; do
    cargo package --locked --allow-dirty -p "$package" --list >/dev/null
    echo "  packaged: $package"
done

echo "[5/5] Check formatting and diff hygiene..."
cargo fmt --all -- --check
git diff --check

if [[ "${1:-}" == "--semver" ]]; then
    for package in "${SEMVER_PACKAGES[@]}"; do
        cargo semver-checks check-release -p "$package"
        echo "  semver checked: $package"
    done
else
    echo "Semantic API check skipped; run with --semver before release."
fi

echo "=== MIGRATION VERIFICATION PASSED ==="
