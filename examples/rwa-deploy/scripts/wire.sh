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
ADDR_FILE="$ROOT_DIR/examples/rwa-deploy/testnet-addresses.json"

SOURCE="${STELLAR_SOURCE:-alice}"
NETWORK="${STELLAR_NETWORK:-testnet}"

if [ ! -f "$ADDR_FILE" ]; then
  echo "ERROR: testnet-addresses.json not found. Run deploy.sh first." >&2
  exit 1
fi

. "$SCRIPT_DIR/common.sh"

ADMIN=$(read_addr "['admin']")
COMPLIANCE=$(read_addr "['contracts']['compliance']")

COUNTRY_ALLOW=$(read_addr "['modules']['country_allow']")
COUNTRY_RESTRICT=$(read_addr "['modules']['country_restrict']")
MAX_BALANCE=$(read_addr "['modules']['max_balance']")
TRANSFER_RESTRICT=$(read_addr "['modules']['transfer_restrict']")
TIME_TRANSFERS=$(read_addr "['modules']['time_transfers_limits']")
SUPPLY_LIMIT=$(read_addr "['modules']['supply_limit']")
INITIAL_LOCKUP=$(read_addr "['modules']['initial_lockup_period']")

require_contract_id "compliance" "$COMPLIANCE"
require_contract_id "country_allow" "$COUNTRY_ALLOW"
require_contract_id "country_restrict" "$COUNTRY_RESTRICT"
require_contract_id "max_balance" "$MAX_BALANCE"
require_contract_id "transfer_restrict" "$TRANSFER_RESTRICT"
require_contract_id "time_transfers_limits" "$TIME_TRANSFERS"
require_contract_id "supply_limit" "$SUPPLY_LIMIT"
require_contract_id "initial_lockup_period" "$INITIAL_LOCKUP"

echo "=== Wiring Modules to Compliance Hooks ==="
echo ""

ensure_hook_registration "CanTransfer" "$COUNTRY_ALLOW" "CountryAllowModule"
ensure_hook_registration "CanCreate"   "$COUNTRY_ALLOW" "CountryAllowModule"

ensure_hook_registration "CanTransfer" "$COUNTRY_RESTRICT" "CountryRestrictModule"
ensure_hook_registration "CanCreate"   "$COUNTRY_RESTRICT" "CountryRestrictModule"

ensure_hook_registration "CanTransfer"  "$MAX_BALANCE" "MaxBalanceModule"
ensure_hook_registration "CanCreate"    "$MAX_BALANCE" "MaxBalanceModule"
ensure_hook_registration "Transferred"  "$MAX_BALANCE" "MaxBalanceModule"
ensure_hook_registration "Created"      "$MAX_BALANCE" "MaxBalanceModule"
ensure_hook_registration "Destroyed"    "$MAX_BALANCE" "MaxBalanceModule"

ensure_hook_registration "CanTransfer" "$TRANSFER_RESTRICT" "TransferRestrictModule"

ensure_hook_registration "CanTransfer"  "$TIME_TRANSFERS" "TimeTransfersLimitsModule"
ensure_hook_registration "Transferred"  "$TIME_TRANSFERS" "TimeTransfersLimitsModule"

ensure_hook_registration "CanCreate"   "$SUPPLY_LIMIT" "SupplyLimitModule"
ensure_hook_registration "Created"     "$SUPPLY_LIMIT" "SupplyLimitModule"
ensure_hook_registration "Destroyed"   "$SUPPLY_LIMIT" "SupplyLimitModule"

ensure_hook_registration "CanTransfer"  "$INITIAL_LOCKUP" "InitialLockupPeriodModule"
ensure_hook_registration "Created"      "$INITIAL_LOCKUP" "InitialLockupPeriodModule"
ensure_hook_registration "Transferred"  "$INITIAL_LOCKUP" "InitialLockupPeriodModule"
ensure_hook_registration "Destroyed"    "$INITIAL_LOCKUP" "InitialLockupPeriodModule"

echo ""
echo "=== Verifying hook wiring for stateful modules ==="
echo ""

verify_hook_wiring_with_retry "$SUPPLY_LIMIT" "SupplyLimitModule"
verify_hook_wiring_with_retry "$INITIAL_LOCKUP" "InitialLockupPeriodModule"
verify_hook_wiring_with_retry "$MAX_BALANCE" "MaxBalanceModule"
verify_hook_wiring_with_retry "$TIME_TRANSFERS" "TimeTransfersLimitsModule"

echo ""
echo "=== Wiring Complete (19 hooks registered, 4 modules verified) ==="
