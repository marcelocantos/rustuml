#!/usr/bin/env python3
"""Generate and run the followup2 rankdir/factory-consumability corpus.

This script intentionally uses the frozen product binary supplied to the
checker and never invokes Cargo.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO_ROOT = ROOT.parents[3]
RUST_BIN = Path("/private/tmp/rustuml-uml-factory-consumability/target/release/rustuml")
JAVA_JAR = Path(
    "/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar"
)

RUST_SHA256 = "49a81f0808d1fd4df88aaac25108b0475ec5fd84c19eb217c81d843d6c4a27e7"
RUST_REVISION = "afee6a17ad5643b93325563d09488d0e710394b4"
JAVA_REVISION = "71806a23780b04a5ccde2f8ceb5121edad5eb711"

NBSP = "\u00a0"
EM_SPACE = "\u2003"


CASES = [
    {
        "id": "followup2_001_direction_only_lr",
        "axes": ["valid rankdir", "direction only", "later Class factory"],
        "source": "@startuml\nleft to right direction\n@enduml\n",
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"]},
    },
    {
        "id": "followup2_002_weak_arrow_tb",
        "axes": ["valid top-to-bottom rankdir", "weak ambiguous arrow"],
        "source": "@startuml\ntop to bottom direction\nAlpha -> Beta : weak\n@enduml\n",
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"], "rust": ["CLASS"]},
    },
    {
        "id": "followup2_003_mixed_case_rankdir_weak_arrow",
        "axes": ["case-insensitive rankdir", "weak ambiguous arrow"],
        "source": "@startuml\nLeFt to RiGhT DiReCtIoN\nGamma -> Delta : mixed case\n@enduml\n",
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"], "rust": ["CLASS"]},
    },
    {
        "id": "followup2_004_nbsp_rankdir_description",
        "axes": ["NBSP internal and final rankdir whitespace", "description declarations"],
        "source": (
            "@startuml\n"
            f"left{NBSP}to{NBSP}right{NBSP}direction\n"
            "database \"Fresh Ledger\" as Ledger\n"
            "queue \"Fresh Pipe\" as Pipe\n"
            "Ledger --> Pipe : ships\n"
            "@enduml\n"
        ),
        "expect": "match_accept",
        "allowed": {"java": ["DESCRIPTION"], "rust": ["DEPLOYMENT", "COMPONENT"]},
    },
    {
        "id": "followup2_005_tab_rankdir_class",
        "axes": ["tab rankdir separators", "class declaration"],
        "source": "@startuml\nleft\tto\tright\tdirection\nclass TabRank\n@enduml\n",
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"], "rust": ["CLASS"]},
    },
    {
        "id": "followup2_006_final_space_repetition",
        "axes": ["one internal separator", "multiple final spaces before direction"],
        "source": "@startuml\nleft to right   direction\nclass FinalSpace\n@enduml\n",
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"], "rust": ["CLASS"]},
    },
    {
        "id": "followup2_007_double_internal_space_invalid",
        "axes": ["invalid doubled internal rankdir separator", "weak arrow"],
        "source": "@startuml\nleft  to right direction\nAlpha -> Beta : should not launder\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_008_double_nbsp_internal_invalid",
        "axes": ["invalid doubled NBSP internal rankdir separator", "weak arrow"],
        "source": f"@startuml\nleft{NBSP}{NBSP}to right direction\nAlpha -> Beta : should not launder\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_009_em_space_invalid_unicode_separator",
        "axes": ["invalid non-Pattern2 Unicode whitespace", "weak arrow"],
        "source": f"@startuml\nleft{EM_SPACE}to{EM_SPACE}right direction\nAlpha -> Beta : em space should not launder\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_010_class_entity_body_rankdir_after",
        "axes": ["rankdir after class body", "earlier Class factory"],
        "source": "@startuml\nentity FreshRecord {\n  id : UUID\n}\ntop to bottom direction\n@enduml\n",
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"], "rust": ["CLASS"]},
    },
    {
        "id": "followup2_011_bare_newpage_shared",
        "axes": ["bare newpage", "shared later command", "rankdir before"],
        "source": "@startuml\nleft to right direction\nnewpage\nclass PageOne\n@enduml\n",
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"]},
    },
    {
        "id": "followup2_012_hide_footbox_shared",
        "axes": ["hide footbox shared variant", "rankdir before"],
        "source": "@startuml\nleft to right direction\nhide footbox\nclass HiddenFoot\n@enduml\n",
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"], "rust": ["CLASS"]},
    },
    {
        "id": "followup2_013_hidefootbox_compact_shared",
        "axes": ["hidefootbox zero-whitespace Java variant", "rankdir before"],
        "source": "@startuml\nleft to right direction\nhidefootbox\nclass CompactFoot\n@enduml\n",
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"], "rust": ["CLASS"]},
    },
    {
        "id": "followup2_014_show_unlinked_shared",
        "axes": ["show unlinked shared variant", "rankdir before"],
        "source": "@startuml\nleft to right direction\nshow unlinked\nclass VisibleUnlinked\n@enduml\n",
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"], "rust": ["CLASS"]},
    },
    {
        "id": "followup2_015_named_note_body_rankdir_class",
        "axes": ["rankdir text inside note body", "autonumber text inside note body", "class"],
        "source": (
            "@startuml\n"
            "note as DispatchMemo\n"
            "left to right direction\n"
            "autonumber 99\n"
            "end note\n"
            "class BodyTextOnly\n"
            "@enduml\n"
        ),
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"], "rust": ["CLASS"]},
    },
    {
        "id": "followup2_016_sequence_note_body_no_rankdir",
        "axes": ["rankdir text inside sequence note payload", "no real rankdir"],
        "source": (
            "@startuml\n"
            "participant Maker\n"
            "participant Checker\n"
            "note over Maker\n"
            "left to right direction\n"
            "autonumber 7\n"
            "end note\n"
            "Maker -> Checker : payload only\n"
            "@enduml\n"
        ),
        "expect": "match_accept",
        "allowed": {"java": ["SEQUENCE"], "rust": ["SEQUENCE"]},
    },
    {
        "id": "followup2_017_comments_fake_rankdir_sequence",
        "axes": ["line comment rankdir", "block comment sequence-only text", "real sequence"],
        "source": (
            "@startuml\n"
            "' left to right direction\n"
            "/' autonumber 5\n"
            "left to right direction\n"
            "'/\n"
            "Alice -> Bob : still sequence\n"
            "@enduml\n"
        ),
        "expect": "match_accept",
        "allowed": {"java": ["SEQUENCE"], "rust": ["SEQUENCE"]},
    },
    {
        "id": "followup2_018_preprocessor_rankdir_class",
        "axes": ["TIM procedure emits rankdir", "class"],
        "source": (
            "@startuml\n"
            "!procedure $dir()\n"
            "left to right direction\n"
            "!endprocedure\n"
            "$dir()\n"
            "class PreprocClass\n"
            "@enduml\n"
        ),
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"], "rust": ["CLASS"]},
    },
    {
        "id": "followup2_019_preprocessor_rankdir_participant",
        "axes": ["TIM procedure emits rankdir", "participant sequence-only"],
        "source": (
            "@startuml\n"
            "!procedure $dir()\n"
            "top to bottom direction\n"
            "!endprocedure\n"
            "$dir()\n"
            "participant PreprocOnly\n"
            "@enduml\n"
        ),
        "expect": "match_error",
    },
    {
        "id": "followup2_020_participant_sequence_only",
        "axes": ["SequenceOnlyCommand::Participant", "rankdir after"],
        "source": "@startuml\nparticipant \"Fresh Maker\" as Maker\nMaker -> Sink : hi\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_021_lifecycle_sequence_only",
        "axes": ["SequenceOnlyCommand::Lifecycle", "rankdir before"],
        "source": "@startuml\nleft to right direction\nactivate Maker #Gold\nMaker -> Sink : hi\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_022_message_extension_sequence_only",
        "axes": ["SequenceOnlyCommand::MessageExtension", "inline activation"],
        "source": "@startuml\nMaker -> Sink ++ #Gold : call\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_023_external_message_sequence_only",
        "axes": ["SequenceOnlyCommand::MessageExtension", "external message boundary"],
        "source": "@startuml\n[-> Maker : inbound\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_024_grouping_sequence_only",
        "axes": ["SequenceOnlyCommand::Grouping", "alt else end", "rankdir after"],
        "source": (
            "@startuml\n"
            "alt #MistyRose first branch\n"
            "Maker -> Sink : first\n"
            "else #HoneyDew second branch\n"
            "Maker -> Sink : second\n"
            "end\n"
            "left to right direction\n"
            "@enduml\n"
        ),
        "expect": "match_error",
    },
    {
        "id": "followup2_025_grouping_par2_also",
        "axes": ["SequenceOnlyCommand::Grouping", "par2 also", "mixed case rankdir"],
        "source": "@startuml\npar2 parallel work\nalso backup work\nend\nToP to BoTtOm direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_026_box_sequence_only",
        "axes": ["SequenceOnlyCommand::Box", "rankdir after"],
        "source": "@startuml\nbox \"Fresh Box\" #LightBlue\nparticipant Maker\nend box\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_027_divider_sequence_only",
        "axes": ["SequenceOnlyCommand::Divider", "rankdir before"],
        "source": "@startuml\nleft to right direction\n== Fresh phase ==\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_028_triple_equals_activity_shared",
        "axes": ["shared triple-equals sync bar", "rankdir before"],
        "source": "@startuml\nleft to right direction\n===SyncPoint===\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_029_delay_sequence_only",
        "axes": ["SequenceOnlyCommand::Delay", "unicode ellipsis", "rankdir after"],
        "source": "@startuml\n… translated wait …\ntop to bottom direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_030_hspace_sequence_only",
        "axes": ["SequenceOnlyCommand::HorizontalSpace", "rankdir before"],
        "source": "@startuml\nleft to right direction\n||37|||\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_031_autonumber_sequence_only",
        "axes": ["SequenceOnlyCommand::Autonumber", "rankdir after"],
        "source": "@startuml\nautonumber 42 3 \"F-%03d\"\nMaker -> Sink : numbered\ntop to bottom direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_032_autonumber_inc_sequence_only",
        "axes": ["SequenceOnlyCommand::Autonumber", "inc variant", "rankdir after"],
        "source": "@startuml\nautonumber 1\nautonumber inc C\nMaker -> Sink : numbered\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_033_autoactivate_sequence_only",
        "axes": ["SequenceOnlyCommand::Autoactivate", "rankdir after"],
        "source": "@startuml\nautoactivate on\nMaker -> Sink : auto\ntop to bottom direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_034_return_sequence_only",
        "axes": ["SequenceOnlyCommand::Return", "rankdir after"],
        "source": "@startuml\nMaker -> Sink : call\nreturn accepted\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_035_reference_sequence_only",
        "axes": ["SequenceOnlyCommand::Reference", "rankdir before"],
        "source": "@startuml\nleft to right direction\nref over Maker, Sink : reviewed path\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_036_reference_multiline_sequence_only",
        "axes": ["SequenceOnlyCommand::Reference", "multiline ref", "rankdir after"],
        "source": "@startuml\nref over Maker, Sink\nmanual approval\nend ref\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_037_pagination_labeled_sequence_only",
        "axes": ["SequenceOnlyCommand::Pagination", "labeled newpage", "rankdir after"],
        "source": "@startuml\n@newpage : second chapter\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_038_pagination_ignore_sequence_only",
        "axes": ["SequenceOnlyCommand::Pagination", "ignore newpage", "rankdir after"],
        "source": "@startuml\nignore newpage\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_039_pagination_autonewpage_sequence_only",
        "axes": ["SequenceOnlyCommand::Pagination", "autonewpage", "rankdir after"],
        "source": "@startuml\nautonewpage 321\ntop to bottom direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_040_footbox_sequence_only",
        "axes": ["SequenceOnlyCommand::Footbox", "footbox off", "rankdir after"],
        "source": "@startuml\nfootbox off\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_041_note_sequence_only",
        "axes": ["SequenceOnlyCommand::Note", "note over", "rankdir after"],
        "source": "@startuml\nnote over Maker : sequence-only note\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_042_note_side_sequence_only",
        "axes": ["SequenceOnlyCommand::Note", "bare note left", "rankdir after"],
        "source": "@startuml\nMaker -> Sink : call\nnote left : side note\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_043_anchor_sequence_only",
        "axes": ["SequenceOnlyCommand::Anchor", "rankdir after"],
        "source": "@startuml\n{from}<->{to} : anchor link\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_044_create_destroy_sequence_only",
        "axes": ["SequenceOnlyCommand::Lifecycle", "create/destroy", "rankdir after"],
        "source": "@startuml\ncreate participant Worker\nMaker -> Worker : build\ndestroy Worker\ntop to bottom direction\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_045_footbox_show_compact_shared",
        "axes": ["showfootbox zero-whitespace Java variant", "rankdir before"],
        "source": "@startuml\nleft to right direction\nshowfootbox\nclass CompactShowFoot\n@enduml\n",
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"], "rust": ["CLASS"]},
    },
    {
        "id": "followup2_046_partition_shared_with_activity",
        "axes": ["partition shared classifier branch", "rankdir before"],
        "source": "@startuml\nleft to right direction\npartition \"Fresh Work\" {\n:do it;\n}\n@enduml\n",
        "expect": "match_error",
    },
    {
        "id": "followup2_047_note_on_link_shared",
        "axes": ["ordinary note on link", "rankdir before", "state/usecase shared note command"],
        "source": "@startuml\nleft to right direction\nA --> B\nnote on link : ordinary link note\n@enduml\n",
        "expect": "match_accept",
        "allowed": {"java": ["CLASS"], "rust": ["CLASS"]},
    },
    {
        "id": "followup2_048_direction_inside_comment_then_real_rankdir",
        "axes": ["comment ignored", "real rankdir after", "participant sequence-only"],
        "source": "@startuml\n' top to bottom direction\nparticipant Commented\nleft to right direction\n@enduml\n",
        "expect": "match_error",
    },
]


def run(command: list[str], *, stdout_path: Path, stderr_path: Path) -> int:
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        completed = subprocess.run(command, cwd=ROOT, stdout=stdout, stderr=stderr, check=False)
    return completed.returncode


def extract_type(svg_path: Path) -> str | None:
    if not svg_path.exists() or svg_path.stat().st_size == 0:
        return None
    text = svg_path.read_text(encoding="utf-8", errors="replace")
    match = re.search(r'data-diagram-type="([^"]+)"', text)
    if match:
        return match.group(1)
    match = re.search(r"Assumed diagram type: ([^)]+)", text)
    if match:
        return "ERROR_ASSUMED_" + match.group(1).upper()
    return None


def status(exit_code: int) -> str:
    return "accept" if exit_code == 0 else "error"


def verdict_for(
    case: dict,
    java_exit: int,
    rust_exit: int,
    java_type: str | None,
    rust_type: str | None,
) -> str:
    expected = case["expect"]
    java_status = status(java_exit)
    rust_status = status(rust_exit)
    if expected == "match_error":
        return "PASS" if java_status == "error" and rust_status == "error" else "FAIL"
    if java_status != "accept" or rust_status != "accept":
        return "FAIL"
    allowed = case.get("allowed", {})
    java_allowed = set(allowed.get("java", []))
    rust_allowed = set(allowed.get("rust", []))
    if java_allowed and java_type not in java_allowed:
        return "FAIL"
    if rust_allowed and rust_type not in rust_allowed:
        return "FAIL"
    return "PASS"


def main() -> int:
    actual_sha = subprocess.check_output(["shasum", "-a", "256", str(RUST_BIN)], text=True).split()[0]
    if actual_sha != RUST_SHA256:
        raise SystemExit(f"wrong frozen rustuml sha256: {actual_sha}")

    results = []
    for case in CASES:
        stem = case["id"]
        puml_path = ROOT / f"{stem}.puml"
        java_svg_tmp = ROOT / f"{stem}.svg"
        java_svg = ROOT / f"{stem}.java.svg"
        rust_svg = ROOT / f"{stem}.rust.svg"
        for suffix in [
            ".java.stdout",
            ".java.stderr",
            ".java.exit",
            ".java.svg",
            ".rust.stderr",
            ".rust.exit",
            ".rust.svg",
        ]:
            path = ROOT / f"{stem}{suffix}"
            if path.exists():
                path.unlink()
        if java_svg_tmp.exists():
            java_svg_tmp.unlink()

        puml_path.write_text(case["source"], encoding="utf-8")
        java_exit = run(
            [
                "java",
                "-Djava.awt.headless=true",
                "-jar",
                str(JAVA_JAR),
                "-tsvg",
                str(puml_path),
            ],
            stdout_path=ROOT / f"{stem}.java.stdout",
            stderr_path=ROOT / f"{stem}.java.stderr",
        )
        (ROOT / f"{stem}.java.exit").write_text(f"{java_exit}\n", encoding="ascii")
        if java_svg_tmp.exists():
            shutil.move(str(java_svg_tmp), str(java_svg))
        else:
            java_svg.write_bytes(b"")

        rust_exit = run(
            [str(RUST_BIN), "-tsvg", str(puml_path)],
            stdout_path=rust_svg,
            stderr_path=ROOT / f"{stem}.rust.stderr",
        )
        (ROOT / f"{stem}.rust.exit").write_text(f"{rust_exit}\n", encoding="ascii")

        java_type = extract_type(java_svg)
        rust_type = extract_type(rust_svg)
        result = {
            "id": stem,
            "axes": case["axes"],
            "input": str(puml_path.relative_to(REPO_ROOT)),
            "java_exit": java_exit,
            "rust_exit": rust_exit,
            "java_status": status(java_exit),
            "rust_status": status(rust_exit),
            "java_type": java_type,
            "rust_type": rust_type,
            "expect": case["expect"],
            "allowed": case.get("allowed"),
            "verdict": verdict_for(case, java_exit, rust_exit, java_type, rust_type),
        }
        results.append(result)
        print(
            f"{stem}: {result['verdict']} "
            f"java={result['java_status']}:{java_type} rust={result['rust_status']}:{rust_type}"
        )

    failures = [result for result in results if result["verdict"] != "PASS"]
    manifest = {
        "schema_version": 1,
        "mechanism": "uml-rankdir-factory-consumability",
        "checker": "followup2",
        "rust_revision": RUST_REVISION,
        "java_revision": JAVA_REVISION,
        "rust_binary": str(RUST_BIN),
        "rust_binary_sha256": RUST_SHA256,
        "java_jar": str(JAVA_JAR),
        "commands": [
            f"java -Djava.awt.headless=true -jar {JAVA_JAR} -tsvg <case>.puml",
            f"{RUST_BIN} -tsvg <case>.puml",
            "shasum -a 256 /private/tmp/rustuml-uml-factory-consumability/target/release/rustuml",
        ],
        "case_count": len(results),
        "failures": [failure["id"] for failure in failures],
        "results": results,
    }
    (ROOT / "followup2-results.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
