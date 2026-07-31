#!/usr/bin/env python3
import collections
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

from PIL import Image


ROOT = Path(__file__).resolve().parent
MARKER = re.compile(r"CHK_[A-Z0-9_]+")
ML_POINT = re.compile(r"[ML]\s*(-?\d+(?:\.\d+)?),(-?\d+(?:\.\d+)?)")


def local_name(tag):
    return tag.rsplit("}", 1)[-1]


def status(stem, suffix):
    path = Path(f"{stem}.{suffix}.status")
    return int(path.read_text().strip()) if path.exists() else None


def parse_svg(path):
    result = {
        "parse_error": None,
        "diagram_type": None,
        "markers": [],
        "marker_text": {},
        "marker_offsets": {},
        "note_boxes": {},
        "error_text": [],
        "clipped_text": [],
    }
    try:
        root = ET.parse(path).getroot()
    except Exception as error:
        result["parse_error"] = str(error)
        return result

    result["diagram_type"] = root.attrib.get("data-diagram-type")
    view_box = [float(value) for value in root.attrib.get("viewBox", "").split()]
    marker_text = collections.defaultdict(list)
    for element in root.iter():
        if local_name(element.tag) != "text":
            continue
        text = "".join(element.itertext()).replace("\u00a0", " ")
        if len(view_box) == 4:
            x = element.attrib.get("x")
            text_length = element.attrib.get("textLength")
            if x is not None and text_length is not None:
                left = float(x)
                right = left + float(text_length)
                view_left = view_box[0]
                view_right = view_box[0] + view_box[2]
                if left < view_left - 0.01 or right > view_right + 0.01:
                    result["clipped_text"].append(
                        {
                            "text": text,
                            "left": left,
                            "right": round(right, 4),
                            "view_left": view_left,
                            "view_right": view_right,
                        }
                    )
        for marker in MARKER.findall(text):
            result["markers"].append(marker)
            marker_text[marker].append(text)
        lower = text.lower()
        if (
            "syntax error" in lower
            or "no link defined" in lower
            or "no note defined" in lower
            or "already created" in lower
            or "no such color" in lower
            or "parse error" in lower
        ):
            result["error_text"].append(text)

    result["markers"].sort()
    result["marker_text"] = {
        marker: sorted(texts) for marker, texts in sorted(marker_text.items())
    }
    for group in root.iter():
        if local_name(group.tag) != "g":
            continue
        qualified_name = group.attrib.get("data-qualified-name")
        if not qualified_name:
            continue
        path = next(
            (child for child in group if local_name(child.tag) == "path"),
            None,
        )
        if path is None:
            continue
        points = [
            (float(x), float(y)) for x, y in ML_POINT.findall(path.attrib.get("d", ""))
        ]
        if not points:
            continue
        xs = [point[0] for point in points]
        ys = [point[1] for point in points]
        result["note_boxes"][qualified_name] = {
            "width": round(max(xs) - min(xs), 4),
            "height": round(max(ys) - min(ys), 4),
        }
        offsets = collections.defaultdict(list)
        for element in group.iter():
            if local_name(element.tag) != "text":
                continue
            text = "".join(element.itertext())
            x = element.attrib.get("x")
            y = element.attrib.get("y")
            if x is None or y is None:
                continue
            for marker in MARKER.findall(text):
                offsets[marker].append(
                    {
                        "x": round(float(x) - min(xs), 4),
                        "y": round(float(y) - min(ys), 4),
                    }
                )
        if offsets:
            result["marker_offsets"][qualified_name] = dict(sorted(offsets.items()))
    return result


def border_ink(path):
    if not path.exists():
        return None
    try:
        image = Image.open(path).convert("RGBA")
    except Exception:
        return None
    width, height = image.size
    border = set()
    for offset in range(min(2, width, height)):
        for x in range(width):
            border.add((x, offset))
            border.add((x, height - 1 - offset))
        for y in range(height):
            border.add((offset, y))
            border.add((width - 1 - offset, y))
    return sum(
        1
        for x, y in border
        if image.getpixel((x, y))[3] > 0 and min(image.getpixel((x, y))[:3]) < 235
    )


cases = {}
mismatches = []
for source in sorted(ROOT.glob("*.puml")):
    stem = source.with_suffix("")
    java = parse_svg(Path(f"{stem}.java.svg"))
    rust = parse_svg(Path(f"{stem}.rust.svg"))
    java_valid = java["diagram_type"] is not None and not java["error_text"]
    rust_valid = (
        status(stem, "rust") == 0
        and rust["diagram_type"] is not None
        and not rust["error_text"]
    )
    rust_stderr_path = Path(f"{stem}.rust.stderr")
    rust_stderr = (
        rust_stderr_path.read_text(errors="replace") if rust_stderr_path.exists() else ""
    )
    case_mismatches = []
    if "panicked at" in rust_stderr:
        case_mismatches.append(
            {
                "kind": "rust_panic",
                "rust_status": status(stem, "rust"),
                "message": next(
                    (
                        line
                        for line in rust_stderr.splitlines()
                        if "DESCRIPTION SVEK plan contains duplicate" in line
                    ),
                    "panic",
                ),
            }
        )
    if java_valid != rust_valid:
        case_mismatches.append(
            {
                "kind": "acceptance",
                "java_valid": java_valid,
                "rust_valid": rust_valid,
            }
        )
    if rust_valid and rust["clipped_text"]:
        case_mismatches.append(
            {"kind": "svg_clipping", "text": rust["clipped_text"]}
        )
    if java_valid and rust_valid:
        if java["diagram_type"] != rust["diagram_type"]:
            case_mismatches.append(
                {
                    "kind": "diagram_type",
                    "java": java["diagram_type"],
                    "rust": rust["diagram_type"],
                }
            )
        if java["markers"] != rust["markers"]:
            case_mismatches.append(
                {
                    "kind": "visible_markers",
                    "java_only": sorted(
                        (
                            collections.Counter(java["markers"])
                            - collections.Counter(rust["markers"])
                        ).elements()
                    ),
                    "rust_only": sorted(
                        (
                            collections.Counter(rust["markers"])
                            - collections.Counter(java["markers"])
                        ).elements()
                    ),
                }
            )
        shared = set(java["marker_text"]) & set(rust["marker_text"])
        whitespace_differences = {
            marker: {
                "java": java["marker_text"][marker],
                "rust": rust["marker_text"][marker],
            }
            for marker in sorted(shared)
            if java["marker_text"][marker] != rust["marker_text"][marker]
        }
        if whitespace_differences:
            case_mismatches.append(
                {"kind": "visible_marker_text", "differences": whitespace_differences}
            )

    case = {
        "java_status": status(stem, "java"),
        "rust_status": status(stem, "rust"),
        "rust_yaml_status": status(stem, "rust-yaml"),
        "java_valid": java_valid,
        "rust_valid": rust_valid,
        "java_diagram_type": java["diagram_type"],
        "rust_diagram_type": rust["diagram_type"],
        "java_error_text": java["error_text"],
        "rust_error_text": rust["error_text"],
        "java_clipped_text": java["clipped_text"],
        "rust_clipped_text": rust["clipped_text"],
        "java_markers": java["markers"],
        "rust_markers": rust["markers"],
        "java_marker_text": java["marker_text"],
        "rust_marker_text": rust["marker_text"],
        "java_marker_offsets": java["marker_offsets"],
        "rust_marker_offsets": rust["marker_offsets"],
        "java_note_boxes": java["note_boxes"],
        "rust_note_boxes": rust["note_boxes"],
        "java_border_ink": border_ink(Path(f"{stem}.java.png")) if java_valid else None,
        "rust_border_ink": border_ink(Path(f"{stem}.rust.png")) if rust_valid else None,
        "mismatches": case_mismatches,
    }
    cases[source.stem] = case
    mismatches.extend(
        {"case": source.stem, **mismatch} for mismatch in case_mismatches
    )


metamorphic = []
for before, after, qualified_name in [
    (
        "01_component_named_zero_rows",
        "02_component_named_one_blank_row",
        "RowMemo",
    ),
    (
        "03_component_named_internal_no_trailing",
        "04_component_named_internal_with_trailing",
        "TrailingMemo",
    ),
]:
    comparison = {"before": before, "after": after, "entity": qualified_name}
    for implementation in ("java", "rust"):
        before_box = cases[before][f"{implementation}_note_boxes"].get(qualified_name)
        after_box = cases[after][f"{implementation}_note_boxes"].get(qualified_name)
        comparison[implementation] = {
            "before": before_box,
            "after": after_box,
            "height_delta": (
                round(after_box["height"] - before_box["height"], 4)
                if before_box and after_box
                else None
            ),
        }
    java_delta = comparison["java"]["height_delta"]
    rust_delta = comparison["rust"]["height_delta"]
    comparison["mismatch"] = (
        java_delta is not None
        and rust_delta is not None
        and abs(java_delta - rust_delta) > 0.5
    )
    if comparison["mismatch"]:
        mismatches.append({"case": f"{before} -> {after}", "kind": "row_cardinality"})
    metamorphic.append(comparison)

dedent_checks = []
for case_name, qualified_name in [
    ("05_component_short_space_row_dedent", "SpaceDedentMemo"),
    ("06_deployment_short_tab_row_dedent", "TabDedentMemo"),
]:
    java_offsets = cases[case_name]["java_marker_offsets"].get(qualified_name, {})
    rust_offsets = cases[case_name]["rust_marker_offsets"].get(qualified_name, {})
    common_markers = sorted(set(java_offsets) & set(rust_offsets))
    check = {
        "case": case_name,
        "entity": qualified_name,
        "java": java_offsets,
        "rust": rust_offsets,
        "mismatch": any(
            abs(java_offsets[marker][0]["x"] - rust_offsets[marker][0]["x"]) > 1.0
            for marker in common_markers
        ),
    }
    if check["mismatch"]:
        mismatches.append({"case": case_name, "kind": "common_dedent"})
    dedent_checks.append(check)

summary = {
    "case_count": len(cases),
    "java_valid_count": sum(case["java_valid"] for case in cases.values()),
    "rust_valid_count": sum(case["rust_valid"] for case in cases.values()),
    "mismatch_count": len(mismatches),
    "verdict": "REJECT" if mismatches else "ACCEPT",
    "mismatches": mismatches,
    "metamorphic": metamorphic,
    "dedent_checks": dedent_checks,
    "cases": cases,
}
json.dump(summary, sys.stdout, indent=2, ensure_ascii=False, sort_keys=True)
sys.stdout.write("\n")
sys.exit(1 if mismatches else 0)
