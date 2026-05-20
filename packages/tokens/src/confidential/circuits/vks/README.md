# Verification keys

UltraHonk verification keys for the per-operation circuits, one binary file
per circuit. These are committed artifacts -- the integration contract with
the verifier (#701).

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
