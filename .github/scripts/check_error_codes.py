#!/usr/bin/env python3
"""Fails when two `#[contracterror]` enums under `packages/` share a code.

A failed invocation reports `Error(Contract, #N)` without naming the contract
that raised it, so every library error code must be unique across the
workspace. Enums defined in test files are skipped.
"""

import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PACKAGES = ROOT / "packages"

ATTRIBUTE = re.compile(r"#\[(?:\w+::)*contracterror\]")
ENUM = re.compile(r"(?:pub(?:\([^)]*\))?\s+)?enum\s+(\w+)\s*\{")
VARIANT = re.compile(r"(\w+)\s*=\s*(\d[\d_]*)\s*,?")


def is_test_file(path):
    parts = path.relative_to(ROOT).parts
    return path.name == "test.rs" or "test" in parts or "tests" in parts


def parse_enums(path):
    """Yields `(enum, variant, code, line)` for every contract error variant."""
    lines = path.read_text().splitlines()
    i = 0
    while i < len(lines):
        if not ATTRIBUTE.fullmatch(lines[i].strip()):
            i += 1
            continue
        while not (header := ENUM.fullmatch(lines[i].strip())):
            i += 1
        name = header.group(1)
        i += 1
        while (line := lines[i].strip()) != "}":
            if line and not line.startswith(("//", "#[")):
                variant = VARIANT.fullmatch(line)
                if variant is None:
                    sys.exit(f"{path.relative_to(ROOT)}:{i + 1}: cannot parse `{line}` in `{name}`")
                yield name, variant.group(1), int(variant.group(2)), i + 1
            i += 1


def main():
    codes = defaultdict(list)
    enums = set()
    for path in sorted(PACKAGES.rglob("*.rs")):
        if "target" in path.parts or is_test_file(path):
            continue
        for name, variant, code, line in parse_enums(path):
            enums.add((path, name))
            codes[code].append(f"{name}::{variant} ({path.relative_to(ROOT)}:{line})")

    if not enums:
        sys.exit("no `#[contracterror]` enums found under packages/")

    duplicates = {code: owners for code, owners in sorted(codes.items()) if len(owners) > 1}
    for code, owners in duplicates.items():
        print(f"duplicate error code {code}:")
        for owner in owners:
            print(f"  {owner}")
    if duplicates:
        sys.exit(1)

    print(f"{len(codes)} error codes across {len(enums)} enums, all unique")


if __name__ == "__main__":
    main()
