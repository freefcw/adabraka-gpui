#!/usr/bin/env bash
set -euo pipefail

readonly PUBLIC_PACKAGE="adabraka-gpui"
readonly CORE_PACKAGE="adabraka-gpui-core"
readonly WGPU_PACKAGE="adabraka-gpui-wgpu"

check_compile_baseline() {
    cargo check --locked -p "$PUBLIC_PACKAGE"
    cargo check --locked -p "$PUBLIC_PACKAGE" --no-default-features
    cargo check --locked -p "$WGPU_PACKAGE"
}

test_core() {
    if [[ $# -gt 0 && "$1" != -* ]]; then
        local matching_tests
        matching_tests="$(
            cargo test --locked -p "$CORE_PACKAGE" --lib --features test-support -- "$1" --list
        )"
        if ! grep -q ': test$' <<<"$matching_tests"; then
            echo "No core tests matched filter: $1" >&2
            return 1
        fi
    fi

    cargo test --locked -p "$CORE_PACKAGE" --lib --features test-support -- "$@"
}

test_public_contracts() {
    cargo test --locked -p "$PUBLIC_PACKAGE" --tests --features test-support
}
