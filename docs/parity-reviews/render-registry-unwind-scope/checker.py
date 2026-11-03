#!/usr/bin/env python3
# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Compile independent support without modifying workspace manifests or sources.

uv run --no-project checker.py compile BUILD_JSONL OUTPUT
uv run --no-project checker.py generate REPO REFERENCE_JSON
All runtime reports and binaries belong in an external scratch directory.
"""
import json
from pathlib import Path
import subprocess
import sys


def main():
    if sys.argv[1] == "compile":
        artifacts = {}
        for line in Path(sys.argv[2]).read_text().splitlines():
            record = json.loads(line)
            if record.get("reason") == "compiler-artifact":
                for filename in record["filenames"]:
                    if filename.endswith(".rlib"):
                        artifacts[record["target"]["name"]] = filename
        output = Path(sys.argv[3]).resolve()
        deps = Path(artifacts["rustuml_render"]).parent / "deps"
        if not deps.is_dir():
            deps = Path(artifacts["rustuml_render"]).parent
        command = ["rustc", "--edition=2024", "-C", "opt-level=2",
                   str(Path(__file__).with_suffix(".rs")), "-o", str(output),
                   "-L", f"dependency={deps}"]
        for name in ["rustuml_render", "rustuml_parser", "rustuml_oracle", "serde_json"]:
            command.extend(["--extern", f"{name}={artifacts[name]}"])
        subprocess.run(command, check=True)
    elif sys.argv[1] == "generate":
        root = Path(sys.argv[2]).resolve()
        reference = json.loads(Path(sys.argv[3]).read_text())
        directory = root / "test-diagrams/perturbations/render-registry-unwind-scope"
        directory.mkdir(parents=True, exist_ok=True)
        cases = {
            "checker_sequence_plain": "@startuml\nOrchid -> Copper : unpack parcel\nCopper --> Orchid : receipt 27\n@enduml\n",
            "checker_sequence_background": "@startuml\nGrove -> Flint : <back:coral>fresh cargo</back>\nFlint --> Grove : <back:navy>signed</back>\n@enduml\n",
            "checker_sequence_reused_color": "@startuml\nparticipant Elm\nparticipant Cobalt\nparticipant Slate\nElm -> Cobalt : <back:#90EE90>verify 63</back>\nCobalt -> Slate : <back:lightgreen>handoff</back>\nSlate --> Elm : <back:gold>complete</back>\n@enduml\n",
            "checker_activity_plain": "@startuml\nstart\n:Archive plum;\n:Release crate;\nstop\n@enduml\n",
            "checker_activity_background": "@startuml\nstart\n:<back:cyan>Sort cobalt</back>;\n:<back:yellow>Seal fern</back>;\nstop\n@enduml\n",
            "checker_sequence_reverse": "@startuml\nparticipant Tern\nparticipant Willow\nWillow -> Tern : <back:plum>reserve ticket</back>\nTern --> Willow : <back:#FFE4B5>confirm</back>\n@enduml\n",
        }
        for name, source in cases.items():
            path = directory / f"{name}.puml"
            golden = path.with_suffix(".svg")
            # Never replace evidence on a rerun, including failed references.
            if path.exists() or golden.exists():
                raise FileExistsError(path)
            path.write_text(source)
            result = subprocess.run([reference["java"]["executable"],
                "-Djava.awt.headless=true", "-jar", reference["jar"]["path"],
                "-tsvg", "-pipe"], input=source.encode(), capture_output=True, check=False)
            golden.write_bytes(result.stdout)
            if result.returncode:
                raise RuntimeError(f"Java rejected {path}: {result.stderr.decode()}")
    else:
        raise ValueError("expected compile or generate")


if __name__ == "__main__":
    main()
