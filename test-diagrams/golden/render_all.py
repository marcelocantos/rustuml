#!/usr/bin/env python3
"""Batch render .puml files via PlantUML picoweb server.

Uses the PlantUML text encoding (deflate + custom base64) to construct
GET URLs, matching the picoweb protocol.
"""

import os
import sys
import zlib
import urllib.request
import urllib.error
import concurrent.futures
from pathlib import Path

PLANTUML_URL = os.environ.get("PLANTUML_URL", "http://localhost:8787")

# PlantUML's custom base64 alphabet
PLANTUML_ALPHABET = (
    "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_"
)


def _encode6bit(b):
    return PLANTUML_ALPHABET[b & 0x3F]


def _encode3bytes(b1, b2, b3):
    c1 = b1 >> 2
    c2 = ((b1 & 0x3) << 4) | (b2 >> 4)
    c3 = ((b2 & 0xF) << 2) | (b3 >> 6)
    c4 = b3 & 0x3F
    return _encode6bit(c1) + _encode6bit(c2) + _encode6bit(c3) + _encode6bit(c4)


def plantuml_encode(text):
    """Encode text using PlantUML's deflate + custom base64."""
    data = zlib.compress(text.encode("utf-8"))[2:-4]  # raw deflate (strip zlib header/checksum)
    result = ""
    for i in range(0, len(data), 3):
        if i + 2 < len(data):
            result += _encode3bytes(data[i], data[i + 1], data[i + 2])
        elif i + 1 < len(data):
            result += _encode3bytes(data[i], data[i + 1], 0)
        else:
            result += _encode3bytes(data[i], 0, 0)
    return result


def render_one(puml_path):
    """Render a single .puml file, return (path, status, detail)."""
    svg_path = puml_path.with_suffix(".svg")

    # Skip if SVG exists and is newer
    if svg_path.exists() and svg_path.stat().st_mtime > puml_path.stat().st_mtime:
        return (str(puml_path), "SKIP", "")

    try:
        text = puml_path.read_text(encoding="utf-8")
    except Exception as e:
        return (str(puml_path), "FAIL", f"read error: {e}")

    encoded = plantuml_encode(text)
    url = f"{PLANTUML_URL}/svg/{encoded}"

    try:
        req = urllib.request.Request(url)
        with urllib.request.urlopen(req, timeout=60) as resp:
            svg_data = resp.read()
    except urllib.error.HTTPError as e:
        # Try following redirect manually
        if e.code in (301, 302) and e.headers.get("Location"):
            loc = e.headers["Location"]
            if not loc.startswith("http"):
                loc = f"{PLANTUML_URL}{loc}"
            try:
                with urllib.request.urlopen(loc, timeout=60) as resp:
                    svg_data = resp.read()
            except Exception as e2:
                return (str(puml_path), "FAIL", f"redirect failed: {e2}")
        else:
            return (str(puml_path), "FAIL", f"HTTP {e.code}")
    except Exception as e:
        return (str(puml_path), "FAIL", f"request error: {e}")

    if not svg_data:
        return (str(puml_path), "FAIL", "empty response")

    svg_text = svg_data.decode("utf-8", errors="replace")
    if "Syntax Error" in svg_text:
        # Still save it — it's a valid test case (error rendering)
        svg_path.write_bytes(svg_data)
        return (str(puml_path), "ERR", "syntax error in output")

    svg_path.write_bytes(svg_data)
    return (str(puml_path), "OK", "")


def main():
    golden_dir = Path(__file__).parent
    puml_files = sorted(golden_dir.rglob("*.puml"))

    print(f"Rendering {len(puml_files)} .puml files")
    print(f"Server: {PLANTUML_URL}")
    print(f"Workers: 20")
    print()

    ok = err = fail = skip = 0

    with concurrent.futures.ThreadPoolExecutor(max_workers=20) as pool:
        futures = {pool.submit(render_one, f): f for f in puml_files}
        for i, future in enumerate(concurrent.futures.as_completed(futures), 1):
            path, status, detail = future.result()
            if status == "OK":
                ok += 1
            elif status == "ERR":
                err += 1
                print(f"ERR  {path}: {detail}")
            elif status == "FAIL":
                fail += 1
                print(f"FAIL {path}: {detail}")
            elif status == "SKIP":
                skip += 1

            if i % 500 == 0:
                print(f"... {i}/{len(puml_files)} processed (OK:{ok} ERR:{err} FAIL:{fail} SKIP:{skip})")

    print()
    print("=== Results ===")
    print(f"Total:  {len(puml_files)}")
    print(f"OK:     {ok}")
    print(f"Errors: {err}")
    print(f"Failed: {fail}")
    print(f"Skipped:{skip}")


if __name__ == "__main__":
    main()
