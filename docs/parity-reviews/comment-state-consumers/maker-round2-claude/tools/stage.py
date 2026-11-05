"""Stage an immutable private reference-request cache from a probe spec.

Usage: python3 -I stage.py <probes.json> <cache-dir>
Writes <cache-dir>/sources/<name>.puml, reference-manifest.json and
reference-input-bundle.json.gz, then prints both SHA256s. Refuses to
overwrite an existing cache. Positive probes must not contain PUA
(U+E000-U+E0FF) or C0 controls other than LF.
"""
import base64, gzip, hashlib, json, os, pathlib, sys

JAVA_REVISION = "71806a23780b04a5ccde2f8ceb5121edad5eb711"
JAR_SHA256 = "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053"
INVOCATION = ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD",
              "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"]

spec_path, cache = sys.argv[1], pathlib.Path(sys.argv[2])
if cache.exists():
    sys.exit(f"{cache} exists; caches are immutable")
probes = json.load(open(spec_path))
(cache / "sources").mkdir(parents=True)
cases, files = [], []
for probe in probes:
    text = "\n".join(probe["source"]) + "\n"
    if probe["intended_valid"]:
        bad = [c for c in text if 0xE000 <= ord(c) <= 0xE0FF or (ord(c) < 0x20 and c != "\n")]
        if bad:
            sys.exit(f"{probe['name']}: positive probe contains PUA/C0 {bad!r}")
    data = text.encode("utf-8")
    path = cache / "sources" / f"{probe['name']}.puml"
    path.write_bytes(data)
    os.chmod(path, 0o444)
    digest = hashlib.sha256(data).hexdigest()
    cases.append({"name": probe["name"], "source": str(path), "source_sha256": digest,
                  "cwd": "PRIVATE_CWD", "timezone": "GMT+11:00", "invocation": INVOCATION,
                  "env": {}, "intended_valid": probe["intended_valid"], "axes": probe["axes"]})
    files.append({"path": f"{probe['name']}.puml", "sha256": digest,
                  "base64": base64.b64encode(data).decode()})
manifest = {"schema_version": 1, "java_revision": JAVA_REVISION, "jar_sha256": JAR_SHA256, "cases": cases}
manifest_bytes = (json.dumps(manifest, indent=1, sort_keys=True) + "\n").encode()
(cache / "reference-manifest.json").write_bytes(manifest_bytes)
manifest_sha = hashlib.sha256(manifest_bytes).hexdigest()
bundle = {"manifest": manifest, "manifest_sha256": manifest_sha, "files": files, "resources": []}
with gzip.open(cache / "reference-input-bundle.json.gz", "wb") as out:
    out.write(json.dumps(bundle, sort_keys=True).encode())
bundle_sha = hashlib.sha256((cache / "reference-input-bundle.json.gz").read_bytes()).hexdigest()
for name in ("reference-manifest.json", "reference-input-bundle.json.gz"):
    os.chmod(cache / name, 0o444)
print(len(cases), "cases")
print(cache / "reference-manifest.json", manifest_sha)
print(cache / "reference-input-bundle.json.gz", bundle_sha)
