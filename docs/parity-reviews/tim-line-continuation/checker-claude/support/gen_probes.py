"""Write checker batch tc1 (T14.22 continuation review) probe sources into DEST. Usage: python3 -I gen_probes.py DEST"""
import json, sys
from pathlib import Path
dest = Path(sys.argv[1]); dest.mkdir(parents=True, exist_ok=False)
C = []  # (name, text, intended_valid, axes, crlf)
def add(name, body, valid=True, axes=(), crlf=False, start="uml", end=True):
    text = f"@start{start}\n{body}\n" + (f"@end{start}\n" if end else "")
    C.append((name, text, valid, list(axes), crlf))
# 1 quoted strings and URLs
add("tc1_seq_quoted_backslash", 'participant "Wide \\\nName" as W\nW -> W : self', axes=["sequence", "backslash inside quotes", "pre-TIM"])
add("tc1_seq_url_backslash", "Kea -> Tui : see [[http://example.com/a go]] \\\nnow", axes=["sequence", "URL then backslash", "pre-TIM"])
add("tc1_class_quoted_label_backslash", 'class Fir\nclass Yew\nFir --> Yew : "alpha \\\nbeta"', axes=["class", "quoted link label", "pre-TIM"])
# 2 comment lines ending with a backslash
add("tc1_seq_quote_comment_backslash", "' remark \\\nKea -> Tui : eaten\nTui -> Kea : kept", axes=["sequence", "' line ends with backslash", "swallows next line"])
add("tc1_class_quote_comment_backslash", "class Ash\n' remark \\\nclass Gone\nclass Elm", axes=["class", "' line ends with backslash", "swallows next line"])
add("tc1_seq_next_apostrophe_leading", "Kea -> Tui : it \\\n  's skipped\nTui -> Kea : z", axes=["sequence", "next line starts with ' after spaces (skipped)"])
# 3 continuation into and inside block comments
add("tc1_seq_block_comment_inner_backslash", "/'\ntext \\\n'/\nKea -> Tui : hidden\n'/\nTui -> Kea : shown", axes=["sequence", "backslash inside long comment", "'/ skipped as quote line"])
add("tc1_class_block_comment_inner_backslash", "/'\nnote \\\n'/\nclass Hidden\n'/\nclass Shown", axes=["class", "backslash inside long comment", "'/ skipped as quote line"])
add("tc1_seq_next_long_comment_tail", "Kea -> Tui : x \\\n/' long\nend '/\nTui -> Kea : after\nKea -> Kea : more", axes=["sequence", "long comment skipped then next real line"])
add("tc1_inv_seq_long_comment_unterminated_tail", "Kea -> Tui : x \\\n/' long\nend '/ tail\nTui -> Kea : z", valid=False, axes=["sequence", "long comment never ends (tail after '/)", "swallows end"])
add("tc1_inv_seq_quote_comment_then_end", "Kea -> Tui : x \\\n' c", valid=False, axes=["sequence", "comment line then end swallowed"])
# 4 multiline command bodies
add("tc1_seq_legend_body", "Kea -> Tui : hi\nlegend\nline one \\\ntwo\nthree\nendlegend", axes=["sequence", "legend body"])
add("tc1_class_note_body", "class Oak\nnote right of Oak\nfirst \\\nsecond\nthird\nend note", axes=["class", "note body"])
add("tc1_yaml_body", "fruit: apple \\\npie\nveg: leek", start="yaml", axes=["yaml", "pre-TIM join"])
add("tc1_salt_body", "{\n[Press \\\nHere]\n\"field\"\n}", start="salt", axes=["salt", "pre-TIM join"])
add("tc1_json_two_pieces_nested", "{\n\"k\": [1, \\\n2, \\\n3],\n\"z\": \"q\"\n}", start="json", axes=["json", "three pieces"])
add("tc1_mindmap_three", "* Root \\\nnode \\\nname\n** Leaf", start="mindmap", axes=["mindmap", "three pieces"])
add("tc1_activity_legacy_label", "(*) --> \"First \\\nline\"\n--> (*)", axes=["activity legacy", "quoted label continued"])
# 5 includes (post-TIM join, @startuml only)
add("tc1_seq_include_tail_before_end", "Kea -> Tui : head\n!include tc1_tail.iuml", axes=["sequence", "!include last line backslash then @enduml", "post-TIM pending tail"])
add("tc1_seq_include_then_directive", "!include tc1_tail.iuml\n!$v = 2\nTui -> Kea : two $v", axes=["sequence", "!include backslash then TIM directive (Rust placeholder)", "post-TIM"])
add("tc1_seq_include_then_define", "!include tc1_tail.iuml\n!define LBL two\nTui -> Kea : LBL", axes=["sequence", "!include backslash then !define (Rust placeholder)", "post-TIM"])
add("tc1_seq_include_then_main_line", "!include tc1_tail.iuml\nmore\nTui -> Kea : after", axes=["sequence", "!include backslash joined with main line", "post-TIM numbering"])
add("tc1_class_include_three", "!include tc1_three.iuml\nclass After", axes=["class", "!include three pieces", "post-TIM last-line location"])
add("tc1_mindmap_include_no_postjoin", "!include tc1_mm.iuml\n** Leaf", start="mindmap", axes=["mindmap", "!include backslash not joined (no SEQUENCE)"])
add("tc1_seq_include_directive_continued", "!include \\\ntc1_plain.iuml\nTui -> Kea : main", axes=["sequence", "!include directive split by continuation"])
add("tc1_seq_theme_local_postjoin", "!theme tc1 from .\nKea -> Tui : hi", axes=["sequence", "local !theme file with backslash", "post-TIM"])
# 6 stage-2 from TIM output in the main source
add("tc1_seq_chr_backslash_postjoin", "Kea -> Tui : one%chr(92)\nTui -> Kea : two\nKea -> Kea : three", axes=["sequence", "TIM output ends with backslash", "post-TIM join", "last-line location"])
add("tc1_mindmap_chr_backslash_no_postjoin", "* Root%chr(92)\n** Leaf", start="mindmap", axes=["mindmap", "TIM output backslash, no post-TIM join"])
# 7 numbering of 2, 3, 4 pieces (pre-TIM keeps the first line)
add("tc1_seq_two_pieces_lines", "Kea -> Tui : p1 \\\np2\nTui -> Kea : after", axes=["sequence", "two pieces", "following data-source-line"])
add("tc1_seq_four_pieces_lines", "Kea -> Tui : p1 \\\np2 \\\np3 \\\np4\nTui -> Kea : after\nKea -> Kea : last", axes=["sequence", "four pieces", "following data-source-line"])
add("tc1_class_four_pieces_lines", "class Ash \\\n<<one>> \\\n#pink \\\n##red\nclass Birch\nclass Cedar", axes=["class", "four pieces", "following data-source-line"])
add("tc1_class_two_merges_lines", "class Ash \\\n<<s>>\nclass Birch \\\n<<t>>\nclass Cedar", axes=["class", "two separate merges", "following data-source-line"])
# 8 pipe reader: indented end after continuation
add("tc1_seq_indented_end_after_backslash", "Kea -> Tui : last \\\n  @enduml", end=False, axes=["sequence", "indented @enduml after backslash", "Pipe#readSingleDiagram startsWith"])
# 9 CRLF with comment skip
add("tc1_class_crlf_comment_skip", "class Ash\nAsh --> Ash : one \\\n' gap\ntwo\nclass Birch", crlf=True, axes=["class", "CRLF", "comment skipped"])
add("tc1_seq_separator_continued", "Kea -> Tui : a\n== Phase \\\nOne ==\nTui -> Kea : b", axes=["sequence", "separator continued"])
# 10 sprite interplay (whole sprite line)
add("tc1_seq_sprite_header_continued", "sprite $dot \\\n[2x2/16] {\nF0\n0F\n}\nKea -> Tui : <$dot> hi", axes=["sequence", "sprite header continued", "sprite ownership"])
add("tc1_seq_sprite_row_continued", "sprite $dot [4x2/16] {\nF0 \\\n0F\nFFFF\n}\nKea -> Tui : <$dot> hi", axes=["sequence", "sprite row continued", "sprite ownership"])
add("tc1_seq_sprite_svg_continued", 'sprite $sq <svg viewBox="0 0 4 4"><rect width="4" height="4" fill="#ff0000"/>\\\n</svg>\nKea -> Tui : <$sq> hi', axes=["sequence", "inline svg sprite continued", "sprite ownership"])
# 11 controls: included-line numbering without any continuation
add("tc1_ctl_class_include_plain", "!include tc1_plain_classes.iuml\nLantern --> Wick", axes=["control", "class", "!include without continuation", "included data-source-line"])
add("tc1_ctl_seq_include_plain", "Kea -> Tui : first\n!include tc1_plain.iuml\nTui -> Kea : after", axes=["control", "sequence", "!include without continuation", "included data-source-line"])
for name, text, valid, axes, crlf in C:
    data = text.replace("\n", "\r\n") if crlf else text
    (dest / f"{name}.puml").write_bytes(data.encode())
R = {"tc1_tail.iuml": "Kea -> Tui : tail \\\n", "tc1_three.iuml": "class Inc \\\n<<i1>> \\\n#lightblue\nclass Inc2\n",
     "tc1_mm.iuml": "* Root \\\n", "tc1_plain.iuml": "Kea -> Tui : included\n", "tc1_plain_classes.iuml": "class Lantern\nclass Wick\n",
     "puml-theme-tc1.puml": "skinparam ArrowColor \\\n#DD0000\n"}
for p, t in R.items(): (dest / p).write_bytes(t.encode())
(dest / "cases.json").write_text(json.dumps([dict(name=n, intended_valid=v, axes=a, crlf=c) for n, _, v, a, c in C], indent=1))
(dest / "resources.json").write_text(json.dumps(sorted(R)))
print(len(C))
