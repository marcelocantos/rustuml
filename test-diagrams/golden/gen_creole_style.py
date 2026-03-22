#!/usr/bin/env python3
"""
Generator for comprehensive PlantUML test cases covering:
- Creole markup in various diagram contexts
- Skinparams for all major diagram types
- Preprocessing features

Target: ~1500+ .puml files
"""

import os
import itertools

BASE = os.path.dirname(os.path.abspath(__file__))
CREOLE_DIR = os.path.join(BASE, "creole")
SKINPARAM_DIR = os.path.join(BASE, "skinparam")
PREPROC_DIR = os.path.join(BASE, "preprocessing")

for d in [CREOLE_DIR, SKINPARAM_DIR, PREPROC_DIR]:
    os.makedirs(d, exist_ok=True)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def write(path, content):
    with open(path, "w") as f:
        f.write(content)
    return path

files_written = []

def w(directory, name, content):
    path = os.path.join(directory, name)
    write(path, content)
    files_written.append(path)

# ---------------------------------------------------------------------------
# CREOLE MARKUP
# ---------------------------------------------------------------------------

# --- Bold ---
def gen_creole_bold():
    w(CREOLE_DIR, "creole_bold_in_class.puml", """\
@startuml
class "**BoldClassName**" {
  **boldField**: String
  **boldMethod**()
}
@enduml
""")

    w(CREOLE_DIR, "creole_bold_in_sequence.puml", """\
@startuml
Alice -> Bob : **bold message**
Bob --> Alice : **bold reply**
note right : **bold note**
@enduml
""")

    w(CREOLE_DIR, "creole_bold_in_activity.puml", """\
@startuml
:**//** bold italic label//;
:**bold activity label**;
@enduml
""")

    w(CREOLE_DIR, "creole_bold_in_note.puml", """\
@startuml
note as N
  **bold text in note**
  **another bold line**
end note
@enduml
""")

    w(CREOLE_DIR, "creole_bold_in_title.puml", """\
@startuml
title **Bold Title**
Alice -> Bob : hello
@enduml
""")

    w(CREOLE_DIR, "creole_bold_in_state.puml", """\
@startuml
state "**BoldState**" as S
[*] --> S
@enduml
""")

    w(CREOLE_DIR, "creole_bold_in_component.puml", """\
@startuml
component "**BoldComponent**" as C
@enduml
""")

    w(CREOLE_DIR, "creole_bold_in_usecase.puml", """\
@startuml
usecase "**Bold Use Case**" as UC
actor "**Bold Actor**" as A
A --> UC
@enduml
""")

    w(CREOLE_DIR, "creole_bold_in_header_footer.puml", """\
@startuml
header **Bold Header**
footer **Bold Footer**
Alice -> Bob : hello
@enduml
""")

    w(CREOLE_DIR, "creole_bold_in_legend.puml", """\
@startuml
Alice -> Bob : hello
legend
  **bold legend text**
end legend
@enduml
""")

    w(CREOLE_DIR, "creole_bold_in_caption.puml", """\
@startuml
caption **Bold Caption**
Alice -> Bob : hello
@enduml
""")

# --- Italic ---
def gen_creole_italic():
    w(CREOLE_DIR, "creole_italic_in_class.puml", """\
@startuml
class "//ItalicClassName//" {
  //italicField//: String
  //italicMethod//()
}
@enduml
""")

    w(CREOLE_DIR, "creole_italic_in_sequence.puml", """\
@startuml
Alice -> Bob : //italic message//
Bob --> Alice : //italic reply//
note right : //italic note//
@enduml
""")

    w(CREOLE_DIR, "creole_italic_in_activity.puml", """\
@startuml
://italic activity label//;
@enduml
""")

    w(CREOLE_DIR, "creole_italic_in_note.puml", """\
@startuml
note as N
  //italic text in note//
  //another italic line//
end note
@enduml
""")

    w(CREOLE_DIR, "creole_italic_in_title.puml", """\
@startuml
title //Italic Title//
Alice -> Bob : hello
@enduml
""")

    w(CREOLE_DIR, "creole_italic_in_state.puml", """\
@startuml
state "//ItalicState//" as S
[*] --> S
@enduml
""")

    w(CREOLE_DIR, "creole_italic_in_component.puml", """\
@startuml
component "//ItalicComponent//" as C
@enduml
""")

    w(CREOLE_DIR, "creole_italic_in_usecase.puml", """\
@startuml
usecase "//Italic Use Case//" as UC
@enduml
""")

    w(CREOLE_DIR, "creole_italic_in_header_footer.puml", """\
@startuml
header //Italic Header//
footer //Italic Footer//
Alice -> Bob : hello
@enduml
""")

    w(CREOLE_DIR, "creole_italic_in_legend.puml", """\
@startuml
Alice -> Bob : hello
legend
  //italic legend text//
end legend
@enduml
""")

# --- Monospace ---
def gen_creole_mono():
    w(CREOLE_DIR, "creole_mono_in_class.puml", """\
@startuml
class "\"\"MonoClass\"\"" {
  ""monoField"": String
  ""monoMethod""()
}
@enduml
""")

    w(CREOLE_DIR, "creole_mono_in_sequence.puml", """\
@startuml
Alice -> Bob : ""mono message""
Bob --> Alice : ""mono reply""
note right : ""mono note""
@enduml
""")

    w(CREOLE_DIR, "creole_mono_in_activity.puml", """\
@startuml
:""mono activity"";
@enduml
""")

    w(CREOLE_DIR, "creole_mono_in_note.puml", """\
@startuml
note as N
  ""monospace text""
  ""another mono line""
end note
@enduml
""")

    w(CREOLE_DIR, "creole_mono_in_title.puml", """\
@startuml
title ""Mono Title""
Alice -> Bob : hello
@enduml
""")

    w(CREOLE_DIR, "creole_mono_in_state.puml", """\
@startuml
state "\"\"MonoState\"\"" as S
[*] --> S
@enduml
""")

    w(CREOLE_DIR, "creole_mono_in_usecase.puml", """\
@startuml
usecase "\"\"Mono Use Case\"\"" as UC
@enduml
""")

    w(CREOLE_DIR, "creole_mono_in_legend.puml", """\
@startuml
Alice -> Bob : hello
legend
  ""mono legend text""
end legend
@enduml
""")

# --- Strike ---
def gen_creole_strike():
    w(CREOLE_DIR, "creole_strike_in_class.puml", """\
@startuml
class MyClass {
  --strikeField--: String
  --strikeMethod--()
}
@enduml
""")

    w(CREOLE_DIR, "creole_strike_in_sequence.puml", """\
@startuml
Alice -> Bob : ~~strike message~~
Bob --> Alice : ~~strike reply~~
note right : ~~strike note~~
@enduml
""")

    w(CREOLE_DIR, "creole_strike_in_activity.puml", """\
@startuml
:~~struck activity~~;
@enduml
""")

    w(CREOLE_DIR, "creole_strike_in_note.puml", """\
@startuml
note as N
  ~~strikethrough text~~
  ~~another struck line~~
end note
@enduml
""")

    w(CREOLE_DIR, "creole_strike_in_title.puml", """\
@startuml
title ~~Strike Title~~
Alice -> Bob : hello
@enduml
""")

    w(CREOLE_DIR, "creole_strike_in_state.puml", """\
@startuml
state "~~StrikeState~~" as S
[*] --> S
@enduml
""")

    w(CREOLE_DIR, "creole_strike_in_legend.puml", """\
@startuml
Alice -> Bob : hello
legend
  ~~struck legend~~
end legend
@enduml
""")

# --- Underline ---
def gen_creole_underline():
    w(CREOLE_DIR, "creole_underline_in_class.puml", """\
@startuml
class MyClass {
  __underlineField__: String
  __underlineMethod__()
}
@enduml
""")

    w(CREOLE_DIR, "creole_underline_in_sequence.puml", """\
@startuml
Alice -> Bob : __underline message__
Bob --> Alice : __underline reply__
note right : __underline note__
@enduml
""")

    w(CREOLE_DIR, "creole_underline_in_activity.puml", """\
@startuml
:__underlined activity__;
@enduml
""")

    w(CREOLE_DIR, "creole_underline_in_note.puml", """\
@startuml
note as N
  __underlined text__
  __another underlined line__
end note
@enduml
""")

    w(CREOLE_DIR, "creole_underline_in_title.puml", """\
@startuml
title __Underline Title__
Alice -> Bob : hello
@enduml
""")

    w(CREOLE_DIR, "creole_underline_in_state.puml", """\
@startuml
state "__UnderlineState__" as S
[*] --> S
@enduml
""")

    w(CREOLE_DIR, "creole_underline_in_legend.puml", """\
@startuml
Alice -> Bob : hello
legend
  __underlined legend__
end legend
@enduml
""")

# --- Combinations ---
def gen_creole_combinations():
    combos = [
        ("bold_italic", "**//bold italic//**", "bold_italic"),
        ("bold_mono", "**\"\"bold mono\"\"**", "bold_mono"),
        ("bold_underline", "**__bold underline__**", "bold_underline"),
        ("italic_mono", "//\"\"italic mono\"\"//", "italic_mono"),
        ("italic_underline", "//__italic underline__//", "italic_underline"),
        ("bold_strike", "**~~bold strike~~**", "bold_strike"),
        ("all_styles", "**//~~__all styles__~~//**", "all_styles"),
    ]

    for name, markup, desc in combos:
        w(CREOLE_DIR, f"creole_combo_{name}_in_sequence.puml", f"""\
@startuml
Alice -> Bob : {markup}
note right : {markup}
@enduml
""")
        w(CREOLE_DIR, f"creole_combo_{name}_in_class.puml", f"""\
@startuml
class MyClass {{
  {markup}: field
}}
@enduml
""")
        w(CREOLE_DIR, f"creole_combo_{name}_in_note.puml", f"""\
@startuml
note as N
  {markup}
end note
@enduml
""")
        w(CREOLE_DIR, f"creole_combo_{name}_in_activity.puml", f"""\
@startuml
:{markup};
@enduml
""")

# --- Colors ---
def gen_creole_colors():
    color_tags = [
        ("color_red", "<color:red>red text</color>"),
        ("color_blue", "<color:blue>blue text</color>"),
        ("color_green", "<color:#00AA00>green text</color>"),
        ("color_hex", "<color:#FF6600>hex color</color>"),
        ("back_yellow", "<back:yellow>yellow bg</back>"),
        ("back_cyan", "<back:cyan>cyan bg</back>"),
        ("back_hex", "<back:#FFEECC>hex bg</back>"),
        ("size_small", "<size:10>small text</size>"),
        ("size_large", "<size:24>large text</size>"),
        ("size_huge", "<size:36>huge text</size>"),
        ("font_name", "<font:Courier>courier font</font>"),
        ("color_bold", "<color:red>**bold red**</color>"),
        ("color_italic", "<color:blue>//italic blue//</color>"),
        ("nested_color", "<color:red><back:yellow>red on yellow</back></color>"),
        ("color_in_bold", "**<color:green>bold green</color>**"),
    ]

    contexts = [
        ("sequence", lambda markup: f"""\
@startuml
Alice -> Bob : {markup}
note right : {markup}
@enduml
"""),
        ("class", lambda markup: f"""\
@startuml
class MyClass {{
  {markup}: field
}}
@enduml
"""),
        ("note", lambda markup: f"""\
@startuml
note as N
  {markup}
end note
@enduml
"""),
        ("activity", lambda markup: f"""\
@startuml
:{markup};
@enduml
"""),
        ("title", lambda markup: f"""\
@startuml
title {markup}
Alice -> Bob : hello
@enduml
"""),
    ]

    for tag_name, markup in color_tags:
        for ctx_name, tmpl in contexts:
            w(CREOLE_DIR, f"creole_{tag_name}_in_{ctx_name}.puml", tmpl(markup))

# --- Links ---
def gen_creole_links():
    links = [
        ("bare", "[[https://plantuml.com]]"),
        ("with_label", "[[https://plantuml.com PlantUML]]"),
        ("with_tooltip", "[[https://plantuml.com{PlantUML website} click me]]"),
        ("local", "[[#anchor local link]]"),
    ]

    for name, markup in links:
        w(CREOLE_DIR, f"creole_link_{name}_in_sequence.puml", f"""\
@startuml
Alice -> Bob : {markup}
note right : {markup}
@enduml
""")
        w(CREOLE_DIR, f"creole_link_{name}_in_note.puml", f"""\
@startuml
note as N
  {markup}
end note
@enduml
""")
        w(CREOLE_DIR, f"creole_link_{name}_in_class.puml", f"""\
@startuml
class MyClass {{
  {markup}: field
}}
@enduml
""")

# --- Images ---
def gen_creole_images():
    imgs = [
        ("url", "<img:https://plantuml.com/img/logo.png>"),
        ("url_sized", "<img:https://plantuml.com/img/logo.png{scale=0.5}>"),
        ("local", "<img:sprite.png>"),
    ]

    for name, markup in imgs:
        w(CREOLE_DIR, f"creole_img_{name}_in_sequence.puml", f"""\
@startuml
Alice -> Bob : message with {markup}
note right : note with {markup}
@enduml
""")
        w(CREOLE_DIR, f"creole_img_{name}_in_note.puml", f"""\
@startuml
note as N
  image: {markup}
end note
@enduml
""")

# --- Lists ---
def gen_creole_lists():
    w(CREOLE_DIR, "creole_list_unordered_in_note.puml", """\
@startuml
note as N
  * item one
  * item two
  * item three
end note
@enduml
""")

    w(CREOLE_DIR, "creole_list_ordered_in_note.puml", """\
@startuml
note as N
  # first
  # second
  # third
end note
@enduml
""")

    w(CREOLE_DIR, "creole_list_nested_unordered_in_note.puml", """\
@startuml
note as N
  * level 1
  ** level 2
  *** level 3
  ** back to 2
  * back to 1
end note
@enduml
""")

    w(CREOLE_DIR, "creole_list_nested_ordered_in_note.puml", """\
@startuml
note as N
  # first
  ## sub-first
  ## sub-second
  # second
end note
@enduml
""")

    w(CREOLE_DIR, "creole_list_mixed_in_note.puml", """\
@startuml
note as N
  * unordered item
  # ordered item
  * another unordered
end note
@enduml
""")

    w(CREOLE_DIR, "creole_list_in_class.puml", """\
@startuml
class MyClass {
  * field list item 1
  * field list item 2
}
@enduml
""")

    w(CREOLE_DIR, "creole_list_in_sequence_note.puml", """\
@startuml
Alice -> Bob : hello
note over Bob
  * point one
  * point two
  * point three
end note
@enduml
""")

    w(CREOLE_DIR, "creole_list_with_markup_in_note.puml", """\
@startuml
note as N
  * **bold item**
  * //italic item//
  * ""mono item""
end note
@enduml
""")

# --- Horizontal rules ---
def gen_creole_hlines():
    w(CREOLE_DIR, "creole_hline_in_note.puml", """\
@startuml
note as N
  above line
  ----
  below line
end note
@enduml
""")

    w(CREOLE_DIR, "creole_hline_double_in_note.puml", """\
@startuml
note as N
  section one
  ====
  section two
end note
@enduml
""")

    w(CREOLE_DIR, "creole_hline_dotted_in_note.puml", """\
@startuml
note as N
  before dots
  ....
  after dots
end note
@enduml
""")

    w(CREOLE_DIR, "creole_hline_titled_in_note.puml", """\
@startuml
note as N
  before
  == Section Title ==
  after
end note
@enduml
""")

    w(CREOLE_DIR, "creole_hline_in_class.puml", """\
@startuml
class MyClass {
  field1: String
  --
  method1()
  ==
  static_method()
  ..
  abstract_method()
}
@enduml
""")

    w(CREOLE_DIR, "creole_hline_labeled_in_class.puml", """\
@startuml
class MyClass {
  field: String
  -- fields --
  other: Int
  == methods ==
  doSomething()
  .. helpers ..
  helper()
}
@enduml
""")

# --- Tables ---
def gen_creole_tables():
    w(CREOLE_DIR, "creole_table_simple_in_note.puml", """\
@startuml
note as N
  |= Header1 |= Header2 |
  | cell1    | cell2    |
  | cell3    | cell4    |
end note
@enduml
""")

    w(CREOLE_DIR, "creole_table_colored_in_note.puml", """\
@startuml
note as N
  |= Name |= Value |
  |<#red> error | 42 |
  |<#green> ok | 0 |
end note
@enduml
""")

    w(CREOLE_DIR, "creole_table_mixed_markup_in_note.puml", """\
@startuml
note as N
  |= **Bold** |= //Italic// |
  | ""mono"" | __under__ |
end note
@enduml
""")

    w(CREOLE_DIR, "creole_table_in_sequence_note.puml", """\
@startuml
Alice -> Bob : hello
note over Alice, Bob
  |= Key |= Value |
  | name | Alice |
  | role | Sender |
end note
@enduml
""")

    w(CREOLE_DIR, "creole_table_three_cols_in_note.puml", """\
@startuml
note as N
  |= Col1 |= Col2 |= Col3 |
  | a | b | c |
  | d | e | f |
  | g | h | i |
end note
@enduml
""")

    w(CREOLE_DIR, "creole_table_row_bg_in_note.puml", """\
@startuml
note as N
  |<#LightBlue> blue row | data |
  |<#LightGreen> green row | data |
  |<#LightYellow> yellow row | data |
end note
@enduml
""")

# --- Tree notation ---
def gen_creole_tree():
    w(CREOLE_DIR, "creole_tree_simple_in_note.puml", """\
@startuml
note as N
  |_ root
  |_ child1
  |__ grandchild1
  |__ grandchild2
  |_ child2
end note
@enduml
""")

    w(CREOLE_DIR, "creole_tree_deep_in_note.puml", """\
@startuml
note as N
  |_ level1
  |__ level2
  |___ level3
  |____ level4
end note
@enduml
""")

# --- Code blocks ---
def gen_creole_code():
    w(CREOLE_DIR, "creole_code_block_in_note.puml", """\
@startuml
note as N
  <code>
  int x = 42;
  printf("%d\\n", x);
  </code>
end note
@enduml
""")

    w(CREOLE_DIR, "creole_code_inline_in_sequence.puml", """\
@startuml
Alice -> Bob : call <code>doSomething()</code>
@enduml
""")

    w(CREOLE_DIR, "creole_code_multiline_in_note.puml", """\
@startuml
note as N
  <code>
  def hello():
      print("Hello, World!")

  hello()
  </code>
end note
@enduml
""")

# --- LaTeX ---
def gen_creole_latex():
    w(CREOLE_DIR, "creole_latex_formula_in_note.puml", """\
@startuml
note as N
  <latex>x = \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}</latex>
end note
@enduml
""")

    w(CREOLE_DIR, "creole_latex_simple_in_sequence.puml", """\
@startuml
Alice -> Bob : <latex>E = mc^2</latex>
@enduml
""")

    w(CREOLE_DIR, "creole_latex_in_class.puml", """\
@startuml
class MathClass {
  <latex>\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}</latex>
}
@enduml
""")

# --- Unicode ---
def gen_creole_unicode():
    unicode_cases = [
        ("arrows", "→ ← ↑ ↓ ↔ ⇒ ⇐ ⇔"),
        ("math", "∀ ∃ ∈ ∉ ⊂ ⊃ ∩ ∪ ∑ ∏"),
        ("greek", "α β γ δ ε ζ η θ"),
        ("emoji", "✓ ✗ ★ ☆ ♥ ♦"),
        ("box_drawing", "┌─┐ │ │ └─┘"),
        ("accented", "café naïve résumé"),
        ("cjk", "中文 日本語 한국어"),
        ("rtl_hint", "text with some Arabic: مرحبا"),
    ]

    for name, text in unicode_cases:
        w(CREOLE_DIR, f"creole_unicode_{name}_in_sequence.puml", f"""\
@startuml
Alice -> Bob : {text}
note right : {text}
@enduml
""")
        w(CREOLE_DIR, f"creole_unicode_{name}_in_note.puml", f"""\
@startuml
note as N
  {text}
end note
@enduml
""")

# --- HTML tags ---
def gen_creole_html():
    html_cases = [
        ("b_tag", "<b>bold b tag</b>"),
        ("i_tag", "<i>italic i tag</i>"),
        ("u_tag", "<u>underline u tag</u>"),
        ("s_tag", "<s>strike s tag</s>"),
        ("font_color", "<font color='red'>font color</font>"),
        ("font_size", "<font size='18'>font size</font>"),
        ("font_face", "<font face='Courier'>font face</font>"),
        ("span_color", "<span style='color:blue'>span color</span>"),
        ("br_tag", "line1<br/>line2"),
        ("sup_tag", "x<sup>2</sup> + y<sup>2</sup>"),
        ("sub_tag", "H<sub>2</sub>O"),
        ("del_tag", "<del>deleted text</del>"),
        ("ins_tag", "<ins>inserted text</ins>"),
        ("em_tag", "<em>em tag</em>"),
        ("strong_tag", "<strong>strong tag</strong>"),
    ]

    for name, markup in html_cases:
        w(CREOLE_DIR, f"creole_html_{name}_in_sequence.puml", f"""\
@startuml
Alice -> Bob : {markup}
note right : {markup}
@enduml
""")
        w(CREOLE_DIR, f"creole_html_{name}_in_note.puml", f"""\
@startuml
note as N
  {markup}
end note
@enduml
""")

# --- Escaped characters ---
def gen_creole_escape():
    escapes = [
        ("tilde_bold", "~**not bold~**"),
        ("tilde_italic", "~/not italic~/"),
        ("tilde_mono", "~\"\"not mono~\"\""),
        ("tilde_strike", "~~~not strike~~~"),
        ("tilde_underline", "~__not underline~__"),
        ("backslash_n", "line1\\nline2"),
        ("literal_tilde", "show ~~ tilde"),
    ]

    for name, text in escapes:
        w(CREOLE_DIR, f"creole_escape_{name}_in_sequence.puml", f"""\
@startuml
Alice -> Bob : {text}
@enduml
""")
        w(CREOLE_DIR, f"creole_escape_{name}_in_note.puml", f"""\
@startuml
note as N
  {text}
end note
@enduml
""")

# --- Empty creole ---
def gen_creole_empty():
    w(CREOLE_DIR, "creole_empty_bold_in_note.puml", """\
@startuml
note as N
  before ** ** after
end note
@enduml
""")

    w(CREOLE_DIR, "creole_empty_italic_in_note.puml", """\
@startuml
note as N
  before //// after
end note
@enduml
""")

    w(CREOLE_DIR, "creole_empty_color_in_note.puml", """\
@startuml
note as N
  <color:red></color>
end note
@enduml
""")

# --- Nested creole ---
def gen_creole_nested():
    w(CREOLE_DIR, "creole_nested_bold_in_color_in_note.puml", """\
@startuml
note as N
  <color:blue>**bold blue**</color>
end note
@enduml
""")

    w(CREOLE_DIR, "creole_nested_italic_in_bold_in_note.puml", """\
@startuml
note as N
  **//bold italic//**
end note
@enduml
""")

    w(CREOLE_DIR, "creole_nested_color_in_bold_in_italic_in_note.puml", """\
@startuml
note as N
  //**<color:red>bold italic red</color>**//
end note
@enduml
""")

    w(CREOLE_DIR, "creole_nested_size_color_in_note.puml", """\
@startuml
note as N
  <size:20><color:green>large green</color></size>
end note
@enduml
""")

    w(CREOLE_DIR, "creole_nested_triple_in_sequence.puml", """\
@startuml
Alice -> Bob : **//""bold italic mono""**//**
@enduml
""")

    w(CREOLE_DIR, "creole_nested_five_levels_in_note.puml", """\
@startuml
note as N
  <color:red><size:18>**//~~__5 levels__~~//**</size></color>
end note
@enduml
""")

# --- Line breaks ---
def gen_creole_linebreaks():
    w(CREOLE_DIR, "creole_linebreak_backslash_in_sequence.puml", """\
@startuml
Alice -> Bob : line1\\nline2\\nline3
@enduml
""")

    w(CREOLE_DIR, "creole_linebreak_in_note.puml", """\
@startuml
note as N
  first line
  second line
  third line
end note
@enduml
""")

    w(CREOLE_DIR, "creole_linebreak_in_class_method.puml", """\
@startuml
class MyClass {
  multiLineMethod(\\nparam1: String,\\nparam2: Int): void
}
@enduml
""")

# --- Combinatorial context tests ---
def gen_creole_combinatorial():
    """Generate cross-product of markup features x diagram contexts."""
    markups = [
        ("bold", "**bold**"),
        ("italic", "//italic//"),
        ("mono", '""mono""'),
        ("underline", "__underline__"),
        ("bold_italic", "**//bi//**"),
        ("color_red", "<color:red>red</color>"),
        ("size_large", "<size:20>large</size>"),
        ("back_yellow", "<back:yellow>bg</back>"),
    ]

    diagrams = [
        ("class", lambda m, i: f"""\
@startuml
class Class{i} {{
  field: {m}
  method{i}(): void
}}
class Class{i}b {{
  info: String
}}
Class{i} --> Class{i}b : {m}
@enduml
"""),
        ("sequence", lambda m, i: f"""\
@startuml
participant "Part {i}" as P
Alice -> P : {m}
P --> Alice : {m} reply
@enduml
"""),
        ("activity", lambda m, i: f"""\
@startuml
:{m} start;
if ({m} condition?) then (yes)
  :{m} true branch;
else (no)
  :{m} false branch;
endif
@enduml
"""),
        ("state", lambda m, i: f"""\
@startuml
state "{m} State" as S{i}
[*] --> S{i}
S{i} : {m} description
@enduml
"""),
        ("component", lambda m, i: f"""\
@startuml
component "{m} comp" as C{i}
interface "{m} iface" as I{i}
C{i} -- I{i}
@enduml
"""),
        ("usecase", lambda m, i: f"""\
@startuml
actor "{m} Actor" as A{i}
usecase "{m} Case" as UC{i}
A{i} --> UC{i}
@enduml
"""),
    ]

    for (m_name, markup), (d_name, tmpl) in itertools.product(markups, diagrams):
        idx = hash(m_name + d_name) % 1000
        w(CREOLE_DIR, f"creole_combo_{m_name}_in_{d_name}.puml", tmpl(markup, idx))

# --- Creole in every context ---
def gen_creole_all_contexts():
    """Test a single markup in all possible annotation positions."""
    markup = "**bold** //italic// __under__"

    w(CREOLE_DIR, "creole_allctx_title.puml", f"""\
@startuml
title {markup}
Alice -> Bob : hello
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_header.puml", f"""\
@startuml
header {markup}
Alice -> Bob : hello
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_footer.puml", f"""\
@startuml
footer {markup}
Alice -> Bob : hello
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_caption.puml", f"""\
@startuml
caption {markup}
Alice -> Bob : hello
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_legend.puml", f"""\
@startuml
Alice -> Bob : hello
legend
  {markup}
end legend
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_note_floating.puml", f"""\
@startuml
note as N
  {markup}
end note
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_note_over.puml", f"""\
@startuml
Alice -> Bob : hello
note over Alice
  {markup}
end note
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_note_left.puml", f"""\
@startuml
Alice -> Bob : hello
note left : {markup}
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_note_right.puml", f"""\
@startuml
Alice -> Bob : hello
note right : {markup}
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_participant.puml", f"""\
@startuml
participant "{markup}" as P
P -> P : self
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_class_name.puml", f"""\
@startuml
class "MyClass\\n{markup}" {{
}}
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_class_field.puml", f"""\
@startuml
class MyClass {{
  {markup}: field
}}
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_class_method.puml", f"""\
@startuml
class MyClass {{
  {markup}()
}}
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_activity_label.puml", f"""\
@startuml
:{markup};
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_state_name.puml", f"""\
@startuml
state "{markup}" as S
[*] --> S
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_state_desc.puml", f"""\
@startuml
state S
S : {markup}
[*] --> S
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_message.puml", f"""\
@startuml
Alice -> Bob : {markup}
@enduml
""")

    w(CREOLE_DIR, "creole_allctx_return_message.puml", f"""\
@startuml
Alice -> Bob : request
Bob --> Alice : {markup}
@enduml
""")

# ---------------------------------------------------------------------------
# SKINPARAMS
# ---------------------------------------------------------------------------

def gen_skinparam_global():
    global_params = [
        ("backgroundColor_white", "backgroundColor white"),
        ("backgroundColor_lightblue", "backgroundColor LightBlue"),
        ("backgroundColor_hex", "backgroundColor #FAFAFA"),
        ("backgroundColor_gradient", "backgroundColor #FFFFFF/#E0E0FF"),
        ("shadowing_true", "shadowing true"),
        ("shadowing_false", "shadowing false"),
        ("handwritten_true", "handwritten true"),
        ("monochrome_true", "monochrome true"),
        ("monochrome_reverse", "monochrome reverse"),
        ("dpi_72", "dpi 72"),
        ("dpi_150", "dpi 150"),
        ("dpi_300", "dpi 300"),
        ("defaultFontSize_10", "defaultFontSize 10"),
        ("defaultFontSize_14", "defaultFontSize 14"),
        ("defaultFontSize_18", "defaultFontSize 18"),
        ("defaultFontColor_black", "defaultFontColor black"),
        ("defaultFontColor_navy", "defaultFontColor navy"),
        ("defaultFontColor_hex", "defaultFontColor #333333"),
        ("defaultFontName_courier", "defaultFontName Courier"),
        ("defaultFontName_arial", "defaultFontName Arial"),
        ("defaultFontStyle_bold", "defaultFontStyle bold"),
        ("defaultFontStyle_italic", "defaultFontStyle italic"),
        ("defaultFontStyle_plain", "defaultFontStyle plain"),
        ("roundCorner_5", "roundCorner 5"),
        ("roundCorner_15", "roundCorner 15"),
        ("roundCorner_25", "roundCorner 25"),
        ("padding_5", "padding 5"),
        ("padding_10", "padding 10"),
        ("padding_20", "padding 20"),
        ("defaultTextAlignment_left", "defaultTextAlignment left"),
        ("defaultTextAlignment_center", "defaultTextAlignment center"),
        ("defaultTextAlignment_right", "defaultTextAlignment right"),
        ("wrapWidth_100", "wrapWidth 100"),
        ("wrapWidth_200", "wrapWidth 200"),
        ("maxMessageSize_50", "maxMessageSize 50"),
        ("maxMessageSize_100", "maxMessageSize 100"),
        ("guillemet_true", "guillemet true"),
        ("guillemet_false", "guillemet false"),
        ("nodesep_20", "nodesep 20"),
        ("nodesep_60", "nodesep 60"),
        ("ranksep_30", "ranksep 30"),
        ("ranksep_80", "ranksep 80"),
        ("arrowColor_red", "arrowColor red"),
        ("arrowColor_blue", "arrowColor blue"),
        ("arrowColor_hex", "arrowColor #444444"),
        ("arrowThickness_1", "arrowThickness 1"),
        ("arrowThickness_2", "arrowThickness 2"),
        ("arrowThickness_3", "arrowThickness 3"),
        ("arrowFontColor_red", "arrowFontColor red"),
        ("arrowFontSize_10", "arrowFontSize 10"),
        ("arrowFontSize_14", "arrowFontSize 14"),
        ("borderColor_black", "borderColor black"),
        ("borderColor_gray", "borderColor gray"),
        ("borderColor_hex", "borderColor #888888"),
        ("borderThickness_1", "borderThickness 1"),
        ("borderThickness_2", "borderThickness 2"),
        ("backgroundColor_note_yellow", "noteBackgroundColor yellow"),
        ("backgroundColor_class_white", "classBackgroundColor white"),
    ]

    diagram = """\
Alice -> Bob : hello
Bob --> Alice : world
note right : note
"""
    wrap_seq = lambda sp: f"@startuml\nskinparam {sp}\n{diagram}@enduml\n"

    for name, param in global_params:
        w(SKINPARAM_DIR, f"skin_global_{name}.puml", wrap_seq(param))

def gen_skinparam_sequence():
    params = [
        ("arrowColor_red", "sequenceArrowColor red"),
        ("arrowColor_blue", "sequenceArrowColor blue"),
        ("arrowColor_hex", "sequenceArrowColor #0066CC"),
        ("arrowFontColor_red", "sequenceArrowFontColor red"),
        ("arrowFontColor_green", "sequenceArrowFontColor green"),
        ("arrowFontSize_10", "sequenceArrowFontSize 10"),
        ("arrowFontSize_16", "sequenceArrowFontSize 16"),
        ("arrowFontStyle_bold", "sequenceArrowFontStyle bold"),
        ("arrowFontStyle_italic", "sequenceArrowFontStyle italic"),
        ("arrowThickness_1", "sequenceArrowThickness 1"),
        ("arrowThickness_3", "sequenceArrowThickness 3"),
        ("lifelineBorderColor_black", "sequenceLifeLineBorderColor black"),
        ("lifelineBorderColor_blue", "sequenceLifeLineBorderColor blue"),
        ("lifelineBorderThickness_1", "sequenceLifeLineBorderThickness 1"),
        ("lifelineBorderThickness_2", "sequenceLifeLineBorderThickness 2"),
        ("lifelineBackgroundColor_white", "sequenceLifeLineBackgroundColor white"),
        ("lifelineBackgroundColor_lightblue", "sequenceLifeLineBackgroundColor LightBlue"),
        ("participantBg_lightgray", "sequenceParticipantBackgroundColor LightGray"),
        ("participantBg_lightyellow", "sequenceParticipantBackgroundColor LightYellow"),
        ("participantBg_white", "sequenceParticipantBackgroundColor white"),
        ("participantBorder_black", "sequenceParticipantBorderColor black"),
        ("participantBorder_navy", "sequenceParticipantBorderColor navy"),
        ("participantBorderThickness_1", "sequenceParticipantBorderThickness 1"),
        ("participantBorderThickness_2", "sequenceParticipantBorderThickness 2"),
        ("participantFontColor_black", "sequenceParticipantFontColor black"),
        ("participantFontColor_red", "sequenceParticipantFontColor red"),
        ("participantFontSize_12", "sequenceParticipantFontSize 12"),
        ("participantFontSize_16", "sequenceParticipantFontSize 16"),
        ("participantFontStyle_bold", "sequenceParticipantFontStyle bold"),
        ("participantFontStyle_italic", "sequenceParticipantFontStyle italic"),
        ("participantPadding_5", "sequenceParticipantPadding 5"),
        ("participantPadding_15", "sequenceParticipantPadding 15"),
        ("actorBg_white", "sequenceActorBackgroundColor white"),
        ("actorBg_lightblue", "sequenceActorBackgroundColor LightBlue"),
        ("actorBorder_black", "sequenceActorBorderColor black"),
        ("actorBorder_red", "sequenceActorBorderColor red"),
        ("actorFontColor_black", "sequenceActorFontColor black"),
        ("actorFontSize_12", "sequenceActorFontSize 12"),
        ("actorFontStyle_plain", "sequenceActorFontStyle plain"),
        ("groupBg_lightgray", "sequenceGroupBackgroundColor LightGray"),
        ("groupBorder_black", "sequenceGroupBorderColor black"),
        ("groupFontColor_black", "sequenceGroupFontColor black"),
        ("groupFontSize_12", "sequenceGroupFontSize 12"),
        ("groupFontStyle_bold", "sequenceGroupFontStyle bold"),
        ("groupHeaderFontColor_black", "sequenceGroupHeaderFontColor black"),
        ("groupHeaderFontSize_12", "sequenceGroupHeaderFontSize 12"),
        ("groupHeaderFontStyle_bold", "sequenceGroupHeaderFontStyle bold"),
        ("dividerBg_lightgray", "sequenceDividerBackgroundColor LightGray"),
        ("dividerBorder_black", "sequenceDividerBorderColor black"),
        ("dividerFontColor_black", "sequenceDividerFontColor black"),
        ("dividerFontSize_12", "sequenceDividerFontSize 12"),
        ("referenceBg_white", "sequenceReferenceBackgroundColor white"),
        ("referenceBorder_black", "sequenceReferenceBorderColor black"),
        ("referenceFontColor_black", "sequenceReferenceFontColor black"),
        ("messagealign_center", "sequenceMessageAlign center"),
        ("messagealign_left", "sequenceMessageAlign left"),
        ("messagealign_right", "sequenceMessageAlign right"),
        ("numbering_true", "autonumber"),
        ("delay_shown", ""),  # handled separately
    ]

    base = """\
@startuml
skinparam {param}
participant Alice
participant Bob
Alice -> Bob : hello
activate Bob
Bob --> Alice : world
deactivate Bob
group my group
  Alice -> Bob : grouped msg
end
note over Alice : note content
@enduml
"""
    for name, param in params:
        if param:
            content = base.replace("{param}", param)
            w(SKINPARAM_DIR, f"skin_sequence_{name}.puml", content)

    # autonumber special case
    w(SKINPARAM_DIR, "skin_sequence_autonumber.puml", """\
@startuml
autonumber
Alice -> Bob : hello
Bob --> Alice : world
Alice -> Bob : again
@enduml
""")

    w(SKINPARAM_DIR, "skin_sequence_autonumber_start.puml", """\
@startuml
autonumber 10
Alice -> Bob : starting at 10
Bob --> Alice : 11
@enduml
""")

    w(SKINPARAM_DIR, "skin_sequence_autonumber_step.puml", """\
@startuml
autonumber 1 10
Alice -> Bob : 1
Bob --> Alice : 11
Alice -> Bob : 21
@enduml
""")

    w(SKINPARAM_DIR, "skin_sequence_autonumber_format.puml", """\
@startuml
autonumber "<b>[000]"
Alice -> Bob : hello
Bob --> Alice : world
@enduml
""")

def gen_skinparam_class():
    params = [
        ("bg_lightblue", "classBackgroundColor LightBlue"),
        ("bg_white", "classBackgroundColor white"),
        ("bg_hex", "classBackgroundColor #F0F8FF"),
        ("bg_gradient", "classBackgroundColor #FFFFFF/#E0E0FF"),
        ("border_black", "classBorderColor black"),
        ("border_navy", "classBorderColor navy"),
        ("border_hex", "classBorderColor #003366"),
        ("borderThickness_1", "classBorderThickness 1"),
        ("borderThickness_2", "classBorderThickness 2"),
        ("borderThickness_3", "classBorderThickness 3"),
        ("fontColor_black", "classFontColor black"),
        ("fontColor_navy", "classFontColor navy"),
        ("fontColor_hex", "classFontColor #333333"),
        ("fontSize_10", "classFontSize 10"),
        ("fontSize_14", "classFontSize 14"),
        ("fontSize_18", "classFontSize 18"),
        ("fontStyle_bold", "classFontStyle bold"),
        ("fontStyle_italic", "classFontStyle italic"),
        ("fontStyle_plain", "classFontStyle plain"),
        ("fontName_courier", "classFontName Courier"),
        ("fontName_arial", "classFontName Arial"),
        ("headerBg_lightgray", "classHeaderBackgroundColor LightGray"),
        ("headerBg_steelblue", "classHeaderBackgroundColor SteelBlue"),
        ("headerBg_hex", "classHeaderBackgroundColor #4472C4"),
        ("arrowColor_black", "classArrowColor black"),
        ("arrowColor_red", "classArrowColor red"),
        ("arrowColor_hex", "classArrowColor #003366"),
        ("arrowFontColor_black", "classArrowFontColor black"),
        ("arrowFontSize_10", "classArrowFontSize 10"),
        ("arrowFontStyle_italic", "classArrowFontStyle italic"),
        ("arrowThickness_1", "classArrowThickness 1"),
        ("arrowThickness_2", "classArrowThickness 2"),
        ("roundCorner_0", "classRoundCorner 0"),
        ("roundCorner_10", "classRoundCorner 10"),
        ("roundCorner_20", "classRoundCorner 20"),
        ("attributeIconSize_10", "classAttributeIconSize 10"),
        ("stereotype_c_bold", 'stereotype C "<< (C,#FF8C00) >>"'),
        ("attributeFontColor_red", "classAttributeFontColor red"),
        ("attributeFontSize_12", "classAttributeFontSize 12"),
        ("attributeFontStyle_italic", "classAttributeFontStyle italic"),
        ("stereotypeFontColor_gray", "stereotypeFontColor gray"),
        ("stereotypeFontSize_10", "stereotypeFontSize 10"),
        ("stereotypeFontStyle_italic", "stereotypeFontStyle italic"),
    ]

    base = """\
@startuml
skinparam {param}
class Animal {{
  -name: String
  +getName(): String
  +setName(n: String): void
}}
class Dog extends Animal {{
  +bark(): void
}}
interface Flyable {{
  +fly(): void
}}
Animal <|-- Dog
Dog ..|> Flyable
@enduml
"""
    for name, param in params:
        w(SKINPARAM_DIR, f"skin_class_{name}.puml", base.replace("{param}", param))

def gen_skinparam_activity():
    params = [
        ("bg_lightyellow", "activityBackgroundColor LightYellow"),
        ("bg_lightblue", "activityBackgroundColor LightBlue"),
        ("bg_white", "activityBackgroundColor white"),
        ("bg_hex", "activityBackgroundColor #FFFACD"),
        ("bg_gradient", "activityBackgroundColor #FFFFFF/#FFFACD"),
        ("border_black", "activityBorderColor black"),
        ("border_red", "activityBorderColor red"),
        ("border_hex", "activityBorderColor #666666"),
        ("borderThickness_1", "activityBorderThickness 1"),
        ("borderThickness_2", "activityBorderThickness 2"),
        ("fontColor_black", "activityFontColor black"),
        ("fontColor_navy", "activityFontColor navy"),
        ("fontSize_10", "activityFontSize 10"),
        ("fontSize_14", "activityFontSize 14"),
        ("fontStyle_bold", "activityFontStyle bold"),
        ("fontStyle_italic", "activityFontStyle italic"),
        ("fontName_courier", "activityFontName Courier"),
        ("arrowColor_black", "activityArrowColor black"),
        ("arrowColor_blue", "activityArrowColor blue"),
        ("arrowFontColor_black", "activityArrowFontColor black"),
        ("arrowFontSize_10", "activityArrowFontSize 10"),
        ("startColor_black", "activityStartColor black"),
        ("startColor_green", "activityStartColor green"),
        ("endColor_black", "activityEndColor black"),
        ("endColor_red", "activityEndColor red"),
        ("diamondBg_white", "activityDiamondBackgroundColor white"),
        ("diamondBg_lightyellow", "activityDiamondBackgroundColor LightYellow"),
        ("diamondFontColor_black", "activityDiamondFontColor black"),
        ("diamondFontSize_12", "activityDiamondFontSize 12"),
        ("barColor_black", "activityBarColor black"),
        ("barColor_gray", "activityBarColor gray"),
        ("roundCorner_5", "activityRoundCorner 5"),
        ("roundCorner_15", "activityRoundCorner 15"),
        ("swimlaneBg_white", "swimlaneBackgroundColor white"),
        ("swimlaneBg_lightgray", "swimlaneBackgroundColor LightGray"),
        ("swimlaneBorder_black", "swimlaneBorderColor black"),
        ("swimlaneFontColor_black", "swimlaneFontColor black"),
        ("swimlaneFontSize_12", "swimlaneFontSize 12"),
        ("swimlaneTitleFontStyle_bold", "swimlaneTitleFontStyle bold"),
    ]

    base = """\
@startuml
skinparam {param}
start
:step one;
if (condition?) then (yes)
  :true branch;
else (no)
  :false branch;
endif
:step two;
fork
  :parallel one;
fork again
  :parallel two;
end fork
stop
@enduml
"""
    for name, param in params:
        w(SKINPARAM_DIR, f"skin_activity_{name}.puml", base.replace("{param}", param))

def gen_skinparam_state():
    params = [
        ("bg_lightblue", "stateBackgroundColor LightBlue"),
        ("bg_white", "stateBackgroundColor white"),
        ("bg_hex", "stateBackgroundColor #E0F0FF"),
        ("bg_gradient", "stateBackgroundColor #FFFFFF/#D0E8FF"),
        ("border_black", "stateBorderColor black"),
        ("border_red", "stateBorderColor red"),
        ("border_hex", "stateBorderColor #003366"),
        ("borderThickness_1", "stateBorderThickness 1"),
        ("borderThickness_2", "stateBorderThickness 2"),
        ("fontColor_black", "stateFontColor black"),
        ("fontColor_navy", "stateFontColor navy"),
        ("fontSize_10", "stateFontSize 10"),
        ("fontSize_14", "stateFontSize 14"),
        ("fontStyle_bold", "stateFontStyle bold"),
        ("fontStyle_italic", "stateFontStyle italic"),
        ("fontName_courier", "stateFontName Courier"),
        ("arrowColor_black", "stateArrowColor black"),
        ("arrowColor_blue", "stateArrowColor blue"),
        ("arrowFontColor_black", "stateArrowFontColor black"),
        ("arrowFontSize_10", "stateArrowFontSize 10"),
        ("arrowFontStyle_italic", "stateArrowFontStyle italic"),
        ("startColor_black", "stateStartColor black"),
        ("startColor_navy", "stateStartColor navy"),
        ("endColor_black", "stateEndColor black"),
        ("endColor_red", "stateEndColor red"),
        ("roundCorner_0", "stateRoundCorner 0"),
        ("roundCorner_10", "stateRoundCorner 10"),
        ("attributeFontColor_gray", "stateAttributeFontColor gray"),
        ("attributeFontSize_10", "stateAttributeFontSize 10"),
    ]

    base = """\
@startuml
skinparam {param}
[*] --> Idle
Idle : entry / init
Idle --> Active : start
Active : do / process
Active --> Paused : pause
Paused --> Active : resume
Active --> [*] : stop
@enduml
"""
    for name, param in params:
        w(SKINPARAM_DIR, f"skin_state_{name}.puml", base.replace("{param}", param))

def gen_skinparam_component():
    params = [
        ("bg_lightyellow", "componentBackgroundColor LightYellow"),
        ("bg_white", "componentBackgroundColor white"),
        ("bg_hex", "componentBackgroundColor #FFFFF0"),
        ("border_black", "componentBorderColor black"),
        ("border_navy", "componentBorderColor navy"),
        ("border_hex", "componentBorderColor #003366"),
        ("borderThickness_1", "componentBorderThickness 1"),
        ("borderThickness_2", "componentBorderThickness 2"),
        ("fontColor_black", "componentFontColor black"),
        ("fontColor_navy", "componentFontColor navy"),
        ("fontSize_12", "componentFontSize 12"),
        ("fontSize_16", "componentFontSize 16"),
        ("fontStyle_bold", "componentFontStyle bold"),
        ("fontStyle_italic", "componentFontStyle italic"),
        ("arrowColor_black", "componentArrowColor black"),
        ("arrowColor_blue", "componentArrowColor blue"),
        ("arrowFontColor_black", "componentArrowFontColor black"),
        ("arrowFontSize_10", "componentArrowFontSize 10"),
        ("interfaceBg_white", "interfaceBackgroundColor white"),
        ("interfaceBorder_black", "interfaceBorderColor black"),
        ("interfaceFontColor_black", "interfaceFontColor black"),
        ("databaseBg_lightblue", "databaseBackgroundColor LightBlue"),
        ("databaseBorder_black", "databaseBorderColor black"),
        ("cloudBg_lightcyan", "cloudBackgroundColor LightCyan"),
        ("cloudBorder_black", "cloudBorderColor black"),
        ("nodeBg_lightgray", "nodeBackgroundColor LightGray"),
        ("nodeBorder_black", "nodeBorderColor black"),
        ("storageBg_lightgray", "storageBackgroundColor LightGray"),
        ("storageBorder_black", "storageBorderColor black"),
        ("frameBg_white", "frameBackgroundColor white"),
        ("frameBorder_black", "frameBorderColor black"),
        ("roundCorner_5", "componentRoundCorner 5"),
        ("roundCorner_15", "componentRoundCorner 15"),
    ]

    base = """\
@startuml
skinparam {param}
component WebApp
component Database
component Cache
interface API
WebApp --> Database : uses
WebApp --> Cache : caches
WebApp - API
@enduml
"""
    for name, param in params:
        w(SKINPARAM_DIR, f"skin_component_{name}.puml", base.replace("{param}", param))

def gen_skinparam_usecase():
    params = [
        ("bg_lightyellow", "usecaseBackgroundColor LightYellow"),
        ("bg_white", "usecaseBackgroundColor white"),
        ("bg_hex", "usecaseBackgroundColor #FAFAD2"),
        ("bg_gradient", "usecaseBackgroundColor #FFFFFF/#FAFAD2"),
        ("border_black", "usecaseBorderColor black"),
        ("border_navy", "usecaseBorderColor navy"),
        ("borderThickness_1", "usecaseBorderThickness 1"),
        ("borderThickness_2", "usecaseBorderThickness 2"),
        ("fontColor_black", "usecaseFontColor black"),
        ("fontSize_12", "usecaseFontSize 12"),
        ("fontSize_16", "usecaseFontSize 16"),
        ("fontStyle_bold", "usecaseFontStyle bold"),
        ("fontStyle_italic", "usecaseFontStyle italic"),
        ("actorBg_white", "actorBackgroundColor white"),
        ("actorBg_lightblue", "actorBackgroundColor LightBlue"),
        ("actorBorder_black", "actorBorderColor black"),
        ("actorFontColor_black", "actorFontColor black"),
        ("actorFontSize_12", "actorFontSize 12"),
        ("actorFontStyle_plain", "actorFontStyle plain"),
        ("arrowColor_black", "usecaseArrowColor black"),
        ("arrowColor_blue", "usecaseArrowColor blue"),
        ("arrowFontColor_black", "usecaseArrowFontColor black"),
        ("arrowFontSize_10", "usecaseArrowFontSize 10"),
        ("borderBg_lightgray", "usecaseBorderColor LightGray"),
        ("systemBg_white", "rectangleBackgroundColor white"),
        ("systemBorder_black", "rectangleBorderColor black"),
    ]

    base = """\
@startuml
skinparam {param}
actor Customer
actor Admin
rectangle System {{
  usecase "Login" as UC1
  usecase "Logout" as UC2
  usecase "Browse" as UC3
  usecase "Manage Users" as UC4
}}
Customer --> UC1
Customer --> UC2
Customer --> UC3
Admin --> UC4
UC1 .> UC2 : includes
@enduml
"""
    for name, param in params:
        w(SKINPARAM_DIR, f"skin_usecase_{name}.puml", base.replace("{param}", param))

def gen_skinparam_note():
    params = [
        ("bg_yellow", "noteBackgroundColor yellow"),
        ("bg_white", "noteBackgroundColor white"),
        ("bg_lightblue", "noteBackgroundColor LightBlue"),
        ("bg_hex", "noteBackgroundColor #FFFACD"),
        ("bg_gradient", "noteBackgroundColor #FFFACD/#FFFFFF"),
        ("border_black", "noteBorderColor black"),
        ("border_red", "noteBorderColor red"),
        ("border_hex", "noteBorderColor #999900"),
        ("borderThickness_1", "noteBorderThickness 1"),
        ("borderThickness_2", "noteBorderThickness 2"),
        ("fontColor_black", "noteFontColor black"),
        ("fontColor_navy", "noteFontColor navy"),
        ("fontSize_10", "noteFontSize 10"),
        ("fontSize_14", "noteFontSize 14"),
        ("fontStyle_italic", "noteFontStyle italic"),
        ("fontStyle_bold", "noteFontStyle bold"),
        ("fontName_courier", "noteFontName Courier"),
        ("shadowing_true", "noteShadowing true"),
        ("shadowing_false", "noteShadowing false"),
        ("roundCorner_0", "noteRoundCorner 0"),
        ("roundCorner_5", "noteRoundCorner 5"),
        ("roundCorner_10", "noteRoundCorner 10"),
        ("textAlignment_left", "noteTextAlignment left"),
        ("textAlignment_center", "noteTextAlignment center"),
    ]

    base = """\
@startuml
skinparam {param}
note as N1
  This is a floating note
  with multiple lines
end note
note as N2
  Another note
end note
@enduml
"""
    for name, param in params:
        w(SKINPARAM_DIR, f"skin_note_{name}.puml", base.replace("{param}", param))

def gen_skinparam_package():
    params = [
        ("bg_lightyellow", "packageBackgroundColor LightYellow"),
        ("bg_white", "packageBackgroundColor white"),
        ("bg_hex", "packageBackgroundColor #F5F5DC"),
        ("border_black", "packageBorderColor black"),
        ("border_navy", "packageBorderColor navy"),
        ("borderThickness_1", "packageBorderThickness 1"),
        ("borderThickness_2", "packageBorderThickness 2"),
        ("fontColor_black", "packageFontColor black"),
        ("fontSize_12", "packageFontSize 12"),
        ("fontSize_16", "packageFontSize 16"),
        ("fontStyle_bold", "packageFontStyle bold"),
        ("fontStyle_italic", "packageFontStyle italic"),
        ("fontName_arial", "packageFontName Arial"),
        ("style_default", "packageStyle default"),
        ("style_node", "packageStyle node"),
        ("style_card", "packageStyle card"),
        ("style_cloud", "packageStyle cloud"),
        ("style_database", "packageStyle database"),
        ("style_frame", "packageStyle frame"),
        ("style_rectangle", "packageStyle rectangle"),
        ("style_folder", "packageStyle folder"),
    ]

    base = """\
@startuml
skinparam {param}
package "Package A" {{
  class ClassA
  class ClassB
}}
package "Package B" {{
  class ClassC
  class ClassD
}}
ClassA --> ClassC
@enduml
"""
    for name, param in params:
        w(SKINPARAM_DIR, f"skin_package_{name}.puml", base.replace("{param}", param))

def gen_skinparam_linetype():
    w(SKINPARAM_DIR, "skin_linetype_ortho_class.puml", """\
@startuml
skinparam linetype ortho
class A
class B
class C
A --> B
B --> C
A --> C
@enduml
""")

    w(SKINPARAM_DIR, "skin_linetype_polyline_class.puml", """\
@startuml
skinparam linetype polyline
class A
class B
class C
A --> B
B --> C
A --> C
@enduml
""")

    w(SKINPARAM_DIR, "skin_linetype_ortho_component.puml", """\
@startuml
skinparam linetype ortho
component A
component B
component C
A --> B
B --> C
A --> C
@enduml
""")

def gen_skinparam_style():
    w(SKINPARAM_DIR, "skin_style_strictuml_class.puml", """\
@startuml
skinparam style strictuml
class Animal {
  +name: String
}
class Dog extends Animal {
  +bark(): void
}
@enduml
""")

def gen_skinparam_stereotype():
    stereotypes = [
        ("entity", "<<entity>>", "LightGreen", "DarkGreen"),
        ("service", "<<service>>", "LightBlue", "Navy"),
        ("repository", "<<repository>>", "LightYellow", "DarkOrange"),
        ("controller", "<<controller>>", "LightCoral", "DarkRed"),
        ("valueobject", "<<vo>>", "LightPink", "DarkMagenta"),
    ]

    for name, stereo, bg, border in stereotypes:
        w(SKINPARAM_DIR, f"skin_stereotype_{name}.puml", f"""\
@startuml
skinparam class{stereo} {{
  BackgroundColor {bg}
  BorderColor {border}
  FontStyle bold
}}
class UserEntity {stereo} {{
  id: Long
  name: String
}}
class OrderEntity {stereo} {{
  id: Long
  total: Double
}}
UserEntity --> OrderEntity : places
@enduml
""")

def gen_skinparam_combined():
    combos = [
        ("sequence_themed", """\
skinparam sequenceArrowColor #336699
skinparam sequenceArrowFontColor #336699
skinparam sequenceParticipantBackgroundColor #E8F4F8
skinparam sequenceParticipantBorderColor #336699
skinparam sequenceLifeLineBorderColor #336699
skinparam noteBackgroundColor #FFFACD
skinparam noteBorderColor #999900
"""),
        ("class_themed", """\
skinparam classBackgroundColor #F0F8FF
skinparam classBorderColor #003366
skinparam classHeaderBackgroundColor #003366
skinparam classFontColor #003366
skinparam classArrowColor #003366
skinparam roundCorner 10
"""),
        ("dark_theme", """\
skinparam backgroundColor #1a1a2e
skinparam classBackgroundColor #16213e
skinparam classBorderColor #0f3460
skinparam classFontColor #e94560
skinparam arrowColor #e94560
skinparam noteBackgroundColor #16213e
skinparam noteFontColor #e0e0e0
"""),
        ("pastel_theme", """\
skinparam backgroundColor #FFF5E4
skinparam classBackgroundColor #FFE3E1
skinparam classBorderColor #FF9B9B
skinparam sequenceParticipantBackgroundColor #FFE3E1
skinparam sequenceParticipantBorderColor #FF9B9B
skinparam noteBackgroundColor #FFF5E4
skinparam noteBorderColor #FF9B9B
"""),
        ("monochrome_styled", """\
skinparam monochrome true
skinparam shadowing false
skinparam defaultFontName Courier
skinparam defaultFontSize 12
"""),
        ("big_font", """\
skinparam defaultFontSize 18
skinparam defaultFontStyle bold
skinparam classHeaderBackgroundColor LightBlue
skinparam roundCorner 15
"""),
        ("thin_lines", """\
skinparam classBorderThickness 1
skinparam arrowThickness 1
skinparam shadowing false
"""),
        ("thick_lines", """\
skinparam classBorderThickness 3
skinparam arrowThickness 3
skinparam shadowing true
"""),
        ("gradient_all", """\
skinparam classBackgroundColor #FFFFFF/#E0E8FF
skinparam classHeaderBackgroundColor #4472C4/#2E4A8A
skinparam noteBackgroundColor #FFFACD/#FFE680
"""),
        ("compact_padding", """\
skinparam padding 2
skinparam nodesep 10
skinparam ranksep 20
"""),
    ]

    class_diag = """\
class Animal {
  +name: String
  +getName(): String
}
class Dog extends Animal {
  +bark(): void
}
Animal <|-- Dog
"""

    seq_diag = """\
Alice -> Bob : request
Bob --> Alice : response
note right : note
"""

    for name, params in combos:
        w(SKINPARAM_DIR, f"skin_combined_{name}_class.puml",
          f"@startuml\n{params}\n{class_diag}@enduml\n")
        w(SKINPARAM_DIR, f"skin_combined_{name}_sequence.puml",
          f"@startuml\n{params}\n{seq_diag}@enduml\n")

def gen_skinparam_gradient():
    colors = [
        ("blue_white", "#4472C4/#FFFFFF", "#4472C4"),
        ("green_white", "#70AD47/#FFFFFF", "#70AD47"),
        ("orange_white", "#ED7D31/#FFFFFF", "#ED7D31"),
        ("red_white", "#FF0000/#FFFFFF", "#FF0000"),
        ("yellow_white", "#FFD700/#FFFFFF", "#DAA520"),
        ("vertical_blue", "#FFFFFF|#4472C4", "#4472C4"),
        ("diagonal_green", "#FFFFFF\\#70AD47", "#70AD47"),
    ]

    for name, bg, border in colors:
        w(SKINPARAM_DIR, f"skin_gradient_{name}_class.puml", f"""\
@startuml
skinparam classBackgroundColor {bg}
skinparam classBorderColor {border}
class GradientClass {{
  field: String
  method(): void
}}
@enduml
""")
        w(SKINPARAM_DIR, f"skin_gradient_{name}_sequence.puml", f"""\
@startuml
skinparam sequenceParticipantBackgroundColor {bg}
skinparam sequenceParticipantBorderColor {border}
Alice -> Bob : gradient test
@enduml
""")

def gen_skinparam_arrow_types():
    arrow_styles = [
        ("color_variants", [
            ("red", "red"), ("blue", "blue"), ("green", "green"),
            ("orange", "orange"), ("purple", "purple"),
        ]),
        ("thickness_variants", None),
    ]

    for color_name, color in [("red", "red"), ("blue", "blue"), ("green", "green"),
                               ("orange", "orange"), ("purple", "#800080"),
                               ("navy", "navy"), ("teal", "teal")]:
        w(SKINPARAM_DIR, f"skin_arrow_color_{color_name}.puml", f"""\
@startuml
skinparam arrowColor {color}
A -> B : solid
A --> B : dashed
A ->> B : thin
@enduml
""")

    for thick in [1, 2, 3, 4, 5]:
        w(SKINPARAM_DIR, f"skin_arrow_thickness_{thick}.puml", f"""\
@startuml
skinparam arrowThickness {thick}
A -> B : arrow thickness {thick}
@enduml
""")

# ---------------------------------------------------------------------------
# PREPROCESSING
# ---------------------------------------------------------------------------

def gen_preproc_define():
    w(PREPROC_DIR, "preproc_define_simple.puml", """\
@startuml
!define ACTOR Alice
!define TARGET Bob
ACTOR -> TARGET : hello
@enduml
""")

    w(PREPROC_DIR, "preproc_define_with_args.puml", """\
@startuml
!define ARROW(from, to, msg) from -> to : msg
ARROW(Alice, Bob, hello)
ARROW(Bob, Alice, world)
@enduml
""")

    w(PREPROC_DIR, "preproc_define_class_shorthand.puml", """\
@startuml
!define CLASS(name) class name {
!define FIELD(type, name) name: type
!define END_CLASS }
CLASS(MyClass)
  FIELD(String, name)
  FIELD(int, age)
END_CLASS
@enduml
""")

    w(PREPROC_DIR, "preproc_define_color.puml", """\
@startuml
!define PRIMARY_COLOR #4472C4
!define SECONDARY_COLOR #ED7D31

skinparam classBackgroundColor PRIMARY_COLOR
skinparam classBorderColor SECONDARY_COLOR

class ColoredClass {
  field: String
}
@enduml
""")

    w(PREPROC_DIR, "preproc_define_numeric.puml", """\
@startuml
!define FONT_SIZE 14
!define BORDER_THICK 2

skinparam defaultFontSize FONT_SIZE
skinparam classBorderThickness BORDER_THICK

class NumericClass {
  value: int
}
@enduml
""")

    w(PREPROC_DIR, "preproc_define_redefine.puml", """\
@startuml
!define COLOR red
skinparam classBackgroundColor COLOR
class First {}
!define COLOR blue
skinparam classBackgroundColor COLOR
class Second {}
@enduml
""")

    w(PREPROC_DIR, "preproc_undef.puml", """\
@startuml
!define MACRO defined_value
note as N1
  MACRO is defined here
end note
!undef MACRO
note as N2
  MACRO is undefined here (shows literal)
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_definelong_simple.puml", """\
@startuml
!definelong SIGNATURE(name, ret)
  +name(): ret
!enddefinelong

class MyClass {
  SIGNATURE(getName, String)
  SIGNATURE(getAge, int)
  SIGNATURE(isActive, boolean)
}
@enduml
""")

    w(PREPROC_DIR, "preproc_definelong_multiline.puml", """\
@startuml
!definelong PERSON_CLASS(name, field1, field2)
class name {
  +field1: String
  +field2: int
  +toString(): String
}
!enddefinelong

PERSON_CLASS(Customer, customerName, customerId)
PERSON_CLASS(Employee, employeeName, employeeId)
Customer --> Employee : managed by
@enduml
""")

def gen_preproc_ifdef():
    w(PREPROC_DIR, "preproc_ifdef_basic.puml", """\
@startuml
!define SHOW_DETAILS

!ifdef SHOW_DETAILS
class DetailedClass {
  +name: String
  +age: int
  +email: String
}
!else
class SimpleClass {
  +name: String
}
!endif
@enduml
""")

    w(PREPROC_DIR, "preproc_ifdef_not_defined.puml", """\
@startuml
!ifdef UNDEFINED_MACRO
class ShouldNotAppear {
  +field: String
}
!else
class ShouldAppear {
  +visible: boolean
}
!endif
@enduml
""")

    w(PREPROC_DIR, "preproc_ifndef_basic.puml", """\
@startuml
!ifndef PRODUCTION
skinparam noteBackgroundColor LightYellow
note as Dev
  Development mode
end note
!endif
Alice -> Bob : hello
@enduml
""")

    w(PREPROC_DIR, "preproc_ifndef_with_define.puml", """\
@startuml
!ifndef MAX_SIZE
!define MAX_SIZE 100
!endif

note as N
  MAX_SIZE = MAX_SIZE
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_ifdef_nested.puml", """\
@startuml
!define LEVEL1
!define LEVEL2

!ifdef LEVEL1
  !ifdef LEVEL2
    class BothDefined {
      level: int = 2
    }
  !else
    class OnlyLevel1 {
      level: int = 1
    }
  !endif
!else
  class NeitherDefined {
    level: int = 0
  }
!endif
@enduml
""")

    w(PREPROC_DIR, "preproc_ifdef_chained.puml", """\
@startuml
!define MODE_B

!ifdef MODE_A
note as N
  Mode A active
end note
!else ifdef MODE_B
note as N
  Mode B active
end note
!else
note as N
  No mode active
end note
!endif
@enduml
""")

    w(PREPROC_DIR, "preproc_ifdef_diagram_type.puml", """\
@startuml
!define SEQUENCE_DIAGRAM

!ifdef SEQUENCE_DIAGRAM
Alice -> Bob : sequence mode
Bob --> Alice : reply
!else
class NonSequenceClass {
  mode: String = "class"
}
!endif
@enduml
""")

def gen_preproc_variables():
    w(PREPROC_DIR, "preproc_variable_string.puml", """\
@startuml
!$title = "My Diagram"
!$author = "John Doe"
!$version = "1.0"

title $title
header Author: $author (v$version)
Alice -> Bob : hello
@enduml
""")

    w(PREPROC_DIR, "preproc_variable_arithmetic.puml", """\
@startuml
!$x = 10
!$y = 20
!$sum = $x + $y
!$product = $x * $y

note as N
  x = $x
  y = $y
  sum = $sum
  product = $product
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_variable_string_concat.puml", """\
@startuml
!$first = "Hello"
!$second = "World"
!$greeting = $first + " " + $second

note as N
  $greeting
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_variable_boolean.puml", """\
@startuml
!$enabled = %true()
!$disabled = %false()

note as N
  enabled = $enabled
  disabled = $disabled
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_variable_in_skinparam.puml", """\
@startuml
!$bg_color = "LightBlue"
!$border_color = "Navy"

skinparam classBackgroundColor $bg_color
skinparam classBorderColor $border_color

class StyledClass {
  field: String
}
@enduml
""")

    w(PREPROC_DIR, "preproc_variable_reassign.puml", """\
@startuml
!$counter = 0
!$counter = $counter + 1
!$counter = $counter + 1
!$counter = $counter + 1

note as N
  counter = $counter
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_variable_interpolation_in_label.puml", """\
@startuml
!$name = "Alice"
!$role = "sender"

participant "$name ($role)" as P
P -> Bob : from $name
@enduml
""")

def gen_preproc_functions():
    w(PREPROC_DIR, "preproc_function_simple.puml", """\
@startuml
!function $double($x)
  !return $x * 2
!endfunction

!$val = $double(5)
note as N
  double(5) = $val
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_function_with_string.puml", """\
@startuml
!function $greet($name)
  !return "Hello, " + $name + "!"
!endfunction

note as N
  $greet("World")
  $greet("PlantUML")
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_function_multiarg.puml", """\
@startuml
!function $add($a, $b)
  !return $a + $b
!endfunction

!function $multiply($a, $b)
  !return $a * $b
!endfunction

!$r1 = $add(3, 4)
!$r2 = $multiply(3, 4)
!$r3 = $add($r1, $r2)

note as N
  add(3,4) = $r1
  multiply(3,4) = $r2
  add($r1,$r2) = $r3
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_function_default_arg.puml", """\
@startuml
!function $greet($name = "World")
  !return "Hello, " + $name + "!"
!endfunction

note as N
  $greet()
  $greet("Alice")
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_function_nested_call.puml", """\
@startuml
!function $double($x)
  !return $x * 2
!endfunction

!function $quadruple($x)
  !return $double($double($x))
!endfunction

!$result = $quadruple(3)
note as N
  quadruple(3) = $result
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_function_local_var.puml", """\
@startuml
!function $compute($x)
  !local $temp = $x * $x
  !return $temp + $x
!endfunction

!$r = $compute(5)
note as N
  compute(5) = $r (= 5*5 + 5 = 30)
end note
@enduml
""")

def gen_preproc_procedures():
    w(PREPROC_DIR, "preproc_procedure_simple.puml", """\
@startuml
!procedure $box($name)
  class $name {
    +id: long
    +name: String
  }
!endprocedure

$box(Customer)
$box(Product)
$box(Order)
@enduml
""")

    w(PREPROC_DIR, "preproc_procedure_with_relationship.puml", """\
@startuml
!procedure $entity($name, $parent)
  class $name extends $parent {
    +id: long
  }
!endprocedure

class BaseEntity {
  +createdAt: Date
  +updatedAt: Date
}
$entity(User, BaseEntity)
$entity(Post, BaseEntity)
$entity(Comment, BaseEntity)
@enduml
""")

    w(PREPROC_DIR, "preproc_procedure_sequence.puml", """\
@startuml
!procedure $request($from, $to, $msg)
  $from -> $to : $msg
  activate $to
  $to --> $from : response
  deactivate $to
!endprocedure

$request(Client, Server, "GET /users")
$request(Client, Server, "POST /users")
$request(Client, Server, "DELETE /users/1")
@enduml
""")

def gen_preproc_builtins():
    w(PREPROC_DIR, "preproc_builtin_strlen.puml", """\
@startuml
!$str = "Hello, World!"
!$len = %strlen($str)

note as N
  string = "$str"
  length = $len
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_builtin_substr.puml", """\
@startuml
!$str = "Hello, World!"
!$sub = %substr($str, 7, 5)

note as N
  original = "$str"
  substr(7,5) = "$sub"
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_builtin_intval.puml", """\
@startuml
!$str_num = "42"
!$int_val = %intval($str_num)
!$result = $int_val + 8

note as N
  intval("42") = $int_val
  42 + 8 = $result
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_builtin_true_false.puml", """\
@startuml
!$t = %true()
!$f = %false()
!$not_true = !$t
!$not_false = !$f

note as N
  true = $t
  false = $f
  !true = $not_true
  !false = $not_false
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_builtin_string_funcs.puml", """\
@startuml
!$s = "Hello World"
!$upper = %upper($s)
!$lower = %lower($s)

note as N
  original: $s
  upper: $upper
  lower: $lower
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_builtin_date.puml", """\
@startuml
!$now = %date("yyyy-MM-dd")

header Generated: $now
Alice -> Bob : dated diagram
@enduml
""")

    w(PREPROC_DIR, "preproc_builtin_is_defined.puml", """\
@startuml
!define MY_MACRO

!if %is_defined("MY_MACRO")
note as N1
  MY_MACRO is defined
end note
!endif

!if !%is_defined("MISSING_MACRO")
note as N2
  MISSING_MACRO is not defined
end note
!endif
@enduml
""")

    w(PREPROC_DIR, "preproc_builtin_filename.puml", """\
@startuml
note as N
  file: %filename()
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_builtin_chr.puml", """\
@startuml
!$newline = %chr(10)
!$tab = %chr(9)
!$space = %chr(32)

note as N
  ascii 65 = %chr(65)
  ascii 97 = %chr(97)
end note
@enduml
""")

def gen_preproc_conditionals():
    w(PREPROC_DIR, "preproc_if_comparison.puml", """\
@startuml
!$x = 10
!if $x > 5
note as N
  x ($x) is greater than 5
end note
!endif
@enduml
""")

    w(PREPROC_DIR, "preproc_if_string_eq.puml", """\
@startuml
!$mode = "production"

!if $mode == "production"
skinparam backgroundColor #FAFAFA
Alice -> Bob : Production mode
!else
skinparam backgroundColor LightYellow
Alice -> Bob : Other mode: $mode
!endif
@enduml
""")

    w(PREPROC_DIR, "preproc_if_elseif.puml", """\
@startuml
!$level = 2

!if $level == 1
note as N
  Level 1
end note
!elseif $level == 2
note as N
  Level 2
end note
!elseif $level == 3
note as N
  Level 3
end note
!else
note as N
  Unknown level: $level
end note
!endif
@enduml
""")

    w(PREPROC_DIR, "preproc_if_and_or.puml", """\
@startuml
!$x = 5
!$y = 10

!if ($x > 0) && ($y > 0)
note as N1
  Both positive
end note
!endif

!if ($x > 100) || ($y > 5)
note as N2
  At least one large
end note
!endif
@enduml
""")

    w(PREPROC_DIR, "preproc_if_not.puml", """\
@startuml
!$debug = %false()

!if !$debug
Alice -> Bob : release mode
!else
Alice -> Bob : debug mode
!endif
@enduml
""")

def gen_preproc_while():
    w(PREPROC_DIR, "preproc_while_counter.puml", """\
@startuml
!$i = 1
!while $i <= 3
  participant "P$i" as P$i
  !$i = $i + 1
!endwhile

P1 -> P2 : step 1
P2 -> P3 : step 2
@enduml
""")

    w(PREPROC_DIR, "preproc_while_classes.puml", """\
@startuml
!$count = 1
!while $count <= 4
  class Entity$count {
    +id: long
    +name: String
  }
  !$count = $count + 1
!endwhile
@enduml
""")

    w(PREPROC_DIR, "preproc_while_states.puml", """\
@startuml
!$n = 1
[*] --> State1
!while $n < 5
  !$next = $n + 1
  State$n --> State$next : transition $n
  !$n = $next
!endwhile
State5 --> [*]
@enduml
""")

def gen_preproc_include():
    # Self-contained includes using !startsub/!endsub
    w(PREPROC_DIR, "preproc_include_startsub.puml", """\
@startuml
!startsub HEADER_PART
header Included Header
!endsub
!startsub BODY_PART
Alice -> Bob : from included sub
!endsub
!includesub preproc_include_startsub.puml!HEADER_PART
!includesub preproc_include_startsub.puml!BODY_PART
@enduml
""")

    # File-level include pair
    w(PREPROC_DIR, "preproc_include_base.iuml", """\
!startsub BASE_STYLES
skinparam classBackgroundColor LightBlue
skinparam classBorderColor Navy
!endsub
!startsub BASE_CLASSES
class BaseEntity {
  +id: Long
  +createdAt: Date
}
!endsub
""")

    w(PREPROC_DIR, "preproc_include_main.puml", """\
@startuml
!includesub preproc_include_base.iuml!BASE_STYLES
!includesub preproc_include_base.iuml!BASE_CLASSES
class User extends BaseEntity {
  +name: String
  +email: String
}
@enduml
""")

    w(PREPROC_DIR, "preproc_include_theme_file.iuml", """\
!startsub DARK_THEME
skinparam backgroundColor #1E1E2E
skinparam classBackgroundColor #313244
skinparam classBorderColor #89DCEB
skinparam classFontColor #CDD6F4
skinparam arrowColor #89DCEB
!endsub
!startsub LIGHT_THEME
skinparam backgroundColor #EFF1F5
skinparam classBackgroundColor #FFFFFF
skinparam classBorderColor #4C4F69
skinparam classFontColor #4C4F69
skinparam arrowColor #4C4F69
!endsub
""")

    w(PREPROC_DIR, "preproc_include_dark_theme.puml", """\
@startuml
!includesub preproc_include_theme_file.iuml!DARK_THEME
class DarkClass {
  +field: String
  +method(): void
}
@enduml
""")

    w(PREPROC_DIR, "preproc_include_light_theme.puml", """\
@startuml
!includesub preproc_include_theme_file.iuml!LIGHT_THEME
class LightClass {
  +field: String
  +method(): void
}
@enduml
""")

def gen_preproc_pragma():
    w(PREPROC_DIR, "preproc_pragma_teoz.puml", """\
@startuml
!pragma teoz true
Alice -> Bob : first
Bob -> Carol : second
Carol --> Alice : done
@enduml
""")

    w(PREPROC_DIR, "preproc_pragma_horizontal_line.puml", """\
@startuml
!pragma horizontalLineBetweenDifferentPackageAllowed
package A {
  class ClassA
}
package B {
  class ClassB
}
ClassA --> ClassB
@enduml
""")

    w(PREPROC_DIR, "preproc_pragma_useverticalif.puml", """\
@startuml
!pragma useVerticalIf on
!if %true()
  note as N1
    vertical if true
  end note
!endif
@enduml
""")

    w(PREPROC_DIR, "preproc_pragma_svgsize.puml", """\
@startuml
!pragma svgSize 800 600
Alice -> Bob : sized SVG
@enduml
""")

def gen_preproc_theme():
    themes = [
        "cerulean", "cerulean-outline", "crt-amber", "crt-green",
        "mars", "minty", "sketchy", "sketchy-outline",
        "spacelab", "superhero", "toy", "united", "vibrant",
        "materia", "sandstone", "slate", "solar", "cyborg",
        "darkly", "flatly", "journal", "litera", "lumen",
        "lux", "maia", "minty", "morph", "pulse", "quartz",
        "simplex", "sketchy", "spacelab", "superhero", "yeti",
        "zephyr",
    ]

    diagram_types = [
        ("sequence", "Alice -> Bob : hello\\nBob --> Alice : world"),
        ("class", "class Animal {\\n  +name: String\\n}\\nclass Dog extends Animal {}"),
        ("activity", "start\\n:step one;\\n:step two;\\nstop"),
    ]

    seen = set()
    for theme in themes:
        if theme in seen:
            continue
        seen.add(theme)
        for diag_type, diag_body in diagram_types:
            w(PREPROC_DIR, f"preproc_theme_{theme}_{diag_type}.puml", f"""\
@startuml
!theme {theme}
{diag_body.replace(chr(92) + 'n', chr(10))}
@enduml
""")

def gen_preproc_log_assert():
    w(PREPROC_DIR, "preproc_log_simple.puml", """\
@startuml
!$val = 42
!log "val = " + $val
note as N
  val = $val
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_assert_true.puml", """\
@startuml
!$x = 5
!assert $x > 0 : "x must be positive"
note as N
  assertion passed: x = $x
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_log_multiple.puml", """\
@startuml
!$a = "first"
!$b = "second"
!log "Processing: " + $a
!log "Then: " + $b
Alice -> Bob : $a
Bob -> Carol : $b
@enduml
""")

def gen_preproc_complex():
    w(PREPROC_DIR, "preproc_complex_crud_generator.puml", """\
@startuml
!function $entity($name)
  !return $name + "Entity"
!endfunction

!function $repo($name)
  !return $name + "Repository"
!endfunction

!procedure $crud_stack($name)
  class $entity($name) {
    +id: Long
    +name: String
    +createdAt: Date
  }
  interface $repo($name) {
    +findById(id: Long): $entity($name)
    +save(e: $entity($name)): void
    +delete(id: Long): void
  }
  $repo($name) ..> $entity($name) : manages
!endprocedure

$crud_stack(User)
$crud_stack(Product)
$crud_stack(Order)

UserEntity --> OrderEntity : places
OrderEntity --> ProductEntity : contains
@enduml
""")

    w(PREPROC_DIR, "preproc_complex_conditional_diagram.puml", """\
@startuml
!define SHOW_ABSTRACT
!define USE_COLORS

!ifdef USE_COLORS
skinparam classBackgroundColor #E8F4F8
skinparam classBorderColor #336699
skinparam classArrowColor #336699
!else
skinparam monochrome true
!endif

!ifdef SHOW_ABSTRACT
abstract class Shape {
  +area(): double
  +perimeter(): double
}
!else
class Shape {
  +area(): double
}
!endif

class Circle extends Shape {
  +radius: double
  +area(): double
}
class Rectangle extends Shape {
  +width: double
  +height: double
  +area(): double
}
@enduml
""")

    w(PREPROC_DIR, "preproc_complex_loop_sequence.puml", """\
@startuml
!$services = 3
!$i = 1
!while $i <= $services
  participant "Service$i" as S$i
  !$i = $i + 1
!endwhile

Client -> S1 : request
!$i = 1
!while $i < $services
  !$next = $i + 1
  S$i -> S$next : forward
  !$i = $next
!endwhile
S3 --> Client : response
@enduml
""")

    w(PREPROC_DIR, "preproc_complex_mixed_defines_functions.puml", """\
@startuml
!define VERSION "2.0"
!define MAX_DEPTH 3
!$base_color = "#4472C4"

!function $version_label()
  !return "v" + VERSION
!endfunction

!function $depth_check($d)
  !return $d <= MAX_DEPTH
!endfunction

skinparam classBackgroundColor $base_color

title System $version_label()
header MAX_DEPTH = MAX_DEPTH

class Root {
  +depth: int = 0
}

!$d = 1
!while $depth_check($d)
  class Level$d {
    +depth: int = $d
  }
  !if $d == 1
    Root --> Level1 : child
  !else
    !$prev = $d - 1
    Level$prev --> Level$d : child
  !endif
  !$d = $d + 1
!endwhile
@enduml
""")

    w(PREPROC_DIR, "preproc_complex_theme_override.puml", """\
@startuml
!theme cerulean

' Override specific theme settings
skinparam classBorderColor #FF0000
skinparam classArrowColor #FF0000

class OverriddenClass {
  +customField: String
}
class AnotherClass {
  +otherField: int
}
OverriddenClass --> AnotherClass
@enduml
""")

def gen_preproc_variable_types():
    w(PREPROC_DIR, "preproc_vartype_integer_ops.puml", """\
@startuml
!$a = 15
!$b = 4
!$add = $a + $b
!$sub = $a - $b
!$mul = $a * $b
!$div = $a / $b
!$mod = $a % $b

note as N
  $a + $b = $add
  $a - $b = $sub
  $a * $b = $mul
  $a / $b = $div
  $a % $b = $mod
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_vartype_string_ops.puml", """\
@startuml
!$hello = "Hello"
!$world = "World"
!$greeting = $hello + ", " + $world + "!"
!$len = %strlen($greeting)

note as N
  $greeting
  length: $len
end note
@enduml
""")

    w(PREPROC_DIR, "preproc_vartype_boolean_ops.puml", """\
@startuml
!$t = %true()
!$f = %false()
!$and = $t && $f
!$or = $t || $f
!$not_t = !$t

note as N
  true && false = $and
  true || false = $or
  !true = $not_t
end note
@enduml
""")

def gen_preproc_extra():
    # Multiple !define/!undef in sequence
    for i in range(1, 11):
        w(PREPROC_DIR, f"preproc_define_variant_{i:02d}.puml", f"""\
@startuml
!define VARIANT {i}
!define LABEL "Variant VARIANT"

class Class{i} {{
  variant: int = VARIANT
  label: String = LABEL
}}
@enduml
""")

    # Multiple themes x diagram types
    themes_extra = ["superhero", "mars", "minty", "cerulean", "united"]
    for theme in themes_extra:
        w(PREPROC_DIR, f"preproc_theme_{theme}_state.puml", f"""\
@startuml
!theme {theme}
[*] --> Idle
Idle --> Running : start
Running --> Stopped : stop
Stopped --> [*]
@enduml
""")
        w(PREPROC_DIR, f"preproc_theme_{theme}_component.puml", f"""\
@startuml
!theme {theme}
component WebApp
component Database
WebApp --> Database : queries
@enduml
""")
        w(PREPROC_DIR, f"preproc_theme_{theme}_usecase.puml", f"""\
@startuml
!theme {theme}
actor User
usecase "Login" as UC1
usecase "Browse" as UC2
User --> UC1
User --> UC2
@enduml
""")

def gen_extra_skinparams():
    """Generate additional skinparam variants to hit target counts."""
    # Boundary / Participant types in sequence
    participant_types = [
        ("actor", "actor"),
        ("boundary", "boundary"),
        ("control", "control"),
        ("entity", "entity"),
        ("database", "database"),
        ("collections", "collections"),
        ("queue", "queue"),
    ]

    for ptype, keyword in participant_types:
        for color in ["LightBlue", "LightGreen", "LightYellow", "LightCoral", "White"]:
            safe_color = color.lower().replace(" ", "_")
            w(SKINPARAM_DIR, f"skin_sequence_{ptype}_{safe_color}.puml", f"""\
@startuml
skinparam {keyword}BackgroundColor {color}
{keyword} Alice
Alice -> Bob : hello
@enduml
""")

    # Class diagram visibility icons
    for vis in ["public", "private", "protected", "package"]:
        for color in ["red", "blue", "green", "black"]:
            w(SKINPARAM_DIR, f"skin_class_attr_{vis}_icon_{color}.puml", f"""\
@startuml
skinparam classAttributeIconSize 12
class MyClass {{
  +publicField: String
  -privateField: int
  #protectedField: boolean
  ~packageField: Date
}}
@enduml
""")
            break  # just one color per visibility to keep it manageable

    # Combinations of roundCorner with different types
    for elem in ["class", "note", "state", "activity"]:
        for corner in [0, 5, 10, 15, 20, 25]:
            w(SKINPARAM_DIR, f"skin_roundcorner_{elem}_{corner}.puml", f"""\
@startuml
skinparam {elem}RoundCorner {corner}
{"class RoundClass { +field: String }" if elem == "class" else ""}
{"note as N\\n  rounded note\\nend note" if elem == "note" else ""}
{"[*] --> S\\nstate S" if elem == "state" else ""}
{"start\\n:action;\\nstop" if elem == "activity" else ""}
@enduml
""")

    # Font size sweep
    for diag_type in ["class", "sequence", "activity", "state"]:
        for size in [8, 10, 12, 14, 16, 18, 20, 24]:
            diag_content = {
                "class": "class FontClass {\n  +field: String\n  +method(): void\n}",
                "sequence": "Alice -> Bob : font test\nnote right : sized",
                "activity": "start\n:action with text;\nstop",
                "state": "[*] --> State\nState : description\nState --> [*]",
            }[diag_type]
            w(SKINPARAM_DIR, f"skin_fontsize_{diag_type}_{size}.puml", f"""\
@startuml
skinparam defaultFontSize {size}
{diag_content}
@enduml
""")

    # Arrow style combos
    arrow_colors = ["red", "blue", "green", "orange", "purple", "navy", "teal", "maroon"]
    arrow_widths = [1, 2, 3]
    for color in arrow_colors:
        for width in arrow_widths:
            w(SKINPARAM_DIR, f"skin_arrow_{color}_w{width}_class.puml", f"""\
@startuml
skinparam classArrowColor {color}
skinparam classArrowThickness {width}
class A
class B
class C
A --> B : assoc
B <|-- C : extends
A ..> C : dep
@enduml
""")

    # Background color sweep
    bg_colors = [
        "white", "black", "LightGray", "DarkGray",
        "LightBlue", "LightGreen", "LightYellow", "LightCoral",
        "LightCyan", "LightPink", "LavenderBlush", "MintCream",
        "#FAFAFA", "#F0F8FF", "#FFF8DC", "#F5F5DC",
    ]
    for bg in bg_colors:
        safe = bg.replace("#", "hex_").replace(" ", "_")
        w(SKINPARAM_DIR, f"skin_bg_{safe}.puml", f"""\
@startuml
skinparam backgroundColor {bg}
Alice -> Bob : background test
note right : {bg}
@enduml
""")

def gen_extra_creole():
    """Generate additional creole test cases to hit target."""
    # More color combinations
    colors = ["red", "blue", "green", "orange", "purple", "cyan", "magenta",
              "yellow", "pink", "navy", "teal", "maroon", "olive", "lime",
              "#FF6600", "#0066FF", "#00FF66", "#660066", "#FF0066"]
    markup_types = ["bold", "italic", "underline"]
    diagrams = ["sequence", "class", "activity", "note"]

    for color in colors:
        safe_color = color.replace("#", "hex").lower()
        for m_type in markup_types:
            markup_map = {"bold": "**", "italic": "//", "underline": "__"}
            m = markup_map[m_type]
            for diag in diagrams:
                content_map = {
                    "sequence": f"@startuml\nAlice -> Bob : <color:{color}>{m}text{m}</color>\n@enduml\n",
                    "class": f"@startuml\nclass C {{\n  <color:{color}>{m}field{m}</color>: String\n}}\n@enduml\n",
                    "activity": f"@startuml\n:<color:{color}>{m}action{m}</color>;\n@enduml\n",
                    "note": f"@startuml\nnote as N\n  <color:{color}>{m}note text{m}</color>\nend note\n@enduml\n",
                }
                w(CREOLE_DIR, f"creole_color_{safe_color}_{m_type}_in_{diag}.puml",
                  content_map[diag])

    # Size variations
    sizes = [8, 10, 12, 14, 16, 18, 20, 24, 28, 32, 36]
    for size in sizes:
        w(CREOLE_DIR, f"creole_size_{size}_in_sequence.puml", f"""\
@startuml
Alice -> Bob : <size:{size}>size {size} text</size>
note right : <size:{size}>size {size} note</size>
@enduml
""")
        w(CREOLE_DIR, f"creole_size_{size}_in_note.puml", f"""\
@startuml
note as N
  <size:{size}>sized text at {size}pt</size>
end note
@enduml
""")
        w(CREOLE_DIR, f"creole_size_{size}_in_class.puml", f"""\
@startuml
class MyClass {{
  <size:{size}>sized field</size>: String
}}
@enduml
""")

    # HTML entities and special characters
    special_chars = [
        ("ampersand", "a &amp; b"),
        ("lt_gt", "a &lt; b &gt; c"),
        ("nbsp", "word&nbsp;word"),
        ("quote", "say &quot;hello&quot;"),
        ("apos", "it&apos;s fine"),
        ("hash", "color #FF0000"),
        ("at_sign", "@startuml nested @enduml"),
        ("pipe", "a | b | c"),
        ("brackets", "[link] <tag>"),
        ("curly", "{field}: {value}"),
    ]

    for name, text in special_chars:
        w(CREOLE_DIR, f"creole_special_{name}_in_note.puml", f"""\
@startuml
note as N
  {text}
end note
@enduml
""")
        w(CREOLE_DIR, f"creole_special_{name}_in_sequence.puml", f"""\
@startuml
Alice -> Bob : {text}
@enduml
""")

    # Creole in group/loop/alt labels
    creole_in_seq_blocks = [
        ("group", f"group **bold group**\n  Alice -> Bob : inside\nend"),
        ("loop", f"loop 3 times\n  Alice -> Bob : **looping**\nend"),
        ("alt", f"alt **happy path**\n  Alice -> Bob : ok\nelse //sad path//\n  Alice -> Bob : error\nend"),
        ("opt", f"opt __optional__\n  Alice -> Bob : maybe\nend"),
        ("par", f"par **parallel 1**\n  Alice -> Bob : p1\nelse //parallel 2//\n  Alice -> Bob : p2\nend"),
        ("break", f"break **break condition**\n  Alice -> Bob : breaking\nend"),
        ("critical", f"critical //critical section//\n  Alice -> Bob : locked\nend"),
        ("ref", f"ref over Alice, Bob\n  **reference label**\nend ref"),
    ]

    for name, block in creole_in_seq_blocks:
        w(CREOLE_DIR, f"creole_in_seq_{name}.puml", f"""\
@startuml
{block}
@enduml
""")

    # Creole in class stereotypes
    stereotypes = [
        ("entity", "<<entity>>"),
        ("service", "<<service>>"),
        ("controller", "<<controller>>"),
        ("repository", "<<repository>>"),
        ("boundary", "<<boundary>>"),
        ("value", "<<value object>>"),
    ]

    for name, stereo in stereotypes:
        w(CREOLE_DIR, f"creole_in_stereotype_{name}.puml", f"""\
@startuml
class "**{name.title()}Class** {stereo}" as C {{
  +**boldField**: String
  +//italicMethod//(): void
}}
@enduml
""")

    # Creole in package names
    for markup, name in [("**bold**", "bold"), ("//italic//", "italic"),
                          ("<color:blue>blue</color>", "colored")]:
        w(CREOLE_DIR, f"creole_in_package_name_{name}.puml", f"""\
@startuml
package "{markup} Package" {{
  class Inner {{
    +field: String
  }}
}}
@enduml
""")

    # Long creole strings (stress tests)
    long_bold = "**" + "x" * 50 + "**"
    long_italic = "//" + "y" * 50 + "//"
    w(CREOLE_DIR, "creole_long_bold_in_note.puml", f"""\
@startuml
note as N
  {long_bold}
end note
@enduml
""")
    w(CREOLE_DIR, "creole_long_italic_in_note.puml", f"""\
@startuml
note as N
  {long_italic}
end note
@enduml
""")

    # Mixed content lines
    mixed_lines = [
        ("bold_plain_italic", "**bold** normal //italic//"),
        ("mono_plain_color", '""mono"" plain <color:red>red</color>'),
        ("all_inline", "**b** //i// __u__ ~~s~~ <color:blue>c</color>"),
        ("size_color_bold", "<size:16><color:navy>**big navy bold**</color></size>"),
        ("multiformat", "start **bold** middle //italic// end __under__"),
    ]

    for name, line in mixed_lines:
        w(CREOLE_DIR, f"creole_mixed_{name}_in_sequence.puml", f"""\
@startuml
Alice -> Bob : {line}
@enduml
""")
        w(CREOLE_DIR, f"creole_mixed_{name}_in_note.puml", f"""\
@startuml
note as N
  {line}
end note
@enduml
""")
        w(CREOLE_DIR, f"creole_mixed_{name}_in_class.puml", f"""\
@startuml
class C {{
  {line}: field
}}
@enduml
""")


# ---------------------------------------------------------------------------
# Run all generators
# ---------------------------------------------------------------------------

def gen_preproc_define_bulk():
    """Bulk define/undef test cases covering more patterns."""
    # Simple constant defines for various diagram elements
    for i in range(1, 21):
        w(PREPROC_DIR, f"preproc_define_participant_{i:02d}.puml", f"""\
@startuml
!define P{i} Participant{i}
participant P{i} as A
A -> A : self message {i}
note right : P{i} = Participant{i}
@enduml
""")

    # Define-based color palettes
    palettes = [
        ("blue", "#4472C4", "#2E4A8A", "#E8F0FB"),
        ("green", "#70AD47", "#4F7A33", "#EBF3E3"),
        ("orange", "#ED7D31", "#A55720", "#FBE9D8"),
        ("red", "#FF0000", "#AA0000", "#FFE0E0"),
        ("purple", "#7030A0", "#4A1F6B", "#EFE0FB"),
        ("teal", "#00B0F0", "#0072A0", "#D0F0FF"),
        ("gray", "#808080", "#404040", "#F0F0F0"),
    ]

    for pname, primary, dark, light in palettes:
        w(PREPROC_DIR, f"preproc_define_palette_{pname}.puml", f"""\
@startuml
!define PRIMARY {primary}
!define DARK {dark}
!define LIGHT {light}

skinparam classBackgroundColor LIGHT
skinparam classBorderColor PRIMARY
skinparam classHeaderBackgroundColor PRIMARY
skinparam classArrowColor PRIMARY
skinparam classFontColor DARK

class "{pname.title()} Themed Class" {{
  +field: String
  +method(): void
}}
@enduml
""")

    # Define-based diagram switcher
    for mode in ["simple", "detailed", "compact"]:
        if mode == "simple":
            body = "class Simple { +name: String }"
        elif mode == "detailed":
            body = "class Detailed {\n  +id: Long\n  +name: String\n  +email: String\n  +createdAt: Date\n  +updatedAt: Date\n}"
        else:
            body = "class C { +x: int }"

        w(PREPROC_DIR, f"preproc_define_mode_{mode}.puml", f"""\
@startuml
!define MODE_{mode.upper()}
{body}
@enduml
""")

    # Nested define usage
    for n in range(1, 6):
        w(PREPROC_DIR, f"preproc_define_nested_{n}.puml", f"""\
@startuml
!define LEVEL {n}
!define PREFIX Level_LEVEL_

class PREFIX{n} {{
  depth: int = LEVEL
}}
@enduml
""")


def gen_preproc_ifdef_bulk():
    """More ifdef/ifndef combinations."""
    flags = ["DEBUG", "PRODUCTION", "TESTING", "STAGING", "DEVELOPMENT"]

    for flag in flags:
        w(PREPROC_DIR, f"preproc_ifdef_flag_{flag.lower()}_defined.puml", f"""\
@startuml
!define {flag}

!ifdef {flag}
skinparam noteBackgroundColor LightGreen
note as N
  {flag} mode is active
end note
!else
note as N
  {flag} mode is inactive
end note
!endif
Alice -> Bob : {flag.lower()} mode test
@enduml
""")

        w(PREPROC_DIR, f"preproc_ifdef_flag_{flag.lower()}_undefined.puml", f"""\
@startuml
!ifdef {flag}
skinparam noteBackgroundColor LightGreen
note as N
  {flag} mode active
end note
!else
note as N
  {flag} mode inactive (not defined)
end note
!endif
Alice -> Bob : {flag.lower()} undefined test
@enduml
""")

    # Multi-flag combinations
    for flag1, flag2 in [("DEBUG", "VERBOSE"), ("PROD", "SECURE"), ("TEST", "MOCK")]:
        w(PREPROC_DIR, f"preproc_ifdef_multi_{flag1.lower()}_{flag2.lower()}.puml", f"""\
@startuml
!define {flag1}
!define {flag2}

!ifdef {flag1}
  !ifdef {flag2}
    note as N
      Both {flag1} and {flag2} active
    end note
  !else
    note as N
      Only {flag1} active
    end note
  !endif
!else
  !ifdef {flag2}
    note as N
      Only {flag2} active
    end note
  !else
    note as N
      Neither active
    end note
  !endif
!endif
@enduml
""")

    # Diagram-type conditional guards
    diagram_guards = [
        ("sequence", "SEQUENCE", "Alice -> Bob : guarded\nBob --> Alice : reply"),
        ("class", "CLASS", "class GuardedClass { +field: String }"),
        ("activity", "ACTIVITY", "start\n:guarded action;\nstop"),
        ("state", "STATE", "[*] --> GuardedState\nGuardedState --> [*]"),
        ("component", "COMPONENT", "component GuardedComp"),
        ("usecase", "USECASE", "actor A\nusecase UC\nA --> UC"),
    ]

    for diag_type, guard, body in diagram_guards:
        w(PREPROC_DIR, f"preproc_ifdef_guard_{diag_type}.puml", f"""\
@startuml
!define RENDER_{guard}

!ifdef RENDER_{guard}
{body}
!endif
@enduml
""")


def gen_preproc_variable_bulk():
    """More variable/arithmetic test cases."""
    # Fibonacci-like
    w(PREPROC_DIR, "preproc_var_fibonacci.puml", """\
@startuml
!$a = 0
!$b = 1
!$c = $a + $b
!$a = $b
!$b = $c
!$c = $a + $b
!$a = $b
!$b = $c
!$c = $a + $b

note as N
  fib: 0, 1, 1, 2, 3, 5...
  computed: $a, $b, $c
end note
@enduml
""")

    # String building
    w(PREPROC_DIR, "preproc_var_string_build.puml", """\
@startuml
!$prefix = "API"
!$version = "v2"
!$endpoint = "/users"
!$full_path = $prefix + "_" + $version + $endpoint

note as N
  path: $full_path
end note
@enduml
""")

    # Variable-driven skinparams
    skin_vars = [
        ("blue", "LightBlue", "Navy"),
        ("green", "LightGreen", "DarkGreen"),
        ("yellow", "LightYellow", "DarkOrange"),
        ("pink", "LightPink", "DarkRed"),
        ("gray", "LightGray", "DarkGray"),
        ("cyan", "LightCyan", "DarkCyan"),
    ]

    for name, bg, border in skin_vars:
        w(PREPROC_DIR, f"preproc_var_skinparam_{name}.puml", f"""\
@startuml
!$bg = "{bg}"
!$border = "{border}"

skinparam classBackgroundColor $bg
skinparam classBorderColor $border

class VarSkinClass {{
  theme: String = "{name}"
}}
@enduml
""")

    # Counter-driven class generation
    for count in [2, 3, 5, 8]:
        lines = []
        for i in range(1, count + 1):
            lines.append(f"  Entity{i} : data")
        w(PREPROC_DIR, f"preproc_var_loop_{count}_entities.puml", f"""\
@startuml
!$n = {count}
!$i = 1
!while $i <= $n
  class Entity$i {{
    +id: Long
    +index: int = $i
  }}
  !$i = $i + 1
!endwhile
@enduml
""")


def gen_preproc_function_bulk():
    """More function test cases."""
    # Type-checking functions
    w(PREPROC_DIR, "preproc_func_type_check.puml", """\
@startuml
!function $is_positive($n)
  !return $n > 0
!endfunction

!function $is_even($n)
  !return ($n % 2) == 0
!endfunction

!$tests = 6
!$i = 1
!while $i <= $tests
  note as N$i
    $i: positive=$is_positive($i), even=$is_even($i)
  end note
  !$i = $i + 1
!endwhile
@enduml
""")

    # String manipulation functions
    w(PREPROC_DIR, "preproc_func_string_utils.puml", """\
@startuml
!function $repeat($s, $n)
  !$result = ""
  !$i = 0
  !while $i < $n
    !$result = $result + $s
    !$i = $i + 1
  !endwhile
  !return $result
!endfunction

note as N
  repeat("ab", 3) = $repeat("ab", 3)
end note
@enduml
""")

    # Functions generating diagram elements
    diagram_funcs = [
        ("class_stub", """\
!function $stub_class($name)
  !return "class " + $name + " { +id: Long }"
!endfunction
""", '$stub_class("MyClass")'),
        ("seq_label", """\
!function $method_call($obj, $method, $args)
  !return $obj + "." + $method + "(" + $args + ")"
!endfunction
""", 'Alice -> Bob : $method_call("svc", "get", "id=1")'),
    ]

    for name, func_def, usage in diagram_funcs:
        w(PREPROC_DIR, f"preproc_func_{name}.puml", f"""\
@startuml
{func_def}
{usage}
@enduml
""")

    # Function with multiple return paths
    w(PREPROC_DIR, "preproc_func_conditional_return.puml", """\
@startuml
!function $classify($n)
  !if $n < 0
    !return "negative"
  !elseif $n == 0
    !return "zero"
  !elseif $n < 10
    !return "small"
  !else
    !return "large"
  !endif
!endfunction

note as N
  classify(-5) = $classify(-5)
  classify(0) = $classify(0)
  classify(7) = $classify(7)
  classify(42) = $classify(42)
end note
@enduml
""")


def gen_preproc_theme_bulk():
    """More theme combinations with different diagram types."""
    extra_themes = [
        "aws-orange", "black-knight", "blueprint",
        "cloudscape-design", "hacker", "metal",
        "mimeograph", "plain", "reddress-darkblue",
        "reddress-darkgreen", "reddress-darkorange",
        "reddress-darkred", "reddress-lightblue",
        "reddress-lightgreen", "reddress-lightorange",
        "reddress-lightred", "silver", "spacelab",
        "sunlust", "toy",
    ]

    for theme in extra_themes:
        safe = theme.replace("-", "_")
        # sequence
        w(PREPROC_DIR, f"preproc_theme_{safe}_sequence.puml", f"""\
@startuml
!theme {theme}
participant Alice
participant Bob
Alice -> Bob : hello from {theme}
Bob --> Alice : reply
note right : {theme} theme
@enduml
""")
        # class
        w(PREPROC_DIR, f"preproc_theme_{safe}_class.puml", f"""\
@startuml
!theme {theme}
class ThemeTest {{
  +name: String
  +method(): void
}}
class Child extends ThemeTest {{
  +extra: int
}}
@enduml
""")
        # state
        w(PREPROC_DIR, f"preproc_theme_{safe}_state.puml", f"""\
@startuml
!theme {theme}
[*] --> Idle
Idle --> Active : start
Active --> Done : finish
Done --> [*]
@enduml
""")
        # activity
        w(PREPROC_DIR, f"preproc_theme_{safe}_activity.puml", f"""\
@startuml
!theme {theme}
start
:action one;
if (check?) then (yes)
  :branch a;
else (no)
  :branch b;
endif
stop
@enduml
""")


def gen_preproc_misc():
    """Miscellaneous preprocessing patterns."""
    # !local in nested functions
    w(PREPROC_DIR, "preproc_local_var_nested.puml", """\
@startuml
!function $outer($x)
  !local $inner_val = $x * 2
  !return $inner_val + 1
!endfunction

!function $chain($a, $b)
  !local $sum = $a + $b
  !local $doubled = $sum * 2
  !return $doubled
!endfunction

note as N
  outer(5) = $outer(5)
  chain(3, 4) = $chain(3, 4)
end note
@enduml
""")

    # Parameterized note templates
    for style in ["info", "warning", "error", "success"]:
        color_map = {"info": "LightBlue", "warning": "LightYellow",
                     "error": "LightCoral", "success": "LightGreen"}
        color = color_map[style]
        w(PREPROC_DIR, f"preproc_proc_note_{style}.puml", f"""\
@startuml
!procedure $note_{style}($msg)
  note as N_{style}
    <back:{color}>**{style.upper()}**: $msg</back>
  end note
!endprocedure

$note_{style}("This is a {style} message")
@enduml
""")

    # Variable used in group labels
    w(PREPROC_DIR, "preproc_var_in_group_label.puml", """\
@startuml
!$operation = "data sync"
!$version = "v3"
group $operation ($version)
  Client -> Server : request
  Server --> Client : response
end
@enduml
""")

    # Conditional skinparams
    for env in ["dev", "staging", "prod"]:
        if env == "dev":
            param = "skinparam backgroundColor LightYellow\nskinparam noteBackgroundColor LightYellow"
        elif env == "staging":
            param = "skinparam backgroundColor LightBlue\nskinparam noteBackgroundColor LightBlue"
        else:
            param = "skinparam backgroundColor white\nskinparam noteBackgroundColor white"

        w(PREPROC_DIR, f"preproc_conditional_skin_{env}.puml", f"""\
@startuml
!define ENV_{env.upper()}

!ifdef ENV_{env.upper()}
{param}
!endif

title Environment: {env}
Alice -> Bob : deployed to {env}
@enduml
""")

    # Recursive-style using while (depth-limited)
    w(PREPROC_DIR, "preproc_while_tree_build.puml", """\
@startuml
!$depth = 1
[*] --> Node1
!while $depth <= 4
  !$next = $depth + 1
  state Node$depth
  Node$depth --> Node$next : deeper
  !$depth = $next
!endwhile
Node5 --> [*]
@enduml
""")

    # Pragma combinations
    pragmas = [
        ("nopo", "nopo", "Alice -> Bob : no partial output\nBob --> Alice : ok"),
        ("revision", "revision", "Alice -> Bob : revision test"),
        ("charset", "charset UTF-8", "Alice -> Bob : charset UTF-8"),
    ]

    for name, pragma_val, body in pragmas:
        w(PREPROC_DIR, f"preproc_pragma_{name}.puml", f"""\
@startuml
!pragma {pragma_val}
{body}
@enduml
""")

    # Multiple variable reassignments
    for ops in range(3, 9):
        w(PREPROC_DIR, f"preproc_var_reassign_{ops}_times.puml", f"""\
@startuml
!$x = 0
{''.join(f'!$x = $x + {i}{chr(10)}' for i in range(1, ops + 1))}
note as N
  final x = $x (sum 1..{ops} = {sum(range(1, ops + 1))})
end note
@enduml
""")

    # Function recursion depth test (using while to simulate)
    w(PREPROC_DIR, "preproc_func_accumulate.puml", """\
@startuml
!function $sum_to($n)
  !local $result = 0
  !local $i = 1
  !while $i <= $n
    !$result = $result + $i
    !$i = $i + 1
  !endwhile
  !return $result
!endfunction

note as N
  sum(1..5) = $sum_to(5)
  sum(1..10) = $sum_to(10)
  sum(1..20) = $sum_to(20)
end note
@enduml
""")

    # Multiline defines in sequence diagrams
    for i in range(1, 11):
        w(PREPROC_DIR, f"preproc_define_seq_step_{i:02d}.puml", f"""\
@startuml
!define STEP {i}
!define LABEL "Step STEP"

note over Alice
  LABEL
end note
Alice -> Bob : step STEP of 10
@enduml
""")

    # Variable-driven participant names
    participant_names = [
        ("frontend", "Frontend", "Backend"),
        ("client_server", "Client", "Server"),
        ("producer_consumer", "Producer", "Consumer"),
        ("sender_receiver", "Sender", "Receiver"),
        ("requester_responder", "Requester", "Responder"),
        ("source_sink", "Source", "Sink"),
        ("writer_reader", "Writer", "Reader"),
        ("pub_sub", "Publisher", "Subscriber"),
    ]

    for name, a, b in participant_names:
        w(PREPROC_DIR, f"preproc_var_participants_{name}.puml", f"""\
@startuml
!$left = "{a}"
!$right = "{b}"
!$channel = "{a.lower()}_to_{b.lower()}"

participant "$left" as L
participant "$right" as R

L -> R : request via $channel
R --> L : response via $channel
note over L, R
  $left talks to $right
  channel: $channel
end note
@enduml
""")


def main():
    # Creole
    gen_creole_bold()
    gen_creole_italic()
    gen_creole_mono()
    gen_creole_strike()
    gen_creole_underline()
    gen_creole_combinations()
    gen_creole_colors()
    gen_creole_links()
    gen_creole_images()
    gen_creole_lists()
    gen_creole_hlines()
    gen_creole_tables()
    gen_creole_tree()
    gen_creole_code()
    gen_creole_latex()
    gen_creole_unicode()
    gen_creole_html()
    gen_creole_escape()
    gen_creole_empty()
    gen_creole_nested()
    gen_creole_linebreaks()
    gen_creole_combinatorial()
    gen_creole_all_contexts()
    gen_extra_creole()

    # Skinparams
    gen_skinparam_global()
    gen_skinparam_sequence()
    gen_skinparam_class()
    gen_skinparam_activity()
    gen_skinparam_state()
    gen_skinparam_component()
    gen_skinparam_usecase()
    gen_skinparam_note()
    gen_skinparam_package()
    gen_skinparam_linetype()
    gen_skinparam_style()
    gen_skinparam_stereotype()
    gen_skinparam_combined()
    gen_skinparam_gradient()
    gen_skinparam_arrow_types()
    gen_extra_skinparams()

    # Preprocessing
    gen_preproc_define()
    gen_preproc_ifdef()
    gen_preproc_variables()
    gen_preproc_functions()
    gen_preproc_procedures()
    gen_preproc_builtins()
    gen_preproc_conditionals()
    gen_preproc_while()
    gen_preproc_include()
    gen_preproc_pragma()
    gen_preproc_theme()
    gen_preproc_log_assert()
    gen_preproc_complex()
    gen_preproc_variable_types()
    gen_preproc_extra()
    gen_preproc_define_bulk()
    gen_preproc_ifdef_bulk()
    gen_preproc_variable_bulk()
    gen_preproc_function_bulk()
    gen_preproc_theme_bulk()
    gen_preproc_misc()

    # Summary
    creole_count = sum(1 for p in files_written if "/creole/" in p)
    skin_count = sum(1 for p in files_written if "/skinparam/" in p)
    preproc_count = sum(1 for p in files_written if "/preprocessing/" in p)
    total = len(files_written)

    print(f"Generated {total} files total:")
    print(f"  creole/       : {creole_count}")
    print(f"  skinparam/    : {skin_count}")
    print(f"  preprocessing/: {preproc_count}")

if __name__ == "__main__":
    main()
