#!/usr/bin/env python3
"""Rebuild Helvetica ASCII arrays and face_cmap_tables rows from the TSVs and diff."""
import hashlib
import json
import re
import subprocess
from pathlib import Path

REPO = Path("/Users/marcelo/.t3/worktrees/rustuml/t3-361864a6")
CMAP = REPO / "tools/font-chain-reference/cmap-advances-g2"
ASCII = REPO / "tools/font-chain-reference/ascii-advances-round11"
RELAY = Path.home() / "Library/Caches/rustuml-relay-parent/grok-g2-mm-cmap"
OUT = Path.home() / "Library/Caches/rustuml-grok-g2-checker/provenance"

SLOTS = {
    "ARIAL": {
        "plain": "ArialMT.plain",
        "bold": "Arial-BoldMT.bold",
        "italic": "Arial-ItalicMT.italic",
        "bold_italic": "Arial-BoldItalicMT.bolditalic",
    },
    "VERDANA": {
        "plain": "Verdana.plain",
        "bold": "Verdana-Bold.bold",
        "italic": "Verdana-Italic.italic",
        "bold_italic": "Verdana-BoldItalic.bolditalic",
    },
    "HELVETICA": {
        "plain": "Helvetica.plain",
        "bold": "Helvetica-Bold.bold",
        "italic": "Helvetica-Oblique.italic",
        "bold_italic": "Helvetica-BoldOblique.bolditalic",
    },
}
ASCII_CONST = {
    "plain": "HELVETICA_ASCII",
    "bold": "HELVETICA_BOLD_ASCII",
    "italic": "HELVETICA_ITALIC_ASCII",
    "bold_italic": "HELVETICA_BOLD_ITALIC_ASCII",
}


def parse_tsv(path: Path):
    rows = []
    for line in path.read_text().splitlines():
        if not line or line.startswith("#"):
            continue
        kind, code, _glyphs, width = line.split("\t")
        rows.append((kind, int(code, 16), int(float(width))))
    return rows


def rust_rows(text: str, name: str):
    match = re.search(
        rf"static {name}: &\[\(u32, u16\)\] = &\[(.*?)\];",
        text,
        re.S,
    )
    if not match:
        raise SystemExit(f"missing static {name}")
    return [
        (int(code, 16), int(units))
        for code, units in re.findall(r"\(0x([0-9A-Fa-f]+),\s*(\d+)\)", match.group(1))
    ]


def rust_ascii(text: str, name: str):
    match = re.search(rf"const {name}: \[u16; 95\] = \[(.*?)\];", text, re.S)
    if not match:
        raise SystemExit(f"missing const {name}")
    values = [int(item) for item in re.findall(r"\d+", match.group(1))]
    if len(values) != 95:
        raise SystemExit(f"{name} has {len(values)} values")
    return values


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    cmap_src = (REPO / "crates/rustuml-render/src/face_cmap_tables.rs").read_text()
    text_src = (REPO / "crates/rustuml-render/src/text_render.rs").read_text()
    report = {"cmap": [], "ascii": [], "relay_bad": []}
    sums = (CMAP / "SHA256SUMS").read_text().splitlines()
    for line in sums:
        digest, name = line.split()
        data = (CMAP / name).read_bytes()
        got = hashlib.sha256(data).hexdigest()
        relay = RELAY / "out" / name
        relay_ok = relay.is_file() and relay.read_bytes() == data
        if got != digest or not relay_ok:
            report["relay_bad"].append(
                {"name": name, "sha_ok": got == digest, "relay_bytes_match": relay_ok}
            )
    for family, slots in SLOTS.items():
        for slot, stem in slots.items():
            rows = parse_tsv(CMAP / f"{stem}.cmap.tsv")
            expected = [
                (code, units)
                for kind, code, units in rows
                if kind == "M" and not (0x20 <= code <= 0x7E)
            ]
            actual = rust_rows(cmap_src, f"{family}_CMAP_{slot.upper()}")
            mismatch = [
                {"index": i, "expected": exp, "actual": act}
                for i, (exp, act) in enumerate(zip(expected, actual))
                if exp != act
            ]
            if len(expected) != len(actual):
                mismatch.append({"len_expected": len(expected), "len_actual": len(actual)})
            report["cmap"].append(
                {
                    "table": f"{family}_CMAP_{slot.upper()}",
                    "tsv": f"{stem}.cmap.tsv",
                    "rows": len(expected),
                    "mismatches": len(mismatch),
                    "sample": mismatch[:5],
                }
            )
            ascii_rows = {
                code: units
                for kind, code, units in rows
                if kind == "M" and 0x20 <= code <= 0x7E
            }
            ascii_file = parse_tsv(ASCII / f"{stem}.ascii.tsv")
            ascii_from_file = [
                units for kind, code, units in ascii_file if kind == "A" and 0x20 <= code <= 0x7E
            ]
            cmap_ascii = [ascii_rows[code] for code in range(0x20, 0x7F)]
            if family == "HELVETICA":
                rust = rust_ascii(text_src, ASCII_CONST[slot])
                report["ascii"].append(
                    {
                        "const": ASCII_CONST[slot],
                        "tsv": f"{stem}.ascii.tsv",
                        "matches_ascii_tsv": rust == ascii_from_file,
                        "matches_cmap_ascii_column": rust == cmap_ascii,
                        "len": len(rust),
                    }
                )
            else:
                report["ascii"].append(
                    {
                        "family": family,
                        "slot": slot,
                        "cmap_ascii_matches_ascii_tsv": cmap_ascii == ascii_from_file,
                    }
                )
    literal = re.compile(r"\d\.\d{3,}")
    tip_lits = literal.findall(text_src)
    base = subprocess.check_output(
        [
            "git",
            "-C",
            str(REPO),
            "show",
            "cb550c0d67cc729ce764895005b7e86310c9f313:crates/rustuml-render/src/text_render.rs",
        ]
    ).decode()
    base_lits = literal.findall(base)
    from collections import Counter

    base_c = Counter(base_lits)
    tip_c = Counter(tip_lits)
    added = tip_c - base_c
    removed = base_c - tip_c
    report["literals"] = {
        "base": len(base_lits),
        "tip": len(tip_lits),
        "added": dict(added),
        "removed": dict(removed),
    }
    # Production literals are those outside #[cfg(test)] modules. The file's
    # tests start at `#[cfg(test)]`.
    test_at = text_src.find("\n#[cfg(test)]\n")
    prod = text_src[:test_at]
    test = text_src[test_at:]
    report["literals"]["tip_production"] = len(literal.findall(prod))
    report["literals"]["tip_test"] = len(literal.findall(test))
    report["literals"]["base_production"] = len(literal.findall(base[: base.find("\n#[cfg(test)]\n")]))
    (OUT / "regeneration.json").write_text(json.dumps(report, indent=2) + "\n")
    bad_cmap = [row for row in report["cmap"] if row["mismatches"]]
    bad_ascii = [
        row
        for row in report["ascii"]
        if row.get("matches_ascii_tsv") is False or row.get("cmap_ascii_matches_ascii_tsv") is False
    ]
    print(
        json.dumps(
            {
                "cmap_tables": len(report["cmap"]),
                "cmap_bad": len(bad_cmap),
                "ascii_bad": len(bad_ascii),
                "relay_bad": report["relay_bad"],
                "literals": report["literals"],
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
