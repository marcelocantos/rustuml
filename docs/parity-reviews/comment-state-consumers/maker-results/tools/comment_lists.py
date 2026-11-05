"""Compare SVG comment lists of a RustUML CLI against Java references.

The strict SVG comparator ignores XML comments, so comment producers need
their own oracle. For every `X.puml` with a sibling `X.svg` under the given
roots, run `<bin> -pipe -tsvg` with the source on stdin (cwd = source dir)
and record the ordered `<!--...-->` lists of both outputs.

Usage: python3 -I comment_lists.py <bin> <out.json> <root> [<root> ...]
"""

import concurrent.futures
import json
import pathlib
import re
import subprocess
import sys

COMMENT = re.compile(r"<!--(.*?)-->", re.S)
TIMEOUT_SECONDS = 30
WORKERS = 6


def comments(text):
    return COMMENT.findall(text)


def run_one(binary, source):
    reference = source.with_suffix(".svg")
    java = reference.read_text(encoding="utf-8", errors="replace")
    try:
        proc = subprocess.run(
            [binary, "-pipe", "-tsvg"],
            input=source.read_bytes(),
            capture_output=True,
            cwd=source.parent,
            timeout=TIMEOUT_SECONDS,
        )
        rust = proc.stdout.decode("utf-8", errors="replace")
        status = proc.returncode
    except subprocess.TimeoutExpired:
        rust, status = "", "timeout"
    return str(source), {
        "status": status,
        "rust": comments(rust),
        "java": comments(java),
    }


def main():
    binary, out, *roots = sys.argv[1:]
    sources = sorted(
        path
        for root in roots
        for path in pathlib.Path(root).rglob("*.puml")
        if path.with_suffix(".svg").exists()
    )
    results = {}
    with concurrent.futures.ThreadPoolExecutor(WORKERS) as pool:
        for name, record in pool.map(lambda s: run_one(binary, s), sources):
            results[name] = record
    pathlib.Path(out).write_text(json.dumps(results, sort_keys=True))
    exact = sum(1 for r in results.values() if r["rust"] == r["java"])
    print(f"{len(results)} pairs, {exact} exact comment lists")


if __name__ == "__main__":
    main()
