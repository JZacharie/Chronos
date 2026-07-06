#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# local-ci.sh for Chronos
# Miroir de la CI GitHub, adapté à la plateforme hôte.
# ============================================================

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'
PASS=0
FAIL=0

OS="$(uname -s)"

check() {
    local name="$1"
    shift
    echo -e "${YELLOW}━━━ [$name] ━━━${NC}"
    if "$@" 2>&1; then
        echo -e "${GREEN}✓ $name passed${NC}"
        PASS=$((PASS + 1))
    else
        echo -e "${RED}✗ $name failed${NC}"
        FAIL=$((FAIL + 1))
    fi
    echo
}

echo -e "${YELLOW}══════════════════════════════════════${NC}"
echo -e "${YELLOW}  Local CI — Chronos (${OS})${NC}"
echo -e "${YELLOW}  $(date)${NC}"
echo -e "${YELLOW}══════════════════════════════════════${NC}"
echo

# ── Prechecks ─────────────────────────────────────────────
check "cargo fmt" cargo fmt --all --check

# ── Quality & Tests ───────────────────────────────────────
check "cargo clippy" cargo clippy --workspace --all-targets -- -D warnings

LINUX_DEPS_OK=true
if [[ "$OS" == "Linux" ]]; then
    MISSING_PKGS=()
    for lib in xcb xi gtk+-3.0 xkbcommon xtst; do
        if ! pkg-config --exists "$lib" 2>/dev/null; then
            MISSING_PKGS+=("$lib")
            LINUX_DEPS_OK=false
        fi
    done
    if [ "$LINUX_DEPS_OK" = false ]; then
        echo -e "${RED}✗ Dépendances système Linux manquantes : ${MISSING_PKGS[*]}${NC}"
        echo -e "${YELLOW}  → Installe-les avec :${NC}"
        echo -e "${YELLOW}    sudo apt-get install -y \\${NC}"
        echo -e "${YELLOW}      libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \\${NC}"
        echo -e "${YELLOW}      libxcb1-dev libx11-dev libxi-dev libxtst-dev libxkbcommon-dev \\${NC}"
        echo -e "${YELLOW}      libgtk-3-dev libatk1.0-dev libcairo2-dev libglib2.0-dev libpango1.0-dev \\${NC}"
        echo -e "${YELLOW}      libssl-dev pkg-config${NC}"
        echo -e "${YELLOW}  → Tests sautés. Exécute cargo check comme substitut.${NC}"
        check "cargo check" cargo check --lib
    fi
fi

if [ "$LINUX_DEPS_OK" = true ]; then
    check "cargo test" cargo test --workspace --all-features
fi

# ── Build ─────────────────────────────────────────────────
if [ "$LINUX_DEPS_OK" = true ]; then
    check "cargo build --release" cargo build --release
fi

# ── Summary ───────────────────────────────────────────────
echo -e "${YELLOW}══════════════════════════════════════${NC}"
if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}✓ ALL LOCAL VERIFICATIONS SUCCESSFUL! ($PASS passed)${NC}"
else
    echo -e "${RED}✗ VERIFICATIONS FAILED! ($FAIL failed, $PASS passed)${NC}"
    exit 1
fi
echo -e "${YELLOW}══════════════════════════════════════${NC}"
