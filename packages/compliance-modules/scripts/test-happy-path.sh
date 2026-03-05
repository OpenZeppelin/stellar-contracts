#!/usr/bin/env bash
# Test happy path: register investor identity, mint tokens, verify balance.
#
# Prerequisites:
#   - deploy.sh has been run (modules configured + locked)
#   - wire.sh has been run (modules registered on hooks)
#
# This script does NOT try to configure modules (add_allowed_country, etc.)
# because those functions are locked behind compliance auth after deploy.sh
# calls set_compliance_address. All module config happens in deploy.sh.
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

invoke() {
  stellar contract invoke --id "$1" \
    --source "$SOURCE" --network "$NETWORK" \
    -- "${@:2}"
}

ADMIN=$(read_addr "['admin']")
TOKEN=$(read_addr "['contracts']['token']")
IRS=$(read_addr "['contracts']['irs']")

INVESTOR="$ADMIN"

echo "=== Happy Path Test ==="
echo "Token:    $TOKEN"
echo "Investor: $INVESTOR"
echo ""

# Step 1: Register investor identity in IRS (IRS uses its own admin auth, not compliance)
echo "1. Registering investor identity..."
invoke "$IRS" add_identity \
  --account "$INVESTOR" \
  --identity "$INVESTOR" \
  --initial_profiles '[{"country":{"Individual":{"Citizenship":840}},"metadata":null}]' \
  --operator "$ADMIN" 2>&1 || echo "  (already registered)"

# Step 2: Mint tokens (this triggers compliance hooks: CanCreate -> Created)
echo ""
echo "2. Minting 1000 tokens to investor..."
invoke "$TOKEN" mint --to "$INVESTOR" --amount 1000 --operator "$ADMIN"

# Step 3: Check balance
echo ""
echo "3. Checking balance..."
BALANCE_OUTPUT=$(invoke "$TOKEN" balance --account "$INVESTOR" 2>&1)
BALANCE=$(echo "$BALANCE_OUTPUT" | grep -oE '[0-9]+' | head -1)

echo ""
echo "=== Result ==="
echo "Balance: $BALANCE"
if [ -n "$BALANCE" ] && [ "$BALANCE" -ge 1000 ] 2>/dev/null; then
  echo "PASS: Happy path succeeded!"
else
  echo "FAIL: Unexpected balance: '$BALANCE_OUTPUT'"
  exit 1
fi
