"""Build the comparator list for sweep trees and run checker-cmp. Usage: mkcmp.py <work> <repo> <tag>..."""
import json, pathlib, subprocess, sys
W, REPO, tags = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3:]
CMP = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round4-claude-maker/cmp/target/release/checker-cmp"
for tag in tags:
    st = json.load(open(W / f"sweep-{tag}/status.json"))
    lst = W / f"cmp-{tag}.lst"
    lst.write_text("".join(f"{(REPO / k).with_suffix('.svg')}\t{(W / f'sweep-{tag}' / k).with_suffix('.svg')}\n" for k in sorted(st)))
    out = subprocess.run([str(CMP), str(lst)], capture_output=True, text=True).stdout
    (W / f"cmp-{tag}.tsv").write_text(out)
    print(tag, sum(1 for l in out.splitlines() if "\tMATCH\t" in l), "/", len(st))
# the classify tool reads cmp.tsv for both trees
(W / "cmp.tsv").write_text("".join((W / f"cmp-{t}.tsv").read_text() for t in tags))
