#!/usr/bin/env bash
# Install an extracted LPMT Linux package under $PREFIX (default: ~/.local).
set -euo pipefail

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
prefix="${PREFIX:-$HOME/.local}"
bindir="$prefix/bin"
sharedir="$prefix/share/lpmt"
applications="$prefix/share/applications"
icons="$prefix/share/icons/hicolor/256x256/apps"

install -d "$bindir" "$sharedir" "$applications" "$icons"
install -m755 "$here/lpmt" "$bindir/lpmt"
rm -rf "$sharedir/assets"
cp -R "$here/assets" "$sharedir/assets"
install -m644 "$here/COPYING" "$sharedir/COPYING"
install -m644 "$here/lpmt.desktop" "$applications/lpmt.desktop"
install -m644 "$here/assets/lpmt_splash.png" "$icons/lpmt.png"

echo "Installed LPMT to $prefix"
echo "Run: $bindir/lpmt"
echo "The desktop entry is $applications/lpmt.desktop"
