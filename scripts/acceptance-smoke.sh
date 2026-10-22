#!/usr/bin/env bash
# Automated portion of docs/acceptance.md.
# Run from the repository root.
set -euo pipefail

if [[ ! -f Cargo.toml ]]; then
    echo "run this script from the LPMT repository root" >&2
    exit 2
fi

echo "== formatting =="
cargo fmt --all -- --check

echo "== clippy =="
cargo clippy --workspace --all-targets -- -D warnings

echo "== workspace tests =="
cargo test --workspace

echo "== model/persistence acceptance =="
cargo test -p lpmt-core --test acceptance

echo "== OSC acceptance =="
cargo test -p lpmt-control

echo "== legacy importer acceptance =="
cargo test -p lpmt-import-legacy
legacy_dir="$(mktemp -d)"
trap 'rm -rf "$legacy_dir"' EXIT
cargo run -q -p lpmt-app -- \
    --import-legacy crates/lpmt-import-legacy/tests/fixtures/legacy-show.xml \
    --out "$legacy_dir/show.json"
test -s "$legacy_dir/show.json"

if [[ "${LPMT_ALL_FEATURES:-0}" == "1" ]]; then
    echo "== all-features tests =="
    cargo test --workspace --all-features

    echo "== video decoder smoke =="
    cargo run -p lpmt-media --features video --example decode-video -- \
        assets/video/fingers.mov

    echo "== video playback-control smoke =="
    cargo run -p lpmt-media --features video --example video-controls -- \
        assets/video/fingers.mov
fi

echo "automated acceptance smoke checks passed"
