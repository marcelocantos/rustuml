#!/usr/bin/env python3
"""
Generator for PlantUML golden test cases covering four diagram types with
previously zero golden coverage:

  - DOT        (@startdot/@enddot)   — Graphviz DOT passthrough
  - EBNF       (@startebnf/@endebnf) — Extended Backus-Naur Form railroad diagrams
  - git        (@startgit/@endgit)   — Git branch/commit diagrams
  - chart      (@startboard)         — Chart-like diagrams (board/kanban)

PlantUML version compatibility notes (1.2026.3beta6):
  - DOT:  Feature has been suppressed (GitHub issue #2495). .puml files are
          saved as parser test inputs; no .svg is generated.
  - git:  @startgit renders but produces a 21x21 pixel placeholder SVG —
          the feature is not fully implemented in this build. .puml files
          are saved; SVGs are saved as-is so the oracle suite sees what
          this version actually outputs.
  - chart: @startpie and <chart> are not supported in this version.
           @startboard (kanban/board diagrams) is used instead, as it is
           a chart-like visualization that this version fully supports.
  - EBNF: Fully supported; all 25 cases produce real SVG output.

Usage:
    python3 gen_missing_types.py [--url http://host:port]

Output directories (relative to this script):
    dot/    ebnf/    git/    chart/
"""

import json
import os
import sys
import urllib.request
import urllib.error

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

BASE = os.path.dirname(os.path.abspath(__file__))
PLANTUML_URL = os.environ.get("PLANTUML_URL", "http://127.0.0.1:8787")


# ---------------------------------------------------------------------------
# Rendering helpers
# ---------------------------------------------------------------------------

def render(source: str) -> str | None:
    """
    Render PlantUML source via the picoweb /render endpoint.

    Returns the SVG string if the response contains a real, non-error SVG,
    or None if rendering failed or produced an error/placeholder diagram.
    """
    body = json.dumps({"source": source, "options": ["-tsvg"]}).encode()
    req = urllib.request.Request(
        f"{PLANTUML_URL}/render",
        body,
        {"Content-Type": "application/json"},
    )
    try:
        resp = urllib.request.urlopen(req, timeout=30).read().decode("utf-8")
    except Exception:
        return None

    if "<svg" not in resp:
        return None

    # Detect well-known error/placeholder responses from this PlantUML build:
    #   - Black background (#000000) → version info / syntax error page
    #   - "Diagram not supported" → feature not in this build
    #   - "This feature has been suppressed" → deliberately disabled (DOT)
    #   - 21x21 pixel SVG → git placeholder (feature stub)
    if "background:#000000" in resp[:600]:
        return None
    if "Diagram not supported" in resp:
        return None
    if "This feature has been suppressed" in resp:
        return None
    if "width:21px;height:21px" in resp[:400]:
        return None

    return resp


def render_raw(source: str) -> str | None:
    """
    Like render(), but returns any SVG — including placeholders — so we can
    save what the oracle actually produces for types with partial support.
    """
    body = json.dumps({"source": source, "options": ["-tsvg"]}).encode()
    req = urllib.request.Request(
        f"{PLANTUML_URL}/render",
        body,
        {"Content-Type": "application/json"},
    )
    try:
        resp = urllib.request.urlopen(req, timeout=30).read().decode("utf-8")
    except Exception:
        return None

    if "<svg" not in resp:
        return None
    return resp


# ---------------------------------------------------------------------------
# File I/O
# ---------------------------------------------------------------------------

def write(subdir: str, filename: str, content: str) -> str:
    """Write content to subdir/filename, return full path."""
    os.makedirs(os.path.join(BASE, subdir), exist_ok=True)
    path = os.path.join(BASE, subdir, filename)
    with open(path, "w", encoding="utf-8") as f:
        f.write(content.strip() + "\n")
    return path


def write_svg(subdir: str, stem: str, svg: str) -> None:
    """Write SVG content alongside the .puml file."""
    path = os.path.join(BASE, subdir, stem + ".svg")
    with open(path, "w", encoding="utf-8") as f:
        f.write(svg)


# ---------------------------------------------------------------------------
# DOT diagrams — @startdot / @enddot
#
# NOTE: This feature has been suppressed in PlantUML 1.2026.3beta6 (see
# https://github.com/plantuml/plantuml/issues/2495). The .puml files are
# still useful as parser/preprocessor test inputs for RustUML. No SVG is
# generated because the oracle produces an error SVG, not a real diagram.
# ---------------------------------------------------------------------------

DOT_CASES = [
    # --- Simple directed graphs ---
    ("dot_simple_2nodes", """\
@startdot
digraph G {
  A -> B
}
@enddot
"""),
    ("dot_simple_3nodes_chain", """\
@startdot
digraph G {
  A -> B -> C
}
@enddot
"""),
    ("dot_simple_4nodes_star", """\
@startdot
digraph G {
  Root -> A
  Root -> B
  Root -> C
}
@enddot
"""),
    ("dot_simple_5nodes_diamond", """\
@startdot
digraph G {
  A -> B
  A -> C
  B -> D
  C -> D
  D -> E
}
@enddot
"""),
    ("dot_simple_cycle", """\
@startdot
digraph G {
  A -> B -> C -> A
}
@enddot
"""),
    # --- Undirected graphs ---
    ("dot_undirected_simple", """\
@startdot
graph G {
  A -- B
  B -- C
  C -- A
}
@enddot
"""),
    ("dot_undirected_path", """\
@startdot
graph G {
  node1 -- node2 -- node3 -- node4
}
@enddot
"""),
    # --- Subgraphs / clusters ---
    ("dot_cluster_single", """\
@startdot
digraph G {
  subgraph cluster_0 {
    label = "Cluster A"
    a1 -> a2
  }
  a2 -> b1
}
@enddot
"""),
    ("dot_cluster_two", """\
@startdot
digraph G {
  subgraph cluster_web {
    label = "Web Tier"
    nginx -> app
  }
  subgraph cluster_db {
    label = "DB Tier"
    primary -> replica
  }
  app -> primary
}
@enddot
"""),
    ("dot_subgraph_no_cluster", """\
@startdot
digraph G {
  subgraph sub {
    x -> y
  }
  y -> z
}
@enddot
"""),
    # --- Node attributes ---
    ("dot_node_shapes", """\
@startdot
digraph G {
  A [shape=box]
  B [shape=ellipse]
  C [shape=diamond]
  A -> B -> C
}
@enddot
"""),
    ("dot_node_colors", """\
@startdot
digraph G {
  A [style=filled, fillcolor=lightblue]
  B [style=filled, fillcolor=lightyellow]
  C [style=filled, fillcolor=lightgreen]
  A -> B -> C
}
@enddot
"""),
    ("dot_node_labels", """\
@startdot
digraph G {
  n1 [label="Start"]
  n2 [label="Process\nData"]
  n3 [label="End"]
  n1 -> n2 -> n3
}
@enddot
"""),
    ("dot_node_url", """\
@startdot
digraph G {
  A [URL="http://example.com", label="Click me"]
  B [tooltip="Hover text"]
  A -> B
}
@enddot
"""),
    # --- Edge attributes ---
    ("dot_edge_styles", """\
@startdot
digraph G {
  A -> B [style=dashed]
  B -> C [style=dotted]
  C -> D [style=bold]
}
@enddot
"""),
    ("dot_edge_labels", """\
@startdot
digraph G {
  A -> B [label="forward"]
  B -> A [label="backward", style=dashed]
}
@enddot
"""),
    ("dot_edge_colors", """\
@startdot
digraph G {
  A -> B [color=red]
  A -> C [color=blue]
  A -> D [color=green]
}
@enddot
"""),
    ("dot_edge_arrowhead", """\
@startdot
digraph G {
  A -> B [arrowhead=open]
  B -> C [arrowhead=diamond]
  C -> D [arrowhead=none]
}
@enddot
"""),
    # --- Record nodes ---
    ("dot_record_simple", """\
@startdot
digraph G {
  A [shape=record, label="{name|age|email}"]
}
@enddot
"""),
    ("dot_record_ports", """\
@startdot
digraph G {
  struct1 [shape=record, label="<f0> left|<f1> middle|<f2> right"]
  struct2 [shape=record, label="one|two"]
  struct1:f2 -> struct2
}
@enddot
"""),
    # --- Layout direction ---
    ("dot_rankdir_lr", """\
@startdot
digraph G {
  rankdir=LR
  A -> B -> C -> D
}
@enddot
"""),
    ("dot_rankdir_tb", """\
@startdot
digraph G {
  rankdir=TB
  A -> B -> C -> D
}
@enddot
"""),
    # --- Weighted edges ---
    ("dot_weighted_edges", """\
@startdot
digraph G {
  A -> B [weight=5, label="5"]
  A -> C [weight=1, label="1"]
  A -> D [weight=10, label="10"]
}
@enddot
"""),
    # --- Multiple edges between same nodes ---
    ("dot_multi_edge", """\
@startdot
digraph G {
  A -> B [label="call"]
  A -> B [label="return", style=dashed, dir=back]
  B -> C
}
@enddot
"""),
    ("dot_graph_attributes", """\
@startdot
digraph G {
  graph [label="My Graph", fontsize=20, bgcolor=lightyellow]
  node  [shape=circle, style=filled, fillcolor=white]
  edge  [color=gray]
  A -> B -> C
  A -> C
}
@enddot
"""),
]


def gen_dot(stats: dict) -> None:
    """Generate DOT .puml files. No SVG — feature suppressed in this build."""
    subdir = "dot"
    puml_count = 0
    svg_count = 0

    for stem, source in DOT_CASES:
        write(subdir, stem + ".puml", source)
        puml_count += 1

        # Attempt to render so the suppression notice is documented as-is.
        # We do NOT save the error SVG — it's not a real oracle output.
        svg = render(source)
        if svg:
            write_svg(subdir, stem, svg)
            svg_count += 1

    stats["dot"] = {
        "puml": puml_count,
        "svg": svg_count,
        "note": (
            "DOT passthrough is suppressed in this PlantUML build "
            "(https://github.com/plantuml/plantuml/issues/2495). "
            ".puml files saved as parser test inputs; no SVG generated."
        ),
    }


# ---------------------------------------------------------------------------
# EBNF diagrams — @startebnf / @endebnf
# ---------------------------------------------------------------------------

EBNF_CASES = [
    # --- Simple rules ---
    ("ebnf_digit", """\
@startebnf
digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
@endebnf
"""),
    ("ebnf_letter", """\
@startebnf
letter = "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j"
       | "k" | "l" | "m" | "n" | "o" | "p" | "q" | "r" | "s" | "t"
       | "u" | "v" | "w" | "x" | "y" | "z";
@endebnf
"""),
    ("ebnf_identifier", """\
@startebnf
identifier = letter , { letter | digit | "_" };
letter = "a" | "b" | "c";
digit = "0" | "1" | "2";
@endebnf
"""),
    # --- Alternatives ---
    ("ebnf_alternatives_simple", """\
@startebnf
boolean = "true" | "false";
@endebnf
"""),
    ("ebnf_alternatives_nested", """\
@startebnf
operator = add_op | mul_op;
add_op = "+" | "-";
mul_op = "*" | "/";
@endebnf
"""),
    # --- Repetition ---
    ("ebnf_repetition_items", """\
@startebnf
list = item , { "," , item };
item = digit;
digit = "0" | "1" | "2" | "3" | "4";
@endebnf
"""),
    ("ebnf_repetition_statements", """\
@startebnf
block = "{" , { statement } , "}";
statement = identifier , ";" ;
identifier = letter;
letter = "a" | "b" | "c";
@endebnf
"""),
    # --- Optional ---
    ("ebnf_optional_sign", """\
@startebnf
integer = [ sign ] , digit , { digit };
sign = "+" | "-";
digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
@endebnf
"""),
    ("ebnf_optional_modifier", """\
@startebnf
declaration = [ "const" ] , type , identifier;
type = "int" | "float" | "string";
identifier = letter;
letter = "a" | "b" | "c";
@endebnf
"""),
    # --- Grouping ---
    ("ebnf_grouping_simple", """\
@startebnf
factor = ( "+" | "-" ) , primary;
primary = digit;
digit = "0" | "1" | "2";
@endebnf
"""),
    ("ebnf_grouping_complex", """\
@startebnf
term = factor , { ( "*" | "/" ) , factor };
factor = digit | "(" , expr , ")";
expr = term;
digit = "0" | "1";
@endebnf
"""),
    # --- Terminal vs non-terminal ---
    ("ebnf_terminal_nonterminal", """\
@startebnf
sentence = noun_phrase , verb_phrase;
noun_phrase = [ "the" ] , noun;
verb_phrase = verb , [ noun_phrase ];
noun = "cat" | "dog" | "bird";
verb = "sees" | "chases";
@endebnf
"""),
    # --- Arithmetic expressions ---
    ("ebnf_arithmetic", """\
@startebnf
expression = term , { ( "+" | "-" ) , term };
term = factor , { ( "*" | "/" ) , factor };
factor = number | "(" , expression , ")";
number = digit , { digit };
digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
@endebnf
"""),
    # --- JSON grammar ---
    ("ebnf_json_value", """\
@startebnf
value = string | number | object | array | "true" | "false" | "null";
object = "{" , [ member , { "," , member } ] , "}";
member = string , ":" , value;
array = "[" , [ value , { "," , value } ] , "]";
string = '"' , { char } , '"';
number = digit , { digit };
char = letter | digit;
letter = "a" | "b" | "c";
digit = "0" | "1" | "2";
@endebnf
"""),
    # --- URL grammar ---
    ("ebnf_url", """\
@startebnf
url = scheme , "://" , host , [ ":" , port ] , [ path ];
scheme = "http" | "https" | "ftp";
host = label , { "." , label };
label = letter , { letter | digit };
port = digit , { digit };
path = "/" , { segment , "/" };
segment = { letter | digit };
letter = "a" | "b" | "c";
digit = "0" | "1" | "2";
@endebnf
"""),
    # --- Simple programming language ---
    ("ebnf_simple_language", """\
@startebnf
program = { statement };
statement = if_stmt | while_stmt | assign_stmt;
if_stmt = "if" , condition , "then" , statement , [ "else" , statement ];
while_stmt = "while" , condition , "do" , statement;
assign_stmt = identifier , ":=" , expression;
condition = expression , rel_op , expression;
rel_op = "=" | "<" | ">" | "<=" | ">=";
expression = identifier | number;
identifier = letter;
number = digit;
letter = "a" | "b" | "c";
digit = "0" | "1" | "2";
@endebnf
"""),
    # --- SQL SELECT ---
    ("ebnf_sql_select", """\
@startebnf
select_stmt = "SELECT" , column_list , "FROM" , table_name , [ where_clause ];
column_list = "*" | ( column , { "," , column } );
where_clause = "WHERE" , condition;
condition = column , operator , value;
operator = "=" | "<>" | "<" | ">";
column = identifier;
table_name = identifier;
value = identifier | number;
identifier = letter , { letter | digit };
number = digit , { digit };
letter = "a" | "b" | "c";
digit = "0" | "1" | "2";
@endebnf
"""),
    # --- Email address ---
    ("ebnf_email", """\
@startebnf
email = local , "@" , domain;
local = word , { "." , word };
domain = label , { "." , label };
word = char , { char };
label = char , { char };
char = letter | digit;
letter = "a" | "b" | "c";
digit = "0" | "1" | "2";
@endebnf
"""),
    # --- IPv4 address ---
    ("ebnf_ipv4", """\
@startebnf
ipv4 = octet , "." , octet , "." , octet , "." , octet;
octet = digit | ( non_zero , digit ) | ( "1" , digit , digit )
      | ( "2" , four_to_nine , digit ) | ( "25" , zero_to_five );
non_zero = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
four_to_nine = "4" | "5" | "6" | "7" | "8" | "9";
zero_to_five = "0" | "1" | "2" | "3" | "4" | "5";
digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
@endebnf
"""),
    # --- CSS property ---
    ("ebnf_css_property", """\
@startebnf
declaration = property , ":" , value , [ "!" , "important" ] , ";";
property = ident;
value = keyword | number , [ unit ];
keyword = "auto" | "inherit" | "none";
unit = "px" | "em" | "rem" | "%" | "vh" | "vw";
number = digit , { digit } , [ "." , digit , { digit } ];
ident = letter , { letter | digit | "-" };
letter = "a" | "b" | "c";
digit = "0" | "1" | "2";
@endebnf
"""),
    # --- Markdown heading ---
    ("ebnf_markdown_heading", """\
@startebnf
heading = hashes , " " , text , newline;
hashes = "#" | "##" | "###" | "####" | "#####" | "######";
text = char , { char };
newline = ? newline character ?;
char = ? any character except newline ?;
@endebnf
"""),
    # --- Date format ---
    ("ebnf_date", """\
@startebnf
date = year , "-" , month , "-" , day;
year = digit , digit , digit , digit;
month = ( "0" , non_zero_digit ) | ( "1" , ( "0" | "1" | "2" ) );
day = ( "0" , non_zero_digit ) | ( ( "1" | "2" ) , digit ) | ( "3" , ( "0" | "1" ) );
non_zero_digit = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
@endebnf
"""),
    # --- Phone number ---
    ("ebnf_phone", """\
@startebnf
phone = [ country_code ] , area_code , "-" , exchange , "-" , subscriber;
country_code = "+" , digit , { digit };
area_code = "(" , digit , digit , digit , ")";
exchange = digit , digit , digit;
subscriber = digit , digit , digit , digit;
digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
@endebnf
"""),
    # --- Semantic versioning ---
    ("ebnf_semver", """\
@startebnf
version = major , "." , minor , "." , patch , [ "-" , pre_release ] , [ "+" , build ];
major = numeric_id;
minor = numeric_id;
patch = numeric_id;
pre_release = pre_id , { "." , pre_id };
build = build_id , { "." , build_id };
pre_id = numeric_id | ident;
build_id = numeric_id | ident;
numeric_id = "0" | ( non_zero , { digit } );
ident = { letter | digit | "-" };
non_zero = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
digit = "0" | non_zero;
letter = "a" | "b" | "c";
@endebnf
"""),
    # --- XML element ---
    ("ebnf_xml_element", """\
@startebnf
element = start_tag , content , end_tag | empty_element;
start_tag = "<" , name , { attribute } , ">";
end_tag = "</" , name , ">";
empty_element = "<" , name , { attribute } , "/>";
content = { element | char_data };
attribute = name , "=" , quoted_string;
quoted_string = '"' , { char } , '"';
name = letter , { letter | digit | "-" | "_" };
char_data = { char };
char = letter | digit;
letter = "a" | "b" | "c";
digit = "0" | "1" | "2";
@endebnf
"""),
]


def gen_ebnf(stats: dict) -> None:
    """Generate EBNF .puml files and their golden SVGs."""
    subdir = "ebnf"
    puml_count = 0
    svg_count = 0

    for stem, source in EBNF_CASES:
        write(subdir, stem + ".puml", source)
        puml_count += 1

        svg = render(source)
        if svg:
            write_svg(subdir, stem, svg)
            svg_count += 1

    stats["ebnf"] = {
        "puml": puml_count,
        "svg": svg_count,
        "note": "EBNF is fully supported in this PlantUML build.",
    }


# ---------------------------------------------------------------------------
# Git diagrams — @startgit / @endgit
#
# NOTE: In PlantUML 1.2026.3beta6, @startgit parses but renders a 21x21
# pixel placeholder SVG (the feature is partially implemented). The .puml
# files are saved as parser tests. SVGs are saved as raw oracle output so
# the test suite can verify RustUML matches what the reference produces.
# ---------------------------------------------------------------------------

GIT_CASES = [
    # --- Linear commits ---
    ("git_single_commit", """\
@startgit
commit
@endgit
"""),
    ("git_three_commits", """\
@startgit
commit
commit
commit
@endgit
"""),
    ("git_five_commits", """\
@startgit
commit
commit
commit
commit
commit
@endgit
"""),
    ("git_commit_ids", """\
@startgit
commit id:"init"
commit id:"feat-1"
commit id:"feat-2"
@endgit
"""),
    # --- Branching ---
    ("git_branch_simple", """\
@startgit
commit
branch develop
commit
@endgit
"""),
    ("git_branch_checkout", """\
@startgit
commit
branch develop
checkout develop
commit
commit
@endgit
"""),
    ("git_two_branches", """\
@startgit
commit
branch feat1
checkout feat1
commit
checkout master
branch feat2
checkout feat2
commit
@endgit
"""),
    ("git_branch_from_branch", """\
@startgit
commit
branch develop
checkout develop
commit
branch feature
checkout feature
commit
@endgit
"""),
    # --- Merging ---
    ("git_merge_simple", """\
@startgit
commit
branch develop
checkout develop
commit
checkout master
merge develop
@endgit
"""),
    ("git_merge_two_branches", """\
@startgit
commit
branch feat1
checkout feat1
commit
checkout master
branch feat2
checkout feat2
commit
checkout master
merge feat1
merge feat2
@endgit
"""),
    ("git_merge_with_commits_after", """\
@startgit
commit
branch develop
checkout develop
commit
checkout master
merge develop
commit
commit
@endgit
"""),
    ("git_gitflow_simple", """\
@startgit
commit id:"init"
branch develop
checkout develop
commit id:"dev1"
commit id:"dev2"
checkout master
merge develop tag:"v1.0"
commit id:"post-release"
@endgit
"""),
    # --- Tags ---
    ("git_tag_simple", """\
@startgit
commit tag:"v1.0"
commit
@endgit
"""),
    ("git_tag_with_id", """\
@startgit
commit id:"A" tag:"v0.1"
commit id:"B"
commit id:"C" tag:"v0.2"
@endgit
"""),
    ("git_multiple_tags", """\
@startgit
commit tag:"alpha"
commit tag:"beta"
commit tag:"rc1"
commit tag:"v1.0"
@endgit
"""),
    # --- Cherry-pick ---
    ("git_cherry_pick", """\
@startgit
commit id:"A"
branch dev
checkout dev
commit id:"B"
checkout master
cherry-pick id:"B"
@endgit
"""),
    ("git_cherry_pick_multiple", """\
@startgit
commit id:"base"
branch hotfix
checkout hotfix
commit id:"fix1"
commit id:"fix2"
checkout master
cherry-pick id:"fix1"
cherry-pick id:"fix2"
@endgit
"""),
    # --- Commit types ---
    ("git_commit_type_reverse", """\
@startgit
commit
commit type:REVERSE
commit
@endgit
"""),
    ("git_commit_type_highlight", """\
@startgit
commit
commit type:HIGHLIGHT
commit
@endgit
"""),
    ("git_commit_types_mixed", """\
@startgit
commit id:"normal"
commit id:"bad" type:REVERSE
commit id:"important" type:HIGHLIGHT
commit id:"regular"
@endgit
"""),
    # --- Branch naming ---
    ("git_named_main", """\
@startgit
commit id:"init"
branch main
checkout main
commit id:"A"
@endgit
"""),
    ("git_named_release", """\
@startgit
commit
branch release/1.0
checkout release/1.0
commit tag:"v1.0-rc1"
commit tag:"v1.0"
@endgit
"""),
    ("git_named_feature", """\
@startgit
commit
branch feature/auth
checkout feature/auth
commit id:"add-login"
commit id:"add-logout"
checkout master
merge feature/auth
@endgit
"""),
    # --- Complex scenarios ---
    ("git_full_gitflow", """\
@startgit
commit id:"initial"
branch develop
checkout develop
commit id:"dev-start"
branch feature/api
checkout feature/api
commit id:"api-1"
commit id:"api-2"
checkout develop
merge feature/api
commit id:"dev-release-prep"
checkout master
merge develop tag:"v1.0"
branch hotfix/urgent
checkout hotfix/urgent
commit id:"hotfix"
checkout master
merge hotfix/urgent tag:"v1.0.1"
checkout develop
merge hotfix/urgent
@endgit
"""),
    ("git_trunk_based", """\
@startgit
commit id:"main-1"
commit id:"main-2"
branch short-feature
checkout short-feature
commit id:"feat"
checkout master
merge short-feature
commit id:"main-3"
commit id:"main-4" tag:"v2.0"
@endgit
"""),
]


def gen_git(stats: dict) -> None:
    """
    Generate git .puml files and attempt SVG rendering.

    The oracle produces 21x21 placeholder SVGs for @startgit in this build.
    We save them as raw oracle output — RustUML should match this behaviour.
    """
    subdir = "git"
    puml_count = 0
    svg_count = 0

    for stem, source in GIT_CASES:
        write(subdir, stem + ".puml", source)
        puml_count += 1

        # Use render_raw to capture the placeholder SVG as oracle truth.
        svg = render_raw(source)
        if svg:
            write_svg(subdir, stem, svg)
            svg_count += 1

    real_svg_count = sum(
        1 for _, source in GIT_CASES
        if render(source) is not None
    )

    stats["git"] = {
        "puml": puml_count,
        "svg": svg_count,
        "svg_real": real_svg_count,
        "note": (
            f"@startgit is parsed but renders a 21x21 placeholder SVG in this "
            f"PlantUML build (feature partially implemented). "
            f"{svg_count} SVGs saved as raw oracle output; "
            f"{real_svg_count} are fully-rendered (non-placeholder) diagrams."
        ),
    }


# ---------------------------------------------------------------------------
# Chart diagrams
#
# NOTE: In PlantUML 1.2026.3beta6:
#   - @startpie  → "Diagram not supported by this release of PlantUML"
#   - <chart>    → falls back to version info page (error)
#   - @startboard → fully supported, produces kanban/board visualizations
#
# We use @startboard as the chart-like diagram type that this version
# actually supports. The .puml files follow the @startboard syntax, and
# the directory is named "chart" as requested.
# ---------------------------------------------------------------------------

CHART_CASES = [
    # --- Basic kanban boards ---
    ("chart_kanban_simple", """\
@startboard
Simple Kanban

+To Do+
* Task A
* Task B

+In Progress+
* Task C

+Done+
* Task D
@endboard
"""),
    ("chart_kanban_3col", """\
@startboard
Sprint 1

+Backlog+
* User authentication
* Password reset
* Email verification

+In Progress+
* API design

+Done+
* Project setup
@endboard
"""),
    ("chart_kanban_4col", """\
@startboard
Development Board

+Todo+
* Feature 1
* Feature 2

+In Progress+
* Feature 3

+Review+
* Feature 4

+Done+
* Feature 5
* Feature 6
@endboard
"""),
    ("chart_kanban_5col", """\
@startboard
Full Workflow

+Backlog+
* Story A

+Analysis+
* Story B

+Development+
* Story C

+Testing+
* Story D

+Released+
* Story E
@endboard
"""),
    ("chart_kanban_empty_cols", """\
@startboard
Quiet Sprint

+To Do+

+In Progress+
* Urgent task

+Done+
@endboard
"""),
    # --- Sprint boards ---
    ("chart_sprint_board", """\
@startboard
Sprint 42

+Backlog+
* JIRA-101 Fix login bug
* JIRA-102 Add dark mode
* JIRA-103 Improve performance

+In Progress+
* JIRA-98 Update docs

+Code Review+
* JIRA-97 New dashboard

+QA+
* JIRA-96 Notifications

+Done+
* JIRA-95 User profile
* JIRA-94 API refactor
@endboard
"""),
    ("chart_scrum_board", """\
@startboard
Scrum Board

+Product Backlog+
* Epic: Search
* Epic: Analytics

+Sprint Backlog+
* US-10 Basic search
* US-11 Filters

+In Sprint+
* US-09 Results page

+Done+
* US-08 Index page
@endboard
"""),
    # --- Project management boards ---
    ("chart_project_phases", """\
@startboard
Project Alpha

+Planning+
* Define scope
* Identify stakeholders

+Design+
* Architecture diagram

+Implementation+
* Core module
* API layer

+Testing+
* Unit tests

+Deployment+
* Production release
@endboard
"""),
    ("chart_bug_triage", """\
@startboard
Bug Triage Board

+New+
* BUG-201 Crash on login
* BUG-202 Slow dashboard

+Investigating+
* BUG-199 Memory leak

+Confirmed+
* BUG-198 Wrong timezone

+Fixed+
* BUG-197 Typo in footer
* BUG-196 Bad redirect
@endboard
"""),
    ("chart_release_board", """\
@startboard
Release v2.0 Checklist

+Not Started+
* Update changelog
* Tag release

+In Progress+
* Run regression suite

+Blocked+
* Performance sign-off

+Complete+
* Feature freeze
* Code review done
@endboard
"""),
    # --- Team workflow boards ---
    ("chart_team_tasks", """\
@startboard
Team Weekly

+Alice+
* Write ADR
* Review PRs

+Bob+
* Fix CI flakiness
* Update dependencies

+Carol+
* Demo prep
* Customer meeting
@endboard
"""),
    ("chart_personal_board", """\
@startboard
Personal Tasks

+Today+
* Morning review
* Write tests

+This Week+
* Refactor auth module
* Update runbook

+Someday+
* Learn Rust
* Read DDIA book
@endboard
"""),
    # --- Content management ---
    ("chart_content_pipeline", """\
@startboard
Content Calendar

+Ideas+
* Blog: Rust async guide
* Video: PlantUML intro
* Podcast: Interview

+Writing+
* Blog: Docker basics

+Review+
* Blog: CI/CD pipelines

+Scheduled+
* Blog: Git branching

+Published+
* Blog: Intro to EBNF
@endboard
"""),
    ("chart_editorial_board", """\
@startboard
Editorial Board

+Pitch+
* Article 1
* Article 2

+Assigned+
* Article 3

+Draft+
* Article 4

+Edit+
* Article 5

+Approved+
* Article 6

+Live+
* Article 7
@endboard
"""),
    # --- Software development lifecycle ---
    ("chart_sdlc_board", """\
@startboard
SDLC Workflow

+Requirements+
* FR-001 Login
* FR-002 Search

+Design+
* FR-001 Login UI mockup

+Development+
* FR-001 Backend API

+Integration+
* FR-001 End-to-end

+UAT+
* FR-001 User testing

+Production+
* v1.0.0 release
@endboard
"""),
    ("chart_devops_board", """\
@startboard
DevOps Pipeline

+Code+
* Feature branch

+Build+
* CI compiling

+Test+
* Unit tests passing

+Stage+
* Staging deploy

+Prod+
* v1.2 live
@endboard
"""),
    # --- Support / incident boards ---
    ("chart_incident_board", """\
@startboard
Incident Board

+Reported+
* INC-001 DB latency spike
* INC-002 Login failures

+Investigating+
* INC-003 API timeouts

+Mitigated+
* INC-004 CDN outage

+Resolved+
* INC-005 Memory OOM
@endboard
"""),
    ("chart_support_queue", """\
@startboard
Support Queue

+New+
* TKT-101 Install help
* TKT-102 Billing question

+In Progress+
* TKT-100 API error

+Waiting for Customer+
* TKT-099 Config issue

+Resolved+
* TKT-098 Login problem
* TKT-097 Export failure
@endboard
"""),
    # --- Learning / study boards ---
    ("chart_study_plan", """\
@startboard
Learning Roadmap

+Not Started+
* Rust ownership
* Async/await patterns

+Studying+
* Trait objects

+Practicing+
* Lifetime annotations

+Mastered+
* Basic ownership
* Borrowing rules
@endboard
"""),
    ("chart_book_tracking", """\
@startboard
Reading List

+Want to Read+
* The Pragmatic Programmer
* Clean Architecture

+Reading+
* Designing Data-Intensive Applications

+On Hold+
* Domain-Driven Design

+Finished+
* The Rust Programming Language
* Programming Rust
@endboard
"""),
    # --- Large boards ---
    ("chart_large_board", """\
@startboard
Mega Project

+Icebox+
* Idea A
* Idea B
* Idea C
* Idea D

+Backlog+
* Task 1
* Task 2
* Task 3

+In Progress+
* Task 4
* Task 5

+Review+
* Task 6

+Done+
* Task 7
* Task 8
* Task 9
* Task 10
@endboard
"""),
    # --- Special characters and formatting ---
    ("chart_special_chars", """\
@startboard
Special Cases

+To Do+
* Fix issue #123
* Handle edge case: a > b

+Done+
* Close PR #456
@endboard
"""),
    ("chart_many_items", """\
@startboard
Feature Backlog

+High Priority+
* Feature A
* Feature B
* Feature C
* Feature D
* Feature E

+Medium Priority+
* Feature F
* Feature G
* Feature H

+Low Priority+
* Feature I
* Feature J
@endboard
"""),
    # --- Workflow-specific boards ---
    ("chart_hiring_pipeline", """\
@startboard
Hiring Pipeline

+Applied+
* Candidate A
* Candidate B
* Candidate C

+Phone Screen+
* Candidate D

+Technical Interview+
* Candidate E

+Final Round+
* Candidate F

+Offer Extended+
* Candidate G

+Hired+
* Candidate H
@endboard
"""),
    ("chart_marketing_campaign", """\
@startboard
Q4 Campaign

+Concept+
* Holiday promo
* Year-end review

+Creative+
* Banner designs

+Approval+
* Budget sign-off

+Scheduled+
* Email blast Dec 1

+Live+
* Black Friday campaign
@endboard
"""),
]


def gen_chart(stats: dict) -> None:
    """
    Generate chart .puml files using @startboard syntax and render SVGs.

    @startpie and <chart> are not supported in this PlantUML build.
    @startboard (kanban/board diagrams) is fully supported and serves as
    the chart-like diagram type for these golden tests.
    """
    subdir = "chart"
    puml_count = 0
    svg_count = 0

    for stem, source in CHART_CASES:
        write(subdir, stem + ".puml", source)
        puml_count += 1

        svg = render(source)
        if svg:
            write_svg(subdir, stem, svg)
            svg_count += 1

    stats["chart"] = {
        "puml": puml_count,
        "svg": svg_count,
        "note": (
            "@startpie and <chart> are not supported in this PlantUML build. "
            "@startboard (kanban/board diagrams) is used instead — it is a "
            "chart-like visualization fully supported in this version."
        ),
    }


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    print(f"PlantUML URL: {PLANTUML_URL}")
    print()

    # Verify server is reachable
    try:
        test_src = "@startuml\nAlice -> Bob: hello\n@enduml"
        body = json.dumps({"source": test_src, "options": ["-tsvg"]}).encode()
        req = urllib.request.Request(
            f"{PLANTUML_URL}/render",
            body,
            {"Content-Type": "application/json"},
        )
        urllib.request.urlopen(req, timeout=10).read()
    except Exception as e:
        print(f"ERROR: Cannot reach PlantUML server at {PLANTUML_URL}: {e}")
        print("Start the server with: scripts/plantuml-server.sh &")
        sys.exit(1)

    stats: dict = {}

    print("Generating DOT diagrams...")
    gen_dot(stats)

    print("Generating EBNF diagrams...")
    gen_ebnf(stats)

    print("Generating git diagrams...")
    gen_git(stats)

    print("Generating chart diagrams...")
    gen_chart(stats)

    print()
    print("=" * 70)
    print("Results")
    print("=" * 70)
    for dtype, info in stats.items():
        puml = info["puml"]
        svg = info["svg"]
        print(f"\n{dtype.upper()}")
        print(f"  .puml files created: {puml}")
        print(f"  .svg files rendered: {svg}")
        if "svg_real" in info:
            print(f"  .svg real (non-placeholder): {info['svg_real']}")
        print(f"  Note: {info['note']}")

    total_puml = sum(v["puml"] for v in stats.values())
    total_svg = sum(v["svg"] for v in stats.values())
    print()
    print(f"Total .puml: {total_puml}")
    print(f"Total .svg:  {total_svg}")


if __name__ == "__main__":
    main()
