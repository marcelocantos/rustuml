# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Reproduce historical GET decoding without modifying any frozen pair."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import time
import urllib.request
import xml.etree.ElementTree as ET

ROOT = Path.cwd()
SCRATCH = Path('/tmp/rustuml-tim-checker-d2205b7f')
OUT = Path(__file__).resolve().parent / 'reader-contract'
OUT.mkdir(exist_ok=True)
JAVA_HOME = '/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home'
JAVA = JAVA_HOME + '/bin/java'
JAR = '/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
env = dict(os.environ, JAVA_HOME=JAVA_HOME, PATH=JAVA_HOME+'/bin:'+os.environ['PATH'])
spec = importlib.util.spec_from_file_location('historical', SCRATCH/'historical_render_all.py')
historical = importlib.util.module_from_spec(spec)
spec.loader.exec_module(historical)

def run(args, data=None):
    p = subprocess.run(args, input=data, capture_output=True, env=env, timeout=90)
    if p.returncode:
        raise RuntimeError((args, p.returncode, p.stderr.decode()))
    return p.stdout

def parsed(data):
    return ET.tostring(ET.fromstring(data))

sources = [ROOT/'test-diagrams/golden'/p for p in [
    'activity/act_skinparam2_shadowing_true.puml',
    'activity/act_skinparam_shadowing.puml',
    'edge-cases/edge_whitespace_multiple_blank_lines.puml']]
# A fresh renamed counterpart to the frozen parser assertion, kept executable.
new = ROOT/'test-diagrams/perturbations/tim-evaluated-event-stream/checker_reader_renamed.puml'
new.parent.mkdir(parents=True, exist_ok=True)
new.write_text('@startuml\n\n\n\n\nclass ReaderCedar\n\n\nclass ReaderLarch\nReaderCedar --> ReaderLarch\n@enduml')
sources.append(new)
assertion = new.with_name('checker_reader_assertion_verbatim.puml')
test = (ROOT/'crates/rustuml-parser/src/parse/mod.rs').read_text().split('fn leading_blank_lines_do_not_shift_locations_but_internal_blanks_do()', 1)[1]
literal = re.search(r'let input = "([^"]+)";', test).group(1)
assertion.write_bytes(literal.encode().decode('unicode_escape').encode())
sources.append(assertion)
with socket.socket() as sock:
    sock.bind(('127.0.0.1', 0))
    port = sock.getsockname()[1]
log = (SCRATCH/'private-picoweb.log').open('wb')
server = subprocess.Popen([JAVA, '-Djava.awt.headless=true', '-jar', JAR, f'-picoweb:{port}:127.0.0.1'], stdout=log, stderr=subprocess.STDOUT, env=env)
results = []
try:
    deadline = time.monotonic() + 30
    while True:
        try:
            with socket.create_connection(('127.0.0.1', port), timeout=0.2):
                break
        except OSError:
            if server.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError('private picoweb did not start')
            time.sleep(0.1)
    for source in sources:
        d = OUT/source.stem
        d.mkdir(exist_ok=True)
        raw = source.read_bytes()
        (d/'input.puml').write_bytes(raw)
        encoded = historical.plantuml_encode(raw.decode())
        (d/'encoded.txt').write_text(encoded)
        decoded = run([JAVA, '-cp', str(SCRATCH)+':'+JAR, 'ReaderProbe', 'decode', str(d/'encoded.txt')])
        (d/'decoded.puml').write_bytes(decoded)
        outputs = {}
        for name, data in [('pipe', raw), ('decoded-pipe', decoded)]:
            outputs[name] = run([JAVA, '-Djava.awt.headless=true', '-jar', JAR, '-pipe', '-tsvg'], data)
        with urllib.request.urlopen(f'http://127.0.0.1:{port}/svg/{encoded}', timeout=90) as response:
            outputs['get'] = response.read()
        request = urllib.request.Request(f'http://127.0.0.1:{port}/render', data=json.dumps({'source':raw.decode(), 'options':['-tsvg']}).encode(), headers={'Content-Type':'application/json'})
        with urllib.request.urlopen(request, timeout=90) as response:
            outputs['post'] = response.read()
        # File output is isolated in scratch, never beside a frozen reference.
        file_input = SCRATCH/(source.stem+'.puml')
        file_input.write_bytes(raw)
        run([JAVA, '-Djava.awt.headless=true', '-jar', JAR, '-tsvg', str(file_input)])
        outputs['file'] = file_input.with_suffix('.svg').read_bytes()
        for name, data in outputs.items():
            (d/(name+'.svg')).write_bytes(data)
        if source in (new, assertion):
            source.with_suffix('.svg').write_bytes(outputs['pipe'])
        for name in ['input','decoded']:
            (d/(name+'.uml-source')).write_bytes(run([JAVA, '-Djava.awt.headless=true', '-cp', str(SCRATCH)+':'+JAR, 'ReaderProbe', 'source', str(d/(name+'.puml'))]))
        frozen = source.with_suffix('.svg').read_bytes()
        results.append({'source':str(source.relative_to(ROOT)), 'input_sha256':hashlib.sha256(raw).hexdigest(), 'input_repr':repr(raw), 'decoded_repr':repr(decoded), 'outputs':{name:{'sha256':hashlib.sha256(data).hexdigest(), 'xml_equals_frozen':parsed(data)==parsed(frozen), 'xml_equals_pipe':parsed(data)==parsed(outputs['pipe']), 'xml_equals_get':parsed(data)==parsed(outputs['get'])} for name,data in outputs.items()}})
finally:
    server.terminate()
    try:
        server.wait(timeout=10)
    except subprocess.TimeoutExpired:
        server.kill()
        server.wait()
    log.close()
(OUT/'results.json').write_text(json.dumps(results, indent=2)+'\n')
print(json.dumps(results, indent=2))
