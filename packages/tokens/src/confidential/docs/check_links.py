#!/usr/bin/env python3
"""Checks every reference into the confidential-token documentation set.

Scans the module's Markdown, Rust, and Noir sources for `docs/...md[#anchor]`
paths and for relative Markdown links between docs, and fails when a target
file or heading anchor does not exist, when a heading slug repeats within a
file, or when a file exceeds the per-page LaTeX budget GitHub renders.

Run from anywhere:  python3 packages/tokens/src/confidential/docs/check_links.py
"""
import collections
import pathlib
import re
import sys

MODULE = pathlib.Path(__file__).resolve().parents[1]
DOCS = MODULE / "docs"
LATEX_BUDGET = 500
SKIP_DIRS = {"target", "node_modules", ".git"}

HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*$")
FENCE = re.compile(r"^(```|~~~)")
MD_LINK = re.compile(r"(?<!!)\[[^\]\n]*\]\(([^)\s]+)\)")
PATH_REF = re.compile(r"(?<![\w/.-])((?:\.\./)*docs/[\w./-]+?\.md)(#[\w-]+)?(?![\w/])")


def slug(text):
    text = re.sub(r"`([^`]*)`", r"\1", text)
    text = re.sub(r"\$\$(.*?)\$\$", r"\1", text)
    text = text.replace("\\_", "_").lower()
    text = re.sub(r"[^\w\s-]", "", text)
    return re.sub(r"\s", "-", text.strip())


def headings(path):
    """GitHub-style slugs of the headings in a Markdown file, with de-duplication suffixes."""
    seen = collections.Counter()
    out = []
    in_fence = False
    for line in path.read_text().split("\n"):
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        m = HEADING.match(line)
        if not m:
            continue
        s = slug(m.group(2))
        out.append(s if seen[s] == 0 else f"{s}-{seen[s]}")
        seen[s] += 1
    return out


def sources():
    for p in MODULE.rglob("*"):
        if p.is_file() and p.suffix in (".md", ".rs", ".nr") and not (SKIP_DIRS & set(p.parts)):
            yield p


def main():
    doc_files = sorted(DOCS.rglob("*.md"))
    anchors = {p: headings(p) for p in doc_files}
    errors = []

    for p, hs in anchors.items():
        dupes = [h for h, n in collections.Counter(hs).items() if n > 1]
        if dupes:
            errors.append(f"{p.relative_to(MODULE)}: duplicate heading slugs {dupes}")
        tex = p.read_text().count("$$") // 2
        if tex > LATEX_BUDGET:
            errors.append(f"{p.relative_to(MODULE)}: {tex} LaTeX expressions exceeds the budget of {LATEX_BUDGET}")

    def check(src, lineno, target, anchor):
        if not target.exists():
            errors.append(f"{src.relative_to(MODULE)}:{lineno}: missing file {target}")
            return
        if anchor and anchor not in anchors.get(target.resolve(), headings(target)):
            errors.append(f"{src.relative_to(MODULE)}:{lineno}: no heading #{anchor} in {target.relative_to(MODULE)}")

    for src in sources():
        text = src.read_text()
        for lineno, line in enumerate(text.split("\n"), 1):
            if src.suffix == ".md" and DOCS in src.parents:
                for m in MD_LINK.finditer(line):
                    href = m.group(1)
                    if "://" in href or href.startswith("mailto:"):
                        continue
                    file_part, _, anchor = href.partition("#")
                    target = src if not file_part else (src.parent / file_part)
                    if file_part and not file_part.endswith(".md") and not target.is_dir():
                        continue
                    if target.is_dir():
                        if not target.exists():
                            errors.append(f"{src.relative_to(MODULE)}:{lineno}: missing directory {file_part}")
                        continue
                    check(src, lineno, target.resolve(), anchor or None)
            for m in PATH_REF.finditer(line):
                ref = m.group(1)
                target = ((src.parent / ref) if ref.startswith("../") else (MODULE / ref)).resolve()
                anchor = m.group(2)[1:] if m.group(2) else None
                check(src, lineno, target, anchor)

    if errors:
        print("\n".join(errors))
        print(f"\n{len(errors)} problem(s)")
        return 1
    print(f"ok: {len(doc_files)} docs, references resolve, no duplicate anchors, LaTeX within budget")
    return 0


if __name__ == "__main__":
    sys.exit(main())
