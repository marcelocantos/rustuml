"""Score renders against Java results.
usage: score.py <reference-results.json> <render-outdir> <out.json> label...
Per case and label: exit match, byte-exact, strict MATCH/DIFF n (only when Java exit 0 and rust stdout nonempty)."""
import base64, json, os, subprocess, sys, tempfile
ref, outdir, outjson, labels = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
SCORER = os.path.expanduser("~/Library/Caches/rustuml-opus-g4-recheck/bin/opus-score")
r = json.load(open(ref))
rows = r["results"] if isinstance(r, dict) else r
jdir = os.path.join(outdir, "java"); os.makedirs(jdir, exist_ok=True)
render = json.load(open(os.path.join(outdir, "render.json")))
pairs, keys = [], []
for c in rows:
    n = c["name"]; jp = os.path.join(jdir, n + ".svg")
    open(jp, "wb").write(base64.b64decode(c["stdout_base64"]))
    for l in labels:
        pairs.append(f"{jp}\t{os.path.join(outdir, l, n + '.svg')}"); keys.append((n, l))
out = subprocess.run([SCORER], input="\n".join(pairs) + "\n", capture_output=True, text=True).stdout.splitlines()
verdicts = dict(zip(keys, out))
score = {}
for c in rows:
    n = c["name"]; jb = base64.b64decode(c["stdout_base64"]); e = {"java_exit": c["exit_code"]}
    for l in labels:
        rb = open(os.path.join(outdir, l, n + ".svg"), "rb").read()
        rx = render[n][l]["exit"]
        e[l] = {"exit": rx, "bytes_equal": rb == jb,
                "strict": verdicts[(n, l)] if c["exit_code"] == 0 and rx == 0 else ("EXIT-OK" if c["exit_code"] == rx else "EXIT-MISMATCH")}
    score[n] = e
json.dump(score, open(outjson, "w"), indent=1, sort_keys=True)
for l in labels:
    valid = [n for n in score if score[n]["java_exit"] == 0]
    print(l, "strict MATCH", sum(score[n][l]["strict"] == "MATCH" for n in valid), "/", len(valid),
          "| exit agreement", sum(score[n][l]["exit"] == score[n]["java_exit"] for n in score), "/", len(score))
