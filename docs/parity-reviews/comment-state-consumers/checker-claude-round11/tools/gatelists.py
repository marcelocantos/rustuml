"""Extract gate lists from logs. Usage: gatelists.py <gates_dir> <label>
Writes <label>.pert_fail.txt (failing perturbation sources) and <label>.guard_cats.txt (harness_guards
violation categories with counts, per failing guard test)."""
import re, sys, pathlib, collections
G, L = pathlib.Path(sys.argv[1]), sys.argv[2]
p = (G / f"{L}.parity_perturbations.log").read_text(errors="replace")
fails = sorted(set(re.findall(r"^  (test-diagrams/perturbations/\S+\.puml):", p, re.M)))
(G / f"{L}.pert_fail.txt").write_text("".join(f + "\n" for f in fails))
h = (G / f"{L}.harness_guards.log").read_text(errors="replace")
cats = collections.Counter()
for line in h.splitlines():
    m = re.match(r"^  (?:/\S+|test-diagrams/\S+|docs/\S+|crates/\S+)?:?\s*(.*)$", line)
    if line.startswith("  ") and ": " in line:
        msg = line.strip().split(": ", 1)[1]
        msg = re.sub(r"test-diagrams/\S+|docs/\S+|/Users/\S+", "<path>", msg)
        msg = re.sub(r"\d+", "N", msg)[:110]
        cats[msg] += 1
tests = re.findall(r"^test (\S+) \.\.\. (\w+)", h, re.M)
out = [f"{t} {r}" for t, r in tests] + [""] + [f"{n:6d} {c}" for c, n in cats.most_common()]
(G / f"{L}.guard_cats.txt").write_text("\n".join(out) + "\n")
print(L, "pert fails", len(fails), "| guard tests", [t for t, r in tests if r != "ok"])
