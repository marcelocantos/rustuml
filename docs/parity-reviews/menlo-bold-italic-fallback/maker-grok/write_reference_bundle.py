#!/usr/bin/env python3
"""Write a reference-input-bundle.json.gz the parent runner accepts.

`manifest` is the parsed object, not a JSON string. `files[].path` is the
basename `<name>.puml`, not an absolute path. File bytes and their sha256
are copied from the existing bundle or from the `.puml` sources. The
manifest file on disk is not rewritten.
"""
import base64, gzip, hashlib, json, sys
from pathlib import Path

def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def main() -> None:
    directory = Path(sys.argv[1])
    manifest_bytes = (directory / "reference-manifest.json").read_bytes()
    manifest = json.loads(manifest_bytes)
    old_path = directory / "reference-input-bundle.json.gz"
    if old_path.is_file():
        old = json.loads(gzip.decompress(old_path.read_bytes()))
        files = []
        for entry in old["files"]:
            raw = base64.b64decode(entry["base64"])
            digest = sha(raw)
            if digest != entry["sha256"]:
                raise SystemExit(f"hash mismatch for {entry['path']}")
            files.append({
                "path": Path(entry["path"]).name,
                "sha256": digest,
                "base64": entry["base64"],
            })
    else:
        files = []
        for case in manifest["cases"]:
            raw = Path(case["source"]).read_bytes()
            files.append({
                "path": Path(case["source"]).name,
                "sha256": sha(raw),
                "base64": base64.b64encode(raw).decode(),
            })
    bundle = {
        "manifest": manifest,
        "manifest_sha256": sha(manifest_bytes),
        "files": files,
        "resources": [],
    }
    compressed = gzip.compress(json.dumps(bundle).encode(), mtime=0)
    old_path.write_bytes(compressed)
    print(sha(compressed))

if __name__ == "__main__":
    main()
