#!/usr/bin/env bash
# End-to-end: build -> deploy -> wire -> test.
#
# This is the single script that does everything in the correct order:
#   Phase 1: Build all 11 WASMs (7 modules + 4 infra)
#   Phase 2: Deploy infra + all 7 modules (with ALL config before compliance lock)
#   Phase 3: Wire all 7 modules to compliance hooks
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
echo "║ Phase 2/5: Deploying full stack          ║"
echo "╚══════════════════════════════════════════╝"
echo ""
bash "$SCRIPT_DIR/deploy.sh"
echo ""

# ═══════════════════════════════════════════════════════
# Phase 3: Wire modules to hooks
# ═══════════════════════════════════════════════════════
echo "╔══════════════════════════════════════════╗"
echo "║ Phase 3/5: Wiring modules to hooks       ║"
echo "╚══════════════════════════════════════════╝"
echo ""
bash "$SCRIPT_DIR/wire.sh"
echo ""

# ═══════════════════════════════════════════════════════
# Phase 4: Register investor identity
# ═══════════════════════════════════════════════════════
echo "╔══════════════════════════════════════════╗"
echo "║ Phase 4/5: Registering investor identity ║"
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
echo "  Identity registered (US citizen, country 840)."

# Generate test-only keypairs for country enforcement tests (no funding needed)
stellar keys generate e2e-investor-2 2>/dev/null || true
INVESTOR2=$(stellar keys address e2e-investor-2)
stellar keys generate e2e-investor-3 2>/dev/null || true
INVESTOR3=$(stellar keys address e2e-investor-3)

echo "Registering investor-2 ($INVESTOR2) with Japan (392)..."
invoke "$IRS" add_identity \
  --account "$INVESTOR2" \
  --identity "$INVESTOR2" \
  --initial_profiles '[{"country":{"Individual":{"Citizenship":392}},"metadata":null}]' \
  --operator "$ADMIN"
echo "  Identity registered (Japan, 392 — not in CountryAllow list)."

echo "Registering investor-3 ($INVESTOR3) with DPRK (408)..."
invoke "$IRS" add_identity \
  --account "$INVESTOR3" \
  --identity "$INVESTOR3" \
  --initial_profiles '[{"country":{"Individual":{"Citizenship":408}},"metadata":null}]' \
  --operator "$ADMIN"
echo "  Identity registered (DPRK, 408 — in CountryRestrict list)."
echo ""

# ═══════════════════════════════════════════════════════
# Phase 5: Comprehensive compliance tests
# ═══════════════════════════════════════════════════════
echo "╔══════════════════════════════════════════════╗"
echo "║ Phase 5/5: Compliance module tests           ║"
echo "╚══════════════════════════════════════════════╝"
echo ""

SUPPLY_LIMIT=$(read_addr "['modules']['supply_limit']")
INITIAL_LOCKUP=$(read_addr "['modules']['initial_lockup_period']")

assert_pass() {
  local DESC=$1; shift
  echo "  [$DESC]"
  if "$@" >/dev/null 2>&1; then
    echo "    PASS"
    PASS=$((PASS + 1))
  else
    echo "    FAIL (expected success)"
    FAIL=$((FAIL + 1))
  fi
}

assert_fail() {
  local DESC=$1; shift
  echo "  [$DESC]"
  local OUTPUT
  if OUTPUT=$("$@" 2>&1); then
    echo "    FAIL (expected rejection but succeeded)"
    FAIL=$((FAIL + 1))
  else
    echo "    PASS (correctly rejected)"
    PASS=$((PASS + 1))
  fi
}

assert_eq() {
  local DESC=$1 EXPECTED=$2 ACTUAL=$3
  echo "  [$DESC]"
  if [ "$ACTUAL" = "$EXPECTED" ]; then
    echo "    PASS ($ACTUAL)"
    PASS=$((PASS + 1))
  else
    echo "    FAIL (expected $EXPECTED, got $ACTUAL)"
    FAIL=$((FAIL + 1))
  fi
}

get_balance() {
  local OUT
  OUT=$(invoke "$TOKEN" balance --account "$1" 2>&1)
  echo "$OUT" | grep -oE '"[0-9]+"' | head -1 | tr -d '"' || \
  echo "$OUT" | grep -oE '[0-9]+' | head -1 || echo "0"
}

get_internal_supply() {
  local OUT
  OUT=$(invoke "$SUPPLY_LIMIT" get_internal_supply --token "$TOKEN" 2>&1)
  echo "$OUT" | grep -oE '"[0-9]+"' | head -1 | tr -d '"' || \
  echo "$OUT" | grep -oE '[0-9]+' | head -1 || echo "0"
}

# ---------- Test 1: Mint 1000 tokens ----------
echo "--- Test 1: Mint 1000 tokens ---"
assert_pass "mint 1000" invoke "$TOKEN" mint --to "$INVESTOR" --amount 1000 --operator "$ADMIN"

BAL=$(get_balance "$INVESTOR")
assert_eq "balance = 1000" "1000" "$BAL"

SUPPLY=$(get_internal_supply)
assert_eq "internal supply = 1000" "1000" "$SUPPLY"
echo ""

# ---------- Test 2: Supply limit rejects over-mint ----------
# Supply limit was configured to 10,000,000 in deploy.sh.
# Minting 10M more would bring total to 10,001,000 — should fail.
echo "--- Test 2: Supply limit rejects over-mint ---"
assert_fail "mint 10000000 (exceeds supply limit)" invoke "$TOKEN" mint --to "$INVESTOR" --amount 10000000 --operator "$ADMIN"

BAL=$(get_balance "$INVESTOR")
assert_eq "balance unchanged = 1000" "1000" "$BAL"
echo ""

# ---------- Test 3: Initial lockup blocks transfer ----------
# Lockup was set to 300s in deploy.sh. Tokens were just minted, so
# all 1000 tokens are locked. A transfer should be rejected by
# InitialLockupPeriodModule.can_transfer (free balance = 0).
# NOTE: must use `transfer` (not `forced_transfer` which bypasses can_transfer).
echo "--- Test 3: Initial lockup blocks transfer ---"
assert_fail "transfer 100 during lockup" invoke "$TOKEN" transfer \
  --from "$INVESTOR" --to "$INVESTOR" --amount 100
echo ""

# ---------- Test 4: Lockup blocks burn of locked tokens ----------
# InitialLockupPeriodModule.on_destroyed asserts free_balance >= amount.
# All 1000 tokens are locked so burn should be rejected.
echo "--- Test 4: Lockup blocks burn of locked tokens ---"
assert_fail "burn 500 during lockup" invoke "$TOKEN" burn \
  --from "$INVESTOR" --amount 500 --operator "$ADMIN"

BAL=$(get_balance "$INVESTOR")
assert_eq "balance unchanged = 1000" "1000" "$BAL"
echo ""

# ---------- Test 5: Mint more tokens (supply counter tracks) ----------
# Supply is 1000 from Test 1. Mint 1000 more → internal supply = 2000.
echo "--- Test 5: Mint more (supply counter tracks) ---"
assert_pass "mint 1000 more" invoke "$TOKEN" mint --to "$INVESTOR" --amount 1000 --operator "$ADMIN"

BAL=$(get_balance "$INVESTOR")
assert_eq "balance = 2000" "2000" "$BAL"

SUPPLY=$(get_internal_supply)
assert_eq "internal supply = 2000" "2000" "$SUPPLY"
echo ""

# ---------- Test 6: Mint to MaxBalance ceiling ----------
# MaxBalance is 1,000,000 per identity. Investor has 2000 already.
# Mint 998,000 more to hit the identity cap exactly.
echo "--- Test 6: Mint to max-balance ceiling ---"
assert_pass "mint 998000 (fill to 1M)" invoke "$TOKEN" mint --to "$INVESTOR" --amount 998000 --operator "$ADMIN"

BAL=$(get_balance "$INVESTOR")
assert_eq "balance = 1000000" "1000000" "$BAL"

SUPPLY=$(get_internal_supply)
assert_eq "internal supply = 1000000" "1000000" "$SUPPLY"

assert_fail "mint 1 more (over max-balance)" invoke "$TOKEN" mint --to "$INVESTOR" --amount 1 --operator "$ADMIN"
echo ""

# ---------- Test 7: CountryAllow blocks non-allowed country ----------
# Japan (392) is NOT in the allowed list (US/GB/DE = 840/826/276).
# Mint to investor-2 should be rejected by CountryAllowModule.can_create.
echo "--- Test 7: CountryAllow blocks mint to non-allowed country ---"
assert_fail "mint to Japan investor (392 not allowed)" invoke "$TOKEN" mint --to "$INVESTOR2" --amount 100 --operator "$ADMIN"
echo ""

# ---------- Test 8: CountryRestrict blocks restricted country ----------
# DPRK (408) IS on the restricted list.
# Mint to investor-3 should be rejected by CountryRestrictModule.can_create.
echo "--- Test 8: CountryRestrict blocks mint to restricted country ---"
assert_fail "mint to DPRK investor (408 restricted)" invoke "$TOKEN" mint --to "$INVESTOR3" --amount 100 --operator "$ADMIN"
echo ""

# ═══════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════
TOTAL=$((PASS + FAIL))
echo "╔════════════════════════════════════════════════════╗"
echo "║              E2E RESULTS                           ║"
echo "╠════════════════════════════════════════════════════╣"
echo "║ Build:      11 WASMs compiled                      ║"
echo "║ Deploy:     4 infra + 7 modules configured         ║"
echo "║ Wire:       7 modules on 19 hooks (4 verified)     ║"
echo "║ Identity:   3 investors registered                 ║"
echo "╠════════════════════════════════════════════════════╣"
echo "║ Module tests:                                      ║"
echo "║  - Supply limit:    mint, over-mint rejection      ║"
echo "║  - Max balance:     identity cap enforcement       ║"
echo "║  - Initial lockup:  transfer + burn blocked        ║"
echo "║  - Country allow:   non-allowed country rejected   ║"
echo "║  - Country restrict: restricted country rejected   ║"
echo "║  - Internal state:  supply counter across mints    ║"
echo "╠════════════════════════════════════════════════════╣"
printf "║ Tests: %d passed, %d failed (of %d)                 ║\n" "$PASS" "$FAIL" "$TOTAL"
echo "╚════════════════════════════════════════════════════╝"
echo ""
echo "Addresses: $ADDR_FILE"

if [ $FAIL -gt 0 ]; then
  exit 1
fi
