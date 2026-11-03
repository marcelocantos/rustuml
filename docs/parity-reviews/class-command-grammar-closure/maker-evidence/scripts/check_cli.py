# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Read frozen references; write CLI outputs and strict comparisons to scratch.

Usage: uv run --python 3 check_cli.py BINARY CHECK_SVG SOURCES OUTPUT
CHECK_SVG is check_svg.rs linked against this checkout's rustuml_oracle rlib.
Set PLANTUML_STDLIB and RUSTUML_DEBUG as recorded in review.json.
"""
from pathlib import Path
import json
import subprocess
import sys

binary, comparator, sources, output = map(Path, sys.argv[1:])
output.mkdir(parents=True, exist_ok=True)
results = {}
for source in sorted(sources.glob('*.puml')):
    rendered = subprocess.run([str(binary), '-tsvg', str(source)], capture_output=True, timeout=60)
    svg = output / (source.stem + '.rust.svg')
    svg.write_bytes(rendered.stdout)
    ast = subprocess.run([str(binary), '--ast', str(source)], capture_output=True, timeout=60)
    (output / (source.stem + '.ast.txt')).write_bytes(ast.stdout + ast.stderr)
    compared = subprocess.run([str(comparator), str(source.with_suffix('.svg')), str(svg)], capture_output=True, timeout=60)
    (output / (source.stem + '.diff.txt')).write_bytes(compared.stdout + compared.stderr)
    results[source.stem] = {
        'cli_exit': rendered.returncode,
        'cli_stderr': rendered.stderr.decode(),
        'ast_exit': ast.returncode,
        'rust_factory': ast.stdout.decode().split('(', 1)[0].strip(),
        'comparison_exit': compared.returncode,
        'comparison': (compared.stdout + compared.stderr).decode(),
    }
(output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
print(json.dumps({name: result['comparison_exit'] for name, result in results.items()}, indent=2))
if any(result['cli_exit'] or result['ast_exit'] or result['comparison_exit'] for result in results.values()):
    sys.exit(1)
