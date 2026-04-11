#!/usr/bin/env bash
# Build a single compliance module WASM by short name.
# Usage: ./build-module.sh <module-name>
# Example: ./build-module.sh supply-limit
set -euo pipefail

if [ $# -lt 1 ]; then
  echo "Usage: $0 <module-name>"
  echo ""
  echo "Available modules:"
  echo "  country-allow"
  echo "  country-restrict"
  echo "  initial-lockup-period"
  echo "  max-balance"
  echo "  supply-limit"
  echo "  time-transfers-limits"
  echo "  transfer-restrict"
  exit 1
fi

MODULE="$1"
PKG="rwa-$MODULE"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
WASM_DIR="$ROOT_DIR/examples/rwa-deploy/wasm"

mkdir -p "$WASM_DIR"

echo "=== Building $PKG ==="
cd "$ROOT_DIR"
stellar contract build --package "$PKG" --out-dir "$WASM_DIR"

WASM_NAME="${PKG//-/_}.wasm"
if [ -f "$WASM_DIR/$WASM_NAME" ]; then
  SIZE=$(wc -c < "$WASM_DIR/$WASM_NAME" | tr -d ' ')
  echo "  $WASM_NAME (${SIZE} bytes) -> examples/rwa-deploy/wasm/"
else
  echo "ERROR: $WASM_NAME not found!" >&2
  exit 1
fi
