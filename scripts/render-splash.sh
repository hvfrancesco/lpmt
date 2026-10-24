#!/usr/bin/env bash
# Regenerate the embedded splash texture from the editable SVG source.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
rsvg-convert \
    --width 1940 \
    --height 916 \
    --output "$root/assets/lpmt_splash.png" \
    "$root/assets/lpmt_splash.svg"
