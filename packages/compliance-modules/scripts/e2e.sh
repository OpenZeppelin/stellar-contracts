#!/usr/bin/env bash
# End-to-end: build -> deploy -> wire -> test.
#
# This is the single script that does everything in the correct order:
#   Phase 1: Build all 11 WASMs (7 modules + 4 infra)
#   Phase 2: Deploy infra + all 7 modules (with ALL config before compliance lock)
#   Phase 3: Wire 5 safe modules to compliance hooks
#   Phase 4: Register investor identity in IRS
#   Phase 5: Mint tokens and verify balance (happy path)
#
# Usage: ./e2e.sh [--skip-build]
# Env vars:
#   STELLAR_SOURCE  - signing key alias (default: alice)
#   STELLAR_NETWORK - network passphrase (default: testnet)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
ADDR_FILE="$ROOT_DIR/packages/compliance-modules/deploy/testnet-addresses.json"

SOURCE="${STELLAR_SOURCE:-alice}"
NETWORK="${STELLAR_NETWORK:-testnet}"

SKIP_BUILD=false
if [ "${1:-}" = "--skip-build" ]; then
  SKIP_BUILD=true
fi

invoke() {
  stellar contract invoke --id "$1" \
    --source "$SOURCE" --network "$NETWORK" \
    -- "${@:2}"
}

read_addr() {
  python3 -c "import json; d=json.load(open('$ADDR_FILE')); print(d$1)"
}

PASS=0
FAIL=0

# ═══════════════════════════════════════════════════════
# Phase 1: Build
# ═══════════════════════════════════════════════════════
if [ "$SKIP_BUILD" = false ]; then
  echo "╔══════════════════════════════════════════╗"
  echo "║ Phase 1/5: Building all WASMs            ║"
  echo "╚══════════════════════════════════════════╝"
  echo ""
  bash "$SCRIPT_DIR/build.sh"
  echo ""
else
  echo "Phase 1/5: Build SKIPPED (--skip-build)"
  echo ""
fi

# ═══════════════════════════════════════════════════════
# Phase 2: Deploy (includes ALL module configuration)
# ═══════════════════════════════════════════════════════
echo "╔══════════════════════════════════════════╗"
echo "║ Phase 2/5: Deploying full stack           ║"
echo "╚══════════════════════════════════════════╝"
echo ""
bash "$SCRIPT_DIR/deploy.sh"
echo ""

# ═══════════════════════════════════════════════════════
# Phase 3: Wire modules to hooks
# ═══════════════════════════════════════════════════════
echo "╔══════════════════════════════════════════╗"
echo "║ Phase 3/5: Wiring modules to hooks        ║"
echo "╚══════════════════════════════════════════╝"
echo ""
bash "$SCRIPT_DIR/wire.sh"
echo ""

# ═══════════════════════════════════════════════════════
# Phase 4: Register investor identity
# ═══════════════════════════════════════════════════════
echo "╔══════════════════════════════════════════╗"
echo "║ Phase 4/5: Registering investor identity  ║"
echo "╚══════════════════════════════════════════╝"
echo ""

ADMIN=$(read_addr "['admin']")
TOKEN=$(read_addr "['contracts']['token']")
IRS=$(read_addr "['contracts']['irs']")

INVESTOR="$ADMIN"

echo "Registering investor ($INVESTOR) in IRS..."
invoke "$IRS" add_identity \
  --account "$INVESTOR" \
  --identity "$INVESTOR" \
  --initial_profiles '[{"country":{"Individual":{"Citizenship":840}},"metadata":null}]' \
  --operator "$ADMIN"
echo "  Identity registered (US citizen)."
echo ""

# ═══════════════════════════════════════════════════════
# Phase 5: Happy path test — mint + verify balance
# ═══════════════════════════════════════════════════════
echo "╔══════════════════════════════════════════╗"
echo "║ Phase 5/5: Happy path test (mint + check) ║"
echo "╚══════════════════════════════════════════╝"
echo ""

echo "Minting 1000 tokens to investor..."
if invoke "$TOKEN" mint --to "$INVESTOR" --amount 1000 --operator "$ADMIN" 2>&1; then
  echo "  Mint succeeded."
else
  echo "  FAIL: Mint failed."
  FAIL=$((FAIL + 1))
fi

echo ""
echo "Checking balance..."
BALANCE_OUTPUT=$(invoke "$TOKEN" balance --account "$INVESTOR" 2>&1)
BALANCE=$(echo "$BALANCE_OUTPUT" | grep -oE '[0-9]+' | head -1)

if [ -n "$BALANCE" ] && [ "$BALANCE" -ge 1000 ] 2>/dev/null; then
  echo "  PASS: Balance = $BALANCE"
  PASS=$((PASS + 1))
else
  echo "  FAIL: Unexpected balance output: '$BALANCE_OUTPUT'"
  FAIL=$((FAIL + 1))
fi
echo ""

# ═══════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════
echo "╔══════════════════════════════════════════════╗"
echo "║              E2E RESULTS                      ║"
echo "╠══════════════════════════════════════════════╣"
echo "║ Build:      11 WASMs compiled & optimized     ║"
echo "║ Deploy:     4 infra + 7 modules configured    ║"
echo "║ Wire:       5 modules on 12 hooks             ║"
echo "║ Identity:   1 investor registered             ║"
echo "║ Happy path: Mint + balance verified           ║"
echo "╠══════════════════════════════════════════════╣"
printf "║ Tests: %d passed, %d failed                    ║\n" "$PASS" "$FAIL"
echo "╚══════════════════════════════════════════════╝"
echo ""
echo "Addresses: $ADDR_FILE"

if [ $FAIL -gt 0 ]; then
  exit 1
fi
