"""Sub-classify 'content change without SVG change' YAML rows: which top-level model keys differ.

Usage: python3 -I explain_model.py RUNS BEFORE_LABEL AFTER_LABEL MODEL_JSON OUT"""
import collections, hashlib, json, sys
from pathlib import Path
import yaml  # noqa: available in the global venv; read-only use

class Loader(yaml.SafeLoader):
    pass

def untagged(loader, suffix, node):
    if isinstance(node, yaml.MappingNode):
        return {"!tag": suffix, **loader.construct_mapping(node, deep=True)}
    if isinstance(node, yaml.SequenceNode):
        return loader.construct_sequence(node, deep=True)
    return {"!tag": suffix, "value": loader.construct_scalar(node)}

Loader.add_multi_constructor("!", untagged)
runs, bl, al, model, out = sys.argv[1:6]
key = lambda s: hashlib.sha256(s.encode()).hexdigest()[:24]
def walk(a, b, path, diffs):
    if isinstance(a, dict) and isinstance(b, dict):
        for k in set(a) | set(b):
            walk(a.get(k), b.get(k), path + [str(k)], diffs)
    elif isinstance(a, list) and isinstance(b, list) and len(a) == len(b):
        for i, (x, y) in enumerate(zip(a, b)):
            walk(x, y, path + ["[]"], diffs)
    elif a != b:
        diffs.add("/".join(path))
cats = collections.Counter(); rows = []
for row in json.loads(Path(model).read_text())["rows"]:
    if row["mode"] != "yaml" or row["reason"] != "content change without SVG change":
        continue
    k = key(row["source"])
    a = yaml.load(open(f"{runs}/{bl}/{k}.yaml"), Loader); b = yaml.load(open(f"{runs}/{al}/{k}.yaml"), Loader)
    diffs = set(); walk(a, b, [], diffs)
    label = " + ".join(sorted(diffs)) or "equal after parse (formatting)"
    cats[label] += 1; rows.append(dict(source=row["source"], differing_paths=sorted(diffs)))
for c, n in cats.most_common():
    print(n, c)
Path(out).write_text(json.dumps(dict(categories=dict(cats), rows=rows), indent=1))
