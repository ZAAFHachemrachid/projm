#!/usr/bin/env bash
# projm installer — place binary in CARGO_HOME/bin
set -euo pipefail
echo "projm installer — manual use:"
echo "  1. Pick the binary for your platform from:"
echo "     $(dirname "$0")/"
echo "  2. Place it in ~/.cargo/bin/ (or anywhere on \$PATH)"
echo "  3. Run: projm init"
