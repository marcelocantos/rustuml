#!/usr/bin/env python3
# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Fail a Monkey Man capture unless the reference census matches.

``seq_capture.py`` calls ``require_reference_env()`` after the batch has
been validated and immediately before ``new-worker``. Only exit 0 passes.
Exit 1 (a pinned fact moved) and exit 2 (a hash or scalar is still
unpinned) both abort the capture. This hook does not install packages
or create missing ancestors of its output directory. ``record`` refuses
a symlink output directory and a symlink in its ancestry under scratch.

Install
-------
On Monkey Man, these three files live in
``/Users/marcelo/rustuml-reference/reference-env/``:

- ``reference_env_census.py`` (the checkout's ``scripts/reference_env_census.py``)
- ``reference_env_gate.py`` (this file)
- ``census.txt`` (the committed census)

``/Users/marcelo/rustuml-reference/scratch`` must already exist. The hook
creates a temporary directory there and deletes it before returning.

Call site, inserted in ``seq_capture.py`` immediately before ``new-worker``::

    import importlib.util
    _gate_path = Path('/Users/marcelo/rustuml-reference/reference-env/reference_env_gate.py')
    _spec = importlib.util.spec_from_file_location('reference_env_gate', _gate_path)
    _gate = importlib.util.module_from_spec(_spec)
    _spec.loader.exec_module(_gate)
    _gate.require_reference_env()

    worker = Path(subprocess.check_output([str(ROOT / 'bin/new-worker')], text=True).strip())

When the census legitimately changes, replace ``census.txt`` at that path
with the reviewed record. Replace the two scripts only when their source
changes. The census to install is the adopted ``grok-g7-census4`` record.
The earlier ``grok-g7-census3`` file has no cascade ``font`` rows, so
``check`` exits 1 against it.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

INSTALL = Path("/Users/marcelo/rustuml-reference/reference-env")
CENSUS = INSTALL / "census.txt"
SCRIPT = INSTALL / "reference_env_census.py"
SCRATCH = Path("/Users/marcelo/rustuml-reference/scratch")
PYTHON = "/opt/homebrew/bin/python3"


def require_reference_env() -> None:
    """Record a fresh census and accept the capture only when check exits 0."""
    if not SCRATCH.is_dir():
        raise SystemExit(f"{SCRATCH} does not exist; the gate does not create it")
    if not CENSUS.is_file() or not SCRIPT.is_file():
        raise SystemExit(f"{INSTALL} is missing census.txt or reference_env_census.py")
    with tempfile.TemporaryDirectory(prefix="census-gate-", dir=SCRATCH) as out:
        record = subprocess.run(
            [PYTHON, "-I", str(SCRIPT), "record", out],
            check=False,
        )
        if record.returncode != 0:
            raise SystemExit(f"reference census record exited {record.returncode}")
        checked = subprocess.run(
            [PYTHON, "-I", str(SCRIPT), "check", out, "--committed", str(CENSUS)],
            check=False,
        )
        if checked.returncode != 0:
            raise SystemExit(
                f"reference census check exited {checked.returncode}; only exit 0 passes"
            )


if __name__ == "__main__":
    require_reference_env()
    sys.exit(0)
