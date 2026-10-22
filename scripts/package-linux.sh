#!/usr/bin/env bash
# Build a portable Linux release directory and tarball.
#
# Usage:
#   ./scripts/package-linux.sh
#   LPMT_FEATURES=video,audio,midi ./scripts/package-linux.sh
#   LPMT_SKIP_BUILD=1 ./scripts/package-linux.sh
set -euo pipefail

if [[ ! -f Cargo.toml ]]; then
    echo "run this script from the LPMT repository root" >&2
    exit 2
fi

version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
if [[ -z "$version" ]]; then
    echo "could not read the workspace version from Cargo.toml" >&2
    exit 2
fi
arch="${LPMT_TARGET_ARCH:-$(uname -m)}"
features="${LPMT_FEATURES:-}"
package="lpmt-${version}-linux-${arch}"
dist="$(pwd)/dist"
stage="$dist/$package"
binary="target/release/lpmt-app"

if [[ "${LPMT_SKIP_BUILD:-0}" != "1" ]]; then
    echo "== building release (${features:-default features}) =="
    if [[ -n "$features" ]]; then
        cargo build --release -p lpmt-app --features "$features"
    else
        cargo build --release -p lpmt-app
    fi
fi
if [[ ! -x "$binary" ]]; then
    echo "missing release binary $binary; run without LPMT_SKIP_BUILD=1" >&2
    exit 2
fi

rm -rf "$stage"
mkdir -p "$stage/assets" "$stage/docs" "$stage/packaging"

install -m755 "$binary" "$stage/lpmt"
cp -R assets/. "$stage/assets/"
cp README.md COPYING AUTHORS "$stage/"
cp docs/acceptance.md docs/rewrite-rfc.md docs/packaging.md "$stage/docs/"
cp packaging/linux/lpmt.desktop packaging/linux/install.sh "$stage/"
chmod +x "$stage/install.sh"

cat > "$stage/BUILD_INFO.txt" <<EOF
LPMT $version
Target: linux-$arch
Features: ${features:-default}
Built: $(date -u +%Y-%m-%dT%H:%M:%SZ)
Commit: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)
Rust: $(rustc --version 2>/dev/null || echo unknown)
EOF

tar -C "$dist" -czf "$dist/$package.tar.gz" "$package"
if command -v sha256sum >/dev/null 2>&1; then
    (cd "$dist" && sha256sum "$package.tar.gz" > "$package.tar.gz.sha256")
fi

echo "== package =="
echo "$dist/$package.tar.gz"
if [[ -f "$dist/$package.tar.gz.sha256" ]]; then
    cat "$dist/$package.tar.gz.sha256"
fi
