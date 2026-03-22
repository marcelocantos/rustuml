#!/usr/bin/env python3
"""
Generator for multi-diagram PlantUML test cases.
Generates .puml files containing multiple @startuml/@enduml blocks,
then renders the first block of each via the PlantUML server to produce
golden .svg files.

Usage:
    python3 gen_multi_diagram.py [--url http://host:port]
"""

import json
import os
import sys
import urllib.request
import urllib.error
import zlib
import concurrent.futures
from pathlib import Path

PLANTUML_URL = os.environ.get("PLANTUML_URL", "http://127.0.0.1:8787")

OUTPUT_DIR = Path(__file__).parent / "multi-diagram"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# ---------------------------------------------------------------------------
# PlantUML encoding (for GET-based rendering)
# ---------------------------------------------------------------------------

PLANTUML_ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_"


def _encode6bit(b):
    return PLANTUML_ALPHABET[b & 0x3F]


def _encode3bytes(b1, b2, b3):
    c1 = b1 >> 2
    c2 = ((b1 & 0x3) << 4) | (b2 >> 4)
    c3 = ((b2 & 0xF) << 2) | (b3 >> 6)
    c4 = b3 & 0x3F
    return _encode6bit(c1) + _encode6bit(c2) + _encode6bit(c3) + _encode6bit(c4)


def plantuml_encode(text: str) -> str:
    data = zlib.compress(text.encode("utf-8"))[2:-4]
    result = ""
    for i in range(0, len(data), 3):
        if i + 2 < len(data):
            result += _encode3bytes(data[i], data[i + 1], data[i + 2])
        elif i + 1 < len(data):
            result += _encode3bytes(data[i], data[i + 1], 0)
        else:
            result += _encode3bytes(data[i], 0, 0)
    return result


def render_first_block(source: str) -> bytes:
    """Render the first block via PlantUML server GET endpoint."""
    encoded = plantuml_encode(source)
    url = f"{PLANTUML_URL}/svg/{encoded}"
    req = urllib.request.Request(url)
    with urllib.request.urlopen(req, timeout=60) as resp:
        return resp.read()


# ---------------------------------------------------------------------------
# Diagram snippet helpers
# ---------------------------------------------------------------------------

SEQ1 = """\
Alice -> Bob : Hello
Bob --> Alice : Hi there
"""

SEQ2 = """\
participant Client
participant Server
Client -> Server : GET /api/data
Server --> Client : 200 OK
"""

SEQ3 = """\
Alice -> Bob : Request
Bob -> Carol : Forward
Carol --> Bob : Response
Bob --> Alice : Reply
"""

CLASS1 = """\
class Animal {
  +String name
  +void speak()
}
class Dog {
  +void fetch()
}
Animal <|-- Dog
"""

CLASS2 = """\
class Shape {
  +double area()
}
class Circle {
  +double radius
}
class Rectangle {
  +double width
  +double height
}
Shape <|-- Circle
Shape <|-- Rectangle
"""

STATE1 = """\
[*] --> Idle
Idle --> Running : start
Running --> Idle : stop
Running --> [*] : finish
"""

STATE2 = """\
[*] --> Locked
Locked --> Unlocked : correct_pin
Unlocked --> Locked : lock
Unlocked --> [*] : open
"""

ACTIVITY1 = """\
start
:Step A;
if (condition?) then (yes)
  :Step B;
else (no)
  :Step C;
endif
stop
"""

ACTIVITY2 = """\
start
repeat
  :Process item;
repeat while (more items?) is (yes) not (no)
stop
"""

COMPONENT1 = """\
component Frontend
component Backend
component Database
Frontend --> Backend : REST API
Backend --> Database : SQL
"""

USECASE1 = """\
actor User
actor Admin
usecase Login
usecase ManageUsers
User --> Login
Admin --> Login
Admin --> ManageUsers
"""

DEPLOYMENT1 = """\
node WebServer {
  artifact app.war
}
node Database {
  artifact db.sql
}
WebServer --> Database : JDBC
"""

MINDMAP1 = """\
* Root
** Branch A
*** Leaf 1
*** Leaf 2
** Branch B
*** Leaf 3
"""

GANTT1 = """\
title Project Timeline
[Task A] lasts 7 days
[Task B] starts at [Task A]'s end
[Task B] lasts 5 days
[Task C] starts at [Task B]'s end
[Task C] lasts 10 days
"""

WBS1 = """\
* Project
** Planning
*** Requirements
*** Design
** Development
*** Frontend
*** Backend
** Testing
"""


def make_block(body: str, name: str = "", diagram_type: str = "uml") -> str:
    tag = f"@start{diagram_type}"
    if name:
        tag += f" {name}"
    return f"{tag}\n{body.strip()}\n@end{diagram_type}\n"


# ---------------------------------------------------------------------------
# Test case registry
# ---------------------------------------------------------------------------

cases: list[tuple[str, str]] = []  # (filename, content)


def add(filename: str, content: str):
    cases.append((filename, content))


# ---------------------------------------------------------------------------
# Section 1: Basic multi-block (10 cases)
# ---------------------------------------------------------------------------

# 1a: 2 sequence diagrams
add("basic_01_two_sequence.puml",
    make_block(SEQ1) + "\n" + make_block(SEQ2))

# 1b: 2 class diagrams
add("basic_02_two_class.puml",
    make_block(CLASS1) + "\n" + make_block(CLASS2))

# 1c: 3 blocks (sequence)
add("basic_03_three_blocks.puml",
    make_block(SEQ1) + "\n" + make_block(SEQ2) + "\n" + make_block(SEQ3))

# 1d: 5 blocks (alternating seq/class)
content = ""
for i in range(5):
    body = SEQ1 if i % 2 == 0 else CLASS1
    content += make_block(body) + "\n"
add("basic_04_five_blocks.puml", content)

# 1e: 10 blocks (all sequence)
content = ""
seqs = [SEQ1, SEQ2, SEQ3]
for i in range(10):
    content += make_block(seqs[i % len(seqs)]) + "\n"
add("basic_05_ten_blocks.puml", content)

# 1f: 2 state diagrams
add("basic_06_two_state.puml",
    make_block(STATE1) + "\n" + make_block(STATE2))

# 1g: 2 activity diagrams
add("basic_07_two_activity.puml",
    make_block(ACTIVITY1) + "\n" + make_block(ACTIVITY2))

# 1h: Same diagram repeated twice
add("basic_08_repeated_diagram.puml",
    make_block(CLASS1) + "\n" + make_block(CLASS1))

# 1i: 2 component diagrams
add("basic_09_two_component.puml",
    make_block(COMPONENT1) + "\n" + make_block(COMPONENT1.replace("Frontend", "Mobile")))

# 1j: 2 usecase diagrams
add("basic_10_two_usecase.puml",
    make_block(USECASE1) + "\n" + make_block(USECASE1.replace("Admin", "Manager")))

# ---------------------------------------------------------------------------
# Section 2: Mixed diagram types (15 cases)
# ---------------------------------------------------------------------------

DIAGRAM_SNIPPETS = {
    "sequence": SEQ1,
    "class": CLASS1,
    "state": STATE1,
    "activity": ACTIVITY1,
    "component": COMPONENT1,
    "usecase": USECASE1,
    "deployment": DEPLOYMENT1,
}

TYPES = list(DIAGRAM_SNIPPETS.keys())

# All pairs of different types
pair_count = 0
for i, t1 in enumerate(TYPES):
    for t2 in TYPES[i + 1:]:
        pair_count += 1
        if pair_count > 15:
            break
        add(f"mixed_{pair_count:02d}_{t1}_{t2}.puml",
            make_block(DIAGRAM_SNIPPETS[t1]) + "\n" + make_block(DIAGRAM_SNIPPETS[t2]))
    if pair_count >= 15:
        break

# Ensure we fill up to 15 if pairs ran short
extra_mixed = [
    ("sequence", "class", "state"),
    ("activity", "component", "usecase"),
]
for idx, combo in enumerate(extra_mixed):
    if pair_count >= 15:
        break
    pair_count += 1
    content = ""
    for t in combo:
        content += make_block(DIAGRAM_SNIPPETS[t]) + "\n"
    add(f"mixed_{pair_count:02d}_triple_{'_'.join(combo)}.puml", content)

# ---------------------------------------------------------------------------
# Section 3: Named blocks (10 cases)
# ---------------------------------------------------------------------------

# 3a: Two blocks with simple names
add("named_01_simple_names.puml",
    make_block(SEQ1, name="diagram1") + "\n" + make_block(CLASS1, name="diagram2"))

# 3b: Names with underscores
add("named_02_underscore_names.puml",
    make_block(SEQ1, name="first_diagram") + "\n" + make_block(SEQ2, name="second_diagram"))

# 3c: Names with hyphens
add("named_03_hyphen_names.puml",
    make_block(CLASS1, name="class-overview") + "\n" + make_block(CLASS2, name="class-detail"))

# 3d: Names with numbers
add("named_04_numeric_names.puml",
    make_block(SEQ1, name="seq001") + "\n" + make_block(SEQ2, name="seq002") + "\n" + make_block(SEQ3, name="seq003"))

# 3e: Same name repeated (should produce same diagram twice)
add("named_05_same_name_repeated.puml",
    make_block(SEQ1, name="login_flow") + "\n" + make_block(SEQ1, name="login_flow"))

# 3f: Mixed named and unnamed blocks
add("named_06_mixed_named_unnamed.puml",
    make_block(SEQ1) + "\n" + make_block(CLASS1, name="classes") + "\n" + make_block(STATE1))

# 3g: Names with mixed case
add("named_07_mixed_case_names.puml",
    make_block(SEQ1, name="MyFirstDiagram") + "\n" + make_block(CLASS1, name="MySecondDiagram"))

# 3h: Long descriptive names
add("named_08_long_names.puml",
    make_block(SEQ1, name="user_authentication_sequence") + "\n"
    + make_block(CLASS1, name="domain_model_overview"))

# 3i: Names with numbers and underscores
add("named_09_alphanum_names.puml",
    make_block(SEQ1, name="seq_v2_final") + "\n" + make_block(CLASS1, name="cls_v1_draft"))

# 3j: Three named blocks of different types
add("named_10_three_named.puml",
    make_block(SEQ1, name="flow") + "\n"
    + make_block(CLASS1, name="model") + "\n"
    + make_block(STATE1, name="states"))

# ---------------------------------------------------------------------------
# Section 4: Shared preprocessing (10 cases)
# ---------------------------------------------------------------------------

# 4a: !define before first block, used in both
add("preproc_01_define_both_blocks.puml",
    "!define ALICE Alice\n"
    "!define BOB Bob\n"
    "\n"
    "@startuml\n"
    "ALICE -> BOB : Hello\n"
    "BOB --> ALICE : Hi\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "ALICE -> BOB : Second message\n"
    "BOB --> ALICE : Second reply\n"
    "@enduml\n")

# 4b: !define with complex substitution
add("preproc_02_define_complex.puml",
    "!define SERVER_URL http://api.example.com\n"
    "!define API_VERSION v2\n"
    "\n"
    "@startuml\n"
    "note : Server: SERVER_URL/API_VERSION\n"
    "Alice -> Bob : Request\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "note : Also SERVER_URL/API_VERSION\n"
    "Carol -> Dave : Another request\n"
    "@enduml\n")

# 4c: Variables defined in one block should NOT leak to next
add("preproc_03_variable_isolation.puml",
    "@startuml\n"
    "!$color = \"red\"\n"
    "skinparam backgroundColor $color\n"
    "Alice -> Bob : In red context\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "' $color should not be defined here\n"
    "Alice -> Bob : In default context\n"
    "@enduml\n")

# 4d: !define used in named blocks
add("preproc_04_define_named_blocks.puml",
    "!define TITLE My Diagram Title\n"
    "\n"
    "@startuml first\n"
    "title TITLE (First)\n"
    "Alice -> Bob : Hello\n"
    "@enduml\n"
    "\n"
    "@startuml second\n"
    "title TITLE (Second)\n"
    "Carol -> Dave : Hi\n"
    "@enduml\n")

# 4e: !procedure used as a header macro (multiline-like behaviour)
add("preproc_05_procedure_header.puml",
    "!procedure $stdheader()\n"
    "  title Standard Header\n"
    "  header Generated by RustUML\n"
    "!endprocedure\n"
    "\n"
    "@startuml\n"
    "$stdheader()\n"
    "Alice -> Bob : Hello\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "$stdheader()\n"
    "Carol -> Dave : Hi\n"
    "@enduml\n")

# 4f: !define with arguments
add("preproc_06_define_with_args.puml",
    "!define ARROW(from, to) from -> to : message\n"
    "\n"
    "@startuml\n"
    "ARROW(Alice, Bob)\n"
    "ARROW(Bob, Carol)\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "ARROW(Dave, Eve)\n"
    "ARROW(Eve, Frank)\n"
    "@enduml\n")

# 4g: !if / !else preprocessor conditionals
add("preproc_07_conditional.puml",
    "!$DEBUG = true\n"
    "\n"
    "@startuml\n"
    "!if $DEBUG\n"
    "note left : DEBUG mode\n"
    "!endif\n"
    "Alice -> Bob : Hello\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "!if $DEBUG\n"
    "note right : Also debug\n"
    "!endif\n"
    "Carol -> Dave : Hi\n"
    "@enduml\n")

# 4h: Local variable in first block, same name in second
add("preproc_08_same_var_name.puml",
    "@startuml\n"
    "!$x = 1\n"
    "note : x = $x\n"
    "Alice -> Bob : Message $x\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "!$x = 2\n"
    "note : x = $x\n"
    "Carol -> Dave : Message $x\n"
    "@enduml\n")

# 4i: !procedure defined before blocks, used in both
add("preproc_09_procedure.puml",
    "!procedure $stdflow($from, $to)\n"
    "  $from -> $to : request\n"
    "  $to --> $from : response\n"
    "!endprocedure\n"
    "\n"
    "@startuml\n"
    "$stdflow(\"Alice\", \"Bob\")\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "$stdflow(\"Carol\", \"Dave\")\n"
    "@enduml\n")

# 4j: Multiple defines, some used in first block only, some in both
add("preproc_10_selective_defines.puml",
    "!define SHARED shared_value\n"
    "!define FIRST_ONLY first_only_value\n"
    "\n"
    "@startuml\n"
    "note : SHARED and FIRST_ONLY\n"
    "Alice -> Bob : Uses SHARED\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "note : Only SHARED here\n"
    "Carol -> Dave : Uses SHARED\n"
    "@enduml\n")

# ---------------------------------------------------------------------------
# Section 5: Edge cases (10 cases)
# ---------------------------------------------------------------------------

# 5a: Empty block followed by non-empty
add("edge_01_empty_then_content.puml",
    "@startuml\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "Alice -> Bob : After empty block\n"
    "@enduml\n")

# 5b: Non-empty then empty block
add("edge_02_content_then_empty.puml",
    "@startuml\n"
    "Alice -> Bob : Before empty block\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "@enduml\n")

# 5c: Block with only comments
add("edge_03_comment_only_block.puml",
    "@startuml\n"
    "' This is just a comment\n"
    "' No actual diagram content\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "Alice -> Bob : After comment-only block\n"
    "@enduml\n")

# 5d: Lots of whitespace between blocks
add("edge_04_whitespace_between.puml",
    "@startuml\n"
    "Alice -> Bob : First\n"
    "@enduml\n"
    "\n\n\n\n\n"
    "@startuml\n"
    "Carol -> Dave : Second\n"
    "@enduml\n"
    "\n\n\n"
    "@startuml\n"
    "Eve -> Frank : Third\n"
    "@enduml\n")

# 5e: Very short single-line diagrams
add("edge_05_single_line_diagrams.puml",
    "@startuml\n"
    "A -> B\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "C -> D\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "E -> F\n"
    "@enduml\n")

# 5f: Mixed @startXXX types (mindmap + sequence + gantt)
add("edge_06_mixed_start_types.puml",
    "@startmindmap\n"
    + MINDMAP1.strip() + "\n"
    "@endmindmap\n"
    "\n"
    "@startuml\n"
    + SEQ1.strip() + "\n"
    "@enduml\n"
    "\n"
    "@startgantt\n"
    + GANTT1.strip() + "\n"
    "@endgantt\n")

# 5g: WBS + sequence + class
add("edge_07_wbs_sequence_class.puml",
    "@startwbs\n"
    + WBS1.strip() + "\n"
    "@endwbs\n"
    "\n"
    "@startuml\n"
    + SEQ1.strip() + "\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    + CLASS1.strip() + "\n"
    "@enduml\n")

# 5h: Block with only a title
add("edge_08_title_only_block.puml",
    "@startuml\n"
    "title My Empty Diagram\n"
    "@enduml\n"
    "\n"
    "@startuml\n"
    "title Second Diagram\n"
    "Alice -> Bob : Hello\n"
    "@enduml\n")

# 5i: Mindmap + mindmap (same type twice, non-uml)
add("edge_09_two_mindmaps.puml",
    "@startmindmap\n"
    "* Root A\n"
    "** Child A1\n"
    "** Child A2\n"
    "@endmindmap\n"
    "\n"
    "@startmindmap\n"
    "* Root B\n"
    "** Child B1\n"
    "** Child B2\n"
    "@endmindmap\n")

# 5j: Gantt + WBS + mindmap (all non-uml types)
add("edge_10_all_nonuml_types.puml",
    "@startgantt\n"
    + GANTT1.strip() + "\n"
    "@endgantt\n"
    "\n"
    "@startwbs\n"
    + WBS1.strip() + "\n"
    "@endwbs\n"
    "\n"
    "@startmindmap\n"
    + MINDMAP1.strip() + "\n"
    "@endmindmap\n")

# ---------------------------------------------------------------------------
# Write .puml files and render SVGs
# ---------------------------------------------------------------------------


def render_and_save(filename: str, content: str) -> tuple[str, str, str]:
    """Write .puml, render via server, save .svg. Returns (name, status, detail)."""
    puml_path = OUTPUT_DIR / filename
    svg_path = puml_path.with_suffix(".svg")

    puml_path.write_text(content, encoding="utf-8")

    # Skip if SVG is already up-to-date
    if svg_path.exists() and svg_path.stat().st_mtime > puml_path.stat().st_mtime:
        return (filename, "SKIP", "")

    # Extract first block to render, including any preamble (defines, procedures)
    # before the first @start tag.
    lines = content.splitlines()
    preamble_lines = []
    first_block_lines = []
    in_block = False
    end_tag = None
    for line in lines:
        stripped = line.strip()
        if not in_block and stripped.lower().startswith("@start"):
            in_block = True
            # Determine the end tag
            word = stripped.split()[0].lower()  # e.g. @startmindmap
            end_tag = "@end" + word[len("@start"):]
            first_block_lines.append(line)
            continue
        if in_block:
            first_block_lines.append(line)
            if stripped.lower() == end_tag:
                break
        else:
            # Preamble: !define, !procedure, etc. before first @start
            preamble_lines.append(line)

    if not first_block_lines:
        # Fallback: use whole content
        first_block_source = content
    else:
        # Prepend preamble so defines/procedures are available
        combined = preamble_lines + first_block_lines
        first_block_source = "\n".join(combined)

    try:
        svg_bytes = render_first_block(first_block_source)
    except urllib.error.HTTPError as e:
        return (filename, "FAIL", f"HTTP {e.code}: {e.reason}")
    except Exception as e:
        return (filename, "FAIL", str(e))

    if not svg_bytes:
        return (filename, "FAIL", "empty response")

    svg_path.write_bytes(svg_bytes)

    svg_text = svg_bytes.decode("utf-8", errors="replace")
    if "Syntax Error" in svg_text or "syntax error" in svg_text.lower():
        return (filename, "WARN", "syntax error in SVG output")

    return (filename, "OK", "")


def main():
    print(f"Generating {len(cases)} multi-diagram test cases")
    print(f"Output: {OUTPUT_DIR}")
    print(f"Server: {PLANTUML_URL}")
    print()

    ok = warn = fail = skip = 0

    with concurrent.futures.ThreadPoolExecutor(max_workers=10) as pool:
        futures = {pool.submit(render_and_save, fname, content): fname
                   for fname, content in cases}
        for future in concurrent.futures.as_completed(futures):
            fname, status, detail = future.result()
            if status == "OK":
                ok += 1
            elif status == "WARN":
                warn += 1
                print(f"WARN  {fname}: {detail}")
            elif status == "FAIL":
                fail += 1
                print(f"FAIL  {fname}: {detail}")
            elif status == "SKIP":
                skip += 1

    total_puml = len(cases)
    total_svg = ok + warn + skip

    print()
    print("=== Results ===")
    print(f"Total .puml files: {total_puml}")
    print(f"SVG rendered OK:   {ok}")
    print(f"SVG warnings:      {warn}")
    print(f"SVG failed:        {fail}")
    print(f"SVG skipped:       {skip}")
    print(f"Pairs generated:   {total_svg} .puml + .svg pairs")

    if fail > 0:
        sys.exit(1)


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="Generate multi-diagram golden test cases")
    parser.add_argument("--url", default=None, help="PlantUML server URL")
    args = parser.parse_args()
    if args.url:
        PLANTUML_URL = args.url
    main()
