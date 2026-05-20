# Verification keys

UltraHonk verification keys for the per-operation circuits, one JSON file
per circuit (`<name>.vk.json`). These are committed artifacts -- the
integration contract with the verifier (#701).

**Format:** each file is a JSON array of hex-encoded `Fr` elements, produced
by `bb write_vk --output_format fields`. Used instead of bb's raw `bytes`
format because the latter includes platform-dependent header bytes that
spuriously break cross-platform reproducibility (macOS vs Linux CI).

Reproducible from the circuit sources with the pinned toolchain:

| Tool   | Version          |
|:-------|:-----------------|
| nargo  | `1.0.0-beta.11`  |
| bb     | `0.87.0`         |

Both versions are pinned in `.github/workflows/noir.yml`. CI re-runs the
extraction and diffs against the files here; any drift fails the build.

## Regenerating

```bash
cd packages/tokens/src/confidential/circuits
./scripts/extract_vks.sh
```

If the diff is intentional (the circuit changed), regenerate and commit in
the same PR. If unintentional (e.g. toolchain bumped without an explicit
decision), do **not** regenerate -- track down the source first.
