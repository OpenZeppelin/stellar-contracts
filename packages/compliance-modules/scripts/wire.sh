#!/usr/bin/env bash
# Wire all 7 compliance modules to their required hooks.
# Reads addresses from deploy/testnet-addresses.json (written by deploy.sh).
#
# Hook registrations per module:
#   - CountryAllow:           CanTransfer, CanCreate
#   - CountryRestrict:        CanTransfer, CanCreate
#   - MaxBalance:             CanTransfer, CanCreate, Transferred, Created, Destroyed
#   - TransferRestrict:       CanTransfer
#   - TimeTransfersLimits:    CanTransfer, Transferred
#   - SupplyLimit:            CanCreate, Created, Destroyed
#   - InitialLockupPeriod:    CanTransfer, Created, Transferred, Destroyed
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
ADDR_FILE="$ROOT_DIR/packages/compliance-modules/deploy/testnet-addresses.json"

SOURCE="${STELLAR_SOURCE:-alice}"
NETWORK="${STELLAR_NETWORK:-testnet}"

if [ ! -f "$ADDR_FILE" ]; then
  echo "ERROR: testnet-addresses.json not found. Run deploy.sh first." >&2
  exit 1
fi

read_addr() {
  python3 -c "import json; d=json.load(open('$ADDR_FILE')); print(d$1)"
}

ADMIN=$(read_addr "['admin']")
COMPLIANCE=$(read_addr "['contracts']['compliance']")

COUNTRY_ALLOW=$(read_addr "['modules']['country_allow']")
COUNTRY_RESTRICT=$(read_addr "['modules']['country_restrict']")
MAX_BALANCE=$(read_addr "['modules']['max_balance']")
TRANSFER_RESTRICT=$(read_addr "['modules']['transfer_restrict']")
TIME_TRANSFERS=$(read_addr "['modules']['time_transfers_limits']")
SUPPLY_LIMIT=$(read_addr "['modules']['supply_limit']")
INITIAL_LOCKUP=$(read_addr "['modules']['initial_lockup_period']")

register() {
  local HOOK=$1 MODULE_ADDR=$2 NAME=$3
  echo "  $NAME -> $HOOK"
  stellar contract invoke --id "$COMPLIANCE" \
    --source "$SOURCE" --network "$NETWORK" \
    -- add_module_to --hook "\"$HOOK\"" --module "$MODULE_ADDR" --operator "$ADMIN"
}

echo "=== Wiring Modules to Compliance Hooks ==="
echo ""

register "CanTransfer" "$COUNTRY_ALLOW" "CountryAllowModule"
register "CanCreate"   "$COUNTRY_ALLOW" "CountryAllowModule"

register "CanTransfer" "$COUNTRY_RESTRICT" "CountryRestrictModule"
register "CanCreate"   "$COUNTRY_RESTRICT" "CountryRestrictModule"

register "CanTransfer"  "$MAX_BALANCE" "MaxBalanceModule"
register "CanCreate"    "$MAX_BALANCE" "MaxBalanceModule"
register "Transferred"  "$MAX_BALANCE" "MaxBalanceModule"
register "Created"      "$MAX_BALANCE" "MaxBalanceModule"
register "Destroyed"    "$MAX_BALANCE" "MaxBalanceModule"

register "CanTransfer" "$TRANSFER_RESTRICT" "TransferRestrictModule"

register "CanTransfer"  "$TIME_TRANSFERS" "TimeTransfersLimitsModule"
register "Transferred"  "$TIME_TRANSFERS" "TimeTransfersLimitsModule"

register "CanCreate"   "$SUPPLY_LIMIT" "SupplyLimitModule"
register "Created"     "$SUPPLY_LIMIT" "SupplyLimitModule"
register "Destroyed"   "$SUPPLY_LIMIT" "SupplyLimitModule"

register "CanTransfer"  "$INITIAL_LOCKUP" "InitialLockupPeriodModule"
register "Created"      "$INITIAL_LOCKUP" "InitialLockupPeriodModule"
register "Transferred"  "$INITIAL_LOCKUP" "InitialLockupPeriodModule"
register "Destroyed"    "$INITIAL_LOCKUP" "InitialLockupPeriodModule"

echo ""
echo "=== Verifying hook wiring for stateful modules ==="
echo ""

verify() {
  local MODULE_ADDR=$1 NAME=$2
  echo "  Verifying $NAME..."
  stellar contract invoke --id "$MODULE_ADDR" \
    --source "$SOURCE" --network "$NETWORK" \
    -- verify_hook_wiring
}

verify "$SUPPLY_LIMIT" "SupplyLimitModule"
verify "$INITIAL_LOCKUP" "InitialLockupPeriodModule"
verify "$MAX_BALANCE" "MaxBalanceModule"
verify "$TIME_TRANSFERS" "TimeTransfersLimitsModule"

echo ""
echo "=== Wiring Complete (19 hooks registered, 4 modules verified) ==="
