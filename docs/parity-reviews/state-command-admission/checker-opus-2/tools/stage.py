import base64, gzip, hashlib, json, os, shutil, sys
src_dir, batch = sys.argv[1:3]
invalid = {"rk_f_later_declared_other_region", "rk_h_ancestor_name_from_region", "rk_h_owner_name_from_region2",
           "rk_r_pseudo_box_in_region", "rk_h_sibling_region_name", "rk_h_root_link_to_region_history"}
def axes(n):
    a = []
    if n.startswith("rk_h_"): a.append("history-by-name")
    if n.startswith("rk_f_"): a.append("field-line")
    if n.startswith("rk_r_"): a.append("pseudo-only-group-render")
    for k in ["region_name", "owner_name", "ancestor", "sibling", "unknown", "alias", "nested", "depth2", "depth3", "nosep", "root", "both_dirs", "target", "later_declared", "control", "mixed", "star", "end", "history"]:
        if k in n: a.append(k)
    return a
os.makedirs(os.path.join(batch, "sources"), exist_ok=True)
cases, files = [], []
for f in sorted(os.listdir(src_dir)):
    if not f.endswith(".puml"): continue
    n = f[:-5]; dst = os.path.join(batch, "sources", f); shutil.copyfile(os.path.join(src_dir, f), dst)
    data = open(dst, "rb").read(); h = hashlib.sha256(data).hexdigest()
    cases.append({"name": n, "source": os.path.abspath(dst), "source_sha256": h, "cwd": "PRIVATE_CWD", "timezone": "GMT+11:00",
                  "invocation": ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD", "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"],
                  "env": {}, "intended_valid": n not in invalid, "axes": axes(n)})
    files.append({"path": f, "sha256": h, "base64": base64.b64encode(data).decode()})
manifest = {"schema_version": 1, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
            "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053", "cases": cases}
mb = json.dumps(manifest, indent=1).encode()
open(os.path.join(batch, "reference-manifest.json"), "wb").write(mb)
bundle = {"manifest": manifest, "manifest_sha256": hashlib.sha256(mb).hexdigest(), "files": files, "resources": []}
with gzip.open(os.path.join(batch, "reference-input-bundle.json.gz"), "wb") as g: g.write(json.dumps(bundle).encode())
print(len(cases), hashlib.sha256(mb).hexdigest())
