#!/usr/bin/env bash
# Run a timed release build, print its performance summary and fail if the app
# did not produce one.
#
# Usage:
#   ./scripts/performance-check.sh
#   LPMT_PERF_SECONDS=60 ./scripts/performance-check.sh
#   LPMT_PERF_PROJECT=/path/show.json ./scripts/performance-check.sh
#   LPMT_FEATURES=video,audio,midi ./scripts/performance-check.sh
set -euo pipefail

if [[ ! -f Cargo.toml ]]; then
    echo "run this script from the LPMT repository root" >&2
    exit 2
fi

seconds="${LPMT_PERF_SECONDS:-30}"
profile="${LPMT_PERF_PROFILE:-release}"
features="${LPMT_FEATURES:-}"
if [[ "$profile" == "release" ]]; then
    binary="target/release/lpmt-app"
    build_args=(--release)
elif [[ "$profile" == "debug" ]]; then
    binary="target/debug/lpmt-app"
    build_args=()
else
    echo "LPMT_PERF_PROFILE must be release or debug" >&2
    exit 2
fi

if [[ "${LPMT_PERF_SKIP_BUILD:-0}" != "1" ]]; then
    if [[ -n "$features" ]]; then
        cargo build "${build_args[@]}" -p lpmt-app --features "$features"
    else
        cargo build "${build_args[@]}" -p lpmt-app
    fi
fi
if [[ ! -x "$binary" ]]; then
    echo "missing binary $binary" >&2
    exit 2
fi

args=(--run-seconds "$seconds")
if [[ "${LPMT_PERF_OPEN_OUTPUTS:-1}" == "1" ]]; then
    args+=(--open-outputs)
fi
if [[ -n "${LPMT_PERF_PROJECT:-}" ]]; then
    args+=(--project "$LPMT_PERF_PROJECT")
fi

log="$(mktemp)"
trap 'rm -f "$log"' EXIT

echo "== performance run: ${seconds}s, ${profile}, ${features:-default features} =="
RUST_LOG="${RUST_LOG:-info}" "$binary" "${args[@]}" 2>&1 | tee "$log"

if ! grep -q '^performance: ' "$log"; then
    echo "performance summary missing" >&2
    exit 1
fi

fps="$(sed -n 's/.*avg_fps=\([0-9.]*\).*/\1/p' "$log" | tail -n 1)"
min_fps="${LPMT_MIN_FPS:-0}"
if [[ -n "$fps" ]] && awk -v fps="$fps" -v min="$min_fps" 'BEGIN { exit !(fps + 0 < min + 0) }'; then
    echo "average fps $fps is below LPMT_MIN_FPS=$min_fps" >&2
    exit 1
fi

grep '^performance: ' "$log"
echo "performance check passed"
