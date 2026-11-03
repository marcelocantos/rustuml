#!/usr/bin/env python3
# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0

"""Verify lossless metadata recovery against immutable historical Git blobs.

This checks archival fidelity only. It never accepts a parity mechanism or
replaces harness_guards/parity_perturbations. Run with uv run --no-project python.
"""

import copy
import hashlib
import json
from pathlib import Path
import re
import subprocess


def value_digest(value: object) -> str:
    encoded = json.dumps(
        value, sort_keys=True, ensure_ascii=False, separators=(",", ":")
    ).encode()
    return hashlib.sha256(encoded).hexdigest()


def verify(root: Path) -> tuple[int, int]:
    audit = json.loads(
        (root / "docs/parity-reviews/evidence-recovery-2026-11-03.json").read_text()
    )
    operations = 0
    seen = set()
    for record in audit["normalizations"]:
        relative = record["path"]
        if not re.fullmatch(
            r"docs/parity-reviews/[a-z0-9-]+/(account|review(?:-\d+)?)\.json",
            relative,
        ) or relative in seen:
            raise ValueError(f"invalid or duplicate historical path: {relative}")
        seen.add(relative)
        revision = record["original_revision"]
        blob = record["original_git_blob"]
        if any(not re.fullmatch(r"[0-9a-f]{40}", v) for v in (revision, blob)):
            raise ValueError(f"non-exact historical object: {relative}")
        actual_blob = subprocess.check_output(
            ["git", "rev-parse", f"{revision}:{relative}"], cwd=root, text=True
        ).strip()
        if actual_blob != blob:
            raise ValueError(f"historical path/blob mismatch: {relative}")
        original = subprocess.check_output(["git", "cat-file", "blob", blob], cwd=root)
        if hashlib.sha256(original).hexdigest() != record["original_sha256"]:
            raise ValueError(f"historical byte hash mismatch: {relative}")
        reconstructed = json.loads(original)
        for change in record["changes"]:
            parent = reconstructed
            for key in change["path"][:-1]:
                parent = parent[key]
            key = change["path"][-1]
            if not isinstance(parent, dict):
                raise ValueError(f"expected field update: {relative} {change['path']}")
            if (key in parent) != change["had_value"]:
                raise ValueError(f"field existence mismatch: {relative} {change['path']}")
            old = parent.get(key)
            if value_digest(old) != change["before_sha256"]:
                raise ValueError(f"old value mismatch: {relative} {change['path']}")
            if not change["basis"].strip():
                raise ValueError(f"missing normalization basis: {relative}")
            operation = change["operation"]
            if operation == "copy":
                new = reconstructed
                for component in change["source_path"]:
                    new = new[component]
            elif operation == "json_text":
                new = json.dumps(old, ensure_ascii=False, sort_keys=True)
            elif operation == "json_text_elements":
                new = [
                    item if isinstance(item, str)
                    else json.dumps(item, ensure_ascii=False, sort_keys=True)
                    for item in old
                ]
            elif operation == "command_strings":
                new = [item.get("cmd", item.get("command")) for item in old]
            elif operation == "flatten_command_groups":
                new = []
                for group in old.values():
                    new.extend([group] if isinstance(group, str) else group)
            elif operation == "literal":
                new = change["value"]
            else:
                raise ValueError(f"unknown normalization operation: {operation}")
            if value_digest(new) != change["after_sha256"]:
                raise ValueError(f"new value mismatch: {relative} {change['path']}")
            parent[key] = copy.deepcopy(new)
            operations += 1
        current = (root / relative).read_bytes()
        if hashlib.sha256(current).hexdigest() != record["normalized_sha256"]:
            raise ValueError(f"normalized byte hash mismatch: {relative}")
        if json.loads(current) != reconstructed:
            raise ValueError(f"unrecorded semantic change: {relative}")
    return len(seen), operations


if __name__ == "__main__":
    records, operations = verify(Path(__file__).resolve().parents[1])
    print(f"Verified {records} historical records and {operations} metadata operations.")
    print("Archival fidelity only; no parity acceptance verdict.")
