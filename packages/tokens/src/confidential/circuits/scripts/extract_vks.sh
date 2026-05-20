#!/usr/bin/env bash
# Extracts UltraHonk verification keys for each per-operation circuit and
# writes them under `vks/<circuit>.vk`. The committed artifacts in `vks/` are
# the integration contract with the verifier (#701); CI re-runs this script
# and diffs against the committed copy, so any drift either means the circuit
# changed (regenerate) or the toolchain drifted (don't regenerate -- track
# down the source).
#
# Requires the pinned `nargo` and `bb` versions declared in
# `.github/workflows/noir.yml`. Run from anywhere; the script anchors to the
# circuits/ root.
#
# Add a new circuit by appending its package name to CIRCUITS below.
set -euo pipefail

cd "$(dirname "$0")/.."

CIRCUITS=(
    "register"
)

OUT_DIR="vks"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

mkdir -p "$OUT_DIR"

for name in "${CIRCUITS[@]}"; do
    pkg="circuit_${name}"
    bytecode="target/${pkg}.json"

    echo "==> Compiling ${pkg}"
    nargo compile --package "$pkg"

    echo "==> Extracting VK for ${pkg}"
    # `bb write_vk` writes a file literally named `vk` into the directory
    # passed via `-o`. Stage in a per-circuit temp subdir, then move to the
    # canonical `vks/<name>.vk` path.
    stage="${TMP_DIR}/${name}"
    mkdir -p "$stage"
    bb write_vk -s ultra_honk -b "$bytecode" -o "$stage"
    mv "${stage}/vk" "${OUT_DIR}/${name}.vk"
    echo "    wrote ${OUT_DIR}/${name}.vk ($(wc -c < "${OUT_DIR}/${name}.vk") bytes)"
done

echo "Done."
