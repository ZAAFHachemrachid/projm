#!/usr/bin/env bash
# ── install-deps.sh ──
# Install all cross-compilation dependencies for projm local builds.
# Supports: Arch Linux (pacman), Debian/Ubuntu (apt), macOS (brew)
#
# Usage: bash scripts/install-deps.sh

set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
err()   { echo -e "${RED}[ERR]${NC} $*" >&2; }

detect_pkg_manager() {
  if command -v pacman &>/dev/null; then echo "pacman"
  elif command -v apt &>/dev/null; then echo "apt"
  elif command -v brew &>/dev/null; then echo "brew"
  else echo "unknown"; fi
}

install_rust_targets() {
  info "Installing Rust cross-compilation targets..."
  local targets=(
    aarch64-unknown-linux-gnu    # ARM64 Linux
    x86_64-pc-windows-gnu        # x86_64 Windows (GNU)
  )
  for target in "${targets[@]}"; do
    rustup target add "$target" 2>/dev/null && info "  ✓ $target" || warn "  ✗ $target (may need --force)"
  done
}

install_arch() {
  info "Detected Arch Linux — using pacman"

  sudo pacman -S --noconfirm --needed \
    mingw-w64-gcc mingw-w64-crt mingw-w64-winpthreads \
    mingw-w64-headers mingw-w64-binutils \
    aarch64-linux-gnu-gcc aarch64-linux-gnu-glibc \
    aarch64-linux-gnu-binutils aarch64-linux-gnu-linux-api-headers

  # Optional: llvm-mingw for aarch64 Windows (from chaotic-aur)
  if command -v yay &>/dev/null; then
    warn "llvm-mingw (aarch64-pc-windows) available via: yay -S llvm-mingw"
    warn "osxcross (macOS cross) available via: yay -S osxcross"

    # Install rustup MSVC targets (need llvm-mingw linker)
    rustup target add aarch64-pc-windows-msvc 2>/dev/null || true
    rustup target add x86_64-pc-windows-msvc 2>/dev/null || true
  fi

  install_rust_targets

  info "All Arch dependencies installed."
}

install_debian() {
  info "Detected Debian/Ubuntu — using apt"

  sudo apt update
  sudo apt install -y \
    gcc-aarch64-linux-gnu g++-aarch64-linux-gnu \
    mingw-w64 \
    libgtk-3-dev libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev librsvg2-dev \
    patchelf libssl-dev

  install_rust_targets

  info "All Debian/Ubuntu dependencies installed."
}

install_macos() {
  info "Detected macOS — using Homebrew"

  # Tauri deps (already present on macOS usually)
  # Cross targets already available via rustup on macOS for most Apple targets

  install_rust_targets

  info "All macOS dependencies installed."
}

# ── Main ──
info "projm — Cross-compilation dependency installer"
echo ""

PKG_MANAGER=$(detect_pkg_manager)
case "$PKG_MANAGER" in
  pacman) install_arch ;;
  apt)    install_debian ;;
  brew)   install_macos ;;
  *)      warn "Unknown package manager. Installing Rust targets only..."
          install_rust_targets
          echo ""
          info "System dependencies must be installed manually:"
          info "  - mingw-w64 cross-compiler (for Windows targets)"
          info "  - aarch64-linux-gnu-gcc (for ARM64 Linux targets)"
          info "  - osxcross + macOS SDK (for macOS targets, Linux only)"
          ;;
esac

echo ""
info "Done! Run 'rustup target list --installed' to verify."
