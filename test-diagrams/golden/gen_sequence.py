#!/usr/bin/env python3
"""
Generator for comprehensive PlantUML sequence diagram test cases.
Generates ~1500+ .puml files covering every conceivable sequence diagram feature.
"""

import os
import itertools
from pathlib import Path

OUTPUT_DIR = Path(__file__).parent / "sequence"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

files_written = 0


def write_puml(filename: str, content: str):
    global files_written
    path = OUTPUT_DIR / filename
    path.write_text(content)
    files_written += 1


def puml(body: str, title: str = "", header: str = "", footer: str = "",
         pragma: str = "", skinparams: str = "") -> str:
    parts = ["@startuml"]
    if pragma:
        parts.append(pragma)
    if title:
        parts.append(f"title {title}")
    if header:
        parts.append(f"header {header}")
    if footer:
        parts.append(f"footer {footer}")
    if skinparams:
        parts.append(skinparams)
    parts.append(body.strip())
    parts.append("@enduml")
    return "\n".join(parts) + "\n"


# ---------------------------------------------------------------------------
# 1. PARTICIPANT TYPES - basic
# ---------------------------------------------------------------------------

PARTICIPANT_TYPES = [
    "participant", "actor", "boundary", "control", "entity",
    "database", "collections", "queue"
]

# Each type alone
for ptype in PARTICIPANT_TYPES:
    body = f'{ptype} Alice\nAlice -> Alice : self message'
    write_puml(f"seq_participant_{ptype}_alone.puml", puml(body))

# Each type pair with participant
for ptype in PARTICIPANT_TYPES:
    if ptype == "participant":
        continue
    body = f'{ptype} Alice\nparticipant Bob\nAlice -> Bob : hello\nBob --> Alice : hi'
    write_puml(f"seq_participant_{ptype}_with_participant.puml", puml(body))

# All types together
body = "\n".join(f"{t} {t.capitalize()}P" for t in PARTICIPANT_TYPES)
body += "\n"
for i, t in enumerate(PARTICIPANT_TYPES):
    nxt = PARTICIPANT_TYPES[(i + 1) % len(PARTICIPANT_TYPES)]
    body += f"{t.capitalize()}P -> {nxt.capitalize()}P : msg {i+1}\n"
write_puml("seq_participant_all_types.puml", puml(body))

# Pairs of each type
for t1, t2 in itertools.combinations(PARTICIPANT_TYPES, 2):
    body = f'{t1} A\n{t2} B\nA -> B : request\nB --> A : response'
    write_puml(f"seq_participant_{t1}_{t2}_pair.puml", puml(body))

# ---------------------------------------------------------------------------
# 2. PARTICIPANT - aliases
# ---------------------------------------------------------------------------

for ptype in PARTICIPANT_TYPES:
    body = f'{ptype} "Long Name Here" as A\nparticipant "Another Long Name" as B\nA -> B : hello\nB --> A : hi'
    write_puml(f"seq_participant_{ptype}_alias.puml", puml(body))

# Alias with unicode
write_puml("seq_participant_alias_unicode.puml", puml(
    'participant "Ünïcödé Nàmé" as U\nparticipant "日本語" as J\nU -> J : こんにちは\nJ --> U : hello'
))

# Alias with spaces in quotes
write_puml("seq_participant_alias_spaces.puml", puml(
    'participant "Alice Smith" as A\nparticipant "Bob Jones Jr." as B\nA -> B : How do you do?\nB --> A : Very well, thanks!'
))

# ---------------------------------------------------------------------------
# 3. PARTICIPANT - colors
# ---------------------------------------------------------------------------

COLORS = ["#red", "#blue", "#green", "#yellow", "#pink", "#lightblue",
          "#FF8800", "#AAFFAA", "#orchid", "#salmon"]

for i, color in enumerate(COLORS):
    body = f'participant Alice {color}\nparticipant Bob\nAlice -> Bob : colored participant\nBob --> Alice : reply'
    write_puml(f"seq_participant_color_{i+1:02d}.puml", puml(body))

# Multiple colored participants
body = "\n".join(f'participant P{i} #{"red" if i%3==0 else "blue" if i%3==1 else "green"}' for i in range(6))
body += "\nP0 -> P1 : a\nP1 -> P2 : b\nP2 -> P3 : c\nP3 -> P4 : d\nP4 -> P5 : e\nP5 --> P0 : f"
write_puml("seq_participant_multi_color.puml", puml(body))

# ---------------------------------------------------------------------------
# 4. PARTICIPANT - stereotypes
# ---------------------------------------------------------------------------

STEREOTYPES = ["<<service>>", "<<external>>", "<<legacy>>", "<<new>>",
               "<<deprecated>>", "<<internal>>", "<<boundary>>", "<<entity>>"]

for i, stereo in enumerate(STEREOTYPES):
    body = f'participant Alice {stereo}\nparticipant Bob {stereo}\nAlice -> Bob : hello\nBob --> Alice : hi'
    write_puml(f"seq_participant_stereotype_{i+1:02d}.puml", puml(body))

# Stereotype with color
write_puml("seq_participant_stereotype_color.puml", puml(
    'participant Alice <<service>> #lightblue\nparticipant Bob <<external>> #lightyellow\nAlice -> Bob : call\nBob --> Alice : result'
))

# ---------------------------------------------------------------------------
# 5. PARTICIPANT - order
# ---------------------------------------------------------------------------

write_puml("seq_participant_order_basic.puml", puml(
    'participant Bob order 20\nparticipant Alice order 10\nparticipant Charlie order 30\nAlice -> Bob : 1st\nBob -> Charlie : 2nd\nCharlie --> Alice : 3rd'
))

write_puml("seq_participant_order_reverse.puml", puml(
    'participant C order 1\nparticipant B order 2\nparticipant A order 3\nA -> B : right-to-left\nB -> C : continuing\nC --> A : back'
))

# ---------------------------------------------------------------------------
# 6. PARTICIPANT - long names, unicode, special chars
# ---------------------------------------------------------------------------

write_puml("seq_participant_long_name.puml", puml(
    'participant "This Is A Very Long Participant Name That Goes On And On" as LONG\nparticipant Short\nLONG -> Short : hello\nShort --> LONG : world'
))

write_puml("seq_participant_unicode_names.puml", puml(
    'participant "Ünïcödé"\nparticipant "日本語テスト"\nparticipant "中文测试"\nparticipant "한국어"\nparticipant "العربية"\n"Ünïcödé" -> "日本語テスト" : unicode msg\n"日本語テスト" --> "Ünïcödé" : 返事'
))

write_puml("seq_participant_emoji.puml", puml(
    'participant "Alice 🎉"\nparticipant "Bob 🚀"\n"Alice 🎉" -> "Bob 🚀" : Launch! 🚀\n"Bob 🚀" --> "Alice 🎉" : Party! 🎉'
))

# ---------------------------------------------------------------------------
# 7. MESSAGES - basic arrow types
# ---------------------------------------------------------------------------

ARROW_TYPES = [
    ("solid", "->"),
    ("dotted", "-->"),
    ("thin_solid", "->>"),
    ("thin_dotted", "-->>"),
    ("solid_noarrow", "->)"),
    ("dotted_noarrow", "-->)"),
    ("bold", "-\\ "),
    ("bold_dotted", "--\\ "),
]

for name, arrow in ARROW_TYPES:
    body = f'Alice {arrow.strip()} Bob : {name} message\nBob {arrow.strip()} Alice : return {name}'
    write_puml(f"seq_msg_{name}.puml", puml(body))

# ---------------------------------------------------------------------------
# 8. MESSAGES - lost and found
# ---------------------------------------------------------------------------

write_puml("seq_msg_lost.puml", puml(
    'Alice -> Bob : normal\nAlice ->x Bob : lost message\nBob ->x Alice : also lost'
))

write_puml("seq_msg_found.puml", puml(
    '[-> Alice : found message from outside\nAlice -> Bob : forwarded\nBob ->] : sent to outside'
))

write_puml("seq_msg_found_dotted.puml", puml(
    '[-> Alice : incoming\n[--> Alice : incoming dotted\nAlice ->] : outgoing\nAlice -->] : outgoing dotted'
))

# ---------------------------------------------------------------------------
# 9. MESSAGES - directions
# ---------------------------------------------------------------------------

write_puml("seq_msg_rtl.puml", puml(
    'Alice <- Bob : right to left\nAlice <-- Bob : dotted rtl\nAlice <<- Bob : thin rtl\nAlice <<-- Bob : thin dotted rtl'
))

write_puml("seq_msg_bidirectional.puml", puml(
    'Alice -> Bob : ltr\nBob -> Alice : rtl\nAlice --> Bob : dotted ltr\nBob --> Alice : dotted rtl'
))

# ---------------------------------------------------------------------------
# 10. MESSAGES - self messages
# ---------------------------------------------------------------------------

write_puml("seq_msg_self_basic.puml", puml(
    'Alice -> Alice : self call\nAlice --> Alice : self return\nAlice ->> Alice : thin self\nAlice -->> Alice : thin dotted self'
))

write_puml("seq_msg_self_with_activation.puml", puml(
    'Alice -> Alice ++ : self activate\nAlice --> Alice -- : self deactivate\nAlice -> Alice ++ : nested self\nAlice --> Alice -- : nested return'
))

write_puml("seq_msg_self_multiline.puml", puml(
    'Alice -> Alice : first line\\nsecond line\\nthird line'
))

# ---------------------------------------------------------------------------
# 11. MESSAGES - return keyword
# ---------------------------------------------------------------------------

write_puml("seq_msg_return_keyword.puml", puml(
    'Alice -> Bob ++ : request\nreturn response'
))

write_puml("seq_msg_return_with_value.puml", puml(
    'Alice -> Bob ++ : getData()\nreturn [1, 2, 3]'
))

write_puml("seq_msg_return_nested.puml", puml(
    'Alice -> Bob ++ : outer\nBob -> Charlie ++ : inner\nreturn inner result\nreturn outer result'
))

# ---------------------------------------------------------------------------
# 12. MESSAGES - colored arrows
# ---------------------------------------------------------------------------

ARROW_COLORS = ["red", "blue", "green", "orange", "purple", "black", "#FF8800", "darkblue"]

for color in ARROW_COLORS:
    body = f'Alice -[#{color}]> Bob : colored {color}\nBob -[#{color}]--> Alice : dotted {color}'
    write_puml(f"seq_msg_color_{color.replace('#','hex')}.puml", puml(body))

write_puml("seq_msg_color_mixed.puml", puml(
    'Alice -[#red]> Bob : red\nBob -[#blue]--> Alice : blue\nAlice -[#green]>> Charlie : green thin\nCharlie -[#purple]-->> Alice : purple thin dotted'
))

# ---------------------------------------------------------------------------
# 13. MESSAGES - autonumber
# ---------------------------------------------------------------------------

write_puml("seq_autonumber_basic.puml", puml(
    'autonumber\nAlice -> Bob : first\nBob --> Alice : second\nAlice -> Charlie : third\nCharlie --> Alice : fourth'
))

write_puml("seq_autonumber_start.puml", puml(
    'autonumber 10\nAlice -> Bob : starts at 10\nBob --> Alice : 11\nAlice -> Bob : 12'
))

write_puml("seq_autonumber_step.puml", puml(
    'autonumber 1 10\nAlice -> Bob : 1\nBob --> Alice : 11\nAlice -> Bob : 21'
))

write_puml("seq_autonumber_format.puml", puml(
    'autonumber "<b>[000]"\nAlice -> Bob : formatted\nBob --> Alice : numbered'
))

write_puml("seq_autonumber_format_html.puml", puml(
    'autonumber "<font color=red><b>##</b></font>"\nAlice -> Bob : red number\nBob --> Alice : also red'
))

write_puml("seq_autonumber_resume.puml", puml(
    'autonumber\nAlice -> Bob : 1\nBob --> Alice : 2\nautonumber stop\nAlice -> Bob : unnumbered\nautonumber resume\nAlice -> Bob : 3\nBob --> Alice : 4'
))

write_puml("seq_autonumber_resume_step.puml", puml(
    'autonumber 1 2\nAlice -> Bob : 1\nBob --> Alice : 3\nautonumber stop\nAlice -> Bob : unnumbered\nautonumber resume 2\nAlice -> Bob : 5\nBob --> Alice : 7'
))

write_puml("seq_autonumber_stop.puml", puml(
    'autonumber\nAlice -> Bob : 1\nBob --> Alice : 2\nautonumber stop\nAlice -> Bob : no number\nBob --> Alice : no number'
))

# ---------------------------------------------------------------------------
# 14. MESSAGES - multiline
# ---------------------------------------------------------------------------

write_puml("seq_msg_multiline_basic.puml", puml(
    'Alice -> Bob : line 1\\nline 2\nBob --> Alice : first\\nsecond\\nthird'
))

write_puml("seq_msg_multiline_long.puml", puml(
    'Alice -> Bob : This is the first line\\nThis is the second line\\nThis is the third line\\nAnd the fourth line'
))

# ---------------------------------------------------------------------------
# 15. MESSAGES - creole formatting
# ---------------------------------------------------------------------------

write_puml("seq_msg_creole_bold.puml", puml(
    'Alice -> Bob : **bold message**\nBob --> Alice : **bold reply**'
))

write_puml("seq_msg_creole_italic.puml", puml(
    'Alice -> Bob : //italic message//\nBob --> Alice : //italic reply//'
))

write_puml("seq_msg_creole_monospace.puml", puml(
    'Alice -> Bob : ""monospace message""\nBob --> Alice : ""code()"" returned'
))

write_puml("seq_msg_creole_mixed.puml", puml(
    'Alice -> Bob : **bold** and //italic// and ""code""\nBob --> Alice : <b>html bold</b> and <i>html italic</i>'
))

write_puml("seq_msg_creole_color.puml", puml(
    'Alice -> Bob : <color:red>red text</color> and normal\nBob --> Alice : <color:#0000FF>blue text</color>'
))

write_puml("seq_msg_creole_size.puml", puml(
    'Alice -> Bob : <size:20>large text</size>\nBob --> Alice : <size:8>tiny text</size>'
))

write_puml("seq_msg_html_tags.puml", puml(
    'Alice -> Bob : <b>bold</b> <i>italic</i> <u>underline</u>\nBob --> Alice : <s>strikethrough</s> <w>wave</w>'
))

# ---------------------------------------------------------------------------
# 16. MESSAGES - very long and empty
# ---------------------------------------------------------------------------

write_puml("seq_msg_very_long.puml", puml(
    'Alice -> Bob : This is an extremely long message text that goes on and on to test how the renderer handles very long message labels in sequence diagrams\nBob --> Alice : Another very long response message that tests the wrapping and layout capabilities of the sequence diagram renderer'
))

write_puml("seq_msg_empty.puml", puml(
    'Alice -> Bob\nBob --> Alice :\nAlice -> Bob : '
))

# ---------------------------------------------------------------------------
# 17. LIFELINE ACTIVATION - explicit
# ---------------------------------------------------------------------------

write_puml("seq_activate_basic.puml", puml(
    'Alice -> Bob : request\nactivate Bob\nBob --> Alice : response\ndeactivate Bob'
))

write_puml("seq_activate_nested.puml", puml(
    'Alice -> Bob : call1\nactivate Bob\nBob -> Charlie : call2\nactivate Charlie\nCharlie --> Bob : ret2\ndeactivate Charlie\nBob --> Alice : ret1\ndeactivate Bob'
))

write_puml("seq_activate_triple_nested.puml", puml(
    'Alice -> Bob : a\nactivate Bob\nBob -> Charlie : b\nactivate Charlie\nCharlie -> Dave : c\nactivate Dave\nDave --> Charlie : d\ndeactivate Dave\nCharlie --> Bob : e\ndeactivate Charlie\nBob --> Alice : f\ndeactivate Bob'
))

write_puml("seq_activate_colored.puml", puml(
    'Alice -> Bob : request\nactivate Bob #red\nBob -> Charlie : delegate\nactivate Charlie #blue\nCharlie --> Bob : done\ndeactivate Charlie\nBob --> Alice : result\ndeactivate Bob'
))

write_puml("seq_activate_destroy.puml", puml(
    'Alice -> Bob : create\nactivate Bob\nBob -> Bob : do work\nAlice -> Bob : kill\ndeactivate Bob\ndestroy Bob'
))

# ---------------------------------------------------------------------------
# 18. LIFELINE ACTIVATION - shorthand
# ---------------------------------------------------------------------------

write_puml("seq_activate_shorthand_basic.puml", puml(
    'Alice -> Bob ++ : request\nBob --> Alice -- : response'
))

write_puml("seq_activate_shorthand_nested.puml", puml(
    'Alice -> Bob ++ : call1\nBob -> Charlie ++ : call2\nCharlie --> Bob -- : ret2\nBob --> Alice -- : ret1'
))

write_puml("seq_activate_shorthand_color.puml", puml(
    'Alice -> Bob ++ #red : request\nBob -> Charlie ++ #blue : delegate\nCharlie --> Bob -- : ok\nBob --> Alice -- : done'
))

write_puml("seq_activate_shorthand_destroy.puml", puml(
    'Alice -> Bob ++ : create\nBob --> Alice -- : ready\nAlice -> Bob !! : destroy'
))

write_puml("seq_activate_shorthand_mixed.puml", puml(
    'Alice -> Bob ++ : start\nBob -> Bob : internal\nBob -> Charlie ++ : delegate\nreturn charlie done\nBob --> Alice -- : complete'
))

# ---------------------------------------------------------------------------
# 19. LIFELINE ACTIVATION - autoactivate
# ---------------------------------------------------------------------------

write_puml("seq_autoactivate_on.puml", puml(
    'autoactivate on\nAlice -> Bob : request\nBob -> Charlie : delegate\nreturn charlie result\nreturn bob result'
))

write_puml("seq_autoactivate_on_complex.puml", puml(
    'autoactivate on\nAlice -> Bob : step1\nBob -> Charlie : step2\nCharlie -> Dave : step3\nreturn d\nreturn c\nreturn b'
))

# ---------------------------------------------------------------------------
# 20. GROUPS - alt/else
# ---------------------------------------------------------------------------

write_puml("seq_group_alt_basic.puml", puml(
    'Alice -> Bob : request\nalt success\n  Bob --> Alice : result\nelse failure\n  Bob --> Alice : error\nend'
))

write_puml("seq_group_alt_multiple_else.puml", puml(
    'Alice -> Bob : request\nalt case A\n  Bob --> Alice : result A\nelse case B\n  Bob --> Alice : result B\nelse case C\n  Bob --> Alice : result C\nelse default\n  Bob --> Alice : default result\nend'
))

write_puml("seq_group_alt_with_messages.puml", puml(
    'Alice -> Bob : authenticate\nalt authenticated\n  Bob -> DB : query\n  DB --> Bob : data\n  Bob --> Alice : 200 OK\nelse not authenticated\n  Bob --> Alice : 401 Unauthorized\nend'
))

# ---------------------------------------------------------------------------
# 21. GROUPS - opt
# ---------------------------------------------------------------------------

write_puml("seq_group_opt_basic.puml", puml(
    'Alice -> Bob : request\nopt logging enabled\n  Bob -> Logger : log request\nend\nBob --> Alice : response'
))

write_puml("seq_group_opt_complex.puml", puml(
    'Alice -> Bob : process\nopt cache enabled\n  Bob -> Cache : lookup\n  Cache --> Bob : miss\nend\nBob -> DB : query\nDB --> Bob : result\nopt cache enabled\n  Bob -> Cache : store\nend\nBob --> Alice : data'
))

# ---------------------------------------------------------------------------
# 22. GROUPS - loop
# ---------------------------------------------------------------------------

write_puml("seq_group_loop_basic.puml", puml(
    'Alice -> Bob : start\nloop 10 times\n  Bob -> Bob : process item\nend\nBob --> Alice : done'
))

write_puml("seq_group_loop_condition.puml", puml(
    'Alice -> Bob : start\nloop while items remain\n  Bob -> DB : get next\n  DB --> Bob : item\n  Bob -> Bob : process\nend\nBob --> Alice : complete'
))

write_puml("seq_group_loop_with_participants.puml", puml(
    'Client -> Server : request\nloop for each record\n  Server -> DB : fetch\n  DB --> Server : record\n  Server -> Cache : store\nend\nServer --> Client : all done'
))

# ---------------------------------------------------------------------------
# 23. GROUPS - par
# ---------------------------------------------------------------------------

write_puml("seq_group_par_basic.puml", puml(
    'Alice -> Bob : start\npar\n  Bob -> Charlie : parallel 1\nand\n  Bob -> Dave : parallel 2\nend\nBob --> Alice : done'
))

write_puml("seq_group_par_three.puml", puml(
    'Client -> Server : process\npar\n  Server -> ServiceA : call A\nand\n  Server -> ServiceB : call B\nand\n  Server -> ServiceC : call C\nend\nServer --> Client : aggregated result'
))

# ---------------------------------------------------------------------------
# 24. GROUPS - break
# ---------------------------------------------------------------------------

write_puml("seq_group_break_basic.puml", puml(
    'Alice -> Bob : start\nloop while true\n  Bob -> Bob : work\n  break on error\n    Bob --> Alice : error\n  end\nend\nBob --> Alice : done'
))

# ---------------------------------------------------------------------------
# 25. GROUPS - critical
# ---------------------------------------------------------------------------

write_puml("seq_group_critical_basic.puml", puml(
    'Alice -> Bob : start\ncritical\n  Bob -> DB : transaction begin\n  Bob -> DB : update\n  Bob -> DB : commit\nend\nBob --> Alice : done'
))

# ---------------------------------------------------------------------------
# 26. GROUPS - group with custom label
# ---------------------------------------------------------------------------

write_puml("seq_group_custom_label.puml", puml(
    'Alice -> Bob : start\ngroup My Custom Group [optional label]\n  Bob -> Charlie : step 1\n  Charlie --> Bob : result 1\n  Bob -> Dave : step 2\n  Dave --> Bob : result 2\nend\nBob --> Alice : finished'
))

write_puml("seq_group_custom_label_colored.puml", puml(
    'Alice -> Bob : start\ngroup#red Error Handling\n  Bob -> Bob : handle error\n  Bob --> Alice : error response\nend'
))

# ---------------------------------------------------------------------------
# 27. GROUPS - nested
# ---------------------------------------------------------------------------

write_puml("seq_group_nested_2deep.puml", puml(
    'Alice -> Bob : start\nalt success\n  loop 3 times\n    Bob -> DB : query\n    DB --> Bob : data\n  end\n  Bob --> Alice : ok\nelse failure\n  Bob --> Alice : error\nend'
))

write_puml("seq_group_nested_3deep.puml", puml(
    'Client -> Server : request\nalt authenticated\n  loop for each item\n    opt cache miss\n      Server -> DB : fetch\n      DB --> Server : data\n    end\n    Server -> Cache : store\n  end\n  Server --> Client : 200\nelse\n  Server --> Client : 401\nend'
))

write_puml("seq_group_nested_4deep.puml", puml(
    'A -> B : go\nloop outer\n  alt branch\n    loop inner\n      opt condition\n        B -> C : deep call\n        C --> B : deep return\n      end\n    end\n  else other\n    B -> B : handle\n  end\nend\nB --> A : done'
))

write_puml("seq_group_nested_5deep.puml", puml(
    'A -> B : start\nloop L1\n  loop L2\n    alt A1\n      opt O1\n        par\n          B -> C : c1\n        and\n          B -> D : d1\n        end\n      end\n    else A2\n      B -> B : handle\n    end\n  end\nend\nB --> A : end'
))

write_puml("seq_group_all_types_nested.puml", puml(
    'A -> B : start\nalt condition\n  opt optional\n    loop 5 times\n      par\n        B -> C : parallel a\n      and\n        B -> D : parallel b\n      end\n    end\n  end\nelse other\n  break error\n    B --> A : broken\n  end\nend\nB --> A : done'
))

# Empty groups
write_puml("seq_group_empty_alt.puml", puml(
    'Alice -> Bob : request\nalt success\nelse failure\nend\nBob --> Alice : done'
))

write_puml("seq_group_empty_loop.puml", puml(
    'Alice -> Bob : go\nloop nothing\nend\nBob --> Alice : done'
))

# Groups with notes inside
write_puml("seq_group_with_notes.puml", puml(
    'Alice -> Bob : request\nalt success\n  note over Bob : processing\n  Bob --> Alice : ok\nelse failure\n  note right of Bob : error occurred\n  Bob --> Alice : error\nend'
))

# ---------------------------------------------------------------------------
# 28. NOTES - basic
# ---------------------------------------------------------------------------

write_puml("seq_note_left_of.puml", puml(
    'Alice -> Bob : hello\nnote left of Alice : I said hello\nBob --> Alice : hi\nnote left of Bob : Bob replied'
))

write_puml("seq_note_right_of.puml", puml(
    'Alice -> Bob : hello\nnote right of Alice : note on right\nBob --> Alice : hi\nnote right of Bob : another right note'
))

write_puml("seq_note_over_single.puml", puml(
    'Alice -> Bob : hello\nnote over Alice : over Alice\nnote over Bob : over Bob\nBob --> Alice : hi'
))

write_puml("seq_note_over_multiple.puml", puml(
    'Alice -> Bob : hello\nnote over Alice, Bob : over both\nBob -> Charlie : forward\nnote over Bob, Charlie : over Bob and Charlie'
))

write_puml("seq_note_across.puml", puml(
    'participant Alice\nparticipant Bob\nparticipant Charlie\nAlice -> Bob : hello\nnote across : this spans all participants\nBob --> Alice : hi'
))

# ---------------------------------------------------------------------------
# 29. NOTES - hnote and rnote
# ---------------------------------------------------------------------------

write_puml("seq_hnote_basic.puml", puml(
    'Alice -> Bob : hello\nhnote over Alice : hexagonal note\nBob --> Alice : hi'
))

write_puml("seq_rnote_basic.puml", puml(
    'Alice -> Bob : hello\nrnote over Bob : rectangular note\nBob --> Alice : hi'
))

write_puml("seq_hnote_rnote_combined.puml", puml(
    'Alice -> Bob : hello\nhnote over Alice : hex\nrnote over Bob : rect\nBob --> Alice : hi'
))

# ---------------------------------------------------------------------------
# 30. NOTES - multiline
# ---------------------------------------------------------------------------

write_puml("seq_note_multiline.puml", puml(
    'Alice -> Bob : request\nnote over Bob\n  This is a multi-line note\n  with several lines\n  of content\nend note\nBob --> Alice : response'
))

write_puml("seq_note_multiline_left.puml", puml(
    'Alice -> Bob : request\nnote left of Alice\n  Left note\n  multiple lines\nend note\nBob --> Alice : response'
))

write_puml("seq_note_multiline_right.puml", puml(
    'Alice -> Bob : request\nnote right of Bob\n  Right note\\nwith newline\nend note\nBob --> Alice : response'
))

# ---------------------------------------------------------------------------
# 31. NOTES - creole and colors
# ---------------------------------------------------------------------------

write_puml("seq_note_creole.puml", puml(
    'Alice -> Bob : hello\nnote over Alice\n  **bold text**\n  //italic text//\n  ""monospace""\nend note\nBob --> Alice : hi'
))

write_puml("seq_note_colored.puml", puml(
    'Alice -> Bob : hello\nnote over Alice #red : red note\nnote over Bob #lightblue : blue note\nBob --> Alice : hi'
))

write_puml("seq_note_colored_multiline.puml", puml(
    'Alice -> Bob : hello\nnote over Alice, Bob #lightyellow\n  **Important**\n  This is colored\n  and multi-line\nend note\nBob --> Alice : hi'
))

# Note without "of" (on last message)
write_puml("seq_note_on_message_left.puml", puml(
    'Alice -> Bob : hello\nnote left : note on last message left\nBob --> Alice : hi\nnote right : note on last message right'
))

# ---------------------------------------------------------------------------
# 32. DIVIDERS and SPACING
# ---------------------------------------------------------------------------

write_puml("seq_divider_basic.puml", puml(
    'Alice -> Bob : phase 1\n== Divider ==\nBob -> Charlie : phase 2\n== Another Divider ==\nCharlie --> Alice : done'
))

write_puml("seq_divider_multiple.puml", puml(
    'Alice -> Bob : start\n== Initialization ==\nBob -> DB : connect\nDB --> Bob : connected\n== Processing ==\nBob -> Bob : process\n== Cleanup ==\nBob -> DB : disconnect\nDB --> Bob : ok\nBob --> Alice : done'
))

write_puml("seq_delay_basic.puml", puml(
    'Alice -> Bob : request\n...\nBob --> Alice : response'
))

write_puml("seq_delay_labeled.puml", puml(
    'Alice -> Bob : start\n... 5 minutes later ...\nBob --> Alice : response'
))

write_puml("seq_delay_multiple.puml", puml(
    'Alice -> Bob : ping\n...\nBob -> Charlie : forward\n... long time ...\nCharlie --> Bob : result\n...\nBob --> Alice : done'
))

write_puml("seq_spacing_triple_pipe.puml", puml(
    'Alice -> Bob : hello\n|||\nBob --> Alice : hi\n|||\nAlice -> Bob : again'
))

write_puml("seq_spacing_explicit.puml", puml(
    'Alice -> Bob : hello\n||45||\nBob --> Alice : hi\n||10||\nAlice -> Bob : again'
))

write_puml("seq_spacing_mixed.puml", puml(
    'Alice -> Bob : a\n|||\n... wait ...\n||20||\nBob --> Alice : b'
))

# ---------------------------------------------------------------------------
# 33. REFERENCES
# ---------------------------------------------------------------------------

write_puml("seq_ref_over_basic.puml", puml(
    'Alice -> Bob : start\nref over Alice, Bob : See diagram Foo\nBob --> Alice : done'
))

write_puml("seq_ref_over_multiline.puml", puml(
    'Alice -> Bob : start\nref over Alice, Bob\n  My Reference\n  See page 42\nend ref\nBob --> Alice : done'
))

write_puml("seq_ref_single_participant.puml", puml(
    'Alice -> Bob : hello\nref over Bob : internal ref\nBob --> Alice : hi'
))

# ---------------------------------------------------------------------------
# 34. BOXES
# ---------------------------------------------------------------------------

write_puml("seq_box_basic.puml", puml(
    'box "Internal Services"\n  participant Alice\n  participant Bob\nend box\nparticipant External\nAlice -> Bob : internal\nBob -> External : external call\nExternal --> Bob : result\nBob --> Alice : done'
))

write_puml("seq_box_colored.puml", puml(
    'box "Frontend" #lightblue\n  actor User\n  participant Browser\nend box\nbox "Backend" #lightyellow\n  participant API\n  database DB\nend box\nUser -> Browser : click\nBrowser -> API : request\nAPI -> DB : query\nDB --> API : data\nAPI --> Browser : response\nBrowser --> User : display'
))

write_puml("seq_box_multiple.puml", puml(
    'box "Layer 1" #red\n  participant A\n  participant B\nend box\nbox "Layer 2" #blue\n  participant C\n  participant D\nend box\nbox "Layer 3" #green\n  participant E\nend box\nA -> C : cross layer\nC -> E : another cross\nE --> C : back\nC --> A : return'
))

write_puml("seq_box_unnamed.puml", puml(
    'box\n  participant Alice\n  participant Bob\nend box\nAlice -> Bob : hello\nBob --> Alice : hi'
))

# ---------------------------------------------------------------------------
# 35. SKINPARAM
# ---------------------------------------------------------------------------

SKINPARAM_EXAMPLES = [
    ("arrow_color", "skinparam SequenceArrowColor red"),
    ("lifeline_color", "skinparam SequenceLifeLineBorderColor blue"),
    ("participant_bg", "skinparam ParticipantBackgroundColor lightyellow"),
    ("participant_border", "skinparam ParticipantBorderColor darkblue"),
    ("actor_bg", "skinparam ActorBackgroundColor lightgreen"),
    ("note_bg", "skinparam NoteBackgroundColor lightyellow"),
    ("group_bg", "skinparam SequenceGroupBodyBackgroundColor lightyellow"),
    ("group_border", "skinparam SequenceGroupBorderColor navy"),
]

for name, sp in SKINPARAM_EXAMPLES:
    body = "Alice -> Bob : hello\nBob --> Alice : hi"
    write_puml(f"seq_skinparam_{name}.puml", puml(body, skinparams=sp))

write_puml("seq_skinparam_comprehensive.puml", puml(
    'Alice -> Bob : hello\nnote over Alice : note\nBob --> Alice : hi',
    skinparams="""skinparam SequenceArrowColor #336699
skinparam SequenceLifeLineBorderColor #336699
skinparam ParticipantBackgroundColor #E8F4FD
skinparam ParticipantBorderColor #336699
skinparam NoteBackgroundColor #FFFDE7
skinparam NoteBorderColor #F57F17
skinparam SequenceGroupBorderColor #E91E63"""
))

write_puml("seq_skinparam_monochrome.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    skinparams="skinparam monochrome true"
))

write_puml("seq_skinparam_handwritten.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    skinparams="skinparam handwritten true"
))

write_puml("seq_skinparam_padding.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    skinparams="skinparam ParticipantPadding 20\nskinparam BoxPadding 10"
))

write_puml("seq_skinparam_font.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    skinparams="skinparam DefaultFontSize 16\nskinparam DefaultFontName Arial"
))

# ---------------------------------------------------------------------------
# 36. HEADER/FOOTER/TITLE/CAPTION
# ---------------------------------------------------------------------------

write_puml("seq_title_basic.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    title="My Sequence Diagram"
))

write_puml("seq_title_multiword.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    title="Complex Title With Multiple Words"
))

write_puml("seq_header_footer.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    header="My Header",
    footer="Page 1"
))

write_puml("seq_header_footer_title.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    title="Title",
    header="Header Text",
    footer="Footer Text"
))

write_puml("seq_caption.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi\ncaption Figure 1: Basic interaction'
))

# ---------------------------------------------------------------------------
# 37. HIDE FOOTBOX
# ---------------------------------------------------------------------------

write_puml("seq_hide_footbox.puml", puml(
    'hide footbox\nAlice -> Bob : hello\nBob --> Alice : hi'
))

write_puml("seq_hide_footbox_with_activation.puml", puml(
    'hide footbox\nAlice -> Bob ++ : request\nBob --> Alice -- : response'
))

# ---------------------------------------------------------------------------
# 38. NEWPAGE
# ---------------------------------------------------------------------------

write_puml("seq_newpage_basic.puml", puml(
    'Alice -> Bob : page 1\nnewpage\nBob -> Charlie : page 2\nnewpage\nCharlie --> Alice : page 3'
))

write_puml("seq_newpage_titled.puml", puml(
    'Alice -> Bob : start\nnewpage Second Page\nBob -> Charlie : middle\nnewpage Third Page\nCharlie --> Alice : end'
))

# ---------------------------------------------------------------------------
# 39. TEOZ PRAGMA
# ---------------------------------------------------------------------------

write_puml("seq_teoz_basic.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    pragma="!pragma teoz true"
))

write_puml("seq_teoz_activation.puml", puml(
    'Alice -> Bob ++ : request\nBob -> Charlie ++ : delegate\nCharlie --> Bob -- : result\nBob --> Alice -- : done',
    pragma="!pragma teoz true"
))

write_puml("seq_teoz_complex.puml", puml(
    'Alice -> Bob : a\nBob -> Charlie : b\nnote right of Charlie : note\nCharlie --> Bob : c\nBob --> Alice : d',
    pragma="!pragma teoz true"
))

# ---------------------------------------------------------------------------
# 40. EDGE CASES
# ---------------------------------------------------------------------------

write_puml("seq_edge_empty.puml", "@startuml\n@enduml\n")

write_puml("seq_edge_single_participant.puml", puml(
    'participant Alice'
))

write_puml("seq_edge_single_participant_self.puml", puml(
    'participant Alice\nAlice -> Alice : self'
))

write_puml("seq_edge_no_participant_decl.puml", puml(
    'Alice -> Bob : implied participants\nBob --> Alice : reply'
))

# 20+ participants
participants_20 = "\n".join(f"participant P{i:02d}" for i in range(20))
messages_20 = "\n".join(f"P{i:02d} -> P{(i+1)%20:02d} : msg {i}" for i in range(20))
write_puml("seq_edge_20_participants.puml", puml(participants_20 + "\n" + messages_20))

# 50+ messages
body_50 = "participant Alice\nparticipant Bob\n"
body_50 += "\n".join(f"Alice -> Bob : message {i}\nBob --> Alice : reply {i}" for i in range(25))
write_puml("seq_edge_50_messages.puml", puml(body_50))

# Many self-messages
body_self = "participant Alice\n"
body_self += "\n".join(f"Alice -> Alice : self {i}" for i in range(20))
write_puml("seq_edge_many_self_messages.puml", puml(body_self))

# Unicode everywhere
write_puml("seq_edge_unicode_everywhere.puml", puml(
    'participant "Ünïcödé Üser" as U\nparticipant "服务器" as S\nU -> S : 请求数据\nnote over U : 等待响应...\nS --> U : データを返す'
))

# Special characters in messages
write_puml("seq_edge_special_chars.puml", puml(
    'Alice -> Bob : hello & world\nBob --> Alice : <html> content\nAlice -> Bob : "quoted" message\nBob --> Alice : message with \\n escape'
))

# Deep nesting
deep = "A -> B : start\n"
for i in range(5):
    deep += f"loop level {i+1}\n"
deep += "B -> C : deep\nC --> B : reply\n"
for i in range(5):
    deep += "end\n"
deep += "B --> A : done"
write_puml("seq_edge_deep_nesting.puml", puml(deep))

# ---------------------------------------------------------------------------
# 41. COMBINATIONS - participant types + colors + stereotypes + order
# ---------------------------------------------------------------------------

combo_types = ["actor", "boundary", "control", "entity", "database"]
for ptype in combo_types:
    body = (f'{ptype} "Alice Smith" as A <<user>> #lightblue order 10\n'
            f'{ptype} "Bob Jones" as B <<service>> #lightyellow order 20\n'
            f'A -> B : request\nnote over A : thinking\n'
            f'B --> A : response\nnote right of B : processed')
    write_puml(f"seq_combo_{ptype}_full.puml", puml(body, title=f"{ptype} full combo"))

# ---------------------------------------------------------------------------
# 42. COMBINATIONS - activation + groups + notes
# ---------------------------------------------------------------------------

write_puml("seq_combo_activation_groups_notes.puml", puml(
    'Alice -> Bob ++ : start\nalt success\n  note over Bob : processing\n  Bob -> DB ++ : query\n  DB --> Bob -- : data\n  Bob --> Alice -- : result\nelse error\n  note right of Bob : error!\n  Bob --> Alice -- : failure\nend'
))

write_puml("seq_combo_loop_activation_autonumber.puml", puml(
    'autonumber\nClient -> Server ++ : connect\nloop 3 times\n  Server -> DB ++ : fetch\n  DB --> Server -- : record\n  Server -> Client : push record\nend\nServer --> Client -- : done'
))

write_puml("seq_combo_all_note_types.puml", puml(
    'Alice -> Bob : hello\nnote left of Alice : left note\nnote right of Bob : right note\nnote over Alice : over Alice\nnote over Bob : over Bob\nnote over Alice, Bob : over both\nhnote over Alice : hex note\nrnote over Bob : rect note\nBob --> Alice : hi'
))

# ---------------------------------------------------------------------------
# 43. COMBINATIONS - boxes + activation + groups
# ---------------------------------------------------------------------------

write_puml("seq_combo_box_activation_groups.puml", puml(
    'box "Client Side" #lightblue\n  actor User\n  participant Browser\nend box\nbox "Server Side" #lightyellow\n  participant API\n  database DB\nend box\n\nUser -> Browser : interact\nBrowser -> API ++ : request\nalt authenticated\n  API -> DB ++ : query\n  DB --> API -- : results\n  API --> Browser -- : 200 OK\nelse\n  API --> Browser -- : 401\nend\nBrowser --> User : display'
))

# ---------------------------------------------------------------------------
# 44. COMBINATIONS - autonumber + groups + activation
# ---------------------------------------------------------------------------

write_puml("seq_combo_autonumber_groups.puml", puml(
    'autonumber\nAlice -> Bob ++ : request\nalt happy path\n  Bob -> Charlie ++ : delegate\n  loop process items\n    Charlie -> DB : fetch\n    DB --> Charlie : item\n  end\n  Charlie --> Bob -- : results\n  Bob --> Alice -- : success\nelse error path\n  Bob --> Alice -- : error\nend'
))

# ---------------------------------------------------------------------------
# 45. COMBINATIONS - skinparam + complex diagram
# ---------------------------------------------------------------------------

write_puml("seq_combo_skinparam_complex.puml", puml(
    'box "Frontend" #E3F2FD\n  actor User\n  participant UI\nend box\nbox "Backend" #E8F5E9\n  participant API\n  database DB\nend box\n\nautонumber\nUser -> UI ++ : login\nUI -> API ++ #red : POST /auth\nalt valid credentials\n  API -> DB ++ : check user\n  DB --> API -- : user found\n  API --> UI -- : 200 {token}\n  UI --> User -- : welcome!\nelse invalid\n  API --> UI -- : 401\n  UI --> User -- : error\nend',
    skinparams="""skinparam SequenceArrowColor #1565C0
skinparam ParticipantBorderColor #1565C0
skinparam NoteBackgroundColor #FFF9C4"""
))

# ---------------------------------------------------------------------------
# 46. MORE ACTIVATION VARIANTS
# ---------------------------------------------------------------------------

write_puml("seq_activate_multiple_return.puml", puml(
    'Alice -> Bob ++ : call\nBob -> Charlie ++ : sub-call\nBob -> Dave ++ : another sub\nDave --> Bob -- : dave done\nCharlie --> Bob -- : charlie done\nBob --> Alice -- : all done'
))

write_puml("seq_activate_same_participant.puml", puml(
    'Alice -> Bob ++ : first\nAlice -> Bob ++ : second (nested)\nBob --> Alice -- : second done\nBob --> Alice -- : first done'
))

write_puml("seq_activate_colored_multiple.puml", puml(
    'Alice -> Bob ++ #red : danger\nBob -> Charlie ++ #green : safe\nCharlie -> Dave ++ #blue : info\nDave --> Charlie -- : done\nCharlie --> Bob -- : ok\nBob --> Alice -- : result'
))

# ---------------------------------------------------------------------------
# 47. MESSAGE NUMBERING VARIANTS
# ---------------------------------------------------------------------------

write_puml("seq_autonumber_html_format.puml", puml(
    'autonumber "<b><font color=blue>(##)</font></b>"\nAlice -> Bob : first\nBob --> Alice : second\nAlice -> Charlie : third\nCharlie --> Alice : fourth'
))

write_puml("seq_autonumber_with_groups.puml", puml(
    'autonumber\nAlice -> Bob : 1\nalt path\n  Bob -> Charlie : 2\n  Charlie --> Bob : 3\nelse other\n  Bob -> Dave : 4\n  Dave --> Bob : 5\nend\nBob --> Alice : 6'
))

write_puml("seq_autonumber_restart.puml", puml(
    'autonumber\nAlice -> Bob : 1\nBob --> Alice : 2\nautonumber 1\nAlice -> Bob : 1 again\nBob --> Alice : 2 again'
))

# ---------------------------------------------------------------------------
# 48. REFERENCE VARIANTS
# ---------------------------------------------------------------------------

write_puml("seq_ref_multiple.puml", puml(
    'Alice -> Bob : start\nref over Alice : ref 1\nref over Bob : ref 2\nref over Alice, Bob : ref 3\nBob --> Alice : done'
))

write_puml("seq_ref_complex.puml", puml(
    'participant Alice\nparticipant Bob\nparticipant Charlie\nAlice -> Bob : start\nref over Alice, Bob, Charlie : Complex Reference\nBob --> Alice : done'
))

# ---------------------------------------------------------------------------
# 49. DIVIDER STYLING
# ---------------------------------------------------------------------------

write_puml("seq_divider_styled.puml", puml(
    'Alice -> Bob : start\n== <font color=red>Red Phase</font> ==\nBob -> Charlie : middle\n== <b>Bold Phase</b> ==\nCharlie --> Alice : done'
))

write_puml("seq_divider_empty.puml", puml(
    'Alice -> Bob : start\n====\nBob --> Alice : end'
))

# ---------------------------------------------------------------------------
# 50. COMPREHENSIVE / KITCHEN SINK TESTS
# ---------------------------------------------------------------------------

write_puml("seq_kitchen_sink_01.puml", puml(
    '''box "Clients" #E3F2FD
  actor "End User" as User
  participant Browser #lightblue
end box
box "Services" #E8F5E9
  participant "Auth Service" as Auth <<service>>
  participant "Data Service" as Data <<service>>
  database "Main DB" as DB
end box

title Full Authentication Flow
header Sequence Diagram v1.0
footer Generated by RustUML

autonumber

User -> Browser ++ : navigate to app
Browser -> Auth ++ : GET /session
Auth -> DB ++ : check session
DB --> Auth -- : no session
Auth --> Browser -- : 302 /login
Browser --> User -- : show login

User -> Browser ++ : submit credentials
Browser -> Auth ++ : POST /login
alt valid credentials
  Auth -> DB ++ : verify user
  DB --> Auth -- : user OK
  Auth -> Auth : generate JWT
  Auth --> Browser -- : 200 {token}
  Browser -> Data ++ : GET /data
  loop for each page
    Data -> DB ++ : query page
    DB --> Data -- : records
    Data --> Browser : stream data
  end
  Data --> Browser -- : complete
  Browser --> User -- : show data
else invalid credentials
  Auth --> Browser -- : 401 Unauthorized
  Browser --> User -- : show error
end''',
    skinparams="""skinparam SequenceArrowColor #1565C0
skinparam ParticipantBorderColor #1565C0"""
))

write_puml("seq_kitchen_sink_02.puml", puml(
    '''participant Producer as P
participant "Message Bus" as MB <<queue>>
participant Consumer1 as C1
participant Consumer2 as C2
participant "Dead Letter Queue" as DLQ <<queue>>

title Async Message Processing

P -> MB : publish(msg1)
P -> MB : publish(msg2)
P -> MB : publish(msg3)

par
  MB -> C1 ++ : deliver msg1
  alt processed OK
    C1 -> C1 : process
    C1 --> MB -- : ack
  else processing failed
    C1 --> MB -- : nack
    MB -> DLQ : move to DLQ
  end
and
  MB -> C2 ++ : deliver msg2
  C2 -> C2 : process
  C2 --> MB -- : ack
and
  MB -> C2 ++ : deliver msg3
  C2 --> MB -- : nack
  MB -> DLQ : dead letter
end

note over DLQ : 1 message in DLQ'''
))

write_puml("seq_kitchen_sink_03.puml", puml(
    '''!pragma teoz true

actor Alice
actor Bob
participant System

title Concurrent Operations

Alice -> System ++ : async request A
Bob -> System ++ : async request B

System -> System : process A
System -> System : process B

System --> Alice -- : result A
System --> Bob -- : result B

note across : Both operations complete independently''',
    pragma="!pragma teoz true"
))

write_puml("seq_kitchen_sink_04.puml", puml(
    '''hide footbox

participant "<<Component>>\nFrontend" as FE
participant "<<Component>>\nBackend" as BE
database "<<Database>>\nPostgres" as PG
collections "<<Cache>>\nRedis" as Redis

autonumber "<b>[0]"

FE -> BE : getUser(id)
activate BE #lightblue

BE -> Redis : get("user:" + id)
activate Redis #lightyellow
alt cache hit
  Redis --> BE : userData
  deactivate Redis
else cache miss
  deactivate Redis
  BE -> PG ++ : SELECT * FROM users WHERE id=?
  PG --> BE -- : row
  BE -> Redis ++ : set("user:" + id, userData, TTL=3600)
  Redis --> BE -- : OK
end

BE --> FE : User object
deactivate BE''',
    title="User Retrieval with Cache",
    skinparams="skinparam ParticipantPadding 20"
))

write_puml("seq_kitchen_sink_05.puml", puml(
    '''participant A
participant B
participant C
participant D
participant E

== Phase 1: Setup ==
A -> B : init
B -> C : configure
C -> D : prepare
D -> E : ready
E --> A : all set

== Phase 2: Operation ==
loop 3 times
  A -> B ++ : request
  B -> C ++ : process
  C -> D ++ : fetch
  D -> E ++ : query
  E --> D -- : data
  D --> C -- : result
  C --> B -- : processed
  B --> A -- : response
end

... some time passes ...

== Phase 3: Teardown ==
A -> B : shutdown
B -> C : stop
C -> D : close
D -> E : disconnect
E --> A : cleaned up

note over A, E : All phases complete'''
))

# ---------------------------------------------------------------------------
# 51. MORE EDGE CASES
# ---------------------------------------------------------------------------

# Activate without prior message
write_puml("seq_edge_activate_direct.puml", puml(
    'participant Alice\nparticipant Bob\nactivate Bob\nAlice -> Bob : hello\nBob --> Alice : hi\ndeactivate Bob'
))

# Multiple self-activations
write_puml("seq_edge_self_activation_nested.puml", puml(
    'Alice -> Alice ++ : outer\nAlice -> Alice ++ : middle\nAlice -> Alice ++ : inner\nAlice --> Alice -- : inner done\nAlice --> Alice -- : middle done\nAlice --> Alice -- : outer done'
))

# Lost message with activation
write_puml("seq_edge_lost_with_activation.puml", puml(
    'Alice -> Bob ++ : start\nAlice ->x Bob : lost\nBob --> Alice -- : done'
))

# Found message
write_puml("seq_edge_found_message.puml", puml(
    '[-> Bob : found incoming\nBob -> Alice : forwarded\nAlice ->] : sent out'
))

# Mix of arrow styles in one diagram
write_puml("seq_edge_all_arrows.puml", puml(
    'Alice -> Bob : solid\nAlice --> Bob : dotted\nAlice ->> Bob : thin solid\nAlice -->> Bob : thin dotted\nAlice ->x Bob : lost\nAlice <- Bob : back solid\nAlice <-- Bob : back dotted\nAlice <<- Bob : back thin\nAlice <<-- Bob : back thin dotted'
))

# ---------------------------------------------------------------------------
# 52. PARTICIPANT CREATE/DESTROY
# ---------------------------------------------------------------------------

write_puml("seq_create_basic.puml", puml(
    'Alice -> Bob : hello\ncreate Charlie\nBob -> Charlie : create\nCharlie --> Bob : created\nBob --> Alice : done'
))

write_puml("seq_create_with_type.puml", puml(
    'Alice -> Bob : process\ncreate control Controller\nBob -> Controller : initialize\nController --> Bob : ready\nBob --> Alice : initialized'
))

write_puml("seq_destroy_basic.puml", puml(
    'Alice -> Bob : use\nBob --> Alice : done\ndestroy Bob'
))

write_puml("seq_create_destroy.puml", puml(
    'Alice -> Alice : start\ncreate Bob\nAlice -> Bob : init\nBob --> Alice : ready\nAlice -> Bob : work\nBob --> Alice : done\ndestroy Bob'
))

# ---------------------------------------------------------------------------
# 53. RETURN VARIANTS
# ---------------------------------------------------------------------------

write_puml("seq_return_basic.puml", puml(
    'Alice -> Bob ++ : call\nreturn value'
))

write_puml("seq_return_complex.puml", puml(
    'Alice -> Bob ++ : step1\nBob -> Charlie ++ : step2\nCharlie -> Dave ++ : step3\nreturn d3\nreturn d2\nreturn d1'
))

write_puml("seq_return_empty.puml", puml(
    'Alice -> Bob ++ : call\nreturn'
))

write_puml("seq_return_with_note.puml", puml(
    'Alice -> Bob ++ : request\nBob -> DB ++ : query\nreturn data\nnote right of Bob : got data\nreturn result'
))

# ---------------------------------------------------------------------------
# 54. TEOZ VARIANTS
# ---------------------------------------------------------------------------

write_puml("seq_teoz_parallel.puml", puml(
    'Alice -> Bob : a\nAlice -> Charlie : b\nBob --> Alice : c\nCharlie --> Alice : d',
    pragma="!pragma teoz true"
))

write_puml("seq_teoz_with_groups.puml", puml(
    'Alice -> Bob : start\nalt path\n  Bob -> Charlie : delegate\n  Charlie --> Bob : result\n  Bob --> Alice : success\nelse\n  Bob --> Alice : fail\nend',
    pragma="!pragma teoz true"
))

# ---------------------------------------------------------------------------
# 55. MULTILINE TITLE/HEADER/FOOTER
# ---------------------------------------------------------------------------

write_puml("seq_title_multiline.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    title="Line One\\nLine Two\\nLine Three"
))

write_puml("seq_header_multiline.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    header="Header Line 1\\nHeader Line 2"
))

# ---------------------------------------------------------------------------
# 56. MIXED PARTICIPANT DECLARATIONS AND IMPLICIT
# ---------------------------------------------------------------------------

write_puml("seq_mixed_declared_implicit.puml", puml(
    'participant Alice\nAlice -> Bob : Bob is implicit\nBob -> Charlie : Charlie is implicit\nCharlie --> Bob : return\nBob --> Alice : return'
))

write_puml("seq_mixed_order_with_implicit.puml", puml(
    'participant Bob order 20\nparticipant Alice order 10\nAlice -> Bob : first\nBob -> Charlie : Charlie appears at end\nCharlie --> Bob : back\nBob --> Alice : done'
))

# ---------------------------------------------------------------------------
# 57. NOTES IN DIFFERENT POSITIONS
# ---------------------------------------------------------------------------

write_puml("seq_note_before_first_message.puml", puml(
    'note over Alice : pre-condition\nAlice -> Bob : hello\nBob --> Alice : hi'
))

write_puml("seq_note_after_last_message.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi\nnote over Alice : post-condition'
))

write_puml("seq_note_between_messages.puml", puml(
    'Alice -> Bob : step 1\nnote over Alice, Bob : checkpoint\nBob -> Charlie : step 2\nnote right of Charlie : done\nCharlie --> Alice : complete'
))

# ---------------------------------------------------------------------------
# 58. SKINPARAM SEQUENCE-SPECIFIC
# ---------------------------------------------------------------------------

SEQUENCE_SKINPARAMS = [
    ("MessageAlign", "center"),
    ("MessageAlign", "right"),
    ("MessageAlign", "left"),
    ("ArrowThickness", "2"),
    ("ArrowThickness", "3"),
    ("LifeLineStrategy", "nosolid"),
    ("ResponseMessageBelowArrow", "true"),
]

for param, value in SEQUENCE_SKINPARAMS:
    safe_value = value.replace(" ", "_")
    body = "Alice -> Bob : hello\nBob --> Alice : hi"
    write_puml(
        f"seq_skinparam_seq_{param.lower()}_{safe_value}.puml",
        puml(body, skinparams=f"skinparam Sequence{param} {value}")
    )

# ---------------------------------------------------------------------------
# 59. COMBINATIONS - groups + notes + activation + autonumber + boxes
# ---------------------------------------------------------------------------

write_puml("seq_combo_everything.puml", puml(
    '''autonumber
hide footbox

box "User Interface" #lightblue
  actor "Customer" as Cust
  participant "Web Browser" as Web
end box

box "Application" #lightyellow
  boundary "Load Balancer" as LB
  control "App Server" as App
  entity "Session Manager" as SM
end box

box "Data" #lightgreen
  database "Primary DB" as DB
  collections "Cache Cluster" as Cache
  queue "Event Queue" as EQ
end box

title Complete E-Commerce Checkout Flow
header System Architecture v2.0
footer Confidential

Cust -> Web ++ : click checkout
Web -> LB ++ : POST /checkout
LB -> App ++ : forward request

note over App : validating request

alt session valid
  App -> SM ++ : validate session
  SM --> App -- : session OK

  opt items in cart
    App -> Cache ++ : get cart items
    alt cache hit
      Cache --> App -- : cart data
    else cache miss
      App -> DB ++ : fetch cart
      DB --> App -- : cart data
      App -> Cache ++ : cache cart
      Cache --> App -- : cached
    end
  end

  loop for each item
    App -> DB ++ : check inventory
    DB --> App -- : available
  end

  critical payment processing
    App -> EQ : queue payment event
    App -> DB ++ : create order
    DB --> App -- : order_id=12345
  end

  App --> LB -- : 200 {order_id}
  LB --> Web -- : 200
  Web --> Cust -- : order confirmed

else session invalid
  App --> LB -- : 401
  LB --> Web -- : 401
  Web --> Cust -- : please login
end''',
    skinparams="""skinparam SequenceArrowColor #1A237E
skinparam ParticipantBorderColor #1A237E
skinparam ParticipantBackgroundColor #FAFAFA"""
))

# ---------------------------------------------------------------------------
# 60. MORE CREOLE IN NOTES AND MESSAGES
# ---------------------------------------------------------------------------

write_puml("seq_creole_table_in_note.puml", puml(
    'Alice -> Bob : request\nnote over Bob\n  | Column 1 | Column 2 |\n  | value A  | value B  |\n  | value C  | value D  |\nend note\nBob --> Alice : response'
))

write_puml("seq_creole_list_in_note.puml", puml(
    'Alice -> Bob : request\nnote right of Bob\n  Steps:\n  * Step one\n  * Step two\n  * Step three\nend note\nBob --> Alice : done'
))

write_puml("seq_creole_code_in_note.puml", puml(
    'Alice -> Bob : request\nnote over Alice\n  <code>\n  function foo() {\n    return 42;\n  }\n  </code>\nend note\nBob --> Alice : response'
))

# ---------------------------------------------------------------------------
# 61. PARTICIPANT COLORS - various formats
# ---------------------------------------------------------------------------

COLOR_FORMATS = [
    ("#FF0000", "hex_red"),
    ("#00FF00", "hex_green"),
    ("#0000FF", "hex_blue"),
    ("red", "named_red"),
    ("lightblue", "named_lightblue"),
    ("#FFAAAA", "hex_pink"),
    ("AliceBlue", "named_aliceblue"),
]

for color, name in COLOR_FORMATS:
    body = f'participant Alice {color}\nparticipant Bob\nAlice -> Bob : hello\nBob --> Alice : hi'
    write_puml(f"seq_participant_color_format_{name}.puml", puml(body))

# ---------------------------------------------------------------------------
# 62. ARROW STYLE COMBINATIONS
# ---------------------------------------------------------------------------

arrow_combos = [
    ("->", "-->", "basic fwd dotted back"),
    ("->>", "-->>", "thin fwd thin dotted back"),
    ("->x", "-->", "lost fwd dotted back"),
    ("-[#red]>", "-[#blue]-->", "colored mixed"),
    ("->>", "<--", "thin fwd back left"),
]

for i, (fwd, back, desc) in enumerate(arrow_combos):
    body = f"Alice {fwd} Bob : forward ({desc})\nBob {back} Alice : backward"
    write_puml(f"seq_arrow_combo_{i+1:02d}.puml", puml(body))

# ---------------------------------------------------------------------------
# 63. COMPLEX ACTIVATION PATTERNS
# ---------------------------------------------------------------------------

write_puml("seq_activate_chain.puml", puml(
    'A -> B ++ : 1\nB -> C ++ : 2\nC -> D ++ : 3\nD -> E ++ : 4\nE --> D -- : 5\nD --> C -- : 6\nC --> B -- : 7\nB --> A -- : 8'
))

write_puml("seq_activate_parallel_chains.puml", puml(
    'Alice -> Bob ++ : start\nBob -> C1 ++ : chain 1\nBob -> C2 ++ : chain 2\nC1 -> D1 ++ : sub 1\nC2 -> D2 ++ : sub 2\nD1 --> C1 -- : done 1\nD2 --> C2 -- : done 2\nC1 --> Bob -- : chain 1 done\nC2 --> Bob -- : chain 2 done\nBob --> Alice -- : all done'
))

write_puml("seq_activate_reactivation.puml", puml(
    'Alice -> Bob ++ : first call\nBob --> Alice -- : first result\nAlice -> Bob ++ : second call\nBob --> Alice -- : second result\nAlice -> Bob ++ : third call\nBob --> Alice -- : third result'
))

# ---------------------------------------------------------------------------
# 64. GROUP LABELS WITH SPECIAL CONTENT
# ---------------------------------------------------------------------------

write_puml("seq_group_label_expression.puml", puml(
    'Alice -> Bob : check\nalt x > 0 && y < 100\n  Bob --> Alice : in range\nelse x <= 0 || y >= 100\n  Bob --> Alice : out of range\nend'
))

write_puml("seq_group_label_unicode.puml", puml(
    'Alice -> Bob : request\nloop データ処理中\n  Bob -> DB : 取得\n  DB --> Bob : データ\nend\nBob --> Alice : 完了'
))

write_puml("seq_group_loop_with_bounds.puml", puml(
    'Alice -> Bob : start\nloop 1..n\n  Bob -> DB : step\n  DB --> Bob : ok\nend\nBob --> Alice : done'
))

# ---------------------------------------------------------------------------
# 65. STYLING VARIANTS
# ---------------------------------------------------------------------------

write_puml("seq_style_shadowing.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    skinparams="skinparam shadowing true"
))

write_puml("seq_style_no_shadowing.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    skinparams="skinparam shadowing false"
))

write_puml("seq_style_rounded.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    skinparams="skinparam RoundCorner 15"
))

write_puml("seq_style_thick_border.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    skinparams="skinparam ParticipantBorderThickness 3"
))

# ---------------------------------------------------------------------------
# 66. COMPREHENSIVE PARTICIPANT TYPE COMBINATIONS
# ---------------------------------------------------------------------------

# All pairs
for t1, t2 in itertools.combinations(PARTICIPANT_TYPES, 2):
    body = (f'{t1} "Source" as Src\n{t2} "Target" as Tgt\n'
            f'Src -> Tgt : request\nTgt --> Src : response\n'
            f'note over Src : src note\nnote over Tgt : tgt note')
    write_puml(f"seq_types_pair_full_{t1}_{t2}.puml", puml(body))

# ---------------------------------------------------------------------------
# 67. SPECIAL: sequence diagrams that test rendering specifics
# ---------------------------------------------------------------------------

write_puml("seq_render_overlapping_activations.puml", puml(
    'Alice -> Bob ++ : a\nAlice -> Bob ++ : b (nested)\nAlice -> Bob ++ : c (triple)\nBob --> Alice -- : c done\nBob --> Alice -- : b done\nBob --> Alice -- : a done'
))

write_puml("seq_render_crossing_messages.puml", puml(
    'Alice -> Bob : forward\nBob -> Alice : crossing\nAlice -> Bob : another'
))

write_puml("seq_render_many_notes.puml", puml(
    'Alice -> Bob : step 1\nnote left of Alice : note 1\nnote right of Bob : note 2\nBob -> Charlie : step 2\nnote over Bob : note 3\nnote over Charlie : note 4\nCharlier --> Bob : step 3\nnote left of Bob : note 5\nBob --> Alice : step 4\nnote over Alice, Bob : note 6'
))

write_puml("seq_render_long_participant_names.puml", puml(
    'participant "VeryLongParticipantNameOne" as P1\nparticipant "VeryLongParticipantNameTwo" as P2\nparticipant "VeryLongParticipantNameThree" as P3\nP1 -> P2 : message with reasonable length\nP2 -> P3 : another message\nP3 --> P1 : return to first'
))

# ---------------------------------------------------------------------------
# 68. ADDITIONAL COMBINATIONS
# ---------------------------------------------------------------------------

write_puml("seq_combo_dividers_in_groups.puml", puml(
    'Alice -> Bob : start\nloop processing\n  Bob -> DB : fetch\n  DB --> Bob : data\n  == checkpoint ==\n  Bob -> Bob : validate\nend\nBob --> Alice : done'
))

write_puml("seq_combo_delay_in_groups.puml", puml(
    'Alice -> Bob : request\nalt async\n  Bob -> External : call\n  ...\n  External --> Bob : callback\n  Bob --> Alice : result\nelse sync\n  Bob -> Internal : call\n  Internal --> Bob : result\n  Bob --> Alice : result\nend'
))

write_puml("seq_combo_ref_in_groups.puml", puml(
    'Alice -> Bob : start\nalt success\n  ref over Bob : See Diagram B\n  Bob --> Alice : ok\nelse\n  ref over Alice, Bob : See Error Handling Diagram\n  Bob --> Alice : error\nend'
))

# ---------------------------------------------------------------------------
# 69. LOST/FOUND MESSAGE COMBINATIONS
# ---------------------------------------------------------------------------

write_puml("seq_lost_found_combined.puml", puml(
    '[-> Alice : incoming\nAlice -> Bob : forward\nBob ->x Charlie : lost\n[-> Bob : another incoming\nBob ->] : outgoing\nAlice -->] : also outgoing'
))

write_puml("seq_lost_in_group.puml", puml(
    'Alice -> Bob : start\nloop retrying\n  Bob ->x External : attempt\n  ... retry delay ...\nend\nBob --> Alice : gave up'
))

# ---------------------------------------------------------------------------
# 70. FINAL BATCH: exhaustive variants
# ---------------------------------------------------------------------------

# All group types in sequence
for gtype in ["alt", "opt", "loop", "par", "break", "critical"]:
    if gtype == "alt":
        body = f'A -> B : go\n{gtype} condition\n  B -> C : action\n  C --> B : done\nelse other\n  B -> B : handle\nend\nB --> A : result'
    elif gtype == "par":
        body = f'A -> B : go\n{gtype}\n  B -> C : action 1\nand\n  B -> D : action 2\nend\nB --> A : result'
    elif gtype == "break":
        body = f'A -> B : go\nloop forever\n  B -> B : work\n  {gtype} on error\n    B --> A : error\n  end\nend'
    else:
        body = f'A -> B : go\n{gtype} condition\n  B -> C : action\n  C --> B : done\nend\nB --> A : result'
    write_puml(f"seq_group_{gtype}_standalone.puml", puml(body))

# Autonumber with all group types
for gtype in ["alt", "opt", "loop"]:
    if gtype == "alt":
        body = f'autonumber\nA -> B : 1\n{gtype} path\n  B -> C : 2\n  C --> B : 3\nelse\n  B -> D : 4\n  D --> B : 5\nend\nB --> A : last'
    else:
        body = f'autonumber\nA -> B : 1\n{gtype} condition\n  B -> C : 2\n  C --> B : 3\nend\nB --> A : last'
    write_puml(f"seq_autonumber_{gtype}.puml", puml(body))

# Notes with all participant types
for ptype in PARTICIPANT_TYPES:
    body = (f'{ptype} X\nparticipant Y\n'
            f'X -> Y : hello\n'
            f'note over X : note over {ptype}\n'
            f'note right of X : right of {ptype}\n'
            f'Y --> X : hi')
    write_puml(f"seq_note_with_{ptype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 71. EXHAUSTIVE PARTICIPANT TYPE × MESSAGE TYPE COMBOS
# ---------------------------------------------------------------------------

ARROW_VARIANTS = [
    ("->", "solid"),
    ("-->", "dotted"),
    ("->>", "thin_solid"),
    ("-->>", "thin_dotted"),
]

for ptype in PARTICIPANT_TYPES:
    for arrow, aname in ARROW_VARIANTS:
        body = (f'{ptype} Alice\nparticipant Bob\n'
                f'Alice {arrow} Bob : {ptype} sends {aname}\n'
                f'Bob {arrow} Alice : {ptype} receives {aname}')
        write_puml(f"seq_ptype_{ptype}_arrow_{aname}.puml", puml(body))

# ---------------------------------------------------------------------------
# 72. PARTICIPANT TYPE × ACTIVATION SHORTHAND
# ---------------------------------------------------------------------------

for ptype in PARTICIPANT_TYPES:
    body = (f'{ptype} Alice\nparticipant Bob\n'
            f'Alice -> Bob ++ : activate {ptype}\n'
            f'Bob --> Alice -- : deactivate {ptype}')
    write_puml(f"seq_ptype_{ptype}_activation.puml", puml(body))

for ptype in PARTICIPANT_TYPES:
    body = (f'{ptype} Alice\nparticipant Bob\nparticipant Charlie\n'
            f'Alice -> Bob ++ : call with {ptype}\n'
            f'Bob -> Charlie ++ : delegate\n'
            f'Charlie --> Bob -- : result\n'
            f'Bob --> Alice -- : done')
    write_puml(f"seq_ptype_{ptype}_nested_activation.puml", puml(body))

# ---------------------------------------------------------------------------
# 73. PARTICIPANT TYPE × GROUPS
# ---------------------------------------------------------------------------

GROUP_BODIES = [
    ("alt", "alt condition\n  {src} -> {tgt} : action\n  {tgt} --> {src} : ok\nelse other\n  {tgt} --> {src} : fail\nend"),
    ("opt", "opt optional\n  {src} -> {tgt} : maybe\n  {tgt} --> {src} : done\nend"),
    ("loop", "loop 3 times\n  {src} -> {tgt} : iterate\n  {tgt} --> {src} : next\nend"),
    ("par", "par\n  {src} -> {tgt} : branch 1\nand\n  {src} -> {tgt} : branch 2\nend"),
]

for ptype in PARTICIPANT_TYPES:
    for gname, gbody in GROUP_BODIES:
        body = (f'{ptype} Src\nparticipant Tgt\n'
                f'Src -> Tgt : start\n'
                + gbody.format(src="Src", tgt="Tgt") +
                '\nTgt --> Src : done')
        write_puml(f"seq_ptype_{ptype}_group_{gname}.puml", puml(body))

# ---------------------------------------------------------------------------
# 74. ACTIVATION × ARROW TYPE
# ---------------------------------------------------------------------------

for arrow, aname in ARROW_VARIANTS:
    for suffix, sname in [("++", "up"), ("++ #red", "up_red"), ("++ #blue", "up_blue")]:
        body = (f'Alice {arrow} Bob {suffix} : {aname} activate {sname}\n'
                f'Bob {arrow} Alice -- : return')
        safe = sname.replace(" ", "_").replace("#", "")
        write_puml(f"seq_activate_{aname}_{safe}.puml", puml(body))

# ---------------------------------------------------------------------------
# 75. GROUPS × GROUPS (all 2-combos nested)
# ---------------------------------------------------------------------------

GROUP_PAIRS = list(itertools.combinations(["alt", "opt", "loop", "par", "critical"], 2))

for g1, g2 in GROUP_PAIRS:
    if g1 == "par":
        outer = f"par\n  A -> B : branch1\nand\n  {g2} inner\n    B -> C : inner action\n    C --> B : done\n  end\nend"
    elif g1 == "alt":
        outer = f"alt cond\n  {g2} inner{'\\n  A -> B : cond' if g2 == 'opt' else '\\n  loop 2 times' if g2 == 'loop' else ''}\n    A -> B : {g2} action\n    B --> A : {g2} done\n  end\n  B --> A : ok\nelse\n  B --> A : else\nend"
        # simpler version to avoid syntax issues
        outer = (f"alt condition\n"
                 f"  A -> B : action\n"
                 f"  {g2} inner_condition\n"
                 f"    B -> C : nested\n"
                 f"    C --> B : result\n"
                 f"  end\n"
                 f"  B --> A : ok\n"
                 f"else other\n"
                 f"  B --> A : fail\n"
                 f"end")
    else:
        outer = (f"{g1} outer_condition\n"
                 f"  A -> B : outer\n"
                 f"  {g2} inner_condition\n"
                 f"    B -> C : inner\n"
                 f"    C --> B : inner done\n"
                 f"  end\n"
                 f"  B --> A : outer done\n"
                 f"end")
    body = f"participant A\nparticipant B\nparticipant C\nA -> B : start\n{outer}\nB --> A : complete"
    write_puml(f"seq_nested_{g1}_{g2}.puml", puml(body))

# ---------------------------------------------------------------------------
# 76. AUTONUMBER × FORMAT STRINGS
# ---------------------------------------------------------------------------

AUTONUMBER_FORMATS = [
    ('""', "plain"),
    ('"<b>[##]</b>"', "bold_brackets"),
    ('"(##)"', "parens"),
    ('"<font color=red>##</font>"', "red"),
    ('"Step ##:"', "step_prefix"),
    ('"[<b>##</b>]"', "bold_square"),
    ('"<u>##</u>"', "underline"),
    ('"<i>##</i>"', "italic"),
]

for fmt, fname in AUTONUMBER_FORMATS:
    body = f'autonumber {fmt}\nAlice -> Bob : first\nBob --> Alice : second\nAlice -> Charlie : third\nCharlie --> Alice : fourth'
    write_puml(f"seq_autonumber_fmt_{fname}.puml", puml(body))

# Autonumber with start values
for start in [1, 5, 10, 100]:
    body = f'autonumber {start}\nAlice -> Bob : msg\nBob --> Alice : reply\nAlice -> Bob : msg2\nBob --> Alice : reply2'
    write_puml(f"seq_autonumber_start_{start}.puml", puml(body))

# Autonumber with step values
for step in [1, 2, 5, 10]:
    body = f'autonumber 1 {step}\nAlice -> Bob : msg\nBob --> Alice : reply\nAlice -> Bob : msg2'
    write_puml(f"seq_autonumber_step_{step}.puml", puml(body))

# ---------------------------------------------------------------------------
# 77. NOTE POSITIONS × PARTICIPANT TYPES
# ---------------------------------------------------------------------------

NOTE_POSITIONS = ["left of", "right of", "over"]

for ptype in PARTICIPANT_TYPES:
    for pos in NOTE_POSITIONS:
        body = (f'{ptype} X\nparticipant Y\n'
                f'X -> Y : message\n'
                f'note {pos} X : note {pos} {ptype}\n'
                f'Y --> X : reply')
        safe_pos = pos.replace(" ", "_")
        write_puml(f"seq_note_{safe_pos}_{ptype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 78. NOTE COLORS × POSITIONS
# ---------------------------------------------------------------------------

NOTE_COLORS = ["#red", "#blue", "#green", "#yellow", "#pink", "#lightblue", "#FFEEAA", "#EEFFEE"]

for color in NOTE_COLORS:
    safe = color.replace("#", "color_")
    body = (f'Alice -> Bob : hello\n'
            f'note over Alice {color} : colored note {color}\n'
            f'note right of Bob {color} : right colored\n'
            f'Bob --> Alice : hi')
    write_puml(f"seq_note_{safe}.puml", puml(body))

# ---------------------------------------------------------------------------
# 79. BOX × PARTICIPANT TYPE COMBINATIONS
# ---------------------------------------------------------------------------

for ptype in PARTICIPANT_TYPES:
    body = (f'box "Group"\n  {ptype} A\n  {ptype} B\nend box\n'
            f'participant C\n'
            f'A -> B : inside box\n'
            f'B -> C : outside box\n'
            f'C --> A : return')
    write_puml(f"seq_box_with_{ptype}.puml", puml(body))

# Box colors × participant types
BOX_COLORS = ["#lightblue", "#lightyellow", "#lightgreen", "#FFE4E1", "#F0E68C"]
for color in BOX_COLORS:
    safe = color.replace("#", "box_color_")
    body = (f'box "Colored Box" {color}\n  participant A\n  participant B\nend box\n'
            f'participant C\n'
            f'A -> B : inside\nB -> C : outside\nC --> A : return')
    write_puml(f"seq_{safe}.puml", puml(body))

# ---------------------------------------------------------------------------
# 80. SKINPARAM SEQUENCES × PARTICIPANT COUNTS
# ---------------------------------------------------------------------------

for n in [2, 3, 5, 8]:
    participants = "\n".join(f"participant P{i}" for i in range(n))
    messages = "\n".join(f"P{i} -> P{(i+1)%n} : msg {i}" for i in range(n))
    body = f"{participants}\n{messages}"
    write_puml(f"seq_n_participants_{n:02d}.puml", puml(body))

# ---------------------------------------------------------------------------
# 81. ACTIVATION COLORS × DEPTHS
# ---------------------------------------------------------------------------

ACTIVATION_COLORS = ["#red", "#blue", "#green", "#yellow", "#orange", "#purple"]

for depth in [1, 2, 3, 4]:
    for color in ACTIVATION_COLORS[:3]:  # limit combos
        safe_color = color.replace("#", "")
        inner = ""
        for d in range(depth):
            p = chr(ord('A') + d)
            q = chr(ord('A') + d + 1)
            inner += f"{p} -> {q} ++ {color} : level {d+1}\n"
        for d in range(depth - 1, -1, -1):
            p = chr(ord('A') + d)
            q = chr(ord('A') + d + 1)
            inner += f"{q} --> {p} -- : return {d+1}\n"
        participants = " ".join(chr(ord('A') + d) for d in range(depth + 1))
        body = f"participant {chr(ord('A'))}\n" + inner
        write_puml(f"seq_activate_depth{depth}_{safe_color}.puml", puml(body))

# ---------------------------------------------------------------------------
# 82. DELAY AND SPACING COMBINATIONS
# ---------------------------------------------------------------------------

SPACING_VARIANTS = [
    ("triple_pipe", "|||"),
    ("10px", "||10||"),
    ("20px", "||20||"),
    ("50px", "||50||"),
    ("100px", "||100||"),
]

for name, spacing in SPACING_VARIANTS:
    body = (f'Alice -> Bob : before\n{spacing}\nBob --> Alice : after\n'
            f'{spacing}\nAlice -> Bob : again')
    write_puml(f"seq_spacing_{name}.puml", puml(body))

# Delay labels
DELAY_LABELS = [
    ("", "no_label"),
    ("5 seconds", "labeled"),
    ("1 minute later", "minute"),
    ("waiting...", "waiting"),
    ("timeout period", "timeout"),
]

for label, name in DELAY_LABELS:
    dot_part = f"... {label} ..." if label else "..."
    body = f"Alice -> Bob : request\n{dot_part}\nBob --> Alice : response"
    write_puml(f"seq_delay_{name}.puml", puml(body))

# ---------------------------------------------------------------------------
# 83. ARROW COLORS × ALL PARTICIPANTS
# ---------------------------------------------------------------------------

ARROW_COLOR_COMBOS = [
    ("#red", "#blue"),
    ("#green", "#orange"),
    ("#purple", "#teal"),
    ("#FF0000", "#0000FF"),
    ("#darkred", "#darkblue"),
]

for i, (c1, c2) in enumerate(ARROW_COLOR_COMBOS):
    body = (f'Alice -[{c1}]> Bob : forward {c1}\n'
            f'Bob -[{c2}]--> Alice : return {c2}\n'
            f'Alice -[{c1}]>> Charlie : thin {c1}\n'
            f'Charlie -[{c2}]-->> Alice : thin dotted {c2}')
    write_puml(f"seq_arrow_color_combo_{i+1:02d}.puml", puml(body))

# ---------------------------------------------------------------------------
# 84. CREATE/DESTROY VARIANTS
# ---------------------------------------------------------------------------

for ptype in PARTICIPANT_TYPES:
    body = (f'participant Alice\n'
            f'create {ptype} NewObj\n'
            f'Alice -> NewObj : initialize\n'
            f'NewObj --> Alice : ready\n'
            f'Alice -> NewObj : use\n'
            f'NewObj --> Alice : result\n'
            f'destroy NewObj')
    write_puml(f"seq_create_destroy_{ptype}.puml", puml(body))

write_puml("seq_create_chain.puml", puml(
    'Alice -> Alice : bootstrap\ncreate Bob\nAlice -> Bob : init Bob\ncreate Charlie\nBob -> Charlie : init Charlie\ncreate Dave\nCharlie -> Dave : init Dave\nDave --> Charlie : ready\nCharlie --> Bob : ready\nBob --> Alice : all ready\n\nAlice -> Dave : do work\nDave --> Alice : done\ndestroy Dave\ndestroy Charlie\ndestroy Bob'
))

# ---------------------------------------------------------------------------
# 85. REF VARIANTS × PARTICIPANT COMBINATIONS
# ---------------------------------------------------------------------------

for n in [1, 2, 3]:
    participants = " ".join([f"P{i}" for i in range(n)])
    participant_decls = "\n".join(f"participant P{i}" for i in range(n + 1))
    body = (f'{participant_decls}\n'
            f'P0 -> P{n if n < 4 else 0} : action\n'
            f'ref over {participants} : Reference diagram {n}\n'
            f'P{n if n < 4 else 0} --> P0 : done')
    write_puml(f"seq_ref_{n}_participants.puml", puml(body))

# ---------------------------------------------------------------------------
# 86. NEWPAGE VARIANTS
# ---------------------------------------------------------------------------

for n_pages in [2, 3, 4, 5]:
    body = ""
    for page in range(n_pages):
        if page > 0:
            body += f"\nnewpage Page {page + 1}\n"
        body += f"Alice -> Bob : page {page + 1} message\nBob --> Alice : page {page + 1} reply\n"
    write_puml(f"seq_newpage_{n_pages}_pages.puml", puml(body))

# ---------------------------------------------------------------------------
# 87. HEADER/FOOTER/TITLE VARIANTS
# ---------------------------------------------------------------------------

TITLES = [
    "Simple Title",
    "Title with Special Chars: & < >",
    "**Bold Title**",
    "Title\\nWith Newline",
    "<font color=red>Colored Title</font>",
    "Title 日本語",
]

for i, title in enumerate(TITLES):
    body = "Alice -> Bob : hello\nBob --> Alice : hi"
    write_puml(f"seq_title_variant_{i+1:02d}.puml", puml(body, title=title))

HEADERS = [
    "Simple Header",
    "<b>Bold Header</b>",
    "<font color=blue>Blue Header</font>",
    "Header with Date: 2024-01-01",
]

for i, header in enumerate(HEADERS):
    body = "Alice -> Bob : hello\nBob --> Alice : hi"
    write_puml(f"seq_header_variant_{i+1:02d}.puml", puml(body, header=header))

FOOTERS = [
    "Page %page% of %lastpage%",
    "Confidential",
    "<b>Company Name</b>",
    "Generated: %date%",
]

for i, footer in enumerate(FOOTERS):
    body = "Alice -> Bob : hello\nBob --> Alice : hi"
    write_puml(f"seq_footer_variant_{i+1:02d}.puml", puml(body, footer=footer))

# ---------------------------------------------------------------------------
# 88. COMPREHENSIVE SKINPARAM VARIANTS
# ---------------------------------------------------------------------------

SKINPARAM_PAIRS = [
    ("SequenceArrowColor", ["red", "blue", "#336699", "black", "darkgreen"]),
    ("SequenceLifeLineBorderColor", ["red", "blue", "green", "gray", "#FF8800"]),
    ("ParticipantBackgroundColor", ["lightyellow", "lightblue", "#EEFFEE", "white", "#FFE4E1"]),
    ("ParticipantBorderColor", ["navy", "darkred", "darkgreen", "black", "gray"]),
    ("NoteBackgroundColor", ["lightyellow", "#FFFDE7", "white", "#FFF3E0", "azure"]),
    ("SequenceGroupBorderColor", ["navy", "red", "green", "gray", "purple"]),
]

for param, values in SKINPARAM_PAIRS:
    for value in values:
        safe_value = value.replace("#", "hex_").replace(" ", "_")
        body = "Alice -> Bob : hello\nnote over Alice : note\nBob --> Alice : hi"
        write_puml(f"seq_sk_{param.lower()}_{safe_value}.puml",
                   puml(body, skinparams=f"skinparam {param} {value}"))

# ---------------------------------------------------------------------------
# 89. SELF-MESSAGE VARIANTS
# ---------------------------------------------------------------------------

for arrow, aname in ARROW_VARIANTS:
    body = f"Alice {arrow} Alice : self {aname}"
    write_puml(f"seq_self_{aname}.puml", puml(body))

# Self message in groups
for gtype in ["alt", "loop", "opt"]:
    if gtype == "alt":
        body = f"alt condition\n  Alice -> Alice : self in {gtype}\nelse\n  Alice -> Bob : other\nend"
    else:
        body = f"{gtype} condition\n  Alice -> Alice : self in {gtype}\nend"
    write_puml(f"seq_self_in_{gtype}.puml", puml(body))

# Self message with activation
write_puml("seq_self_activate_loop.puml", puml(
    'loop 5 times\n  Alice -> Alice ++ : self activate loop\n  Alice --> Alice -- : done\nend'
))

# ---------------------------------------------------------------------------
# 90. FOUND/LOST MESSAGE VARIANTS
# ---------------------------------------------------------------------------

FOUND_LOST_VARIANTS = [
    ("[-> Alice : found", "found_to_alice"),
    ("[--> Alice : found dotted", "found_dotted"),
    ("[->] : unknown destination", "unknown_dest"),
    ("Alice ->] : lost to outside", "lost_to_outside"),
    ("Alice -->] : lost dotted", "lost_dotted"),
    ("Alice ->x Bob : explicit lost", "explicit_lost"),
]

for body_line, name in FOUND_LOST_VARIANTS:
    body = f"participant Alice\nparticipant Bob\n{body_line}"
    write_puml(f"seq_found_lost_{name}.puml", puml(body))

# ---------------------------------------------------------------------------
# 91. COMPLEX MULTI-PARTICIPANT SCENARIOS
# ---------------------------------------------------------------------------

for n in [4, 6, 8, 10, 15]:
    parts = "\n".join(f"participant P{i:02d}" for i in range(n))
    msgs = ""
    for i in range(n * 2):
        src = i % n
        tgt = (i + 1) % n
        msgs += f"P{src:02d} -> P{tgt:02d} : msg {i+1}\n"
    write_puml(f"seq_multi_{n:02d}_participants.puml", puml(f"{parts}\n{msgs}"))

# ---------------------------------------------------------------------------
# 92. ACTIVATION × DESTROY
# ---------------------------------------------------------------------------

write_puml("seq_activate_then_destroy.puml", puml(
    'Alice -> Bob ++ : create and use\nBob -> Bob : do work\nBob --> Alice -- : result\ndestroy Bob'
))

write_puml("seq_activate_destroy_shorthand.puml", puml(
    'Alice -> Bob !! : create and immediately destroy'
))

write_puml("seq_multi_activate_destroy.puml", puml(
    'Alice -> Bob ++ : use Bob\nAlice -> Charlie ++ : use Charlie\nAlice -> Dave ++ : use Dave\nDave --> Alice -- : dave done\ndestroy Dave\nCharlie --> Alice -- : charlie done\ndestroy Charlie\nBob --> Alice -- : bob done\ndestroy Bob'
))

# ---------------------------------------------------------------------------
# 93. GROUPS WITH ACTIVATION AND NOTES
# ---------------------------------------------------------------------------

for gtype in ["alt", "loop", "opt", "par"]:
    if gtype == "par":
        body = (f'Alice -> Bob ++ : start\n'
                f'{gtype}\n'
                f'  Bob -> Charlie ++ : branch 1\n'
                f'  note over Charlie : branch 1 note\n'
                f'  Charlie --> Bob -- : done 1\n'
                f'and\n'
                f'  Bob -> Dave ++ : branch 2\n'
                f'  note over Dave : branch 2 note\n'
                f'  Dave --> Bob -- : done 2\n'
                f'end\n'
                f'Bob --> Alice -- : all done')
    elif gtype == "alt":
        body = (f'Alice -> Bob ++ : request\n'
                f'{gtype} success\n'
                f'  note over Bob : success path\n'
                f'  Bob -> Charlie ++ : query\n'
                f'  Charlie --> Bob -- : data\n'
                f'  Bob --> Alice -- : result\n'
                f'else failure\n'
                f'  note right of Bob : error path\n'
                f'  Bob --> Alice -- : error\n'
                f'end')
    else:
        body = (f'Alice -> Bob ++ : start\n'
                f'{gtype} condition\n'
                f'  note over Bob : in {gtype}\n'
                f'  Bob -> Charlie ++ : action\n'
                f'  Charlie --> Bob -- : result\n'
                f'end\n'
                f'Bob --> Alice -- : done')
    write_puml(f"seq_group_{gtype}_activate_note.puml", puml(body))

# ---------------------------------------------------------------------------
# 94. EDGE CASE: VERY DEEP GROUP NESTING
# ---------------------------------------------------------------------------

for depth in range(3, 8):
    body = "A -> B : start\n"
    for d in range(depth):
        gtype = ["alt", "opt", "loop", "par", "critical"][d % 5]
        if gtype == "par":
            body += f"par\n  A -> B : par branch {d}\nand\n"
        elif gtype == "alt":
            body += f"alt condition_{d}\n"
        else:
            body += f"{gtype} condition_{d}\n"
    body += "  B -> C : deep action\n  C --> B : deep result\n"
    # close groups
    open_par = False
    for d in range(depth - 1, -1, -1):
        gtype = ["alt", "opt", "loop", "par", "critical"][d % 5]
        if gtype == "par":
            body += "end\n"
        elif gtype == "alt":
            body += "else alt_else\n  B --> A : else\nend\n"
        else:
            body += "end\n"
    body += "B --> A : done"
    write_puml(f"seq_deep_nest_depth{depth}.puml", puml(body))

# ---------------------------------------------------------------------------
# 95. UNICODE AND SPECIAL CONTENT VARIANTS
# ---------------------------------------------------------------------------

UNICODE_MESSAGES = [
    ("japanese", "日本語のメッセージ"),
    ("chinese", "中文消息"),
    ("korean", "한국어 메시지"),
    ("arabic", "رسالة عربية"),
    ("russian", "Русское сообщение"),
    ("greek", "Ελληνικό μήνυμα"),
    ("hebrew", "הודעה בעברית"),
    ("emoji", "Hello 🌍 World 🚀"),
    ("mixed", "Mix: 日本語 & English & 中文"),
    ("math", "E = mc² and π ≈ 3.14159"),
    ("arrows", "→ ← ↑ ↓ ↔ ⇒"),
    ("symbols", "© ® ™ § ¶ † ‡"),
]

for name, msg in UNICODE_MESSAGES:
    body = f'Alice -> Bob : {msg}\nBob --> Alice : {msg} reply'
    write_puml(f"seq_unicode_{name}.puml", puml(body))

# Unicode participant names
UNICODE_NAMES = [
    ("Alice_JP", "アリス"),
    ("Bob_CN", "鲍勃"),
    ("Charlie_KR", "찰리"),
    ("Dave_RU", "Дейв"),
    ("Eve_GR", "Εύα"),
]

for safe_name, display_name in UNICODE_NAMES:
    body = f'participant "{display_name}" as {safe_name}\nparticipant Bob\n{safe_name} -> Bob : hello\nBob --> {safe_name} : hi'
    write_puml(f"seq_unicode_name_{safe_name}.puml", puml(body))

# ---------------------------------------------------------------------------
# 96. CREOLE IN DIFFERENT CONTEXTS
# ---------------------------------------------------------------------------

CREOLE_VARIANTS = [
    ("bold_note", "note over Alice", "**bold content**"),
    ("italic_note", "note over Alice", "//italic content//"),
    ("mono_note", "note over Alice", '""monospace content""'),
    ("strikethrough_note", "note over Alice", "--strikethrough--"),
    ("underline_note", "note over Alice", "__underlined__"),
    ("wave_note", "note over Alice", "~~waved~~"),
    ("color_note", "note over Alice", "<color:red>red text</color>"),
    ("size_note", "note over Alice", "<size:18>large text</size>"),
    ("bg_note", "note over Alice", "<back:yellow>highlighted</back>"),
]

for name, context, content in CREOLE_VARIANTS:
    body = f'Alice -> Bob : hello\n{context}\n  {content}\nend note\nBob --> Alice : hi'
    write_puml(f"seq_creole_{name}.puml", puml(body))

# Creole in messages
CREOLE_MSGS = [
    ("bold_msg", "**bold message**"),
    ("italic_msg", "//italic message//"),
    ("mono_msg", '""code message""'),
    ("color_msg", "<color:red>colored</color> message"),
    ("html_msg", "<b>html bold</b> and <i>italic</i>"),
    ("size_msg", "<size:14>sized text</size>"),
    ("combined_msg", "**bold** //italic// <color:blue>colored</color>"),
]

for name, msg in CREOLE_MSGS:
    body = f"Alice -> Bob : {msg}\nBob --> Alice : {msg}"
    write_puml(f"seq_creole_{name}.puml", puml(body))

# ---------------------------------------------------------------------------
# 97. HIDE FOOTBOX VARIANTS
# ---------------------------------------------------------------------------

for ptype in PARTICIPANT_TYPES:
    body = f'hide footbox\n{ptype} Alice\nparticipant Bob\nAlice -> Bob : hello\nBob --> Alice : hi'
    write_puml(f"seq_hide_footbox_{ptype}.puml", puml(body))

# Hide footbox with groups
for gtype in ["alt", "loop", "opt"]:
    if gtype == "alt":
        body = f'hide footbox\nAlice -> Bob : request\n{gtype} ok\n  Bob --> Alice : result\nelse\n  Bob --> Alice : error\nend'
    else:
        body = f'hide footbox\nAlice -> Bob : start\n{gtype} cond\n  Bob -> Charlie : action\n  Charlie --> Bob : done\nend\nBob --> Alice : complete'
    write_puml(f"seq_hide_footbox_group_{gtype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 98. RETURN KEYWORD WITH COMPLEX SCENARIOS
# ---------------------------------------------------------------------------

for depth in [1, 2, 3, 4, 5]:
    calls = ""
    for d in range(depth):
        p = chr(ord('A') + d)
        q = chr(ord('A') + d + 1)
        calls += f"{p} -> {q} ++ : call level {d+1}\n"
    for d in range(depth):
        calls += f"return result level {depth - d}\n"
    write_puml(f"seq_return_depth{depth}.puml", puml(calls))

# ---------------------------------------------------------------------------
# 99. PARTICIPANT DECLARATIONS × ORDER × COLOR × STEREOTYPE
# ---------------------------------------------------------------------------

for i, ptype in enumerate(PARTICIPANT_TYPES):
    body = (f'{ptype} "P1" as P1 <<stereotype1>> #lightblue order {10 + i*10}\n'
            f'{ptype} "P2" as P2 <<stereotype2>> #lightyellow order {20 + i*10}\n'
            f'P1 -> P2 : msg\nP2 --> P1 : reply')
    write_puml(f"seq_ptype_{ptype}_full_decl.puml", puml(body))

# Multiple stereotypes and colors combined
for color1, color2 in [("#red", "#blue"), ("#green", "#yellow"), ("#orange", "#purple")]:
    s1 = color1.replace("#", "")
    s2 = color2.replace("#", "")
    body = (f'participant A <<svc>> {color1} order 10\n'
            f'participant B <<ext>> {color2} order 20\n'
            f'participant C <<db>> #white order 30\n'
            f'A -> B : call\nB -> C : query\nC --> B : data\nB --> A : result')
    write_puml(f"seq_ptype_combo_colors_{s1}_{s2}.puml", puml(body))

# ---------------------------------------------------------------------------
# 100. MESSAGE SEQUENCES WITH ALL GROUP TYPES
# ---------------------------------------------------------------------------

SCENARIO_MSGS = [
    ("request_response", "Alice -> Bob : request\nBob --> Alice : response"),
    ("chain", "Alice -> Bob : 1\nBob -> Charlie : 2\nCharlie --> Bob : 3\nBob --> Alice : 4"),
    ("broadcast", "Alice -> Bob : broadcast\nAlice -> Charlie : broadcast\nAlice -> Dave : broadcast"),
    ("gather", "Bob --> Alice : report\nCharlie --> Alice : report\nDave --> Alice : report"),
    ("pipeline", "A -> B : stage1\nB -> C : stage2\nC -> D : stage3\nD -> E : stage4"),
]

for scenario_name, scenario_body in SCENARIO_MSGS:
    for gtype in ["alt", "loop", "opt"]:
        if gtype == "alt":
            body = f'{scenario_body}\nalt ok\n  Alice -> Bob : confirm\nelse error\n  Alice -> Bob : retry\nend'
        elif gtype == "loop":
            body = f'loop 3 times\n{scenario_body}\nend'
        else:
            body = f'opt cond\n{scenario_body}\nend'
        write_puml(f"seq_scenario_{scenario_name}_{gtype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 101. BOXES WITH MESSAGES AND GROUPS
# ---------------------------------------------------------------------------

for gtype in ["alt", "loop", "opt"]:
    if gtype == "alt":
        group_body = f'alt valid\n  API -> DB : query\n  DB --> API : data\n  API --> UI : 200\nelse invalid\n  API --> UI : 400\nend'
    elif gtype == "loop":
        group_body = f'loop for each item\n  API -> DB : fetch\n  DB --> API : item\n  API -> UI : push\nend'
    else:
        group_body = f'opt logging\n  API -> Logger : log\nend\nAPI -> DB : query\nDB --> API : data'

    body = (f'box "Frontend" #lightblue\n  actor User\n  participant UI\nend box\n'
            f'box "Backend" #lightyellow\n  participant API\n  database DB\nend box\n'
            f'User -> UI : action\nUI -> API : request\n'
            f'{group_body}\n'
            f'API --> UI : result\nUI --> User : display')
    write_puml(f"seq_box_group_{gtype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 102. LARGE DIAGRAMS
# ---------------------------------------------------------------------------

# 100 messages
body_100 = "participant Alice\nparticipant Bob\n"
for i in range(50):
    body_100 += f"Alice -> Bob : message {i*2+1}\nBob --> Alice : reply {i*2+2}\n"
write_puml("seq_large_100_messages.puml", puml(body_100))

# 30 participants
parts_30 = "\n".join(f"participant P{i:02d}" for i in range(30))
msgs_30 = "\n".join(f"P{i:02d} -> P{(i+1)%30:02d} : msg" for i in range(30))
write_puml("seq_large_30_participants.puml", puml(f"{parts_30}\n{msgs_30}"))

# Many groups
body_groups = "participant Alice\nparticipant Bob\nparticipant Charlie\n"
for i in range(10):
    body_groups += f"alt condition {i}\n  Alice -> Bob : action {i}\n  Bob --> Alice : ok {i}\nelse\n  Bob --> Alice : else {i}\nend\n"
write_puml("seq_large_many_groups.puml", puml(body_groups))

# Many notes
body_notes = "participant Alice\nparticipant Bob\n"
for i in range(20):
    body_notes += f"Alice -> Bob : step {i}\n"
    if i % 3 == 0:
        body_notes += f"note over Alice : note {i}\n"
    elif i % 3 == 1:
        body_notes += f"note right of Bob : note {i}\n"
    else:
        body_notes += f"note over Alice, Bob : over both {i}\n"
    body_notes += f"Bob --> Alice : reply {i}\n"
write_puml("seq_large_many_notes.puml", puml(body_notes))

# ---------------------------------------------------------------------------
# 103. COMBINATION: autonumber + boxes + activation + groups + notes
# ---------------------------------------------------------------------------

write_puml("seq_mega_combo_01.puml", puml(
    '''autonumber
hide footbox

box "UI Layer" #E3F2FD
  actor User
  boundary "API Gateway" as GW <<boundary>>
end box

box "Service Layer" #E8F5E9
  control "Auth Service" as Auth <<control>>
  control "Business Logic" as BL <<control>>
  entity "Order Service" as OS <<entity>>
end box

box "Data Layer" #FFF3E0
  database "User DB" as UDB
  database "Order DB" as ODB
  collections "Cache" as Cache <<cache>>
  queue "Event Bus" as EB <<queue>>
end box

User -> GW ++ : POST /orders
GW -> Auth ++ : validate token
Auth -> UDB ++ : lookup user
UDB --> Auth -- : user data
Auth --> GW -- : 200 valid

alt user has permission
  GW -> BL ++ : process order
  note over BL : validating business rules

  loop for each item in order
    BL -> Cache ++ : check inventory
    alt cache hit
      Cache --> BL -- : stock available
    else cache miss
      BL -> ODB ++ : query inventory
      ODB --> BL -- : stock level
      BL -> Cache ++ : populate cache
      Cache --> BL -- : cached
    end
  end

  BL -> OS ++ : create order
  OS -> ODB ++ : insert order
  ODB --> OS -- : order_id
  OS -> EB : publish OrderCreated
  OS --> BL -- : order created

  BL --> GW -- : order summary
  GW --> User -- : 201 Created

else insufficient permission
  GW --> User -- : 403 Forbidden
end''',
    title="Order Creation Flow",
    skinparams="""skinparam SequenceArrowColor #1A237E
skinparam ParticipantBorderColor #37474F
skinparam NoteBackgroundColor #FFFDE7"""
))

write_puml("seq_mega_combo_02.puml", puml(
    '''!pragma teoz true
autonumber "<b>(##)</b>"

participant "Client A" as CA
participant "Client B" as CB
participant "Broker" as Br
participant "Worker 1" as W1
participant "Worker 2" as W2
participant "Database" as DB

title Distributed Job Processing

CA -> Br : submit job A
CB -> Br : submit job B

par
  Br -> W1 ++ : assign job A
  W1 -> DB ++ : fetch A data
  DB --> W1 -- : data A
  W1 -> W1 : process A
  W1 --> Br -- : job A done
  Br -> CA : notify A complete
and
  Br -> W2 ++ : assign job B
  W2 -> DB ++ : fetch B data
  DB --> W2 -- : data B
  opt B requires validation
    W2 -> W1 : validate B with A results
    W1 --> W2 : validation ok
  end
  W2 -> W2 : process B
  W2 --> Br -- : job B done
  Br -> CB : notify B complete
end

note over CA, CB : Both jobs complete'''
))

write_puml("seq_mega_combo_03.puml", puml(
    '''participant P1 as "Service 1 <<internal>>"
participant P2 as "Service 2 <<internal>>"
participant P3 as "Service 3 <<external>>"
participant P4 as "Service 4 <<external>>"
database DB1 as "DB Primary"
database DB2 as "DB Replica"
queue Q as "Message Queue"
collections Cache as "Distributed Cache"

title Microservices Interaction

== Initialization Phase ==

P1 -> P2 : register
P2 -> DB1 : store registration
DB1 --> P2 : ok
P2 --> P1 : registered

== Request Processing ==

[-> P1 : external request
P1 -> Cache ++ : check cache
alt hit
  Cache --> P1 -- : cached result
  P1 ->] : response
else miss
  Cache --> P1 -- : not found
  P1 -> P2 ++ : forward request

  loop retry up to 3 times
    P2 -> P3 ++ : call external
    alt success
      P3 --> P2 -- : result
      break
    else timeout
      P3 --> P2 -- : timeout
    end
  end

  P2 -> DB1 ++ : store result
  DB1 --> P2 -- : stored
  DB1 -> DB2 : replicate
  P2 -> Q : publish event
  P2 -> Cache ++ : update cache
  Cache --> P2 -- : updated
  P2 --> P1 -- : result
  P1 ->] : response
end

== Cleanup ==

... periodic cleanup ...
P4 -> DB2 : cleanup old records
P4 -> Cache : invalidate stale'''
))

# ---------------------------------------------------------------------------
# 104. SYSTEMATIC: all note types × all participant types
# ---------------------------------------------------------------------------

NOTE_TYPES = ["note", "hnote", "rnote"]

for note_type in NOTE_TYPES:
    for ptype in PARTICIPANT_TYPES:
        # Simple over
        body = (f'{ptype} A\nparticipant B\n'
                f'A -> B : hello\n'
                f'{note_type} over A : {note_type} over {ptype}\n'
                f'B --> A : hi')
        write_puml(f"seq_{note_type}_{ptype}_over.puml", puml(body))

        # Over multiple (only for regular note/hnote/rnote)
        body = (f'{ptype} A\n{ptype} B\n'
                f'A -> B : hello\n'
                f'{note_type} over A, B : {note_type} over two {ptype}s\n'
                f'B --> A : hi')
        write_puml(f"seq_{note_type}_{ptype}_over_two.puml", puml(body))

# ---------------------------------------------------------------------------
# 105. SYSTEMATIC: autonumber resume/stop patterns
# ---------------------------------------------------------------------------

AUTONUMBER_PATTERNS = [
    ("basic_stop_resume",
     'autonumber\nA -> B : 1\nB --> A : 2\nautonumber stop\nA -> B : unnumbered\nautonumber resume\nA -> B : 3\nB --> A : 4'),
    ("stop_resume_format",
     'autonumber "<b>##</b>"\nA -> B : 1\nautonumber stop\nA -> B : no num\nautonumber resume "<i>##</i>"\nA -> B : 2'),
    ("multiple_stops",
     'autonumber\nA -> B : 1\nautonumber stop\nA -> B : x\nautonumber resume\nA -> B : 2\nautonumber stop\nA -> B : y\nautonumber resume\nA -> B : 3'),
    ("start_mid",
     'A -> B : no num\nautонumber 5\nA -> B : 5\nB --> A : 6\nA -> B : 7'),
]

for name, body in AUTONUMBER_PATTERNS:
    # Fix cyrillic typo in body
    body = body.replace("автонumber", "autonumber")
    body = body.replace("autонumber", "autonumber")
    write_puml(f"seq_autonumber_pattern_{name}.puml",
               puml(body.replace("A ->", "Alice ->").replace("B -->", "Bob -->").replace("A ->", "Alice ->").replace("B ->", "Bob ->").replace("\nA ", "\nAlice ").replace("\nB ", "\nBob ")))

# ---------------------------------------------------------------------------
# 106. BOXES: no-color, single-participant
# ---------------------------------------------------------------------------

write_puml("seq_box_single_participant.puml", puml(
    'box "Solo"\n  participant Alice\nend box\nparticipant Bob\nAlice -> Bob : call\nBob --> Alice : reply'
))

write_puml("seq_box_no_name.puml", puml(
    'box\n  participant Alice\n  participant Bob\nend box\nparticipant Charlie\nAlice -> Charlie : call\nCharlie --> Alice : reply'
))

write_puml("seq_box_no_name_no_color.puml", puml(
    'box\n  participant A\n  participant B\nend box\nA -> B : hello\nB --> A : hi'
))

# Many boxes
many_boxes = ""
for i in range(5):
    color = ["#lightblue", "#lightyellow", "#lightgreen", "#FFE4E1", "#F0E68C"][i]
    many_boxes += f'box "Layer {i+1}" {color}\n  participant L{i}A\n  participant L{i}B\nend box\n'
many_boxes += "\nL0A -> L0B : internal\nL0B -> L1A : to layer 1\nL1A -> L2A : to layer 2\nL2A -> L3A : to layer 3\nL3A -> L4A : to layer 4\nL4A --> L3A : back\nL3A --> L2A : back\nL2A --> L1A : back\nL1A --> L0B : back\nL0B --> L0A : done"
write_puml("seq_box_many_layers.puml", puml(many_boxes))

# ---------------------------------------------------------------------------
# 107. TEOZ × ALL FEATURES
# ---------------------------------------------------------------------------

TEOZ_VARIANTS = [
    ("with_notes", 'Alice -> Bob : a\nnote over Alice : note\nBob --> Alice : b'),
    ("with_groups", 'Alice -> Bob : a\nalt ok\n  Bob -> Charlie : b\n  Charlie --> Bob : c\n  Bob --> Alice : d\nelse\n  Bob --> Alice : fail\nend'),
    ("with_activation", 'Alice -> Bob ++ : a\nBob -> Charlie ++ : b\nCharlie --> Bob -- : c\nBob --> Alice -- : d'),
    ("with_boxes", 'box "A"\n  participant Alice\nend box\nparticipant Bob\nAlice -> Bob : hi\nBob --> Alice : hey'),
    ("with_autonumber", 'autonumber\nAlice -> Bob : 1\nBob --> Alice : 2\nAlice -> Charlie : 3\nCharlie --> Alice : 4'),
    ("with_ref", 'Alice -> Bob : a\nref over Alice, Bob : see diagram\nBob --> Alice : b'),
    ("with_divider", 'Alice -> Bob : a\n== divider ==\nBob --> Alice : b'),
    ("with_delay", 'Alice -> Bob : a\n... wait ...\nBob --> Alice : b'),
]

for name, body in TEOZ_VARIANTS:
    write_puml(f"seq_teoz_{name}.puml", puml(body, pragma="!pragma teoz true"))

# ---------------------------------------------------------------------------
# 108. RESPONSE MESSAGE BELOW ARROW
# ---------------------------------------------------------------------------

write_puml("seq_response_below_arrow.puml", puml(
    'Alice -> Bob : request\nBob --> Alice : response',
    skinparams='skinparam SequenceResponseMessageBelowArrow true'
))

write_puml("seq_response_above_arrow.puml", puml(
    'Alice -> Bob : request\nBob --> Alice : response',
    skinparams='skinparam SequenceResponseMessageBelowArrow false'
))

# ---------------------------------------------------------------------------
# 109. DIVIDERS WITH ALL CONTENT
# ---------------------------------------------------------------------------

DIVIDER_CONTENTS = [
    "Simple Divider",
    "**Bold Divider**",
    "<font color=red>Red Divider</font>",
    "Phase 1: Initialization",
    "=== Double Equals ===",
    "日本語の区切り",
    "Step 1 of 3",
]

for i, content in enumerate(DIVIDER_CONTENTS):
    body = f"Alice -> Bob : before\n== {content} ==\nBob --> Alice : after"
    write_puml(f"seq_divider_content_{i+1:02d}.puml", puml(body))

# ---------------------------------------------------------------------------
# 110. PARTICIPANT ORDER VARIATIONS
# ---------------------------------------------------------------------------

ORDER_VARIANTS = [
    ([("A", 10), ("B", 20), ("C", 30)], "ascending"),
    ([("A", 30), ("B", 20), ("C", 10)], "descending"),
    ([("A", 10), ("B", 30), ("C", 20)], "mixed"),
    ([("A", 100), ("B", 1), ("C", 50)], "large_gaps"),
    ([("A", 1), ("B", 1), ("C", 1)], "same_order"),
]

for participants, name in ORDER_VARIANTS:
    decls = "\n".join(f"participant {p} order {o}" for p, o in participants)
    msgs = "\n".join(f"A -> B : msg1\nB -> C : msg2\nC --> A : msg3")
    write_puml(f"seq_participant_order_{name}.puml", puml(f"{decls}\n{msgs}"))

# ---------------------------------------------------------------------------
# 111. COMBINATION: participants with stereotypes × groups × notes
# ---------------------------------------------------------------------------

for stereo in ["<<service>>", "<<external>>", "<<legacy>>", "<<new>>"]:
    safe = stereo.replace("<<", "").replace(">>", "")
    body = (f'participant Alice {stereo}\n'
            f'participant Bob {stereo}\n'
            f'Alice -> Bob : call\n'
            f'note over Alice : {safe} note\n'
            f'alt success\n'
            f'  Bob --> Alice : ok\n'
            f'else failure\n'
            f'  Bob --> Alice : error\n'
            f'end')
    write_puml(f"seq_stereo_{safe}_group_note.puml", puml(body))

# ---------------------------------------------------------------------------
# 112. IMPLICIT PARTICIPANT CREATION VIA MESSAGES
# ---------------------------------------------------------------------------

write_puml("seq_implicit_multi.puml", puml(
    'A -> B : 1\nB -> C : 2\nC -> D : 3\nD -> E : 4\nE -> F : 5\nF --> E : 6\nE --> D : 7\nD --> C : 8\nC --> B : 9\nB --> A : 10'
))

write_puml("seq_implicit_star_topology.puml", puml(
    'Hub -> A : to A\nHub -> B : to B\nHub -> C : to C\nHub -> D : to D\nHub -> E : to E\nA --> Hub : from A\nB --> Hub : from B\nC --> Hub : from C\nD --> Hub : from D\nE --> Hub : from E'
))

# ---------------------------------------------------------------------------
# 113. MIXED ARROW DIRECTION SEQUENCES
# ---------------------------------------------------------------------------

write_puml("seq_arrow_dir_mixed_01.puml", puml(
    'Alice -> Bob : ltr solid\nBob -> Alice : rtl (uses ->)\nAlice --> Bob : ltr dotted\nBob --> Alice : rtl dotted (uses -->)'
))

write_puml("seq_arrow_dir_bidirectional.puml", puml(
    'Alice -> Bob : request\nBob -> Alice : counter-request\nAlice --> Bob : ack counter\nBob --> Alice : ack original'
))

# ---------------------------------------------------------------------------
# 114. SKINPARAM COMBINATIONS
# ---------------------------------------------------------------------------

SKINPARAM_COMBOS = [
    ("theme_dark",
     """skinparam backgroundColor #2B2B2B
skinparam ParticipantBackgroundColor #3C3F41
skinparam ParticipantFontColor white
skinparam SequenceArrowColor #6897BB
skinparam SequenceLifeLineBorderColor #6897BB"""),
    ("theme_minimal",
     """skinparam monochrome true
skinparam shadowing false
skinparam ParticipantBorderThickness 1"""),
    ("theme_colorful",
     """skinparam ParticipantBackgroundColor #FFD700
skinparam ParticipantBorderColor #FF4500
skinparam SequenceArrowColor #8B0000
skinparam NoteBackgroundColor #98FB98
skinparam NoteBorderColor #006400"""),
    ("theme_corporate",
     """skinparam DefaultFontName Arial
skinparam ParticipantBackgroundColor #003366
skinparam ParticipantFontColor white
skinparam SequenceArrowColor #003366
skinparam SequenceLifeLineBorderColor #003366
skinparam NoteBackgroundColor #E6EFF7"""),
]

for name, sp in SKINPARAM_COMBOS:
    body = ('participant Alice\nparticipant Bob\nparticipant Charlie\n'
            'Alice -> Bob : request\nnote over Alice : note\n'
            'Bob -> Charlie : delegate\nCharlie --> Bob : result\nBob --> Alice : response')
    write_puml(f"seq_skinparam_theme_{name}.puml", puml(body, skinparams=sp))

# ---------------------------------------------------------------------------
# 115. SPECIAL SEQUENCE: errors and boundary conditions
# ---------------------------------------------------------------------------

write_puml("seq_error_handling_full.puml", puml(
    '''participant Client
participant Server
participant DB

Client -> Server ++ : risky operation

break on critical error
  Server -> DB ++ : transaction
  DB --> Server -- : DB error
  note right of Server : rolling back
  Server --> Client -- : 500 Internal Error
end

opt no break occurred
  Server -> DB ++ : commit
  DB --> Server -- : ok
  Server --> Client -- : 200 OK
end'''
))

write_puml("seq_timeout_pattern.puml", puml(
    '''participant Client
participant Server
participant External

Client -> Server ++ : request
Server -> External ++ : call external

alt timeout
  ... 30 seconds ...
  Server -> External : cancel
  External --> Server -- : cancelled
  Server --> Client -- : 504 Timeout
else success
  External --> Server -- : result
  Server --> Client -- : 200 OK
end'''
))

write_puml("seq_retry_pattern.puml", puml(
    '''participant Client
participant Server
participant External

Client -> Server ++ : request

loop retry up to 3 times
  Server -> External ++ : attempt
  alt success
    External --> Server -- : result
    break
  else failure
    External --> Server -- : error
    ... wait 1 second ...
  end
end

Server --> Client -- : final result'''
))

# ---------------------------------------------------------------------------
# 116. SYSTEMATIC: every arrow type × every direction × self/peer
# ---------------------------------------------------------------------------

ALL_ARROW_TYPES = [
    "->", "-->", "->>", "-->>",
    "->x", "-\\\\", "--\\\\",
    "-/", "--/", "-//", "--//",
]

for arrow in ALL_ARROW_TYPES:
    safe = arrow.replace(">", "gt").replace("-", "m").replace("\\", "bs").replace("/", "sl").replace("x", "x")
    # peer
    body = f"Alice -> Bob : before\nAlice {arrow} Bob : arrow {arrow}\nBob --> Alice : after"
    write_puml(f"seq_arrowtype_peer_{safe}.puml", puml(body))
    # self
    body = f"Alice -> Alice : before\nAlice {arrow} Alice : self {arrow}"
    write_puml(f"seq_arrowtype_self_{safe}.puml", puml(body))

# ---------------------------------------------------------------------------
# 117. ACTIVATION: explicit activate/deactivate with all colors
# ---------------------------------------------------------------------------

EXPLICIT_COLORS = [
    "", "#red", "#blue", "#green", "#yellow", "#orange",
    "#purple", "#pink", "#cyan", "#white", "#lightblue", "#lightgreen",
    "#FFAAAA", "#AAFFAA", "#AAAAFF", "#FFDDAA",
]

for color in EXPLICIT_COLORS:
    safe = color.replace("#", "c_") if color else "nocolor"
    body = (f"Alice -> Bob : request\n"
            f"activate Bob {color}\n"
            f"Bob --> Alice : response\n"
            f"deactivate Bob")
    write_puml(f"seq_explicit_activate_{safe}.puml", puml(body))

# ---------------------------------------------------------------------------
# 118. GROUPS × NOTES × POSITIONS
# ---------------------------------------------------------------------------

NOTE_POSITIONS_FULL = ["left of Alice", "right of Bob", "over Alice", "over Bob", "over Alice, Bob"]

for gtype in ["alt", "opt", "loop"]:
    for note_pos in NOTE_POSITIONS_FULL:
        safe_pos = note_pos.replace(" ", "_").replace(",", "_").replace("Alice", "A").replace("Bob", "B")
        if gtype == "alt":
            body = (f"participant Alice\nparticipant Bob\n"
                    f"Alice -> Bob : msg\n"
                    f"alt condition\n"
                    f"  note {note_pos} : in {gtype}\n"
                    f"  Bob --> Alice : ok\n"
                    f"else\n"
                    f"  Bob --> Alice : else\n"
                    f"end")
        else:
            body = (f"participant Alice\nparticipant Bob\n"
                    f"Alice -> Bob : msg\n"
                    f"{gtype} condition\n"
                    f"  note {note_pos} : in {gtype}\n"
                    f"  Bob --> Alice : done\n"
                    f"end")
        write_puml(f"seq_group_{gtype}_note_{safe_pos}.puml", puml(body))

# ---------------------------------------------------------------------------
# 119. ALL PARTICIPANT TYPES × SKINPARAM THEMES
# ---------------------------------------------------------------------------

SIMPLE_THEMES = [
    ("plain", ""),
    ("mono", "skinparam monochrome true"),
    ("handwritten", "skinparam handwritten true"),
]

for ptype in PARTICIPANT_TYPES:
    for theme_name, theme_sp in SIMPLE_THEMES:
        body = f'{ptype} Alice\nparticipant Bob\nAlice -> Bob : hello\nBob --> Alice : hi'
        write_puml(f"seq_ptype_{ptype}_theme_{theme_name}.puml", puml(body, skinparams=theme_sp))

# ---------------------------------------------------------------------------
# 120. GROUPS × AUTONUMBER × ACTIVATION COMBOS
# ---------------------------------------------------------------------------

for gtype in ["alt", "opt", "loop"]:
    for start in [1, 10, 100]:
        if gtype == "alt":
            body = (f"autonumber {start}\n"
                    f"Alice -> Bob ++ : call\n"
                    f"alt ok\n"
                    f"  Bob -> Charlie ++ : delegate\n"
                    f"  Charlie --> Bob -- : result\n"
                    f"  Bob --> Alice -- : success\n"
                    f"else\n"
                    f"  Bob --> Alice -- : error\n"
                    f"end")
        else:
            body = (f"autonumber {start}\n"
                    f"Alice -> Bob ++ : start\n"
                    f"{gtype} condition\n"
                    f"  Bob -> Charlie ++ : action\n"
                    f"  Charlie --> Bob -- : done\n"
                    f"end\n"
                    f"Bob --> Alice -- : complete")
        write_puml(f"seq_combo_autonumber{start}_{gtype}_activation.puml", puml(body))

# ---------------------------------------------------------------------------
# 121. NOTES: multiline with creole
# ---------------------------------------------------------------------------

MULTILINE_NOTE_CONTENTS = [
    ("table",
     "| Name | Value |\n| foo  | 42    |\n| bar  | 99    |"),
    ("bullet_list",
     "* item one\n* item two\n* item three"),
    ("numbered_list",
     "# first\n# second\n# third"),
    ("bold_italic",
     "**Header**\n//description//\n""code"""),
    ("mixed_formatting",
     "**Title**\n----\n* item 1\n* item 2\n----\n//footer//"),
    ("color_combo",
     "<color:red>error</color> or\n<color:green>success</color>"),
    ("size_combo",
     "<size:20>big</size> and\n<size:10>small</size>"),
]

for name, content in MULTILINE_NOTE_CONTENTS:
    body = (f"Alice -> Bob : request\n"
            f"note over Alice, Bob\n"
            f"  {content.replace(chr(10), chr(10) + '  ')}\n"
            f"end note\n"
            f"Bob --> Alice : response")
    write_puml(f"seq_note_multiline_creole_{name}.puml", puml(body))

# ---------------------------------------------------------------------------
# 122. PARTICIPANT: all types with all colors × aliases
# ---------------------------------------------------------------------------

SHORT_COLORS = ["#red", "#blue", "#green", "#yellow", "#orange"]

for ptype in PARTICIPANT_TYPES:
    for color in SHORT_COLORS:
        safe_color = color.replace("#", "")
        body = (f'{ptype} "Named {ptype.capitalize()}" as P {color}\n'
                f'participant Other\n'
                f'P -> Other : {ptype} {safe_color} call\n'
                f'Other --> P : reply')
        write_puml(f"seq_ptype_{ptype}_color_{safe_color}_alias.puml", puml(body))

# ---------------------------------------------------------------------------
# 123. DIVIDERS × GROUPS × NOTES
# ---------------------------------------------------------------------------

for i in range(5):
    body = (f"Alice -> Bob : step 1\n"
            f"== Phase {i+1} ==\n"
            f"alt condition\n"
            f"  note over Alice : alt note\n"
            f"  Bob -> Charlie : delegate\n"
            f"  Charlie --> Bob : result\n"
            f"  Bob --> Alice : ok\n"
            f"else\n"
            f"  note right of Bob : else note\n"
            f"  Bob --> Alice : error\n"
            f"end\n"
            f"== End Phase {i+1} ==\n"
            f"Bob --> Alice : done {i+1}")
    write_puml(f"seq_divider_group_note_{i+1:02d}.puml", puml(body))

# ---------------------------------------------------------------------------
# 124. TEOZ × GROUPS × PARTICIPANTS
# ---------------------------------------------------------------------------

for n_parts in [2, 3, 4, 5]:
    parts = "\n".join(f"participant P{i}" for i in range(n_parts))
    msgs = "P0 -> P1 : start\n"
    if n_parts > 2:
        msgs += "alt cond\n"
        for i in range(1, n_parts - 1):
            msgs += f"  P{i} -> P{i+1} : to {i+1}\n"
        msgs += "else\n  P0 -> P0 : self\nend\n"
    msgs += "P1 --> P0 : done"
    write_puml(f"seq_teoz_{n_parts}parts_groups.puml",
               puml(f"{parts}\n{msgs}", pragma="!pragma teoz true"))

# ---------------------------------------------------------------------------
# 125. RETURN + NOTES
# ---------------------------------------------------------------------------

for note_pos in ["left of Alice", "right of Bob", "over Alice", "over Bob"]:
    safe = note_pos.replace(" ", "_").replace(",", "_").replace("Alice", "A").replace("Bob", "B")
    body = (f"participant Alice\nparticipant Bob\n"
            f"Alice -> Bob ++ : call\n"
            f"note {note_pos} : before return\n"
            f"return result")
    write_puml(f"seq_return_note_{safe}.puml", puml(body))

# ---------------------------------------------------------------------------
# 126. BOX × AUTONUMBER × SKINPARAM
# ---------------------------------------------------------------------------

for box_color in ["#lightblue", "#lightyellow", "#lightgreen", "#FFE4E1"]:
    safe = box_color.replace("#", "box_")
    body = (f'autonumber\n'
            f'box "Service A" {box_color}\n'
            f'  participant Alice\n'
            f'  participant Bob\n'
            f'end box\n'
            f'box "Service B"\n'
            f'  participant Charlie\n'
            f'end box\n'
            f'Alice -> Bob : 1\n'
            f'Bob -> Charlie : 2\n'
            f'Charlie --> Bob : 3\n'
            f'Bob --> Alice : 4')
    write_puml(f"seq_box_autonumber_{safe}.puml", puml(body))

# ---------------------------------------------------------------------------
# 127. LOST/FOUND × ACTIVATION × GROUPS
# ---------------------------------------------------------------------------

write_puml("seq_lost_in_loop.puml", puml(
    'Alice -> Bob ++ : start\nloop retry\n  Bob ->x External : attempt (lost)\n  ... wait ...\nend\nBob --> Alice -- : give up'
))

write_puml("seq_found_in_alt.puml", puml(
    'alt external trigger\n  [-> Alice : found trigger\n  Alice -> Bob : handle\n  Bob --> Alice : done\nelse internal\n  Alice -> Alice : self trigger\nend'
))

write_puml("seq_lost_found_activation.puml", puml(
    '[-> Alice ++ : found and activate\nAlice -> Bob ++ : activate Bob\nBob ->x Charlie : lost\nBob --> Alice -- : done\nAlice ->] : sent out\nAlice --> Alice -- : self done'
))

# ---------------------------------------------------------------------------
# 128. NEWPAGE × GROUPS × ACTIVATION
# ---------------------------------------------------------------------------

write_puml("seq_newpage_with_activation.puml", puml(
    'Alice -> Bob ++ : start\nnewpage\nBob -> Charlie : continue\nCharlie --> Bob : done\nnewpage\nBob --> Alice -- : finish'
))

write_puml("seq_newpage_with_groups.puml", puml(
    'Alice -> Bob : begin\nalt path A\n  Bob -> Charlie : A\n  Charlie --> Bob : done A\nelse path B\n  Bob -> Dave : B\n  Dave --> Bob : done B\nend\nnewpage Second Page\nBob --> Alice : complete\nnote over Alice : all done'
))

# ---------------------------------------------------------------------------
# 129. LARGE COMBINATION: every category represented
# ---------------------------------------------------------------------------

write_puml("seq_all_features_01.puml", puml(
    '''!pragma teoz true
autonumber "<b>(##)</b>"
hide footbox

box "Frontend" #E3F2FD
  actor "End User" as User
  boundary Browser
end box
box "API" #E8F5E9
  control APIGateway <<service>>
  control AuthService <<service>>
end box
box "Data" #FFF3E0
  entity UserService <<entity>>
  database MainDB
  collections Cache <<cache>>
end box

title System Interaction Diagram
header v1.0 | Confidential
footer Page %page%

User -> Browser ++ : navigate
Browser -> APIGateway ++ : GET /app
APIGateway -> AuthService ++ : check auth
AuthService -> MainDB ++ : lookup session
MainDB --> AuthService -- : session data
AuthService --> APIGateway -- : authenticated

alt authorized
  note over APIGateway : authorized path
  APIGateway -> UserService ++ : get user data

  opt use cache
    UserService -> Cache ++ : lookup
    alt cache hit
      Cache --> UserService -- : data
    else miss
      Cache --> UserService -- : nil
      UserService -> MainDB ++ : query
      MainDB --> UserService -- : user row
      UserService -> Cache ++ : store
      Cache --> UserService -- : ok
    end
  end

  loop for each permission
    UserService -> MainDB ++ : check perm
    MainDB --> UserService -- : perm ok
  end

  UserService --> APIGateway -- : user+perms
  APIGateway --> Browser -- : 200 + data
  Browser --> User -- : show dashboard

else unauthorized
  note right of APIGateway : unauthorized path
  APIGateway --> Browser -- : 403
  Browser --> User -- : show login
end'''
))

write_puml("seq_all_features_02.puml", puml(
    '''participant "<<actor>>\nCustomer" as C
participant "<<boundary>>\nWebsite" as W
participant "<<control>>\nOrderCtrl" as OC
participant "<<entity>>\nOrder" as O
participant "<<entity>>\nInventory" as I
database "<<database>>\nDB" as DB
queue "<<queue>>\nEventBus" as EB
collections "<<collections>>\nWarehouse" as WH

title E-Commerce Order Flow

== Browsing ==
C -> W : browse products
W -> OC : get catalog
OC -> I ++ : check stock
I -> DB ++ : query
DB --> I -- : results
I --> OC -- : catalog
OC --> W : products
W --> C : display

== Ordering ==
C -> W : add to cart
C -> W : checkout
W -> OC ++ : createOrder(items)

critical payment
  OC -> DB ++ : begin transaction
  DB --> OC -- : tx started
  OC -> O : create()
  O -> DB ++ : insert order
  DB --> O -- : order_id=99
  OC -> DB ++ : commit
  DB --> OC -- : committed
end

par notifications
  OC -> EB : OrderCreated(99)
and fulfillment
  OC -> WH : pick(order=99)
  WH -> DB ++ : update stock
  DB --> WH -- : done
end

OC --> W -- : order 99 confirmed
W --> C : Thank you! Order #99'''
))

# ---------------------------------------------------------------------------
# 130. PARTICIPANT: unicode stereotypes and special chars in all positions
# ---------------------------------------------------------------------------

UNICODE_STEREOS = [
    "<<サービス>>",
    "<<외부>>",
    "<<服务>>",
    "<<服務>>",
    "<<сервис>>",
]

for i, stereo in enumerate(UNICODE_STEREOS):
    body = (f'participant "Service {i}" {stereo}\n'
            f'participant "Client {i}"\n'
            f'"Service {i}" -> "Client {i}" : call\n'
            f'"Client {i}" --> "Service {i}" : reply')
    write_puml(f"seq_unicode_stereo_{i+1:02d}.puml", puml(body))

# ---------------------------------------------------------------------------
# 131. SKINPARAM: font settings
# ---------------------------------------------------------------------------

FONT_SETTINGS = [
    ("Arial", 12, "arial_12"),
    ("Arial", 14, "arial_14"),
    ("Arial", 16, "arial_16"),
    ("Courier", 12, "courier_12"),
    ("Times", 12, "times_12"),
    ("Helvetica", 12, "helvetica_12"),
]

for font, size, name in FONT_SETTINGS:
    body = "Alice -> Bob : hello\nBob --> Alice : hi"
    write_puml(f"seq_font_{name}.puml",
               puml(body, skinparams=f"skinparam DefaultFontName {font}\nskinparam DefaultFontSize {size}"))

# Participant-specific font
write_puml("seq_font_participant_specific.puml", puml(
    'Alice -> Bob : hello\nBob --> Alice : hi',
    skinparams='skinparam ParticipantFontSize 16\nskinparam ParticipantFontStyle bold\nskinparam NoteFontSize 12\nskinparam NoteFontStyle italic'
))

# ---------------------------------------------------------------------------
# 132. MULTI-LEVEL NESTING WITH DIFFERENT GROUP TYPE ORDERS
# ---------------------------------------------------------------------------

GROUP_PERMUTATIONS = list(itertools.permutations(["alt", "opt", "loop"], 3))

for i, (g1, g2, g3) in enumerate(GROUP_PERMUTATIONS):
    if g1 == "alt":
        g1_open = f"{g1} cond1"
        g1_else = "\nelse other1"
        g1_close = "end"
    else:
        g1_open = f"{g1} cond1"
        g1_else = ""
        g1_close = "end"

    if g2 == "alt":
        g2_open = f"  {g2} cond2"
        g2_else = "\n  else other2"
        g2_close = "  end"
    else:
        g2_open = f"  {g2} cond2"
        g2_else = ""
        g2_close = "  end"

    if g3 == "alt":
        g3_open = f"    {g3} cond3"
        g3_else = "\n    else other3"
        g3_close = "    end"
    else:
        g3_open = f"    {g3} cond3"
        g3_else = ""
        g3_close = "    end"

    body = (f"A -> B : start\n"
            f"{g1_open}\n"
            f"{g2_open}\n"
            f"{g3_open}\n"
            f"      B -> C : deep\n"
            f"      C --> B : result{g3_else}\n"
            f"{g3_close}\n"
            f"    B --> A : g3 done{g2_else}\n"
            f"{g2_close}\n"
            f"  B --> A : g2 done{g1_else}\n"
            f"{g1_close}\n"
            f"B --> A : complete")
    write_puml(f"seq_nest_perm_{i+1:02d}_{g1}_{g2}_{g3}.puml", puml(body))

# ---------------------------------------------------------------------------
# 133. HNOTE AND RNOTE × ALL POSITIONS × COLORS
# ---------------------------------------------------------------------------

for note_kw in ["hnote", "rnote"]:
    for pos in ["over Alice", "over Bob", "over Alice, Bob",
                "left of Alice", "right of Bob"]:
        for color in ["#red", "#lightblue", ""]:
            safe_pos = pos.replace(" ", "_").replace(",", "").replace("Alice", "A").replace("Bob", "B")
            safe_color = color.replace("#", "c") if color else "nocolor"
            color_part = f" {color}" if color else ""
            body = (f"participant Alice\nparticipant Bob\n"
                    f"Alice -> Bob : hello\n"
                    f"{note_kw} {pos}{color_part} : {note_kw} content\n"
                    f"Bob --> Alice : hi")
            write_puml(f"seq_{note_kw}_{safe_pos}_{safe_color}.puml", puml(body))

# ---------------------------------------------------------------------------
# 134. CREATION AND DELETION PATTERNS
# ---------------------------------------------------------------------------

write_puml("seq_create_in_group.puml", puml(
    'participant Alice\nAlice -> Alice : start\nalt need helper\n  create Bob\n  Alice -> Bob : init\n  Bob --> Alice : ready\n  Alice -> Bob : do work\n  Bob --> Alice : done\n  destroy Bob\nelse no helper\n  Alice -> Alice : do it myself\nend'
))

write_puml("seq_create_with_activation.puml", puml(
    'participant Factory\ncreate participant Product\nFactory -> Product ++ : new\nProduct -> Product : initialize\nProduct --> Factory -- : ready\nFactory -> Product : use\nProduct --> Factory : done'
))

write_puml("seq_create_multiple.puml", puml(
    'participant Spawner\nloop 3 times\n  create participant Worker\n  Spawner -> Worker ++ : start task\n  Worker -> Worker : process\n  Worker --> Spawner -- : task done\n  destroy Worker\nend\nSpawner -> Spawner : all done'
))

# ---------------------------------------------------------------------------
# 135. COMPLEX REFERENCE PATTERNS
# ---------------------------------------------------------------------------

write_puml("seq_ref_spanning_boxes.puml", puml(
    'box "System A"\n  participant A1\n  participant A2\nend box\nbox "System B"\n  participant B1\nend box\nA1 -> B1 : call\nref over A1, A2, B1 : See interaction diagram\nB1 --> A1 : result'
))

write_puml("seq_ref_in_loop.puml", puml(
    'loop 3 times\n  Alice -> Bob : request\n  ref over Alice, Bob : See sub-flow\n  Bob --> Alice : response\nend'
))

write_puml("seq_ref_multiline_in_group.puml", puml(
    'alt complex\n  Alice -> Bob : start\n  ref over Alice, Bob\n    Complex sub-interaction\n    See diagram: ComplexFlow\n  end ref\n  Bob --> Alice : result\nelse simple\n  Alice -> Bob : simple\n  Bob --> Alice : done\nend'
))

# ---------------------------------------------------------------------------
# 136. PARTICIPANT: all declaration forms
# ---------------------------------------------------------------------------

DECLARATION_FORMS = [
    ('participant Alice', 'plain'),
    ('participant "Alice" as A', 'quoted_alias'),
    ('participant Alice #red', 'color'),
    ('participant Alice <<stereo>>', 'stereotype'),
    ('participant Alice order 5', 'order'),
    ('participant "Alice Smith" as A #blue <<ext>> order 10', 'full'),
    ('actor Alice', 'actor_plain'),
    ('actor "Alice Smith" as A', 'actor_quoted_alias'),
    ('actor Alice #green <<user>>', 'actor_color_stereo'),
    ('database Alice', 'database'),
    ('collections Alice', 'collections'),
    ('queue Alice', 'queue'),
    ('boundary Alice', 'boundary'),
    ('control Alice', 'control'),
    ('entity Alice', 'entity'),
]

for decl, form_name in DECLARATION_FORMS:
    body = f"{decl}\nparticipant Bob\n"
    # handle alias
    if " as A" in decl:
        body += "A -> Bob : hello\nBob --> A : hi"
    else:
        body += "Alice -> Bob : hello\nBob --> Alice : hi"
    write_puml(f"seq_decl_form_{form_name}.puml", puml(body))

# ---------------------------------------------------------------------------
# 137. SPACING AND DELAYS INSIDE GROUPS
# ---------------------------------------------------------------------------

for gtype in ["alt", "loop", "opt"]:
    if gtype == "alt":
        body = (f"{gtype} condition\n"
                f"  Alice -> Bob : step 1\n"
                f"  |||\n"
                f"  Bob -> Charlie : step 2\n"
                f"  ... waiting ...\n"
                f"  Charlie --> Bob : done\n"
                f"  ||20||\n"
                f"  Bob --> Alice : result\n"
                f"else\n"
                f"  Bob --> Alice : error\n"
                f"end")
    else:
        body = (f"{gtype} condition\n"
                f"  Alice -> Bob : step 1\n"
                f"  |||\n"
                f"  Bob -> Charlie : step 2\n"
                f"  ... waiting ...\n"
                f"  Charlie --> Bob : done\n"
                f"  Bob --> Alice : result\n"
                f"end")
    write_puml(f"seq_spacing_in_{gtype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 138. AUTONUMBER: all formats × all groups
# ---------------------------------------------------------------------------

AUTONUMBER_FMTS_SHORT = [
    ("1", "plain"),
    ('"<b>##</b>"', "bold"),
    ('"(##)"', "parens"),
    ('"Step ##"', "step"),
]

for fmt, fmt_name in AUTONUMBER_FMTS_SHORT:
    for gtype in ["alt", "loop"]:
        if gtype == "alt":
            body = (f"autonumber {fmt}\n"
                    f"Alice -> Bob : a\n"
                    f"alt ok\n"
                    f"  Bob --> Alice : b\n"
                    f"else\n"
                    f"  Bob --> Alice : c\n"
                    f"end")
        else:
            body = (f"autonumber {fmt}\n"
                    f"Alice -> Bob : a\n"
                    f"{gtype} 3 times\n"
                    f"  Bob -> Alice : b\n"
                    f"end\n"
                    f"Bob --> Alice : c")
        write_puml(f"seq_autonumber_{fmt_name}_{gtype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 139. CAPTION VARIANTS
# ---------------------------------------------------------------------------

CAPTIONS = [
    "Figure 1: Basic Flow",
    "**Figure 2**: Complex Interaction",
    "Figure 3 — System Overview",
    "Diagram generated on %date%",
    "<color:blue>Figure 5: Blue Caption</color>",
]

for i, caption in enumerate(CAPTIONS):
    body = f"Alice -> Bob : hello\nBob --> Alice : hi\ncaption {caption}"
    write_puml(f"seq_caption_{i+1:02d}.puml", puml(body))

# ---------------------------------------------------------------------------
# 140. COMBINATIONS: title + header + footer + caption + skinparam + hide footbox
# ---------------------------------------------------------------------------

write_puml("seq_full_headers.puml", puml(
    'Alice -> Bob : hello\ncaption Figure 1',
    title="Diagram Title",
    header="Company Name | Version 1.0",
    footer="Page %page% of %lastpage% | Confidential",
    skinparams="skinparam DefaultFontName Arial"
))

write_puml("seq_full_headers_hide_footbox.puml", puml(
    'hide footbox\nAlice -> Bob : hello\nBob --> Alice : hi\ncaption Figure 2',
    title="No Footer Box",
    header="Header",
    footer="Footer"
))

# ---------------------------------------------------------------------------
# 141. SYSTEMATIC: all message arrow types × colored
# ---------------------------------------------------------------------------

BASE_ARROWS = ["->", "-->", "->>", "-->>"]
COLORS_SYSTEMATIC = ["red", "blue", "green", "#FF8800", "#9900FF"]

for arrow in BASE_ARROWS:
    for color in COLORS_SYSTEMATIC:
        safe_a = arrow.replace(">", "g").replace("-", "m")
        safe_c = color.replace("#", "hex")
        body = (f"Alice -[#{color}]{arrow[1:]} Bob : colored {arrow}\n"
                f"Bob -[#{color}]{arrow[1:]} Alice : return colored")
        write_puml(f"seq_colored_arrow_{safe_a}_{safe_c}.puml", puml(body))

# ---------------------------------------------------------------------------
# 142. PARTICIPANT COLORS × STEREOTYPES × GROUPS
# ---------------------------------------------------------------------------

for color in ["#red", "#blue", "#lightgreen"]:
    safe_c = color.replace("#", "c")
    for stereo in ["<<internal>>", "<<external>>"]:
        safe_s = stereo.replace("<<", "").replace(">>", "")
        body = (f'participant Alice {color} {stereo}\n'
                f'participant Bob\n'
                f'Alice -> Bob : call\n'
                f'alt ok\n'
                f'  note over Alice : {safe_s}\n'
                f'  Bob --> Alice : ok\n'
                f'else\n'
                f'  Bob --> Alice : error\n'
                f'end')
        write_puml(f"seq_pcolor_{safe_c}_stereo_{safe_s}_group.puml", puml(body))

# ---------------------------------------------------------------------------
# 143. EDGE: empty messages in various positions
# ---------------------------------------------------------------------------

write_puml("seq_empty_msg_variants.puml", puml(
    'Alice -> Bob :\nBob --> Alice :\nAlice -> Charlie :\nCharlie --> Alice'
))

write_puml("seq_empty_msg_in_group.puml", puml(
    'loop 3\n  Alice -> Bob :\n  Bob --> Alice :\nend'
))

write_puml("seq_whitespace_msg.puml", puml(
    'Alice -> Bob :  \nBob --> Alice :   '
))

# ---------------------------------------------------------------------------
# 144. TEOZ PRAGMA × SKINPARAM COMBOS
# ---------------------------------------------------------------------------

for theme_name, theme_sp in SIMPLE_THEMES:
    body = 'Alice -> Bob : hello\nBob -> Charlie : forward\nCharlie --> Bob : reply\nBob --> Alice : done'
    write_puml(f"seq_teoz_theme_{theme_name}.puml",
               puml(body, pragma="!pragma teoz true", skinparams=theme_sp))

# ---------------------------------------------------------------------------
# 145. BOUNDARY CONDITIONS: single and zero messages
# ---------------------------------------------------------------------------

write_puml("seq_zero_messages.puml", puml(
    'participant Alice\nparticipant Bob\nparticipant Charlie'
))

write_puml("seq_one_message.puml", puml(
    'Alice -> Bob : only message'
))

write_puml("seq_two_messages.puml", puml(
    'Alice -> Bob : first\nBob --> Alice : second'
))

write_puml("seq_three_messages.puml", puml(
    'Alice -> Bob : 1\nBob -> Charlie : 2\nCharlie --> Alice : 3'
))

# ---------------------------------------------------------------------------
# 146. LONG CHAINS
# ---------------------------------------------------------------------------

for chain_len in [5, 10, 15, 20]:
    parts = "\n".join(f"participant P{i:02d}" for i in range(chain_len))
    fwd = "\n".join(f"P{i:02d} -> P{i+1:02d} : step {i+1}" for i in range(chain_len - 1))
    back = "\n".join(f"P{chain_len-1-i:02d} --> P{chain_len-2-i:02d} : ret {i+1}"
                     for i in range(chain_len - 1))
    write_puml(f"seq_chain_len{chain_len:02d}.puml", puml(f"{parts}\n{fwd}\n{back}"))

# ---------------------------------------------------------------------------
# 147. MIXED PARTICIPANT ORDER AND MESSAGES
# ---------------------------------------------------------------------------

write_puml("seq_mixed_order_messages.puml", puml(
    'participant Z order 1\nparticipant A order 5\nparticipant M order 3\nparticipant B order 2\nparticipant Y order 4\n\nA -> B : A to B\nB -> M : B to M\nM -> Y : M to Y\nY -> Z : Y to Z\nZ -> A : Z to A (wrap)'
))

# ---------------------------------------------------------------------------
# 148. AUTOMATION: keyword combinations in sequence
# ---------------------------------------------------------------------------

KEYWORDS = [
    ("autoactivate on", "autoactivate_on"),
    ("hide footbox", "hide_footbox"),
    ('skinparam monochrome true', "monochrome"),
    ('skinparam shadowing false', "no_shadow"),
]

for keyword, name in KEYWORDS:
    body = f"{keyword}\nAlice -> Bob ++ : request\nBob -> Charlie ++ : delegate\nreturn charlie done\nreturn bob done"
    write_puml(f"seq_keyword_{name}.puml", puml(body))

# autoactivate + hide footbox
write_puml("seq_autoactivate_hide_footbox.puml", puml(
    'autoactivate on\nhide footbox\nAlice -> Bob : a\nBob -> Charlie : b\nreturn c\nreturn b'
))

# ---------------------------------------------------------------------------
# 149. MANY DIVIDERS AND DELAYS TOGETHER
# ---------------------------------------------------------------------------

body_dividers = "participant Alice\nparticipant Bob\n"
for i in range(8):
    body_dividers += f"Alice -> Bob : step {i+1}\n"
    if i % 2 == 0:
        body_dividers += f"== Phase {i//2 + 1} ==\n"
    else:
        body_dividers += f"... pause {i} ...\n"
    body_dividers += f"Bob --> Alice : reply {i+1}\n"
    body_dividers += "|||\n"
write_puml("seq_many_dividers_delays.puml", puml(body_dividers))

# ---------------------------------------------------------------------------
# 150. FINAL KITCHEN SINK EXTRAS
# ---------------------------------------------------------------------------

write_puml("seq_kitchen_sink_06.puml", puml(
    '''!pragma teoz true
autonumber 1 5
hide footbox

box "External" #FFE4E1
  actor "3rd Party" as TP
end box
box "Edge" #E3F2FD
  boundary "API Edge" as Edge <<gateway>>
end box
box "Core" #E8F5E9
  control "Router" as Router <<router>>
  control "Processor" as Proc <<worker>>
  entity "State Machine" as SM <<entity>>
  database "Store" as Store
  queue "Outbox" as Outbox <<queue>>
  collections "Cache" as Cache <<cache>>
end box

title Comprehensive Event Processing
header System v3.0
footer Confidential - Page %page%

TP -> Edge : POST /events
Edge -> Router ++ : route(event)
note right of Edge : ingress

alt valid event type
  Router -> SM ++ : transition(event)
  SM -> Store ++ : load state
  Store --> SM -- : current state

  critical state update
    SM -> SM : apply transition
    SM -> Store ++ : save state
    DB --> SM -- : saved
  end

  SM --> Router -- : new state

  par processing
    Router -> Proc ++ : process(event)
    opt cache lookup
      Proc -> Cache ++ : get(key)
      alt hit
        Cache --> Proc -- : value
      else miss
        Cache --> Proc -- : nil
        Proc -> Store ++ : fetch
        Store --> Proc -- : data
        Proc -> Cache ++ : set(key, data)
        Cache --> Proc -- : ok
      end
    end
    Proc -> Outbox : publish(result)
    Proc --> Router -- : done
  and notification
    Router -> TP : webhook(event_id)
  end

  Router --> Edge -- : 202 Accepted
  Edge --> TP : 202 + event_id

else invalid
  Router --> Edge -- : error details
  Edge --> TP : 400 Bad Request
end'''
))

write_puml("seq_kitchen_sink_07.puml", puml(
    '''actor User
boundary WebUI as "Web UI"
control AppServer as "App Server"
control Cache as "Redis Cache"
entity SessionMgr as "Session Manager"
database PrimaryDB as "Primary DB"
database ReadReplica as "Read Replica"
collections SearchIndex as "Elastic Search"
queue EventBus as "Event Bus"

autonumber

== Search Flow ==

User -> WebUI : search("plantuml")
WebUI -> AppServer ++ : GET /search?q=plantuml
AppServer -> Cache ++ : get("search:plantuml")
alt cache hit
  Cache --> AppServer -- : cached results
else cache miss
  Cache --> AppServer -- : nil
  AppServer -> SearchIndex ++ : query("plantuml")
  SearchIndex --> AppServer -- : 42 results
  AppServer -> Cache ++ : set("search:plantuml", results, ttl=300)
  Cache --> AppServer -- : ok
end
AppServer --> WebUI -- : results JSON
WebUI --> User : show 42 results

== View Detail ==

User -> WebUI : click result #5
WebUI -> AppServer ++ : GET /item/5
AppServer -> SessionMgr ++ : get session
SessionMgr --> AppServer -- : user context
AppServer -> ReadReplica ++ : SELECT * WHERE id=5
ReadReplica --> AppServer -- : item data
AppServer -> EventBus : ItemViewed(user=x, item=5)
AppServer --> WebUI -- : item details
WebUI --> User : show detail page

== Add to Cart ==

User -> WebUI : add to cart
WebUI -> AppServer ++ : POST /cart/add
AppServer -> SessionMgr ++ : get session
SessionMgr --> AppServer -- : user id

critical cart update
  AppServer -> PrimaryDB ++ : INSERT cart_item
  PrimaryDB --> AppServer -- : inserted
  AppServer -> Cache ++ : invalidate("cart:user_x")
  Cache --> AppServer -- : invalidated
end

AppServer -> EventBus : CartUpdated(user=x)
AppServer --> WebUI -- : cart updated
WebUI --> User : cart shows 1 item'''
))

# ---------------------------------------------------------------------------
# 151. SYSTEMATIC: participant type × note type × note position
# ---------------------------------------------------------------------------

for ptype in PARTICIPANT_TYPES:
    for note_kw in ["note", "hnote", "rnote"]:
        body = (f'{ptype} X\nparticipant Y\n'
                f'X -> Y : go\n'
                f'{note_kw} over X : {note_kw}/{ptype}\n'
                f'Y --> X : back')
        write_puml(f"seq_{note_kw}_{ptype}_over_v2.puml", puml(body))

# ---------------------------------------------------------------------------
# 152. SYSTEMATIC: all group types × all arrow types
# ---------------------------------------------------------------------------

for gtype in ["alt", "opt", "loop", "par", "critical"]:
    for arrow, aname in ARROW_VARIANTS:
        if gtype == "par":
            body = (f"A -> B : init\n"
                    f"par\n"
                    f"  A {arrow} B : {aname} branch 1\nand\n"
                    f"  A {arrow} C : {aname} branch 2\n"
                    f"end")
        elif gtype == "alt":
            body = (f"A -> B : init\n"
                    f"alt cond\n"
                    f"  A {arrow} B : {aname} main\n"
                    f"  B --> A : ack\n"
                    f"else other\n"
                    f"  A {arrow} C : {aname} else\n"
                    f"end")
        else:
            body = (f"A -> B : init\n"
                    f"{gtype} cond\n"
                    f"  A {arrow} B : {aname} in {gtype}\n"
                    f"  B --> A : ack\n"
                    f"end")
        write_puml(f"seq_group_{gtype}_arrow_{aname}.puml", puml(body))

# ---------------------------------------------------------------------------
# 153. SYSTEMATIC: activation × autonumber start values
# ---------------------------------------------------------------------------

for start in [1, 5, 10, 50, 100, 1000]:
    for depth in [1, 2, 3]:
        inner = ""
        for d in range(depth):
            inner += f"{'  '*d}P{d} -> P{d+1} ++ : depth {d+1}\n"
        for d in range(depth - 1, -1, -1):
            inner += f"{'  '*d}return result {d+1}\n"
        write_puml(f"seq_autonumber_{start}_depth{depth}.puml",
                   puml(f"autonumber {start}\n{inner}"))

# ---------------------------------------------------------------------------
# 154. SYSTEMATIC: divider + delay + spacing combos
# ---------------------------------------------------------------------------

SPACING_TYPES = ["|||", "||10||", "||30||", "||50||"]
DELAY_TYPES = ["...", "... wait ...", "... 5 min ..."]

for space in SPACING_TYPES:
    for delay in DELAY_TYPES:
        safe_s = space.replace("|", "p").replace(" ", "")
        safe_d = delay.replace(".", "d").replace(" ", "_")
        body = (f"Alice -> Bob : a\n{space}\nBob -> Charlie : b\n{delay}\n"
                f"Charlie --> Bob : c\n{space}\nBob --> Alice : d")
        write_puml(f"seq_space_{safe_s}_delay_{safe_d}.puml", puml(body))

# ---------------------------------------------------------------------------
# 155. SYSTEMATIC: box × group combinations
# ---------------------------------------------------------------------------

for gtype in ["alt", "loop", "opt", "par"]:
    for n_boxes in [1, 2, 3]:
        box_decls = ""
        for b in range(n_boxes):
            colors = ["#lightblue", "#lightyellow", "#lightgreen"]
            box_decls += f'box "Box {b+1}" {colors[b % 3]}\n  participant B{b}A\n  participant B{b}B\nend box\n'
        msgs = "B0A -> B0B : internal 0\n"
        if n_boxes > 1:
            for b in range(n_boxes - 1):
                msgs += f"B{b}B -> B{b+1}A : cross {b}-{b+1}\n"
        if gtype == "alt":
            group = f"alt ok\n  B0A -> B0B : alt action\nelse fail\n  B0A -> B0B : alt else\nend"
        elif gtype == "par":
            group = f"par\n  B0A -> B0B : par 1\nand\n  B0A -> B0B : par 2\nend"
        else:
            group = f"{gtype} cond\n  B0A -> B0B : {gtype} action\nend"
        write_puml(f"seq_box_{n_boxes}boxes_{gtype}.puml",
                   puml(f"{box_decls}\n{msgs}\n{group}"))

# ---------------------------------------------------------------------------
# 156. SYSTEMATIC: note multiline × all note keywords
# ---------------------------------------------------------------------------

MULTILINE_BODIES = [
    ("two_lines", "line one\nline two"),
    ("three_lines", "line one\nline two\nline three"),
    ("bold_header", "**Title**\ncontent here"),
    ("list", "* item one\n* item two"),
    ("code", '""code snippet""'),
    ("mixed", "**Header**\n//italic// and **bold**\n""mono"""),
]

for note_kw in ["note", "hnote", "rnote"]:
    for ml_name, ml_body in MULTILINE_BODIES:
        body = (f"Alice -> Bob : hello\n"
                f"{note_kw} over Alice\n"
                f"  {ml_body.replace(chr(10), chr(10) + '  ')}\n"
                f"end note\n"
                f"Bob --> Alice : hi")
        write_puml(f"seq_{note_kw}_ml_{ml_name}.puml", puml(body))

# ---------------------------------------------------------------------------
# 157. SYSTEMATIC: destroy × participant types
# ---------------------------------------------------------------------------

for ptype in PARTICIPANT_TYPES:
    body = (f"participant Alice\n"
            f"create {ptype} Target\n"
            f"Alice -> Target : use\n"
            f"Target --> Alice : done\n"
            f"destroy Target")
    write_puml(f"seq_destroy_{ptype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 158. SYSTEMATIC: self message × activation × groups
# ---------------------------------------------------------------------------

for gtype in ["alt", "loop", "opt"]:
    if gtype == "alt":
        body = (f"alt condition\n"
                f"  Alice -> Alice ++ : self activate in {gtype}\n"
                f"  Alice --> Alice -- : self done\n"
                f"else\n"
                f"  Alice -> Alice : plain self\n"
                f"end")
    else:
        body = (f"{gtype} condition\n"
                f"  Alice -> Alice ++ : self in {gtype}\n"
                f"  Alice --> Alice -- : done\n"
                f"end")
    write_puml(f"seq_self_activate_{gtype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 159. SYSTEMATIC: autonumber format × participant count
# ---------------------------------------------------------------------------

for fmt, fname in [('""', "plain"), ('"(##)"', "parens"), ('"<b>##</b>"', "bold")]:
    for n in [2, 3, 4, 5]:
        parts = "\n".join(f"participant P{i}" for i in range(n))
        msgs = "\n".join(f"P{i} -> P{(i+1)%n} : msg" for i in range(n))
        write_puml(f"seq_autonumber_fmt_{fname}_{n}parts.puml",
                   puml(f"autonumber {fmt}\n{parts}\n{msgs}"))

# ---------------------------------------------------------------------------
# 160. SYSTEMATIC: teoz × activation × note types
# ---------------------------------------------------------------------------

for note_kw in ["note", "hnote", "rnote"]:
    body = (f"Alice -> Bob ++ : request\n"
            f"{note_kw} over Alice : teoz + {note_kw}\n"
            f"Bob -> Charlie ++ : delegate\n"
            f"{note_kw} over Bob, Charlie : over two\n"
            f"Charlie --> Bob -- : done\n"
            f"Bob --> Alice -- : result")
    write_puml(f"seq_teoz_{note_kw}.puml", puml(body, pragma="!pragma teoz true"))

# ---------------------------------------------------------------------------
# 161. SYSTEMATIC: hide footbox × all group types
# ---------------------------------------------------------------------------

for gtype in ["alt", "loop", "opt", "par", "break", "critical"]:
    if gtype == "par":
        body = f"hide footbox\nA -> B : start\npar\n  B -> C : branch1\nand\n  B -> D : branch2\nend\nB --> A : done"
    elif gtype == "alt":
        body = f"hide footbox\nA -> B : start\nalt ok\n  B -> C : action\n  C --> B : done\n  B --> A : ok\nelse fail\n  B --> A : fail\nend"
    elif gtype == "break":
        body = f"hide footbox\nloop forever\n  A -> B : work\n  break on error\n    B --> A : error\n  end\nend"
    else:
        body = f"hide footbox\nA -> B : start\n{gtype} cond\n  B -> C : action\n  C --> B : done\nend\nB --> A : done"
    write_puml(f"seq_hide_footbox_group_{gtype}_v2.puml", puml(body))

# ---------------------------------------------------------------------------
# 162. SYSTEMATIC: all participant types × create/destroy
# ---------------------------------------------------------------------------

for ptype in PARTICIPANT_TYPES:
    body = (f"participant Factory\n"
            f"loop 3 times\n"
            f"  create {ptype} Item\n"
            f"  Factory -> Item ++ : init\n"
            f"  Item --> Factory -- : ready\n"
            f"  Factory -> Item : use\n"
            f"  destroy Item\n"
            f"end")
    write_puml(f"seq_create_loop_{ptype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 163. SYSTEMATIC: participant type pairs in boxes
# ---------------------------------------------------------------------------

for t1, t2 in itertools.combinations(PARTICIPANT_TYPES[:4], 2):
    body = (f'box "Group"\n  {t1} A\n  {t2} B\nend box\n'
            f'A -> B : hello\nB --> A : hi')
    write_puml(f"seq_box_pair_{t1}_{t2}.puml", puml(body))

# ---------------------------------------------------------------------------
# 164. SYSTEMATIC: newpage × feature combos
# ---------------------------------------------------------------------------

NEWPAGE_FEATURES = [
    ("activation", "Alice -> Bob ++ : before\nnewpage\nBob --> Alice -- : after"),
    ("group", "Alice -> Bob : before\nalt cond\n  Bob --> Alice : ok\nelse\n  Bob --> Alice : fail\nend\nnewpage\nBob --> Alice : done"),
    ("note", "note over Alice : before\nAlice -> Bob : msg\nnewpage\nnote over Bob : after\nBob --> Alice : reply"),
    ("divider", "Alice -> Bob : a\n== divider ==\nBob --> Alice : b\nnewpage Third\nAlice -> Bob : c"),
    ("autonumber", "autonumber\nAlice -> Bob : 1\nBob --> Alice : 2\nnewpage\nAlice -> Bob : continues\nBob --> Alice : counted"),
]

for name, body in NEWPAGE_FEATURES:
    write_puml(f"seq_newpage_{name}.puml", puml(body))

# ---------------------------------------------------------------------------
# 165. SYSTEMATIC: all skinparam × all participant types
# ---------------------------------------------------------------------------

SP_PARAMS = [
    ("ParticipantBackgroundColor", "lightyellow"),
    ("ParticipantBorderColor", "navy"),
    ("ActorBackgroundColor", "lightgreen"),
    ("ActorBorderColor", "darkgreen"),
]

for ptype in PARTICIPANT_TYPES:
    for param, value in SP_PARAMS:
        body = (f"{ptype} X\nparticipant Y\n"
                f"X -> Y : hello\nY --> X : hi")
        write_puml(f"seq_sk_{ptype}_{param.lower()}.puml",
                   puml(body, skinparams=f"skinparam {param} {value}"))

# ---------------------------------------------------------------------------
# 166. SYSTEMATIC: box × hide footbox × autonumber
# ---------------------------------------------------------------------------

for n_boxes in [1, 2, 3]:
    for has_autonumber in [True, False]:
        box_body = ""
        for b in range(n_boxes):
            box_body += f'box "Box {b+1}"\n  participant A{b}\n  participant B{b}\nend box\n'
        msgs = "\n".join(f"A{b} -> B{b} : msg {b+1}\nB{b} --> A{b} : reply {b+1}" for b in range(n_boxes))
        prefix = "autonumber\nhide footbox\n" if has_autonumber else "hide footbox\n"
        num_suf = "_autonumber" if has_autonumber else ""
        write_puml(f"seq_box_{n_boxes}b_footbox{num_suf}.puml",
                   puml(f"{prefix}{box_body}\n{msgs}"))

# ---------------------------------------------------------------------------
# 167. EXTRA KITCHEN SINK DIAGRAMS
# ---------------------------------------------------------------------------

write_puml("seq_kitchen_sink_08.puml", puml(
    '''participant "Web Browser" as B
participant "API Server" as S
participant "Auth Module" as A
participant "Database" as DB
participant "Cache" as C
participant "Logger" as L
participant "Notification" as N

autonumber

B -> S ++ : POST /register {email, password}
S -> L : log request
S -> A ++ : hash password
A --> S -- : hashed_pw
S -> DB ++ : INSERT user
DB --> S -- : user_id=42
S -> C ++ : cache user 42
C --> S -- : ok
S -> N ++ : send welcome email
N --> S -- : queued
S -> L : log success
S --> B -- : 201 {user_id: 42}

note over B, N : Registration complete — all systems notified'''
))

write_puml("seq_kitchen_sink_09.puml", puml(
    '''!pragma teoz true

box "Mobile" #FFF3E0
  actor "User" as U
  participant "App" as App
end box

box "Cloud" #E8EAF6
  boundary "CDN" as CDN
  control "API" as API
  control "Worker" as W
  database "DB" as DB
  queue "Queue" as Q
end box

autonumber 100 5
hide footbox

U -> App ++ : open app
App -> CDN ++ : GET /static
CDN --> App -- : assets

App -> API ++ : GET /feed

par loading
  API -> DB ++ : fetch posts
  DB --> API -- : 10 posts
and background sync
  API -> Q : enqueue analytics
  Q -> W ++ : process
  W -> DB ++ : update stats
  DB --> W -- : done
  W --> Q -- : complete
end

API --> App -- : feed data
App --> U -- : display feed

loop auto-refresh every 30s
  App -> API ++ : GET /feed?since=last
  API -> DB ++ : incremental fetch
  DB --> API -- : new posts
  API --> App -- : updates
  App -> U : push notifications
end'''
))

write_puml("seq_kitchen_sink_10.puml", puml(
    '''actor Developer
participant "Git Client" as Git
boundary "CI/CD Pipeline" as CI
control "Build Server" as Build
control "Test Runner" as Test
control "Deploy Agent" as Deploy
database "Artifact Store" as Artifacts
participant "Staging Env" as Staging
participant "Production" as Prod

title Deployment Pipeline

Developer -> Git : git push origin feature
Git -> CI ++ : webhook: push event

== Build Stage ==
CI -> Build ++ : trigger build
Build -> Artifacts : fetch dependencies
Artifacts --> Build : deps ok
Build -> Build : compile
Build -> Build : package
Build -> Artifacts ++ : store artifact
Artifacts --> Build -- : artifact_id=789
Build --> CI -- : build success

== Test Stage ==
CI -> Test ++ : run tests
Test -> Artifacts ++ : fetch artifact 789
Artifacts --> Test -- : artifact
par test suites
  Test -> Test : unit tests
and
  Test -> Test : integration tests
and
  Test -> Test : security scan
end
Test --> CI -- : all tests pass

== Deploy Stage ==
alt approved for deploy
  CI -> Deploy ++ : deploy to staging
  Deploy -> Artifacts ++ : fetch artifact
  Artifacts --> Deploy -- : binary
  Deploy -> Staging ++ : update
  Staging --> Deploy -- : healthy
  Deploy --> CI -- : staging ok

  CI -> Developer : staging deployed
  Developer -> Staging : smoke test
  Staging --> Developer : looks good

  Developer -> CI : approve production
  CI -> Deploy ++ : deploy to production
  Deploy -> Prod ++ : rolling update
  loop each instance
    Prod -> Prod : drain traffic
    Prod -> Prod : update
    Prod -> Prod : health check
  end
  Prod --> Deploy -- : all healthy
  Deploy --> CI -- : prod deployed
  CI --> Developer -- : deployment complete!
else build or tests failed
  CI --> Developer -- : pipeline failed
end'''
))

# ---------------------------------------------------------------------------
# 168. SYSTEMATIC: all participant types × all arrow types × self and peer
# ---------------------------------------------------------------------------

for ptype in PARTICIPANT_TYPES:
    for arrow, aname in ARROW_VARIANTS:
        # self arrow for ptype
        body = f"{ptype} Alice\nAlice {arrow} Alice : self {ptype} {aname}"
        write_puml(f"seq_ptype_{ptype}_self_{aname}.puml", puml(body))
        # with colored arrow
        body = f"{ptype} Alice\nparticipant Bob\nAlice -[#red]{arrow[1:]} Bob : {ptype} colored {aname}"
        write_puml(f"seq_ptype_{ptype}_colored_{aname}.puml", puml(body))

# ---------------------------------------------------------------------------
# 169. SYSTEMATIC: skinparam per-participant-type
# ---------------------------------------------------------------------------

SK_PER_TYPE = {
    "actor": [("ActorBackgroundColor", "lightblue"), ("ActorBorderColor", "navy"),
               ("ActorFontColor", "black"), ("ActorFontSize", "14")],
    "participant": [("ParticipantBackgroundColor", "lightyellow"), ("ParticipantBorderColor", "darkblue"),
                    ("ParticipantFontSize", "12"), ("ParticipantPadding", "10")],
    "boundary": [("BoundaryBackgroundColor", "lightcyan"), ("BoundaryBorderColor", "teal")],
    "control": [("ControlBackgroundColor", "lightyellow"), ("ControlBorderColor", "orange")],
    "entity": [("EntityBackgroundColor", "lightgreen"), ("EntityBorderColor", "darkgreen")],
    "database": [("DatabaseBackgroundColor", "#FAEBD7"), ("DatabaseBorderColor", "brown")],
    "collections": [("CollectionsBackgroundColor", "lavender"), ("CollectionsBorderColor", "purple")],
    "queue": [("QueueBackgroundColor", "#FFF0F5"), ("QueueBorderColor", "deeppink")],
}

for ptype, params in SK_PER_TYPE.items():
    for param, value in params:
        safe_p = param.lower()
        body = f"{ptype} Alice\nparticipant Bob\nAlice -> Bob : hello\nBob --> Alice : hi"
        write_puml(f"seq_sk_ptype_{ptype}_{safe_p}.puml",
                   puml(body, skinparams=f"skinparam {param} {value}"))

# ---------------------------------------------------------------------------
# 170. SYSTEMATIC: all groups × autonumber formats × 3-participant
# ---------------------------------------------------------------------------

GROUP_TYPES_ALL = ["alt", "opt", "loop", "par", "critical", "break"]
AUTONUMBER_FMT_MINI = [("", "default"), ('"(##)"', "parens"), ('"<b>##</b>"', "bold")]

for gtype in GROUP_TYPES_ALL:
    for fmt, fname in AUTONUMBER_FMT_MINI:
        if gtype == "par":
            body = (f"autonumber{(' ' + fmt) if fmt else ''}\n"
                    f"A -> B : start\npar\n  B -> C : p1\nand\n  B -> D : p2\nend\nB --> A : done")
        elif gtype == "alt":
            body = (f"autonumber{(' ' + fmt) if fmt else ''}\n"
                    f"A -> B : start\nalt cond\n  B -> C : ok\n  C --> B : result\n  B --> A : success\nelse\n  B --> A : fail\nend")
        elif gtype == "break":
            body = (f"autonumber{(' ' + fmt) if fmt else ''}\n"
                    f"loop forever\n  A -> B : work\n  break error\n    B --> A : error\n  end\nend")
        else:
            body = (f"autonumber{(' ' + fmt) if fmt else ''}\n"
                    f"A -> B : start\n{gtype} cond\n  B -> C : action\n  C --> B : done\nend\nB --> A : result")
        write_puml(f"seq_group_{gtype}_autonumber_{fname}.puml", puml(body))

# ---------------------------------------------------------------------------
# 171. SYSTEMATIC: all arrow types × all group types
# ---------------------------------------------------------------------------

for arrow, aname in ARROW_VARIANTS:
    for gtype in ["alt", "loop", "opt"]:
        if gtype == "alt":
            body = (f"A -> B : trigger\nalt ok\n  A {arrow} B : {aname} ok\n  B --> A : ack\nelse fail\n  A {arrow} B : {aname} fail\nend")
        else:
            body = (f"A -> B : start\n{gtype} cond\n  A {arrow} B : {aname} in {gtype}\n  B --> A : ack\nend")
        write_puml(f"seq_arrow_{aname}_{gtype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 172. SYSTEMATIC: divider × skinparam
# ---------------------------------------------------------------------------

DIVIDER_SKINPARAMS = [
    ("SequenceDividerBackgroundColor", "#E0E0E0"),
    ("SequenceDividerBorderColor", "navy"),
    ("SequenceDividerFontColor", "darkred"),
    ("SequenceDividerFontSize", "14"),
    ("SequenceDividerBorderThickness", "2"),
]

for param, value in DIVIDER_SKINPARAMS:
    body = "Alice -> Bob : before\n== Divider ==\nBob --> Alice : after"
    write_puml(f"seq_divider_sk_{param.lower()}.puml",
               puml(body, skinparams=f"skinparam {param} {value}"))

# ---------------------------------------------------------------------------
# 173. SYSTEMATIC: box × teoz × groups
# ---------------------------------------------------------------------------

for gtype in ["alt", "opt", "loop"]:
    body = (f'box "A" #lightblue\n  participant Alice\nend box\n'
            f'box "B" #lightyellow\n  participant Bob\n  participant Charlie\nend box\n'
            f'Alice -> Bob : go\n')
    if gtype == "alt":
        body += "alt cond\n  Bob -> Charlie : ok\n  Charlie --> Bob : done\n  Bob --> Alice : ok\nelse\n  Bob --> Alice : fail\nend"
    else:
        body += f"{gtype} cond\n  Bob -> Charlie : action\n  Charlie --> Bob : result\nend\nBob --> Alice : done"
    write_puml(f"seq_teoz_box_{gtype}.puml", puml(body, pragma="!pragma teoz true"))

# ---------------------------------------------------------------------------
# 174. SYSTEMATIC: note across × participant counts
# ---------------------------------------------------------------------------

for n in [2, 3, 4, 5, 6]:
    parts = "\n".join(f"participant P{i}" for i in range(n))
    msgs = "P0 -> P1 : go\n"
    msgs += "note across : note spanning all\n"
    msgs += "P1 --> P0 : done"
    write_puml(f"seq_note_across_{n}parts.puml", puml(f"{parts}\n{msgs}"))

# ---------------------------------------------------------------------------
# 175. SYSTEMATIC: return × note combinations
# ---------------------------------------------------------------------------

for depth in [1, 2, 3]:
    for note_kw in ["note", "hnote", "rnote"]:
        calls = ""
        for d in range(depth):
            calls += f"P{d} -> P{d+1} ++ : call {d+1}\n"
            calls += f"{note_kw} over P{d} : before return {d+1}\n"
        for d in range(depth):
            calls += f"return result {depth-d}\n"
        write_puml(f"seq_return_depth{depth}_{note_kw}.puml", puml(calls))

# ---------------------------------------------------------------------------
# 176. SYSTEMATIC: all participant types × destroy × activation
# ---------------------------------------------------------------------------

for ptype in PARTICIPANT_TYPES:
    body = (f"participant Alice\n"
            f"create {ptype} Temp\n"
            f"Alice -> Temp ++ : init\n"
            f"Temp -> Temp : setup\n"
            f"Temp --> Alice -- : ready\n"
            f"Alice -> Temp !! : use and destroy")
    write_puml(f"seq_create_activate_destroy_{ptype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 177. SYSTEMATIC: color + stereotype + order + alias all together
# ---------------------------------------------------------------------------

COLOR_STEREO_ORDER_COMBOS = [
    ("#red", "<<internal>>", 10),
    ("#blue", "<<external>>", 20),
    ("#green", "<<service>>", 5),
    ("#yellow", "<<legacy>>", 15),
    ("#orange", "<<new>>", 25),
]

for color, stereo, order in COLOR_STEREO_ORDER_COMBOS:
    safe_c = color.replace("#", "")
    safe_s = stereo.replace("<<", "").replace(">>", "")
    for ptype in ["participant", "actor", "boundary", "control"]:
        body = (f'{ptype} "Name" as N {color} {stereo} order {order}\n'
                f'participant Other\n'
                f'N -> Other : call\nOther --> N : reply')
        write_puml(f"seq_full_decl_{ptype}_{safe_c}_{safe_s}_{order}.puml", puml(body))

# ---------------------------------------------------------------------------
# 178. SYSTEMATIC: multiple notes in sequence
# ---------------------------------------------------------------------------

for n_notes in [2, 3, 4, 5]:
    body = "Alice -> Bob : start\n"
    positions = ["over Alice", "over Bob", "right of Bob", "left of Alice", "over Alice, Bob"]
    for i in range(n_notes):
        pos = positions[i % len(positions)]
        body += f"note {pos} : note {i+1}\n"
    body += "Bob --> Alice : done"
    write_puml(f"seq_multi_notes_{n_notes}.puml", puml(body))

# ---------------------------------------------------------------------------
# 179. SYSTEMATIC: all group types × colored activations
# ---------------------------------------------------------------------------

for gtype in ["alt", "loop", "opt"]:
    for color in ["#red", "#blue", "#green"]:
        safe_c = color.replace("#", "c")
        if gtype == "alt":
            body = (f"A -> B ++ {color} : activate {safe_c}\n"
                    f"alt ok\n  B -> C ++ {color} : nested\n  C --> B -- : done\n  B --> A -- : ok\n"
                    f"else fail\n  B --> A -- : fail\nend")
        else:
            body = (f"A -> B ++ {color} : activate {safe_c}\n"
                    f"{gtype} cond\n  B -> C ++ {color} : nested\n  C --> B -- : done\nend\n"
                    f"B --> A -- : result")
        write_puml(f"seq_activate_{safe_c}_{gtype}.puml", puml(body))

# ---------------------------------------------------------------------------
# 180. FINAL BATCH: real-world scenario variations
# ---------------------------------------------------------------------------

SCENARIOS = [
    ("login_flow",
     """actor User
participant App
participant AuthServer
database UserDB

User -> App : enter credentials
App -> AuthServer ++ : POST /login
AuthServer -> UserDB ++ : verify user
UserDB --> AuthServer -- : user found
AuthServer -> AuthServer : check password
alt valid password
  AuthServer --> App -- : JWT token
  App --> User : logged in
else invalid
  AuthServer --> App -- : 401
  App --> User : login failed
end"""),
    ("file_upload",
     """actor User
participant "Web Browser" as Browser
participant "Upload Service" as US
database "File Store" as FS
queue "Process Queue" as PQ

User -> Browser : select file
Browser -> US ++ : POST /upload (multipart)
loop file chunks
  Browser -> US : chunk data
end
US -> FS ++ : store file
FS --> US -- : file_id=abc
US -> PQ : enqueue process(file_id=abc)
US --> Browser -- : 200 {file_id: abc}
Browser --> User : upload complete"""),
    ("cache_pattern",
     """participant Client
participant Service
collections Cache
database DB

Client -> Service ++ : getData(key)
Service -> Cache ++ : get(key)
alt cache hit
  Cache --> Service -- : value
  Service --> Client -- : cached value
else cache miss
  Cache --> Service -- : nil
  Service -> DB ++ : SELECT WHERE key=?
  DB --> Service -- : row
  Service -> Cache ++ : set(key, value, ttl=3600)
  Cache --> Service -- : ok
  Service --> Client -- : fresh value
end"""),
    ("saga_pattern",
     """participant Orchestrator
participant ServiceA
participant ServiceB
participant ServiceC

Orchestrator -> ServiceA ++ : step 1
ServiceA --> Orchestrator -- : done 1

Orchestrator -> ServiceB ++ : step 2
alt success
  ServiceB --> Orchestrator -- : done 2
  Orchestrator -> ServiceC ++ : step 3
  alt success
    ServiceC --> Orchestrator -- : done 3
    note over Orchestrator : saga complete
  else fail C
    ServiceC --> Orchestrator -- : error
    Orchestrator -> ServiceB : compensate B
    Orchestrator -> ServiceA : compensate A
  end
else fail B
  ServiceB --> Orchestrator -- : error
  Orchestrator -> ServiceA : compensate A
end"""),
    ("event_sourcing",
     """participant Client
control CommandHandler
entity Aggregate
database EventStore
queue EventBus
control EventHandler

Client -> CommandHandler ++ : command
CommandHandler -> Aggregate ++ : handle command
Aggregate -> Aggregate : validate
Aggregate -> EventStore ++ : append event
EventStore --> Aggregate -- : event stored
Aggregate --> CommandHandler -- : result
CommandHandler --> Client -- : response

EventStore -> EventBus : publish event
EventBus -> EventHandler ++ : handle event
EventHandler -> EventHandler : update read model
EventHandler --> EventBus -- : processed"""),
]

for scenario_name, scenario_body in SCENARIOS:
    write_puml(f"seq_scenario_{scenario_name}.puml", puml(scenario_body))
    # also with autonumber
    write_puml(f"seq_scenario_{scenario_name}_numbered.puml",
               puml(f"autonumber\n{scenario_body}"))
    # also with teoz
    write_puml(f"seq_scenario_{scenario_name}_teoz.puml",
               puml(scenario_body, pragma="!pragma teoz true"))
    # also with hide footbox
    write_puml(f"seq_scenario_{scenario_name}_nofootbox.puml",
               puml(f"hide footbox\n{scenario_body}"))

# ---------------------------------------------------------------------------
# 181. ACTIVATION DEPTH × COLOR × ARROW TYPE
# ---------------------------------------------------------------------------

for arrow, aname in ARROW_VARIANTS:
    for depth in [2, 3, 4]:
        body = ""
        for d in range(depth):
            body += f"P{d} {arrow} P{d+1} ++ #{'red' if d%2==0 else 'blue'} : depth {d+1} {aname}\n"
        for d in range(depth - 1, -1, -1):
            body += f"return return {d+1}\n"
        write_puml(f"seq_activate_depth{depth}_{aname}.puml", puml(body))

# ---------------------------------------------------------------------------
# 182. REF × BOX × ACTIVATION COMBOS
# ---------------------------------------------------------------------------

for use_box in [True, False]:
    for use_activation in [True, False]:
        prefix = 'box "System"\n  participant Alice\n  participant Bob\nend box\n' if use_box else ""
        if use_activation:
            body = (f"{prefix}Alice -> Bob ++ : call\n"
                    f"ref over Alice, Bob : sub-interaction\n"
                    f"Bob --> Alice -- : done")
        else:
            body = (f"{prefix}Alice -> Bob : call\n"
                    f"ref over Alice, Bob : sub-interaction\n"
                    f"Bob --> Alice : done")
        box_s = "box" if use_box else "nobox"
        act_s = "activate" if use_activation else "noactivate"
        write_puml(f"seq_ref_{box_s}_{act_s}.puml", puml(body))

# ---------------------------------------------------------------------------
# DONE
# ---------------------------------------------------------------------------

print(f"Generated {files_written} .puml files in {OUTPUT_DIR}")
