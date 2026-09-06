#!/usr/bin/env python3
"""Honesty ratchet — a locked, un-ownable, provenance-bearing metric gate.

Skeleton for the oracle-first honesty-ratchet pattern (see
~/.claude/skills/oracle-first/honesty-ratchet.md). Stdlib only.

A project describes its product-path metric in `ratchet.json`:

  produce   — command that runs the SHIPPED path over the corpus and writes
              one output per case under {out}/<family>/<case>...
  score     — command that compares {out} against the frozen {ref} corpus and
              prints JSON {"<family>": {"pass": N, "eligible": M}} to stdout.
  corpus    — source-derived universe: corpus/<family>/<case>.* — the
              denominator is COUNTED from this tree, never read from the
              scorer or the baseline author. `corpus_pattern` (default
              "**/*") selects which files are cases; case ids are the path
              inside the family directory without extension, so nested
              layouts work.
  reference — frozen expected outputs, reference/<family>/<case>.* (may be
              the same tree as the corpus when goldens sit beside inputs).
  universe  — optional command printing JSON {"<family>": ["<case id>", ...]}
              listing the ELIGIBLE cases when frozen skip rules exclude some
              corpus files. It may only subtract from the corpus tree, never
              add to it, and the raw corpus count is locked alongside it so a
              widening skip rule shows up as UNIVERSE_DRIFT.
  embed_guard — product source globs + forbidden patterns (fixture echo).
  holdout   — out-of-corpus inputs, each checked against a FRESHLY generated
              reference (reference_one) rather than a stored golden.

`ratchet check` blocks (exit 1) on any of:

  PROVENANCE            baseline hand-edited or missing reason/mechanism
  EMBED_GUARD           product source mentions a forbidden pattern
  DENOMINATOR           scorer's eligible count != source-derived universe,
                        or the universe command named a case not in the corpus
  REGRESSION            pass below baseline beyond tolerance
  UNLOCKED_IMPROVEMENT  pass above baseline beyond tolerance (lock it!)
  UNIVERSE_DRIFT        corpus or eligible-universe size changed without a lock
  ECHO_SENTINEL         scorer still passes deliberately poisoned outputs
  HOLDOUT_IN_CORPUS     a holdout input is byte-identical to a corpus input
  HOLDOUT_FAIL          product output != freshly generated reference
  COMMAND_FAILED        produce/score/holdout command exited non-zero

`ratchet lock --reason ... --mechanism ...` re-runs every guard except the
baseline comparison and writes a new baseline carrying a provenance block
whose hash binds the numbers to the stated reason. Hand-editing the numbers
breaks the hash; the next `check` reports PROVENANCE.
"""

from __future__ import annotations

import argparse
import datetime as dt
import glob
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

SCHEMA_VERSION = 1
DEFAULT_CONFIG_NAME = "ratchet.json"
DEFAULT_BASELINE_NAME = "ratchet-baseline.json"
DEFAULT_OUT_DIR = "out"
DEFAULT_CORPUS_PATTERN = "**/*"
POISON_BYTES = b"RATCHET-POISON: this output was deliberately corrupted\n"
EXIT_OK = 0
EXIT_BLOCKED = 1
EXIT_USAGE = 2


class Blocked(Exception):
    """A single ratchet finding: (tag, message)."""

    def __init__(self, tag: str, message: str) -> None:
        super().__init__(f"[{tag}] {message}")
        self.tag = tag
        self.message = message


# ---------------------------------------------------------------- config ---


def load_config(config_path: Path) -> tuple[dict, Path]:
    try:
        config = json.loads(config_path.read_text())
    except (OSError, ValueError) as exc:
        raise Blocked("CONFIG", f"cannot read {config_path}: {exc}") from exc
    if config.get("schema") != SCHEMA_VERSION:
        raise Blocked("CONFIG", f"{config_path}: schema must be {SCHEMA_VERSION}")
    for key in ("produce", "score", "corpus", "reference"):
        if not config.get(key):
            raise Blocked("CONFIG", f"{config_path}: missing required key {key!r}")
    return config, config_path.resolve().parent


def resolve(root: Path, rel: str) -> Path:
    path = (root / rel).resolve()
    if root not in path.parents and path != root:
        raise Blocked("CONFIG", f"path {rel!r} escapes the project root {root}")
    return path


def run_cmd(template: str, root: Path, **fields: str) -> str:
    command = template.format(**fields)
    result = subprocess.run(
        command, shell=True, cwd=root, capture_output=True, text=True
    )
    if result.returncode != 0:
        raise Blocked(
            "COMMAND_FAILED",
            f"`{command}` exited {result.returncode}\n{result.stderr.strip()}",
        )
    return result.stdout


# -------------------------------------------------------------- universe ---


def source_universe(corpus_dir: Path, pattern: str) -> dict[str, list[str]]:
    """family -> sorted case ids, counted from the corpus tree itself.

    A case id is the file's path inside its family directory with the
    extension removed (`nested/dir/name`), so nested layouts and co-located
    reference files (selected away by `pattern`) both work."""
    if not corpus_dir.is_dir():
        raise Blocked("CONFIG", f"corpus dir {corpus_dir} does not exist")
    universe: dict[str, list[str]] = {}
    for family_dir in sorted(p for p in corpus_dir.iterdir() if p.is_dir()):
        if family_dir.name.startswith("."):
            continue
        cases = sorted({
            path.relative_to(family_dir).with_suffix("").as_posix()
            for path in family_dir.glob(pattern)
            if path.is_file() and not path.name.startswith(".")
        })
        if cases:
            universe[family_dir.name] = cases
    if not universe:
        raise Blocked("CONFIG", f"corpus dir {corpus_dir} has no <family>/<case> files")
    return universe


def eligible_universe(
    config: dict, root: Path, corpus_universe: dict[str, list[str]]
) -> tuple[dict[str, list[str]], list[Blocked]]:
    """The denominator: the corpus tree, or the `universe` command's subset
    of it. The command may subtract (frozen skip rules) but never add."""
    command = config.get("universe")
    if not command:
        return corpus_universe, []
    corpus = resolve(root, config["corpus"])
    try:
        listing = json.loads(run_cmd(command, root, corpus=str(corpus)))
    except ValueError as exc:
        raise Blocked("UNIVERSE_FORMAT", f"universe command did not print JSON: {exc}") from exc
    if not isinstance(listing, dict) or not all(
        isinstance(cases, list) and all(isinstance(c, str) for c in cases)
        for cases in listing.values()
    ):
        raise Blocked("UNIVERSE_FORMAT", "universe JSON must map family -> [case id, ...]")
    findings = []
    universe: dict[str, list[str]] = {}
    for family, cases in sorted(listing.items()):
        known = set(corpus_universe.get(family, []))
        invented = sorted(set(cases) - known)
        if invented:
            findings.append(Blocked(
                "DENOMINATOR",
                f"{family}: universe command named {len(invented)} case(s) absent from the "
                f"corpus tree (first: {invented[0]!r}) — the universe may only subtract",
            ))
        if cases:
            universe[family] = sorted(set(cases) & known)
    return universe, findings


def corpus_content_hashes(corpus_dir: Path) -> set[str]:
    hashes = set()
    for path in corpus_dir.rglob("*"):
        if path.is_file() and not path.name.startswith("."):
            hashes.add(hashlib.sha256(path.read_bytes()).hexdigest())
    return hashes


# --------------------------------------------------------------- scoring ---


def parse_metrics(raw: str) -> dict[str, dict[str, int]]:
    try:
        metrics = json.loads(raw)
    except ValueError as exc:
        raise Blocked("SCORE_FORMAT", f"scorer did not print JSON: {exc}") from exc
    if not isinstance(metrics, dict) or not metrics:
        raise Blocked("SCORE_FORMAT", "scorer JSON must be a non-empty object")
    for family, counts in metrics.items():
        if not isinstance(counts, dict) or not {"pass", "eligible"} <= counts.keys():
            raise Blocked(
                "SCORE_FORMAT", f"family {family!r} needs integer pass and eligible"
            )
        for key in ("pass", "eligible"):
            if not isinstance(counts[key], int) or counts[key] < 0:
                raise Blocked("SCORE_FORMAT", f"{family}.{key} must be a non-negative int")
        if counts["pass"] > counts["eligible"]:
            raise Blocked("SCORE_FORMAT", f"{family}: pass exceeds eligible")
    return metrics


def produce_and_score(config: dict, root: Path, out_dir: Path) -> dict[str, dict[str, int]]:
    corpus = resolve(root, config["corpus"])
    ref = resolve(root, config["reference"])
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True)
    run_cmd(config["produce"], root, corpus=str(corpus), out=str(out_dir), ref=str(ref))
    return parse_metrics(
        run_cmd(config["score"], root, out=str(out_dir), ref=str(ref), corpus=str(corpus))
    )


def check_denominators(
    metrics: dict[str, dict[str, int]], universe: dict[str, list[str]]
) -> list[Blocked]:
    findings = []
    for family in sorted(set(metrics) | set(universe)):
        if family not in universe:
            findings.append(Blocked(
                "DENOMINATOR", f"{family}: scorer reports a family absent from the corpus"
            ))
        elif family not in metrics:
            findings.append(Blocked(
                "DENOMINATOR", f"{family}: corpus family missing from scorer output"
            ))
        elif metrics[family]["eligible"] != len(universe[family]):
            findings.append(Blocked(
                "DENOMINATOR",
                f"{family}: scorer eligible={metrics[family]['eligible']} but the corpus "
                f"holds {len(universe[family])} cases — the denominator is not the "
                "scorer's to choose",
            ))
    return findings


# -------------------------------------------------------------- baseline ---


def canonical_metrics(metrics: dict) -> str:
    return json.dumps(metrics, sort_keys=True, separators=(",", ":"))


def metrics_digest(metrics: dict) -> str:
    return hashlib.sha256(canonical_metrics(metrics).encode()).hexdigest()


def load_baseline(path: Path) -> dict:
    try:
        baseline = json.loads(path.read_text())
    except OSError as exc:
        raise Blocked(
            "PROVENANCE",
            f"no baseline at {path} ({exc}); bootstrap with `ratchet lock`",
        ) from exc
    except ValueError as exc:
        raise Blocked("PROVENANCE", f"baseline {path} is not valid JSON: {exc}") from exc
    if baseline.get("schema") != SCHEMA_VERSION:
        raise Blocked("PROVENANCE", f"baseline schema must be {SCHEMA_VERSION}")
    metrics = baseline.get("metrics")
    provenance = baseline.get("provenance")
    if not isinstance(metrics, dict) or not isinstance(provenance, dict):
        raise Blocked("PROVENANCE", "baseline needs `metrics` and `provenance` objects")
    for key in ("reason", "mechanism"):
        if not str(provenance.get(key, "")).strip():
            raise Blocked(
                "PROVENANCE",
                f"baseline provenance.{key} is empty — numbers may only move with a "
                "stated reason and a named mechanism",
            )
    expected = metrics_digest(metrics)
    if provenance.get("metrics_sha256") != expected:
        raise Blocked(
            "PROVENANCE",
            "baseline metrics do not match provenance.metrics_sha256 — the numbers "
            "were edited by hand instead of via `ratchet lock`",
        )
    return baseline


def tolerance_for(config: dict, family: str) -> int:
    tolerance = config.get("tolerance", {})
    return int(tolerance.get("per_family", {}).get(family, tolerance.get("default", 0)))


def compare_with_baseline(
    config: dict,
    baseline_metrics: dict,
    observed: dict[str, dict[str, int]],
    universe: dict[str, list[str]],
    corpus_universe: dict[str, list[str]],
) -> list[Blocked]:
    findings = []
    for family in sorted(set(baseline_metrics) | set(observed)):
        if family not in baseline_metrics:
            findings.append(Blocked(
                "UNIVERSE_DRIFT", f"{family}: new family not in baseline — lock it"
            ))
            continue
        if family not in observed:
            findings.append(Blocked(
                "UNIVERSE_DRIFT", f"{family}: baseline family vanished from the corpus"
            ))
            continue
        base = baseline_metrics[family]
        seen_universe = len(universe.get(family, []))
        seen_corpus = len(corpus_universe.get(family, []))
        if base.get("universe") != seen_universe:
            findings.append(Blocked(
                "UNIVERSE_DRIFT",
                f"{family}: eligible universe is {seen_universe} cases, baseline locked "
                f"{base.get('universe')} — corpus and skip-rule changes must be locked "
                "deliberately",
            ))
        if base.get("corpus") != seen_corpus:
            findings.append(Blocked(
                "UNIVERSE_DRIFT",
                f"{family}: corpus tree holds {seen_corpus} files, baseline locked "
                f"{base.get('corpus')} — corpus changes must be locked deliberately",
            ))
        delta = observed[family]["pass"] - base["pass"]
        tol = tolerance_for(config, family)
        if delta < -tol:
            findings.append(Blocked(
                "REGRESSION",
                f"{family}: pass {observed[family]['pass']} < baseline {base['pass']} "
                f"(tolerance {tol}) — the product path got worse",
            ))
        elif delta > tol:
            findings.append(Blocked(
                "UNLOCKED_IMPROVEMENT",
                f"{family}: pass {observed[family]['pass']} > baseline {base['pass']} "
                f"(tolerance {tol}) — improvements must be locked with a reason and a "
                "mechanism, or a stale baseline hides the next regression",
            ))
    return findings


def write_baseline(
    path: Path,
    observed: dict,
    universe: dict,
    corpus_universe: dict,
    reason: str,
    mechanism: str,
    argv: list[str],
) -> dict:
    metrics = {
        family: {
            "pass": observed[family]["pass"],
            "universe": len(universe[family]),
            "corpus": len(corpus_universe[family]),
        }
        for family in sorted(observed)
    }
    baseline = {
        "schema": SCHEMA_VERSION,
        "metrics": metrics,
        "provenance": {
            "metrics_sha256": metrics_digest(metrics),
            "reason": reason,
            "mechanism": mechanism,
            "locked_at": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
            "command": " ".join(argv),
        },
    }
    path.write_text(json.dumps(baseline, indent=2, sort_keys=True) + "\n")
    return baseline


# ---------------------------------------------------------------- guards ---


def embed_guard(config: dict, root: Path) -> list[Blocked]:
    guard = config.get("embed_guard")
    if not guard:
        return []
    patterns = [re.compile(p) for p in guard.get("forbidden", [])]
    findings = []
    for source_glob in guard.get("sources", []):
        for match in sorted(glob.glob(str(root / source_glob), recursive=True)):
            path = Path(match)
            if not path.is_file():
                continue
            for line_no, line in enumerate(path.read_text(errors="replace").splitlines(), 1):
                for pattern in patterns:
                    if pattern.search(line):
                        findings.append(Blocked(
                            "EMBED_GUARD",
                            f"{path.relative_to(root)}:{line_no}: matches forbidden "
                            f"pattern /{pattern.pattern}/ — product source must not "
                            "reach into the reference corpus",
                        ))
    return findings


def echo_sentinel(config: dict, root: Path, out_dir: Path) -> list[Blocked]:
    """Poison every product output and rescore: a scorer that still passes
    anything is reading its expectation from the artefact under test."""
    ref = resolve(root, config["reference"])
    corpus = resolve(root, config["corpus"])
    with tempfile.TemporaryDirectory(prefix="ratchet-poison-") as tmp:
        poisoned = Path(tmp) / DEFAULT_OUT_DIR
        shutil.copytree(out_dir, poisoned)
        for path in poisoned.rglob("*"):
            if path.is_file():
                path.write_bytes(POISON_BYTES + path.read_bytes())
        metrics = parse_metrics(run_cmd(
            config["score"], root, out=str(poisoned), ref=str(ref), corpus=str(corpus)
        ))
    return [
        Blocked(
            "ECHO_SENTINEL",
            f"{family}: scorer passed {counts['pass']} deliberately poisoned outputs — "
            "it is not comparing against the reference",
        )
        for family, counts in sorted(metrics.items())
        if counts["pass"] > 0
    ]


def holdout_guard(config: dict, root: Path) -> list[Blocked]:
    holdout = config.get("holdout")
    if not holdout:
        return []
    for key in ("inputs", "produce_one", "reference_one"):
        if not holdout.get(key):
            raise Blocked("CONFIG", f"holdout.{key} is required when holdout is set")
    corpus_hashes = corpus_content_hashes(resolve(root, config["corpus"]))
    inputs = sorted(
        Path(p) for p in glob.glob(str(root / holdout["inputs"]), recursive=True)
        if Path(p).is_file()
    )
    if not inputs:
        raise Blocked("CONFIG", f"holdout.inputs {holdout['inputs']!r} matched nothing")
    findings = []
    with tempfile.TemporaryDirectory(prefix="ratchet-holdout-") as tmp:
        for index, input_path in enumerate(inputs):
            rel = input_path.relative_to(root)
            if hashlib.sha256(input_path.read_bytes()).hexdigest() in corpus_hashes:
                findings.append(Blocked(
                    "HOLDOUT_IN_CORPUS",
                    f"{rel}: byte-identical to a corpus input — a holdout must be "
                    "something the corpus never saw",
                ))
                continue
            product_out = Path(tmp) / f"product-{index}"
            fresh_ref = Path(tmp) / f"reference-{index}"
            run_cmd(holdout["produce_one"], root, input=str(input_path), output=str(product_out))
            run_cmd(holdout["reference_one"], root, input=str(input_path), output=str(fresh_ref))
            if product_out.read_bytes() != fresh_ref.read_bytes():
                findings.append(Blocked(
                    "HOLDOUT_FAIL",
                    f"{rel}: product output differs from the freshly generated reference "
                    f"(product {product_out.read_bytes()[:60]!r}, "
                    f"reference {fresh_ref.read_bytes()[:60]!r})",
                ))
    return findings


# ------------------------------------------------------------- commands ---


class GuardRun:
    """Everything except the baseline comparison."""

    def __init__(self, config: dict, root: Path, out_dir: Path) -> None:
        self.findings = embed_guard(config, root)
        corpus_dir = resolve(root, config["corpus"])
        self.corpus_universe = source_universe(
            corpus_dir, config.get("corpus_pattern", DEFAULT_CORPUS_PATTERN)
        )
        self.universe, universe_findings = eligible_universe(config, root, self.corpus_universe)
        self.findings += universe_findings
        self.metrics = produce_and_score(config, root, out_dir)
        self.findings += check_denominators(self.metrics, self.universe)
        self.findings += echo_sentinel(config, root, out_dir)
        self.findings += holdout_guard(config, root)

    def table(self, previous: dict | None = None) -> list[str]:
        lines = []
        for family in sorted(self.metrics):
            line = (f"  {family:<20} pass {self.metrics[family]['pass']:>5} / "
                    f"{len(self.universe.get(family, [])):<5} "
                    f"(corpus {len(self.corpus_universe.get(family, []))})")
            if previous is not None:
                before = previous.get(family, {}).get("pass")
                line += f"  (was {before})" if before is not None else "  (new)"
            lines.append(line)
        return lines


def report(run: GuardRun, findings: list[Blocked], verdict: str) -> None:
    for line in run.table():
        print(line)
    for finding in findings:
        print(f"BLOCKED [{finding.tag}] {finding.message}")
    print(f"ratchet: {verdict}")


def cmd_check(args: argparse.Namespace) -> int:
    config, root = load_config(Path(args.config))
    baseline_path = resolve(root, config.get("baseline", DEFAULT_BASELINE_NAME))
    out_dir = resolve(root, config.get("out", DEFAULT_OUT_DIR))
    findings: list[Blocked] = []
    baseline = None
    try:
        baseline = load_baseline(baseline_path)
    except Blocked as exc:
        findings.append(exc)
    run = GuardRun(config, root, out_dir)
    findings += run.findings
    if baseline is not None:
        findings += compare_with_baseline(
            config, baseline["metrics"], run.metrics, run.universe, run.corpus_universe
        )
    if findings:
        report(run, findings, f"BLOCKED — {len(findings)} finding(s)")
        return EXIT_BLOCKED
    report(run, findings, "clean — product-path metrics match the locked baseline")
    return EXIT_OK


def cmd_lock(args: argparse.Namespace) -> int:
    if not args.reason.strip() or not args.mechanism.strip():
        print("ratchet: --reason and --mechanism must be non-empty", file=sys.stderr)
        return EXIT_USAGE
    config, root = load_config(Path(args.config))
    baseline_path = resolve(root, config.get("baseline", DEFAULT_BASELINE_NAME))
    out_dir = resolve(root, config.get("out", DEFAULT_OUT_DIR))
    run = GuardRun(config, root, out_dir)
    if run.findings:
        report(run, run.findings, "REFUSED to lock — fix the guards first")
        return EXIT_BLOCKED
    previous = None
    if baseline_path.exists():
        try:
            previous = load_baseline(baseline_path)["metrics"]
        except Blocked as exc:
            print(f"note: replacing a baseline that failed provenance: {exc}")
    write_baseline(
        baseline_path, run.metrics, run.universe, run.corpus_universe,
        args.reason, args.mechanism, sys.argv,
    )
    for line in run.table(previous if previous is not None else {}):
        print(line)
    print(f"ratchet: locked {baseline_path.relative_to(root)} — reason: {args.reason!r}; "
          f"mechanism: {args.mechanism!r}")
    return EXIT_OK


def cmd_show(args: argparse.Namespace) -> int:
    config, root = load_config(Path(args.config))
    baseline = load_baseline(resolve(root, config.get("baseline", DEFAULT_BASELINE_NAME)))
    for family, counts in sorted(baseline["metrics"].items()):
        print(f"  {family:<20} pass {counts['pass']:>5} / {counts['universe']:<5} "
              f"(corpus {counts.get('corpus')})")
    prov = baseline["provenance"]
    print(f"locked {prov.get('locked_at')} — {prov['reason']} [{prov['mechanism']}]")
    return EXIT_OK


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--config", default=DEFAULT_CONFIG_NAME, help="path to ratchet.json")
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("check", help="run every guard and compare with the locked baseline")
    lock = sub.add_parser("lock", help="re-run guards and lock the current numbers")
    lock.add_argument("--reason", required=True, help="why the numbers moved")
    lock.add_argument("--mechanism", required=True,
                      help="named reference mechanism the change traces to")
    sub.add_parser("show", help="print the locked baseline")
    args = parser.parse_args(argv)
    handler = {"check": cmd_check, "lock": cmd_lock, "show": cmd_show}[args.command]
    try:
        return handler(args)
    except Blocked as exc:
        print(f"BLOCKED [{exc.tag}] {exc.message}")
        print("ratchet: BLOCKED")
        return EXIT_BLOCKED


if __name__ == "__main__":
    sys.exit(main())
