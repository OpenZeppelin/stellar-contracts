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
ADDR_FILE="$ROOT_DIR/examples/rwa-deploy/testnet-addresses.json"

SOURCE="${STELLAR_SOURCE:-alice}"
NETWORK="${STELLAR_NETWORK:-testnet}"

. "$SCRIPT_DIR/common.sh"

SKIP_BUILD=false
if [ "${1:-}" = "--skip-build" ]; then
  SKIP_BUILD=true
fi

PASS=0
FAIL=0

phase_header() {
  echo "╔══════════════════════════════════════════╗"
  printf "║ %-40s ║\n" "$1"
  echo "╚══════════════════════════════════════════╝"
  echo ""
}

test_header() {
  echo "--- $1 ---"
}

extract_first_number() {
  grep -oE '"[0-9]+"' | head -1 | tr -d '"' || \
  grep -oE '[0-9]+' | head -1 || echo "0"
}

load_addresses() {
  ADMIN=$(read_addr "['admin']")
  TOKEN=$(read_addr "['contracts']['token']")
  IRS=$(read_addr "['contracts']['irs']")
  COUNTRY_ALLOW=$(read_addr "['modules']['country_allow']")
  COUNTRY_RESTRICT=$(read_addr "['modules']['country_restrict']")
  INITIAL_LOCKUP=$(read_addr "['modules']['initial_lockup_period']")
  MAX_BALANCE=$(read_addr "['modules']['max_balance']")
  SUPPLY_LIMIT=$(read_addr "['modules']['supply_limit']")
  TIME_TRANSFERS=$(read_addr "['modules']['time_transfers_limits']")
  TRANSFER_RESTRICT=$(read_addr "['modules']['transfer_restrict']")

  require_contract_id "token" "$TOKEN"
  require_contract_id "irs" "$IRS"
  require_contract_id "country_allow" "$COUNTRY_ALLOW"
  require_contract_id "country_restrict" "$COUNTRY_RESTRICT"
  require_contract_id "initial_lockup_period" "$INITIAL_LOCKUP"
  require_contract_id "max_balance" "$MAX_BALANCE"
  require_contract_id "supply_limit" "$SUPPLY_LIMIT"
  require_contract_id "time_transfers_limits" "$TIME_TRANSFERS"
  require_contract_id "transfer_restrict" "$TRANSFER_RESTRICT"
}

register_test_identity() {
  local alias_name=$1
  local country_code=$2
  local description=$3
  local address

  if [ "$alias_name" = "$SOURCE" ]; then
    address="$ADMIN"
  else
    stellar keys generate "$alias_name" 2>/dev/null || true
    address=$(stellar keys address "$alias_name")
  fi

  echo "Registering $description ($address)..."
  ensure_identity_registered \
    "$IRS" \
    "$address" \
    "$address" \
    "[{\"country\":{\"Individual\":{\"Citizenship\":$country_code}},\"metadata\":null}]"
  echo "  Identity registered."
  REGISTERED_IDENTITY_ADDRESS=$address
}

# ═══════════════════════════════════════════════════════
# Phase 1: Build
# ═══════════════════════════════════════════════════════
if [ "$SKIP_BUILD" = false ]; then
  phase_header "Phase 1/5: Building all WASMs"
  bash "$SCRIPT_DIR/build.sh"
  echo ""
else
  echo "Phase 1/5: Build SKIPPED (--skip-build)"
  echo ""
fi

# ═══════════════════════════════════════════════════════
# Phase 2: Deploy (includes ALL module configuration)
# ═══════════════════════════════════════════════════════
phase_header "Phase 2/5: Deploying full stack"
bash "$SCRIPT_DIR/deploy.sh"
echo ""

# ═══════════════════════════════════════════════════════
# Phase 3: Wire modules to hooks
# ═══════════════════════════════════════════════════════
phase_header "Phase 3/5: Wiring modules to hooks"
bash "$SCRIPT_DIR/wire.sh"
echo ""

# ═══════════════════════════════════════════════════════
# Phase 4: Register investor identity
# ═══════════════════════════════════════════════════════
phase_header "Phase 4/5: Registering investor identity"

load_addresses

register_test_identity "$SOURCE" 840 "investor"
INVESTOR=$REGISTERED_IDENTITY_ADDRESS

# Generate test-only keypairs for country enforcement tests (no funding needed).
register_test_identity "e2e-investor-2" 392 "investor-2"
INVESTOR2=$REGISTERED_IDENTITY_ADDRESS
register_test_identity "e2e-investor-3" 408 "investor-3"
INVESTOR3=$REGISTERED_IDENTITY_ADDRESS
echo ""

# ═══════════════════════════════════════════════════════
# Phase 5: Comprehensive compliance tests
# ═══════════════════════════════════════════════════════
phase_header "Phase 5/5: Compliance module tests"

assert_pass() {
  local DESC=$1; shift
  local OUTPUT
  echo "  [$DESC]"
  if OUTPUT=$("$@" 2>&1); then
    echo "    PASS"
    PASS=$((PASS + 1))
  elif retryable_invoke_error "$OUTPUT"; then
    echo "    FAIL (transient Stellar CLI error)"
    FAIL=$((FAIL + 1))
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
  elif retryable_invoke_error "$OUTPUT"; then
    echo "    FAIL (transient Stellar CLI error)"
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
  OUT=$(invoke_readonly "$TOKEN" balance --account "$1" 2>&1)
  echo "$OUT" | extract_first_number
}

get_internal_supply() {
  local OUT
  OUT=$(invoke_readonly "$SUPPLY_LIMIT" get_internal_supply --token "$TOKEN" 2>&1)
  echo "$OUT" | extract_first_number
}

run_auth_handoff_suite() {
  # deploy.sh already proves the pre-bind bootstrap-admin path because all
  # module configuration succeeds before calling `set_compliance_address`.
  # These checks prove the post-bind handoff with real testnet transactions by
  # asserting that the same externally owned admin can no longer call
  # privileged module config.
  local description contract method
  local arg1 arg2 arg3 arg4 arg5 arg6
  local args

  test_header "Auth handoff: admin config blocked after compliance bind"
  while IFS='|' read -r description contract method arg1 arg2 arg3 arg4 arg5 arg6; do
    [ -n "$description" ] || continue

    args=()
    for arg in "$arg1" "$arg2" "$arg3" "$arg4" "$arg5" "$arg6"; do
      if [ -n "$arg" ]; then
        args+=("$arg")
      fi
    done

    assert_fail "$description" invoke "$contract" "$method" "${args[@]}"
  done <<EOF
country-allow admin cannot add country after bind|$COUNTRY_ALLOW|add_allowed_country|--token|$TOKEN|--country|250||
country-restrict admin cannot add restriction after bind|$COUNTRY_RESTRICT|add_country_restriction|--token|$TOKEN|--country|760||
max-balance admin cannot change max after bind|$MAX_BALANCE|set_max_balance|--token|$TOKEN|--max|1000001||
supply-limit admin cannot change limit after bind|$SUPPLY_LIMIT|set_supply_limit|--token|$TOKEN|--limit|10000001||
time-transfers admin cannot change limit after bind|$TIME_TRANSFERS|set_time_transfer_limit|--token|$TOKEN|--limit|{"limit_time":43200,"limit_value":"50000"}||
initial-lockup admin cannot change lockup after bind|$INITIAL_LOCKUP|set_lockup_period|--token|$TOKEN|--lockup_seconds|301||
transfer-restrict admin cannot allow new user after bind|$TRANSFER_RESTRICT|allow_user|--token|$TOKEN|--user|$INVESTOR2||
EOF
  echo ""
}

run_supply_limit_tests() {
  test_header "Test 1: Mint 1000 tokens"
  assert_pass "mint 1000" invoke_with_retry "$TOKEN" mint --to "$INVESTOR" --amount 1000 --operator "$ADMIN"

  BAL=$(get_balance "$INVESTOR")
  assert_eq "balance = 1000" "1000" "$BAL"

  SUPPLY=$(get_internal_supply)
  assert_eq "internal supply = 1000" "1000" "$SUPPLY"
  echo ""

  # Supply limit was configured to 10,000,000 in deploy.sh.
  # Minting 10M more would bring total to 10,001,000 — should fail.
  test_header "Test 2: Supply limit rejects over-mint"
  assert_fail "mint 10000000 (exceeds supply limit)" invoke "$TOKEN" mint --to "$INVESTOR" --amount 10000000 --operator "$ADMIN"

  BAL=$(get_balance "$INVESTOR")
  assert_eq "balance unchanged = 1000" "1000" "$BAL"
  echo ""
}

run_lockup_tests() {
  # Lockup was set to 300s in deploy.sh. Tokens were just minted, so
  # all 1000 tokens are locked. A transfer should be rejected by
  # InitialLockupPeriodModule.can_transfer (free balance = 0).
  # NOTE: must use `transfer` (not `forced_transfer` which bypasses can_transfer).
  # Self-transfer: can_transfer still fires with both from/to; lockup check executes.
  # A distinct recipient would require pre-allowing in TransferRestrict during deploy.
  test_header "Test 3: Initial lockup blocks transfer"
  assert_fail "transfer 100 during lockup" invoke "$TOKEN" transfer \
    --from "$INVESTOR" --to "$INVESTOR" --amount 100
  echo ""

  # InitialLockupPeriodModule.on_destroyed asserts free_balance >= amount.
  # All 1000 tokens are locked so burn should be rejected.
  test_header "Test 4: Lockup blocks burn of locked tokens"
  assert_fail "burn 500 during lockup" invoke "$TOKEN" burn \
    --user_address "$INVESTOR" --amount 500 --operator "$ADMIN"

  BAL=$(get_balance "$INVESTOR")
  assert_eq "balance unchanged = 1000" "1000" "$BAL"
  echo ""
}

run_balance_tests() {
  # Supply is 1000 from Test 1. Mint 1000 more -> internal supply = 2000.
  test_header "Test 5: Mint more (supply counter tracks)"
  assert_pass "mint 1000 more" invoke_with_retry "$TOKEN" mint --to "$INVESTOR" --amount 1000 --operator "$ADMIN"

  BAL=$(get_balance "$INVESTOR")
  assert_eq "balance = 2000" "2000" "$BAL"

  SUPPLY=$(get_internal_supply)
  assert_eq "internal supply = 2000" "2000" "$SUPPLY"
  echo ""

  # MaxBalance is 1,000,000 per identity. Investor has 2000 already.
  # Mint 998,000 more to hit the identity cap exactly.
  test_header "Test 6: Mint to max-balance ceiling"
  assert_pass "mint 998000 (fill to 1M)" invoke_with_retry "$TOKEN" mint --to "$INVESTOR" --amount 998000 --operator "$ADMIN"

  BAL=$(get_balance "$INVESTOR")
  assert_eq "balance = 1000000" "1000000" "$BAL"

  SUPPLY=$(get_internal_supply)
  assert_eq "internal supply = 1000000" "1000000" "$SUPPLY"

  assert_fail "mint 1 more (over max-balance)" invoke "$TOKEN" mint --to "$INVESTOR" --amount 1 --operator "$ADMIN"
  echo ""
}

run_country_tests() {
  # Japan (392) is NOT in the allowed list (US/GB/DE = 840/826/276).
  # Mint to investor-2 should be rejected by CountryAllowModule.can_create.
  test_header "Test 7: CountryAllow blocks mint to non-allowed country"
  assert_fail "mint to Japan investor (392 not allowed)" invoke "$TOKEN" mint --to "$INVESTOR2" --amount 100 --operator "$ADMIN"
  echo ""

  # DPRK (408) IS on the restricted list.
  # Mint to investor-3 should be rejected by CountryRestrictModule.can_create.
  test_header "Test 8: CountryRestrict blocks mint to restricted country"
  assert_fail "mint to DPRK investor (408 restricted)" invoke "$TOKEN" mint --to "$INVESTOR3" --amount 100 --operator "$ADMIN"
  echo ""
}

run_auth_handoff_suite
run_supply_limit_tests
run_lockup_tests
run_balance_tests
run_country_tests

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
echo "║  - Auth handoff:    admin blocked after bind       ║"
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
