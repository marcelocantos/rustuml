"""Hash whole CLI outputs for every .puml under the given roots.

Usage: python3 -I output_hashes.py <bin> <out.json> <root> [<root> ...]
Records exit status and SHA256 of stdout per source (cwd = source dir).
"""

import concurrent.futures
import hashlib
import json
import pathlib
import subprocess
import sys

TIMEOUT_SECONDS = 30
WORKERS = 6


def run_one(binary, source):
    try:
        proc = subprocess.run(
            [binary, "-pipe", "-tsvg"],
            input=source.read_bytes(),
            capture_output=True,
            cwd=source.parent,
            timeout=TIMEOUT_SECONDS,
        )
        out, status = proc.stdout, proc.returncode
        panicked = b"panicked" in proc.stderr
    except subprocess.TimeoutExpired:
        out, status, panicked = b"", "timeout", False
    return str(source), {
        "status": status,
        "panic": panicked,
        "sha256": hashlib.sha256(out).hexdigest(),
    }


def main():
    binary, out, *roots = sys.argv[1:]
    sources = sorted(p for root in roots for p in pathlib.Path(root).rglob("*.puml"))
    with concurrent.futures.ThreadPoolExecutor(WORKERS) as pool:
        results = dict(pool.map(lambda s: run_one(binary, s), sources))
    pathlib.Path(out).write_text(json.dumps(results, sort_keys=True))
    print(f"{len(results)} sources")


if __name__ == "__main__":
    main()
