"""Run a sequence-envelope relay batch on Monkey Man with the pinned wrapper.

Same validation, staging, invocation, 60s timeout and result schema as the
runner agent's capture-runner.py; parameterised by argv:
  seq_capture.py INPUT_PATH INPUT_SHA ORIGINAL_SHA MANIFEST_SHA COUNT BATCH
"""
import base64, gzip, hashlib, json, os, platform, signal, subprocess, sys, time
from pathlib import Path

ROOT = Path('/Users/marcelo/rustuml-reference')
JAR = 'aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'
JAVA_REV = '71806a23780b04a5ccde2f8ceb5121edad5eb711'
INVOCATION = [str(ROOT / 'bin/reference'), '--cwd', 'PRIVATE_CWD', '--timezone', 'GMT+11:00', '--', '-pipe', '-tsvg']
ALLOWED_FORMATS = ('-tsvg', '-tpng', '-teps', '-tpdf', '-ttxt')
sha = lambda b: hashlib.sha256(b).hexdigest()


def safe_name(name):
    return (isinstance(name, str) and name and name == Path(name).name and name not in ('.', '..')
            and '/' not in name and '\\' not in name and '\x00' not in name)


def main():
    input_path, expected, original, manifest_sha, count, batch = sys.argv[1:7]
    count = int(count)
    inp = Path(input_path)
    z = inp.read_bytes()
    assert sha(z) == expected, 'input hash mismatch'
    raw = gzip.decompress(z)
    payload = json.loads(raw)
    assert payload['original_input_bundle_sha256'] == original
    manifest = base64.b64decode(payload['manifest_base64'], validate=True)
    assert sha(manifest) == payload['manifest_sha256'] == manifest_sha
    mdata = json.loads(manifest)
    assert mdata == payload['manifest']
    cases, files = mdata['cases'], payload['files']
    assert len(cases) == len(files) == count
    resources = []
    for res in payload.get('resources', []):
        # At most one directory level (e.g. theme dirs for `!theme X from dir`); each component safe.
        parts = res['path'].split('/')
        assert 1 <= len(parts) <= 2 and all(safe_name(q) for q in parts), res['path']
        data = base64.b64decode(res['base64'], validate=True)
        assert sha(data) == res['sha256']
        resources.append((res, data))
    by_name, seen = {}, set()
    for f in files:
        assert safe_name(f['path']) and Path(f['path']).suffix == '.puml'
        assert f['path'].casefold() not in seen
        seen.add(f['path'].casefold())
        data = base64.b64decode(f['base64'], validate=True)
        assert sha(data) == f['sha256']
        by_name[f['path']] = (f, data)
    assert not ({r[0]['path'] for r in resources} & set(by_name))
    validated = []
    for case in cases:
        assert case['cwd'] == 'PRIVATE_CWD' and case['timezone'] == 'GMT+11:00'
        assert case.get('env', {}) == case.get('env_overrides', {}) == {}
        # Only the final output-format flag may vary; the wrapper prefix is fixed.
        assert case['invocation'][:-1] == INVOCATION[:-1] and case['invocation'][-1] in ALLOWED_FORMATS, case['invocation']
        name = case['name'] + '.puml'
        f, data = by_name[name]
        assert case['source_sha256'] == f['sha256']
        validated.append((dict(case, remote_name=name), data))
    assert len(validated) == count
    assert sha((ROOT / 'reference.jar').read_bytes()) == JAR

    import importlib.util
    _gate_path = Path('/Users/marcelo/rustuml-reference/reference-env/reference_env_gate.py')
    _spec = importlib.util.spec_from_file_location('reference_env_gate', _gate_path)
    _gate = importlib.util.module_from_spec(_spec)
    _spec.loader.exec_module(_gate)
    _gate.require_reference_env()

    worker = Path(subprocess.check_output([str(ROOT / 'bin/new-worker')], text=True).strip())
    (worker / 'capture-runner.py').write_bytes(Path(__file__).read_bytes())
    (worker / 'reference-wrapper.py').write_bytes((ROOT / 'bin/reference').read_bytes())
    (worker / 'input.json.gz').write_bytes(z)
    (worker / 'manifest.json').write_bytes(manifest)
    newdir = worker / 'new'
    newdir.mkdir(parents=True, exist_ok=True)
    for row, data in validated:
        with (newdir / row['remote_name']).open('xb') as s:
            s.write(data)
    resource_records = []
    for res, data in resources:
        (newdir / res['path']).parent.mkdir(parents=True, exist_ok=True)
        with (newdir / res['path']).open('xb') as s:
            s.write(data)
        resource_records.append({'path': res['path'], 'remote_relative_path': 'new/' + res['path'],
                                 'remote_source': str(newdir / res['path']), 'sha256': sha(data), 'bytes': len(data)})
    home = Path('/opt/homebrew/Cellar/openjdk@21/21.0.5/libexec/openjdk.jdk/Contents/Home')
    env = {
        'batch': batch, 'worker': str(worker), 'hostname': platform.node(), 'platform': platform.platform(),
        'architecture': platform.machine(), 'jar': str(ROOT / 'reference.jar'), 'jar_sha256': JAR,
        'java': str(home / 'bin/java'), 'java_release': (home / 'release').read_text(),
        'java_executable_sha256': sha((home / 'bin/java').read_bytes()), 'wrapper': str(ROOT / 'bin/reference'),
        'wrapper_sha256': sha((ROOT / 'bin/reference').read_bytes()), 'dot': '/opt/homebrew/Cellar/graphviz/12.2.0/bin/dot',
        'headless': True, 'encoding': 'UTF-8', 'locale': 'en_AU', 'input_compressed_sha256': expected,
        'manifest_sha256': manifest_sha, 'original_input_bundle_sha256': original, 'env_overrides': {},
        'environment_policy': 'Inherited environment; wrapper removes JAVA_TOOL_OPTIONS, _JAVA_OPTIONS, JDK_JAVA_OPTIONS, CLASSPATH and sets GRAPHVIZ_DOT',
        'timeout_seconds': 60, 'classification_performed': False, 'java_revision': JAVA_REV,
        'java_argv_provenance': 'Derived from unchanged pinned wrapper execve construction and exact invocation arguments; not independently process-traced',
        'relay_operator': 'parent (Claude) via SSH after Codex runner agent hit its usage limit; same pinned wrapper and schema',
        'resources_staged': resource_records,
    }
    print(f'{batch} VALIDATED {count} sources, {len(resources)} resources; worker={worker}', flush=True)
    results = []
    for i, (row, data) in enumerate(validated, 1):
        cwd = str(newdir)
        argv = [cwd if a == 'PRIVATE_CWD' else a for a in row['invocation']]
        start = time.monotonic()
        p = subprocess.Popen(argv, cwd=cwd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                             start_new_session=True)
        timed_out = False
        try:
            out, err = p.communicate(data, timeout=60)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(p.pid, signal.SIGKILL)
            out, err = p.communicate()
        java_argv = [str(home / 'bin/java'), '-Djava.awt.headless=true', '-Dfile.encoding=UTF-8', '-Duser.language=en',
                     '-Duser.country=AU', '-Duser.timezone=' + row['timezone'], '-jar', str(ROOT / 'reference.jar'),
                     '-pipe', '-tsvg']
        r = {'name': row['name'], 'remote_name': row['remote_name'], 'source': row['source'], 'source_sha256': sha(data),
             'stdout_base64': base64.b64encode(out).decode(), 'stderr_base64': base64.b64encode(err).decode(),
             'exit_code': p.returncode, 'argv': argv, 'cwd': cwd, 'env_overrides': {}, 'timezone': row['timezone'],
             'timed_out': timed_out, 'elapsed_seconds': time.monotonic() - start, 'stdout_sha256': sha(out),
             'stderr_sha256': sha(err), 'java_argv': java_argv, 'requested_source': row['source'],
             'source_bytes': len(data), 'stdin_bytes': len(data), 'stdin_sha256': sha(data),
             'requested_argv': row['invocation'], 'requested_cwd': 'PRIVATE_CWD', 'actual_wrapper_argv': argv,
             'actual_java_argv': java_argv, 'requested_env_overrides': {}, 'remote_source': str(newdir / row['remote_name'])}
        results.append(r)
        if i % 25 == 0 or i == count:
            print(f'{batch} CAPTURED {i}/{count}', flush=True)
    for row, data in validated:
        assert sha((newdir / row['remote_name']).read_bytes()) == row['source_sha256']
    assert sha((ROOT / 'reference.jar').read_bytes()) == JAR
    result_bytes = json.dumps({'environment': env, 'resources': resource_records, 'results': results},
                              separators=(',', ':'), ensure_ascii=True).encode()
    (worker / 'reference-results.json').write_bytes(result_bytes)
    compressed = gzip.compress(result_bytes, compresslevel=9, mtime=0)
    (worker / 'reference-results.json.gz').write_bytes(compressed)
    transport = {'sha256': sha(compressed), 'data': base64.b64encode(compressed).decode()}
    (worker / 'reference-transport.json').write_text(json.dumps(transport, separators=(',', ':')) + '\n')
    tsha = sha((worker / 'reference-transport.json').read_bytes())
    summary = {'batch': batch, 'worker': str(worker), 'cases': len(results),
               'timeouts': sum(r['timed_out'] for r in results), 'exit_codes': {},
               'results_sha256': transport['sha256'], 'transport_sha256': tsha}
    for r in results:
        summary['exit_codes'][str(r['exit_code'])] = summary['exit_codes'].get(str(r['exit_code']), 0) + 1
    (worker / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print('SUMMARY ' + json.dumps(summary), flush=True)


if __name__ == '__main__':
    main()
