#!/usr/bin/env python3
"""Independent checker scorer for one relay directory.

usage: score.py <relay_dir> <work_dir> <out_json> <tree>=<rustuml_bin> [...]

Runs each Rust CLI with `-pipe -tsvg` on the bundled sources and classifies
against the Java reference result:
  match / mismatch           both exit 0; svgcmp (compare_svg_strict) decides
  both_reject_same_message   both non-zero, same line and message
  both_reject_different_message
  over_rejection             Java exit 0 (real diagram), Rust non-zero
  under_rejection            Java non-zero, Rust exit 0
  java_error_page_exit_0     Java exit 0 but stdout is an error/crash page
  panic                      Rust exit 101 or 'panicked' in stderr
"""
import base64
import gzip
import hashlib
import json
import os
import re
import subprocess
import sys

relay, work, out = sys.argv[1:4]
trees = dict(arg.split("=", 1) for arg in sys.argv[4:])
SVGCMP = os.path.expanduser("~/Library/Caches/rustuml-opus-g5-recheck/bin/svgcmp")
os.makedirs(work, exist_ok=True)

results = json.load(open(os.path.join(relay, "reference-results.json")))["results"]
bundle = json.load(gzip.open(os.path.join(relay, "reference-input-bundle.json.gz")))
sources = {f["path"]: base64.b64decode(f["base64"]) for f in bundle["files"]}
SUFFIX = re.compile(r"\s*\(Assumed diagram type: [^)]*\)\s*$")
RUST_ERR = re.compile(r"line (\d+): (.*)$")


def java_error(stderr):
    lines = stderr.splitlines()
    if len(lines) >= 3 and lines[0].strip() == "ERROR":
        return lines[1].strip(), SUFFIX.sub("", lines[2]).strip()
    return "", stderr.strip()


def rust_error(stderr):
    for line in stderr.splitlines():
        m = RUST_ERR.search(line)
        if m:
            return m.group(1), m.group(2).strip()
    return "", stderr.strip()


def is_error_page(svg):
    return any(k in svg for k in ("An error has occured", "java.lang."))


report = {"relay": relay, "reference_results_sha256": hashlib.sha256(
    open(os.path.join(relay, "reference-results.json"), "rb").read()).hexdigest(),
    "trees": {t: {"binary": b, "sha256": hashlib.sha256(open(b, "rb").read()).hexdigest()} for t, b in trees.items()},
    "cases": []}
for r in results:
    name = r["name"]
    src = sources[r.get("remote_name", name + ".puml")]
    java_out = base64.b64decode(r["stdout_base64"]).decode("utf-8", "replace")
    java_err = base64.b64decode(r["stderr_base64"]).decode("utf-8", "replace")
    jexit = r["exit_code"]
    jpath = os.path.join(work, name + ".java.svg")
    open(jpath, "w").write(java_out)
    case = {"name": name, "java_exit": jexit, "java_stderr": java_err.strip()[:300]}
    if jexit != 0:
        case["java_line"], case["java_message"] = java_error(java_err)
    case["java_error_page"] = jexit == 0 and is_error_page(java_out)
    for tree, binary in trees.items():
        try:
            p = subprocess.run([binary, "-pipe", "-tsvg"], input=src, capture_output=True, timeout=20)
        except subprocess.TimeoutExpired:
            case[tree] = {"exit": 124, "class": "hang", "stderr": "killed after 20 s"}
            continue
        rout = p.stdout.decode("utf-8", "replace")
        rerr = p.stderr.decode("utf-8", "replace")
        rpath = os.path.join(work, f"{name}.{tree}.svg")
        open(rpath, "w").write(rout)
        entry = {"exit": p.returncode, "stdout_sha256": hashlib.sha256(p.stdout).hexdigest(),
                 "stderr": rerr.strip()[:300]}
        if p.returncode == 101 or "panicked" in rerr:
            cls = "panic"
        elif jexit == 0 and case["java_error_page"]:
            cls = "java_error_page_exit_0"
            if p.returncode == 0:
                entry["byte_identical"] = p.stdout == java_out.encode()
        elif jexit == 0 and p.returncode == 0:
            c = subprocess.run([SVGCMP, jpath, rpath], capture_output=True, text=True)
            entry["differences"] = c.stdout.strip()
            entry["byte_identical"] = p.stdout == java_out.encode()
            cls = "match" if c.returncode == 0 else "mismatch"
        elif jexit == 0:
            cls = "over_rejection"
        elif p.returncode == 0:
            cls = "under_rejection"
        else:
            line, msg = rust_error(rerr)
            entry["line"], entry["message"] = line, msg
            same = line == case["java_line"] and msg == case["java_message"]
            cls = "both_reject_same_message" if same else "both_reject_different_message"
        entry["class"] = cls
        case[tree] = entry
    report["cases"].append(case)

counts = {}
for tree in trees:
    c = {}
    for case in report["cases"]:
        c[case[tree]["class"]] = c.get(case[tree]["class"], 0) + 1
    counts[tree] = c
report["counts"] = counts
report["total"] = len(report["cases"])
json.dump(report, open(out, "w"), indent=1)
print(json.dumps({"total": report["total"], "counts": counts}))
for case in report["cases"]:
    cl = {t: case[t]["class"] for t in trees}
    if len(set(cl.values())) > 1:
        print("  differs:", case["name"], cl)
