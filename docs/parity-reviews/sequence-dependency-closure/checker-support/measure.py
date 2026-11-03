# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Execute frozen cohorts through real CLI and unchanged library comparators.

Usage: uv run --no-project python measure.py MANIFEST BIN_DIR OUTPUT_DIR [CWD]
MANIFEST is an array of objects with source, golden, cohort and optional axes.
All paths in it are relative to the repository root. Output must be scratch.
"""
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[4]
ENV = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100",
           PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def run(command, cwd, data=None):
    try:
        proc = subprocess.run(command, cwd=cwd, input=data, capture_output=True,
                              env=ENV, timeout=90)
        return dict(exit=proc.returncode, stdout=proc.stdout.decode(errors="replace"),
                    stderr=proc.stderr.decode(errors="replace"))
    except subprocess.TimeoutExpired as error:
        return dict(exit=124, stdout=(error.stdout or b"").decode(errors="replace"),
                    stderr="TIMEOUT: " + (error.stderr or b"").decode(errors="replace"))


def measure(manifest, binaries, output, cwd):
    output.mkdir(parents=True, exist_ok=False)
    sources = json.loads(manifest.read_text())

    def one(item):
        source, golden = ROOT / item["source"], ROOT / item["golden"]
        body = source.read_bytes()
        row = dict(item, source_sha256=digest(body), golden_sha256=digest(golden.read_bytes()))
        name = digest(item["source"].encode())[:20]
        cli = run([str(binaries / "rustuml"), "-tsvg", "-pipe"], cwd, body)
        actual = output / (name + ".rust.svg")
        actual.write_text(cli.pop("stdout"))
        row["cli"] = dict(cli, svg_sha256=digest(actual.read_bytes()), svg=actual.name)
        row["cli_comparison"] = run([str(binaries / "compare_cli"), str(golden), str(actual)], cwd)
        for mode, flags in (("product_library", ["--no-oracle"]), ("assisted_library", [])):
            row[mode] = run([str(binaries / "diff_one"), *flags, str(source)], cwd)
        library = run([str(binaries / "diff_one"), "--no-oracle", "--print-rust", str(source)], cwd)
        row["cli_equals_library"] = (cli["exit"] == library["exit"] == 0
                                       and actual.read_text().strip() == library["stdout"].strip())
        row["library_render_exit"] = library["exit"]
        row["cwd"] = str(cwd)
        return row

    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        rows = list(executor.map(one, sources))
    (output / "measurements.json").write_text(json.dumps(rows, indent=2) + "\n")
    for cohort in sorted({r["cohort"] for r in rows}):
        subset = [r for r in rows if r["cohort"] == cohort]
        print(cohort, len(subset), flush=True)
        for mode in ("cli_comparison", "product_library", "assisted_library"):
            print(mode, {code: sum(r[mode]["exit"] == code for r in subset)
                         for code in sorted({r[mode]["exit"] for r in subset})}, flush=True)
        print("cli_equals_library", sum(r["cli_equals_library"] for r in subset), flush=True)


if __name__ == "__main__":
    measure(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve(),
            Path(sys.argv[3]).resolve(), Path(sys.argv[4]).resolve() if len(sys.argv) > 4 else ROOT)
