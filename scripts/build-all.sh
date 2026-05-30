#!/usr/bin/env bash
# ── build-all.sh ─────────────────────────────────────────────────
# Local cross-platform build for projm.
# Builds the CLI binary for every supported target and the Tauri
# desktop GUI for the native platform.
#
# Usage:
#   bash scripts/build-all.sh              # full build
#   bash scripts/build-all.sh --only-cli   # CLI only, skip frontend+Tauri
#   bash scripts/build-all.sh --skip-tauri # skip Tauri, do CLI + frontend
#
# Prerequisites: run `sudo bash scripts/install-deps.sh` first
#                (or manually install cross-compilers + `rustup target add`)
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
# NO set -e — we handle build failures explicitly
# so one failed target doesn't abort the entire script.

# ── Config ──────────────────────────────────────────────────────
VERSION="0.7.3"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="$PROJECT_ROOT/dist"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

# Targets we CAN build from Linux (with correct toolchain installed)
# Format: "triple:binary-suffix"
TARGETS=(
  "x86_64-unknown-linux-gnu:"                  # native Linux
  "aarch64-unknown-linux-gnu:"                 # ARM64 Linux
  "x86_64-pc-windows-gnu:.exe"                 # x86_64 Windows
)

# macOS targets — only if osxcross is on PATH
TARGETS_MAC=(
  "x86_64-apple-darwin:"                       # Intel Mac
  "aarch64-apple-darwin:"                      # Apple Silicon Mac
)

# ── Helpers ─────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'
info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
err()   { echo -e "${RED}[ERR]${NC}   $*" >&2; }
step()  { echo -e "\n${BLUE}━━━ $* ━━━${NC}"; }

check_tool() { command -v "$1" &>/dev/null; }

# ── Parse flags ─────────────────────────────────────────────────
SKIP_CLI=false
SKIP_TAURI=false
ONLY_CLI=false

for arg in "$@"; do
  case "$arg" in
    --skip-cli)   SKIP_CLI=true ;;
    --skip-tauri) SKIP_TAURI=true ;;
    --only-cli)   ONLY_CLI=true ;;
    *)            warn "Unknown flag: $arg" ;;
  esac
done

mkdir -p "$DIST_DIR"

# ═════════════════════════════════════════════════════════════════
# STEP 1: Next.js Frontend (for Tauri webview)
# ═════════════════════════════════════════════════════════════════
if [ "$ONLY_CLI" = false ] && [ "$SKIP_TAURI" = false ]; then
  step "1/4 — Building Next.js frontend"
  cd "$PROJECT_ROOT/app"
  if [ -d "node_modules" ]; then
    info "node_modules found, running build..."
  else
    info "Installing npm dependencies..."
    npm install
  fi
  npm run build
  info "Frontend built: app/.next/ + app/out/"
  cd "$PROJECT_ROOT"
else
  step "1/4 — Skipping frontend (--only-cli or --skip-tauri)"
fi

# ═════════════════════════════════════════════════════════════════
# STEP 2: Build CLI for each target
# ═════════════════════════════════════════════════════════════════
step "2/4 — Building CLI (projm) for all targets"

CLI_DIR="$DIST_DIR/projm-${VERSION}-cli"
mkdir -p "$CLI_DIR"

build_for_target() {
  local triple="$1"
  local suffix="$2"
  local output_name="${triple}${suffix}"

  info "Building for ${triple}..."
  if RUSTFLAGS="" cargo build --release --target "$triple" -p projm 2>&1; then
    local src="$PROJECT_ROOT/target/$triple/release/projm${suffix}"
    local dest="$CLI_DIR/$output_name"
    if [ -f "$src" ]; then
      cp "$src" "$dest"
      info "  ✓ $output_name  ($(ls -lh "$dest" | awk '{print $5}'))"
    else
      # For windows .exe we appended the suffix already
      local src_exe="$PROJECT_ROOT/target/$triple/release/projm${suffix}"
      if [ -f "$src_exe" ]; then
        cp "$src_exe" "$dest"
        info "  ✓ $output_name  ($(ls -lh "$dest" | awk '{print $5}'))"
      fi
    fi
  else
    warn "  ✗ $triple — build failed (missing linker or std?)"
    return 1
  fi
}

# ── Build Linux + Windows targets ──────────────────────────────
BUILT_COUNT=0
for entry in "${TARGETS[@]}"; do
  triple="${entry%%:*}"
  suffix="${entry#*:}"
  if build_for_target "$triple" "$suffix"; then
    ((BUILT_COUNT++))
  fi
done

# ── Build macOS targets (only if osxcross is installed) ─────────
if command -v x86_64-apple-darwin-cc &>/dev/null || command -v osxcross &>/dev/null; then
  info "osxcross detected — building for macOS targets"
  for entry in "${TARGETS_MAC[@]}"; do
    triple="${entry%%:*}"
    suffix="${entry#*:}"
    if build_for_target "$triple" "$suffix"; then
      ((BUILT_COUNT++))
    fi
  done
else
  info "osxcross not found — skipping macOS targets (x86_64-apple-darwin, aarch64-apple-darwin)"
  info "  Install osxcross from AUR: yay -S osxcross"
  info "  or manually: https://github.com/tpoechtrager/osxcross"
fi

# ── Generate checksums ─────────────────────────────────────────
step "   — Generating checksums"
cd "$CLI_DIR"
sha256sum -- * > "$DIST_DIR/projm-${VERSION}-cli-sha256sums.txt" 2>/dev/null || \
  sha256sum -p -- * > "$DIST_DIR/projm-${VERSION}-cli-sha256sums.txt" 2>/dev/null || true
cd "$PROJECT_ROOT"

info "Built $BUILT_COUNT CLI targets"
info "Artifacts: $CLI_DIR/"

# ═════════════════════════════════════════════════════════════════
# STEP 3: Installer scripts (simplified shell/powershell)
# ═════════════════════════════════════════════════════════════════
step "3/4 — Generating installer scripts"

mkdir -p "$DIST_DIR/installers"
cat > "$DIST_DIR/installers/install.sh" << 'INSTALL_SCRIPT'
#!/usr/bin/env bash
# projm installer — place binary in CARGO_HOME/bin
set -euo pipefail
echo "projm installer — manual use:"
echo "  1. Pick the binary for your platform from:"
echo "     $(dirname "$0")/"
echo "  2. Place it in ~/.cargo/bin/ (or anywhere on \$PATH)"
echo "  3. Run: projm init"
INSTALL_SCRIPT
chmod +x "$DIST_DIR/installers/install.sh"

info "Installer: $DIST_DIR/installers/install.sh"

# ═════════════════════════════════════════════════════════════════
# STEP 4: Tauri Desktop GUI (native platform only)
# ═════════════════════════════════════════════════════════════════
if [ "$ONLY_CLI" = true ] || [ "$SKIP_TAURI" = true ]; then
  step "4/4 — Skipping Tauri GUI"
else
  step "4/4 — Building Tauri desktop GUI (native)"
  info "Building projm-tauri for $(uname -m)..."

  # Tauri app needs the Next.js frontend built first (STEP 1)

  cd "$PROJECT_ROOT"
  if cargo build --release -p projm-tauri 2>&1; then
    tauri_src="$PROJECT_ROOT/target/release/projm-tauri"
    tauri_dest="$DIST_DIR/projm-tauri-${VERSION}-$(uname -m)-linux"
    if [ -f "$tauri_src" ]; then
      cp "$tauri_src" "$tauri_dest"
      info "  ✓ projm-tauri  ($(ls -lh "$tauri_dest" | awk '{print $5}'))"
    fi
  else
    warn "  ✗ projm-tauri — build failed"
    warn "  Ensure Tauri deps are installed: gtk3, webkit2gtk-4.1, librsvg, etc."
  fi
fi

# ═════════════════════════════════════════════════════════════════
# Summary
# ═════════════════════════════════════════════════════════════════
echo ""
echo -e "${GREEN}══════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  Build complete!                                   ${NC}"
echo -e "${GREEN}  Version: $VERSION${NC}"
echo -e "${GREEN}  Output:  $DIST_DIR/${NC}"
echo -e "${GREEN}══════════════════════════════════════════════════${NC}"
echo ""
ls -lh "$DIST_DIR/" 2>/dev/null
echo ""
