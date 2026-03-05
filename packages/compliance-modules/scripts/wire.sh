#!/usr/bin/env bash
# Wire modules to compliance hooks.
# Reads addresses from deploy/testnet-addresses.json (written by deploy.sh).
#
# Registers the 5 working modules on appropriate hooks:
#   - CountryAllow:       CanTransfer, CanCreate
#   - CountryRestrict:    CanTransfer, CanCreate
#   - MaxBalance:         CanTransfer, CanCreate, Transferred, Created, Destroyed
#   - TransferRestrict:   CanTransfer
#   - TimeTransfersLimits: CanTransfer, Transferred
#
# SupplyLimit and InitialLockupPeriod are NOT wired due to the
# Soroban re-entry limitation (see architecture docs).
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

echo ""
echo "=== Wiring Complete (12 hook registrations) ==="
echo "Note: SupplyLimitModule and InitialLockupPeriodModule are NOT wired"
echo "due to Soroban re-entry limitation. See architecture docs."
