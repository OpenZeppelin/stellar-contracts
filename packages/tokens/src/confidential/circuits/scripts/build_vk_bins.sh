#!/usr/bin/env bash
# Builds the packed binary verification keys consumed on-chain by the
# UltraHonk verifier (`ultrahonk-soroban-verifier`) and writes them under
# `vks/<circuit>.vk.bin`, one file per per-operation circuit, alongside the
# human-readable `vks/<circuit>.vk.json` produced by `extract_vks.sh`.
#
# Why a separate binary artifact:
#   - `vks/*.vk.json` is bb's `fields` output (a JSON array of hex `Fr`
#     elements). It is committed for cross-platform-stable code review and is
#     the format diffed by CI, but it is NOT the byte layout the verifier
#     parses.
#   - `ultrahonk-soroban-verifier::load_vk_from_bytes` expects exactly 1760
#     bytes: a 32-byte header of four big-endian u64s
#     (circuit_size, log_circuit_size, public_inputs_size, pub_inputs_offset)
#     followed by 27 G1 commitments at 64 bytes each (x || y, big-endian).
#
# `bb write_vk` (default `bytes` output) emits this exact layout with ONE
# extra 4-byte field appended to the header: a big-endian u32 holding the
# number of user public inputs (= public_inputs_size - PAIRING_POINTS_SIZE),
# which the verifier recomputes and does not store. The bb file is therefore
# 1764 bytes laid out as:
#
#   [0..32)    four big-endian u64 header words   (kept)
#   [32..36)   big-endian u32 user-PI count       (dropped)
#   [36..1764) 27 * 64-byte G1 commitments        (kept)
#
# This script strips bytes [32..36) to obtain the 1760-byte file. The VK bytes
# themselves come straight from bb -- nothing here recomputes or rederives the
# key material.
#
# Requires the pinned `nargo` and `bb` versions declared in
# `.github/workflows/noir.yml`. Run from anywhere; the script anchors to the
# circuits/ root. Keep the circuit list in sync with `extract_vks.sh`.
set -euo pipefail

cd "$(dirname "$0")/.."

CIRCUITS=(
    "register"
    "withdraw"
    "transfer"
    "set_spender"
    "spender_transfer"
    "revoke_spender"
)

OUT_DIR="vks"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

# bb's binary VK layout (see header): 32-byte header, then a 4-byte u32, then
# the commitments. The verifier omits the u32, so the on-chain file is 4 bytes
# shorter.
BB_VK_LEN=1764
HEADER_LEN=32
DROP_LEN=4
PACKED_VK_LEN=1760

mkdir -p "$OUT_DIR"

for name in "${CIRCUITS[@]}"; do
    pkg="circuit_${name}"
    bytecode="target/${pkg}.json"

    echo "==> Compiling ${pkg}"
    nargo compile --package "$pkg"

    echo "==> Writing VK for ${pkg}"
    stage="${TMP_DIR}/${name}"
    mkdir -p "$stage"
    bb write_vk -s ultra_honk -b "$bytecode" -o "$stage"

    src="${stage}/vk"
    src_len="$(wc -c < "$src")"
    if [ "$src_len" -ne "$BB_VK_LEN" ]; then
        echo "    ERROR: expected bb VK of ${BB_VK_LEN} bytes, got ${src_len}." >&2
        echo "    The bb VK layout may have changed; re-validate the strip offsets." >&2
        exit 1
    fi

    out="${OUT_DIR}/${name}.vk.bin"
    # Keep the four u64 header words, drop the u32 user-PI count, keep the rest.
    head -c "$HEADER_LEN" "$src" > "$out"
    tail -c "+$((HEADER_LEN + DROP_LEN + 1))" "$src" >> "$out"

    out_len="$(wc -c < "$out")"
    if [ "$out_len" -ne "$PACKED_VK_LEN" ]; then
        echo "    ERROR: packed VK is ${out_len} bytes, expected ${PACKED_VK_LEN}." >&2
        exit 1
    fi
    echo "    wrote ${out} (${out_len} bytes)"
done

echo "Done."
