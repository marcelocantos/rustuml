# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Independent raw-Java controls; exclusive writes preserve every attempt."""
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[4]
JAVA = "/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java"
JAR = "/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar"


def cases():
    rows = []

    def add(mechanism, name, body, axes, *, teoz=False, reverse=False, style="", negative=False):
        declarations = ['participant "Quartz relay" as Q', 'participant "Ember.i" as E',
                        'participant "Tern archive" as T']
        if reverse:
            declarations.reverse()
        source = ("@startuml\n" + ("!pragma teoz true\n" if teoz else "") + style
                  + "\n".join(declarations) + "\n" + body + "\n@enduml\n")
        rows.append(dict(mechanism=mechanism, name="checker_recovery_" + name,
                         text=source, axes=axes, negative_control=negative,
                         frontend="raw Java CLI -tsvg -pipe -charset UTF-8"))

    for teoz in (False, True):
        engine = "teoz" if teoz else "legacy"
        for reverse in (False, True):
            for wide in (False, True):
                tag = f"{engine}_r{int(reverse)}_w{int(wide)}"
                label = "WW.quartz iii transmission envelope" if wide else "i.f"
                axes = ["legacy versus Teoz", "participant declaration order", "label width"]
                add("sequence-recursive-group-constraints", tag,
                    f"[-> Q : arrive\ngroup outer\nnote left of Q : {label}\n"
                    f"alt selected\nloop retry\nnote left of Q : WWWMi\nQ -> E : relay\n"
                    "end\nelse alternate\nnote over E,T : other\nend\nend",
                    axes + ["direct versus nested member competition"], teoz=teoz, reverse=reverse)
                add("sequence-self-return-message-model", tag,
                    f"Q -> Q ++ : {label}\nQ -> Q ++ : inner\nreturn nested\nreturn final",
                    axes + ["two owned self returns"], teoz=teoz, reverse=reverse)
                add("sequence-return-live-message-model", tag,
                    f"T -> Q ++ : {label}\nQ -> Q ++ : local\nQ -> E ++ : delegate\n"
                    "return delegated\nreturn local\nreturn sender",
                    axes + ["nested live endpoints and return ownership"], teoz=teoz, reverse=reverse)
                add("sequence-self-arrow-async-stroke", tag,
                    f"Q ->> Q ++ : {label}\nQ ->> Q ++ : nested\nQ -->> Q -- : close\nreturn outer",
                    axes + ["open self arrow with activation and deactivation"], teoz=teoz, reverse=reverse)
                add("sequence-component-padding-flow", tag,
                    f"alt {label}\\nsecond guard\nQ -> E ++ : call\nnote left\n{label}\nline\nend note\n"
                    "else\ngroup inner\nnote over Q,T : span\nreturn done\nend\nend",
                    axes + ["padding 1 versus 6", "multiline versus empty branch guards", "attached notes"],
                    teoz=teoz, reverse=reverse, style=f"skinparam Padding {6 if wide else 1}\n")
                add("sequence-note-component-envelope", tag,
                    f"T -> Q ++ : enter\nQ -> Q ++ : recursive\ngroup notes\nQ -> E : {label}\n"
                    "note left : attached while live\nnote right of Q\nline one\nline two\nend note\n"
                    "... pause ...\nreturn unwind\nend\nreturn release",
                    axes + ["shadow on/off", "note across activation and delay"], teoz=teoz, reverse=reverse,
                    style=f"skinparam shadowing {'true' if wide else 'false'}\nskinparam Padding 2\n")
                add("sequence-participant-shadow-envelope", tag,
                    f"group perimeter\nQ -> T : {label}\nnote over Q,T : spanning participant heads\n"
                    "opt nested\nT -> E : settle\nend\nend",
                    axes + ["participant shadow on/off", "nested group and spanning note"], teoz=teoz,
                    reverse=reverse, style=f"skinparam shadowing {'true' if wide else 'false'}\n")
                add("sequence-self-arrow-envelope", tag,
                    f"group initial\nE -> E ++ : {label}\nnote right : attached\nreturn finish\nend\n"
                    "newpage Followup\nE -> E ++ : later wide WWWWWWWW\nE -> E ++ : nested\nreturn inner\nreturn outer",
                    axes + ["all-page self extent", "padding and attached note"], teoz=teoz, reverse=reverse,
                    style="skinparam Padding 3\n")
                add("sequence-page-textblock-composition", tag,
                    f"T -> Q ++ : {label}\ngroup window\nnote left of Q : wide WWWWWW\nQ -> E : next\nend\n"
                    "newpage Followup\nQ -> Q ++ : second page\nreturn inner\nreturn done",
                    axes + ["wide versus narrow page chrome", "cross-page activation clipping"], teoz=teoz,
                    reverse=reverse, style=f"title {label}\nheader Header {label}\ncaption Caption {label}\nfooter Footer {label}\n")
                add("sequence-self-arrow-attached-lifecycle", tag,
                    f"T -> E ++ : initial\nE ->> E : {label}\nactivate E\nnote left of E : attachment gap\n"
                    "deactivate E\nE -> T : subsequent\ndeactivate E",
                    axes + ["explicit activate/deactivate attached across a note", "pre-existing recursion"],
                    teoz=teoz, reverse=reverse)

    for boundary in ("[", "]"):
        for incoming in (False, True):
            for teoz in (False, True):
                token = f"{boundary}->> Q" if incoming else f"{boundary}<<- Q"
                name = f"{'left' if boundary == '[' else 'right'}_in{int(incoming)}_teoz{int(teoz)}"
                add("sequence-external-arrow-grammar", name,
                    f"{token} ++ : boundary traffic\nnote right of Q : live channel\nreturn dispatch\nQ -> E : after",
                    ["left/right border", "incoming/outgoing dressing", "external return lifecycle", "engine"], teoz=teoz)

    for mechanism in ("sequence-teoz-note-tile-bounds", "sequence-teoz-self-tile-history", "sequence-teoz-life-event-bounds"):
        for reverse in (False, True):
            for depth in (1, 3):
                for side in ("left", "right"):
                    body = "T -> E ++ : begin\n" + "E -> E ++ : recurse\n" * (depth - 1)
                    body += (f"group outer\nopt nested\nnote {side} of E : fresh WWW.i envelope\n"
                             "E -> E : history point\nactivate E\nnote over E : intervening tile\ndeactivate E\n"
                             "E -> Q ++ : forward\nreturn delegated\nend\nend\n")
                    body += "return unwind\n" * depth
                    add(mechanism, f"r{int(reverse)}_d{depth}_{side}", body,
                        ["recursion depth", "note side", "declaration order", "explicit lifecycle separated by note"],
                        teoz=True, reverse=reverse)

    # Probe independent CommandReturn configuration beyond the maker's labels.
    for teoz in (False, True):
        for self_call in (False, True):
            destination = "Q" if self_call else "E"
            mechanism = "sequence-self-return-message-model" if self_call else "sequence-return-live-message-model"
            add(mechanism, f"colored_teoz{int(teoz)}", f"Q -> {destination} ++ : call\nreturn #purple colored reply",
                ["explicit return color", "self versus interparticipant ownership", "engine"], teoz=teoz)
        add("sequence-self-return-message-model", f"negative_nowhere_teoz{int(teoz)}", "return orphan",
            ["no preceding activating message", "engine"], teoz=teoz, negative=True)
    return rows


def focused_cases():
    rows = []

    def add(mechanism, name, body, axes, teoz=False, style=""):
        rows.append(dict(mechanism=mechanism, name="checker_focus_" + name,
                         text="@startuml\n" + ("!pragma teoz true\n" if teoz else "") + style
                         + 'participant "Mica.i" as M\nparticipant "Long heron" as H\n'
                         + body + "\n@enduml\n", axes=axes, negative_control=False,
                         frontend="raw Java CLI -tsvg -pipe -charset UTF-8"))

    for teoz in (False, True):
        for reverse in (False, True):
            for depth in (0, 2):
                arrow = "<<-" if reverse else "->>"
                body = "H -> M ++ : earlier\n" * depth
                body += f"M {arrow} M : same row\nactivate M\ndeactivate M\nM -> H : after\n"
                body += "deactivate M\n" * depth
                add("sequence-self-arrow-async-stroke", f"e{int(teoz)}_r{int(reverse)}_d{depth}", body,
                    ["self-loop direction", "prior depth", "both lifecycle flags", "engine"], teoz)
        for head in ("//", "\\\\"):
            for attached in (False, True):
                body = f"M -{head} M ++ : half loop\n"
                if attached:
                    body += "note right : attached note\n"
                body += "return complete"
                add("sequence-self-arrow-async-stroke", f"half_{'top' if head == '//' else 'bottom'}_n{int(attached)}_e{int(teoz)}",
                    body, ["async half-head direction", "attached note", "engine"], teoz)
        for side in ("left", "right"):
            add("sequence-note-component-envelope", f"attached_padding_{side}_e{int(teoz)}",
                f"M -> H : sample\nnote {side}\nfirst\nsecond\nend note\nH -> M : after",
                ["attached note side", "nonzero padding", "engine"], teoz, "skinparam Padding 2\n")
        for spanning in (False, True):
            add("sequence-participant-shadow-envelope", f"shadow_span{int(spanning)}_e{int(teoz)}",
                "group frame\nM -> H : sample\n" + ("note over M,H : span\n" if spanning else "") + "end",
                ["spanning note presence", "shadowed participant envelope", "engine"], teoz,
                "skinparam shadowing true\n")
        for multiline in (False, True):
            caption = "caption first\\nsecond\n" if multiline else "caption first\n"
            add("sequence-page-textblock-composition", f"caption_multi{int(multiline)}_e{int(teoz)}",
                "M -> H : probe", ["multiline caption", "legend and title wrapper composition", "engine"], teoz,
                "title A wide title WWWWWWWWWWWWWWWWWWWWWWW\nlegend left\nkey\nendlegend\n" + caption)
    return rows


def main(output, selected):
    output.mkdir(parents=True, exist_ok=False)
    command = [JAVA, "-Djava.awt.headless=true", "-jar", JAR, "-tsvg", "-pipe", "-charset", "UTF-8"]

    def generate(row):
        folder = ROOT / "test-diagrams/perturbations" / row["mechanism"]
        folder.mkdir(parents=True, exist_ok=True)
        source = folder / (row["name"] + ".puml")
        with source.open("x") as stream:
            stream.write(row.pop("text"))
        result = subprocess.run(command, input=source.read_bytes(), capture_output=True,
                                env=dict(os.environ, JAVA_HOME=str(Path(JAVA).parent.parent)), timeout=90)
        golden = source.with_suffix(".svg")
        with golden.open("xb") as stream:
            stream.write(result.stdout)
        try:
            root = ET.fromstring(result.stdout)
            xml_svg = root.tag.endswith("svg")
        except ET.ParseError:
            xml_svg = False
        row.update(source=str(source.relative_to(ROOT)), golden=str(golden.relative_to(ROOT)),
                   cohort="checker-" + row["mechanism"], java_exit=result.returncode,
                   java_stderr=result.stderr.decode(errors="replace"), java_command=command,
                   xml_svg=xml_svg, java_sha256=hashlib.sha256(result.stdout).hexdigest())
        return row

    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        rows = list(executor.map(generate, selected))
    (output / "generated.json").write_text(json.dumps(rows, indent=2) + "\n")
    print("Preserved", len(rows), "pairs; Java nonzero exits:", sum(r["java_exit"] != 0 for r in rows), flush=True)


if __name__ == "__main__":
    main(Path(sys.argv[1]).resolve(), focused_cases() if "--focused" in sys.argv else cases())
