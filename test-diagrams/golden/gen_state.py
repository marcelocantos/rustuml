#!/usr/bin/env python3
"""
Generate comprehensive PlantUML state diagram test cases.
Target: ~1000+ .puml files covering every conceivable state diagram feature.
"""

import os
import itertools

OUT_DIR = os.path.join(os.path.dirname(__file__), "state")
os.makedirs(OUT_DIR, exist_ok=True)

files_written = 0


def write(name: str, content: str) -> None:
    global files_written
    path = os.path.join(OUT_DIR, name if name.endswith(".puml") else name + ".puml")
    with open(path, "w") as f:
        f.write(content.strip() + "\n")
    files_written += 1


def diagram(body: str, title: str = None, skinparams: str = None) -> str:
    parts = ["@startuml"]
    if title:
        parts.append(f"title {title}")
    if skinparams:
        parts.append(skinparams)
    parts.append(body.strip())
    parts.append("@enduml")
    return "\n".join(parts) + "\n"


# ─── 1. BASIC STATES ──────────────────────────────────────────────────────────

write("state_empty", diagram(""))

write("state_single", diagram("[*] --> A"))

write("state_two_states", diagram("""
[*] --> A
A --> B
B --> [*]
"""))

write("state_three_states", diagram("""
[*] --> A
A --> B
B --> C
C --> [*]
"""))

write("state_chain_5", diagram("""
[*] --> S1
S1 --> S2
S2 --> S3
S3 --> S4
S4 --> S5
S5 --> [*]
"""))

write("state_chain_10", diagram("""
[*] --> S1
S1 --> S2
S2 --> S3
S3 --> S4
S4 --> S5
S5 --> S6
S6 --> S7
S7 --> S8
S8 --> S9
S9 --> S10
S10 --> [*]
"""))

write("state_named_alias", diagram("""
state "Long State Name" as LSN
[*] --> LSN
LSN --> [*]
"""))

write("state_multiple_named_aliases", diagram("""
state "Waiting for Input" as WFI
state "Processing Data" as PD
state "Sending Response" as SR
state "Error Handling" as EH

[*] --> WFI
WFI --> PD : input received
PD --> SR : processed
PD --> EH : error
SR --> WFI : done
EH --> WFI : retry
"""))

write("state_with_description", diagram("""
state A : This is state A
state B : This is state B
[*] --> A
A --> B
B --> [*]
"""))

write("state_with_multiline_description", diagram("""
state A : line one
state A : line two
state A : line three
[*] --> A
A --> [*]
"""))

write("state_initial_only", diagram("""
[*] --> A
"""))

write("state_final_only", diagram("""
A --> [*]
"""))

write("state_initial_and_final", diagram("""
[*] --> A
A --> [*]
"""))

write("state_multiple_initial", diagram("""
[*] --> A
[*] --> B
A --> [*]
B --> [*]
"""))

write("state_multiple_final", diagram("""
[*] --> A
A --> [*]
A --> [*]
"""))

# ─── 2. TRANSITIONS ───────────────────────────────────────────────────────────

write("state_transition_label", diagram("""
[*] --> A
A --> B : event
B --> [*]
"""))

write("state_transition_guard", diagram("""
[*] --> A
A --> B : [condition]
B --> [*]
"""))

write("state_transition_action", diagram("""
[*] --> A
A --> B : / action
B --> [*]
"""))

write("state_transition_label_guard", diagram("""
[*] --> A
A --> B : event [guard]
B --> [*]
"""))

write("state_transition_label_action", diagram("""
[*] --> A
A --> B : event / action
B --> [*]
"""))

write("state_transition_guard_action", diagram("""
[*] --> A
A --> B : [guard] / action
B --> [*]
"""))

write("state_transition_full", diagram("""
[*] --> A
A --> B : event [guard] / action
B --> [*]
"""))

write("state_self_transition", diagram("""
[*] --> A
A --> A : self
A --> [*]
"""))

write("state_self_transition_labeled", diagram("""
[*] --> A
A --> A : retry [n < 3] / n++
A --> [*]
"""))

write("state_multiple_self_transitions", diagram("""
[*] --> A
A --> A : event1
A --> A : event2
A --> A : event3
A --> [*]
"""))

write("state_multiple_transitions_between_same", diagram("""
[*] --> A
A --> B : event1
A --> B : event2
A --> B : event3
B --> [*]
"""))

# Arrow directions
for direction in ["up", "down", "left", "right"]:
    write(f"state_arrow_{direction}", diagram(f"""
[*] --> A
A -{direction}-> B
B --> [*]
"""))

# Arrow lengths
for length, dashes in [("short", "-"), ("medium", "--"), ("long", "---")]:
    write(f"state_arrow_length_{length}", diagram(f"""
[*] --> A
A {dashes}> B
B --> [*]
"""))

# Arrow colors
for color in ["red", "blue", "green", "orange", "purple", "#FF00FF"]:
    safe = color.replace("#", "hash")
    write(f"state_arrow_color_{safe}", diagram(f"""
[*] --> A
A -[#{color.lstrip('#')}]-> B
B --> [*]
"""))

# Dotted arrows
write("state_arrow_dotted", diagram("""
[*] --> A
A ..> B
B --> [*]
"""))

write("state_arrow_dotted_labeled", diagram("""
[*] --> A
A ..> B : dotted transition
B --> [*]
"""))

write("state_arrow_dashed_labeled", diagram("""
[*] --> A
A --> B : label
B ..> C : dotted label
C --> [*]
"""))

# Long labels
write("state_long_label", diagram("""
[*] --> A
A --> B : This is a very long transition label that describes what happens in detail
B --> [*]
"""))

write("state_multiword_label", diagram("""
[*] --> A
A --> B : event received / update state and notify observer
B --> [*]
"""))

# Many transitions
write("state_fan_out", diagram("""
[*] --> A
A --> B : b
A --> C : c
A --> D : d
A --> E : e
B --> [*]
C --> [*]
D --> [*]
E --> [*]
"""))

write("state_fan_in", diagram("""
[*] --> A
[*] --> B
[*] --> C
[*] --> D
A --> E
B --> E
C --> E
D --> E
E --> [*]
"""))

# ─── 3. COMPOSITE / NESTED STATES ─────────────────────────────────────────────

write("state_composite_basic", diagram("""
[*] --> Outer
state Outer {
  [*] --> Inner
  Inner --> [*]
}
Outer --> [*]
"""))

write("state_composite_two_inner", diagram("""
[*] --> Outer
state Outer {
  [*] --> A
  A --> B
  B --> [*]
}
Outer --> [*]
"""))

write("state_composite_nested_2", diagram("""
[*] --> L1
state L1 {
  [*] --> L2
  state L2 {
    [*] --> S
    S --> [*]
  }
  L2 --> [*]
}
L1 --> [*]
"""))

write("state_composite_nested_3", diagram("""
[*] --> L1
state L1 {
  [*] --> L2
  state L2 {
    [*] --> L3
    state L3 {
      [*] --> S
      S --> [*]
    }
    L3 --> [*]
  }
  L2 --> [*]
}
L1 --> [*]
"""))

write("state_composite_nested_4", diagram("""
[*] --> L1
state L1 {
  [*] --> L2
  state L2 {
    [*] --> L3
    state L3 {
      [*] --> L4
      state L4 {
        [*] --> S
        S --> [*]
      }
      L4 --> [*]
    }
    L3 --> [*]
  }
  L2 --> [*]
}
L1 --> [*]
"""))

write("state_composite_nested_5", diagram("""
[*] --> L1
state L1 {
  [*] --> L2
  state L2 {
    [*] --> L3
    state L3 {
      [*] --> L4
      state L4 {
        [*] --> L5
        state L5 {
          [*] --> S
          S --> [*]
        }
        L5 --> [*]
      }
      L4 --> [*]
    }
    L3 --> [*]
  }
  L2 --> [*]
}
L1 --> [*]
"""))

write("state_composite_multiple_siblings", diagram("""
[*] --> A
state A {
  [*] --> A1
  A1 --> [*]
}
state B {
  [*] --> B1
  B1 --> [*]
}
A --> B
B --> [*]
"""))

write("state_composite_three_siblings", diagram("""
[*] --> A
state A {
  [*] --> A1
  A1 --> A2
  A2 --> [*]
}
state B {
  [*] --> B1
  B1 --> [*]
}
state C {
  [*] --> C1
  C1 --> C2
  C2 --> C3
  C3 --> [*]
}
A --> B
B --> C
C --> [*]
"""))

write("state_composite_transition_into", diagram("""
[*] --> Outer
Outer --> Inner
state Outer {
  [*] --> S1
  S1 --> [*]
  state Inner {
    [*] --> S2
    S2 --> [*]
  }
}
"""))

write("state_composite_named_alias", diagram("""
state "Composite State" as CS {
  [*] --> s1
  s1 --> s2
  s2 --> [*]
}
[*] --> CS
CS --> [*]
"""))

# Entry/exit points
write("state_entry_point", diagram("""
state S1 <<entryPoint>>
[*] --> S1
S1 --> A
A --> [*]
"""))

write("state_exit_point", diagram("""
state S1 <<exitPoint>>
A --> S1
S1 --> [*]
[*] --> A
"""))

write("state_entry_exit_points", diagram("""
state S1 <<entryPoint>>
state S2 <<exitPoint>>
[*] --> S1
S1 --> A
A --> B
B --> S2
S2 --> [*]
"""))

write("state_input_pin", diagram("""
state pin1 <<inputPin>>
[*] --> pin1
pin1 --> A
A --> [*]
"""))

write("state_output_pin", diagram("""
state pin1 <<outputPin>>
A --> pin1
pin1 --> [*]
[*] --> A
"""))

write("state_expansion_input", diagram("""
state exp1 <<expansionInput>>
[*] --> exp1
exp1 --> A
A --> [*]
"""))

write("state_expansion_output", diagram("""
state exp1 <<expansionOutput>>
A --> exp1
exp1 --> [*]
[*] --> A
"""))

write("state_entry_exit_in_composite", diagram("""
state Composite {
  state entry1 <<entryPoint>>
  state exit1 <<exitPoint>>
  entry1 --> Inner
  Inner --> exit1
}
[*] --> Composite
Composite --> [*]
"""))

# ─── 4. CONCURRENT / ORTHOGONAL STATES ────────────────────────────────────────

write("state_concurrent_2_regions", diagram("""
state Concurrent {
  [*] --> A
  A --> [*]
  --
  [*] --> B
  B --> [*]
}
[*] --> Concurrent
Concurrent --> [*]
"""))

write("state_concurrent_3_regions", diagram("""
state Concurrent {
  [*] --> A
  A --> [*]
  --
  [*] --> B
  B --> [*]
  --
  [*] --> C
  C --> [*]
}
[*] --> Concurrent
Concurrent --> [*]
"""))

write("state_concurrent_4_regions", diagram("""
state Concurrent {
  [*] --> A
  A --> [*]
  --
  [*] --> B
  B --> [*]
  --
  [*] --> C
  C --> [*]
  --
  [*] --> D
  D --> [*]
}
[*] --> Concurrent
Concurrent --> [*]
"""))

write("state_concurrent_with_transitions", diagram("""
state Fork {
  [*] --> A1
  A1 --> A2
  A2 --> [*]
  --
  [*] --> B1
  B1 --> B2
  B2 --> [*]
}
[*] --> Fork
Fork --> [*]
"""))

write("state_concurrent_labeled_regions", diagram("""
state Concurrent {
  state "Region 1" as R1 {
    [*] --> R1A
    R1A --> [*]
  }
  --
  state "Region 2" as R2 {
    [*] --> R2A
    R2A --> [*]
  }
}
[*] --> Concurrent
Concurrent --> [*]
"""))

write("state_concurrent_nested", diagram("""
state Outer {
  state Inner {
    [*] --> X
    X --> [*]
    --
    [*] --> Y
    Y --> [*]
  }
  [*] --> Inner
  Inner --> [*]
  --
  [*] --> Z
  Z --> [*]
}
[*] --> Outer
Outer --> [*]
"""))

write("state_concurrent_complex", diagram("""
state Processing {
  [*] --> Parsing
  Parsing --> Validated
  Validated --> [*]
  --
  [*] --> Logging
  Logging --> LogDone
  LogDone --> [*]
  --
  [*] --> Auditing
  Auditing --> AuditDone
  AuditDone --> [*]
}
[*] --> Processing
Processing --> Done
Done --> [*]
"""))

# ─── 5. PSEUDO-STATES ─────────────────────────────────────────────────────────

write("state_fork_join", diagram("""
state fork1 <<fork>>
state join1 <<join>>

[*] --> fork1
fork1 --> A
fork1 --> B
A --> join1
B --> join1
join1 --> [*]
"""))

write("state_fork_three_way", diagram("""
state fork1 <<fork>>
state join1 <<join>>

[*] --> fork1
fork1 --> A
fork1 --> B
fork1 --> C
A --> join1
B --> join1
C --> join1
join1 --> [*]
"""))

write("state_fork_join_with_labels", diagram("""
state fork1 <<fork>>
state join1 <<join>>

[*] --> fork1
fork1 --> A : branch A
fork1 --> B : branch B
A --> join1 : done A
B --> join1 : done B
join1 --> [*]
"""))

write("state_choice", diagram("""
state choice1 <<choice>>

[*] --> A
A --> choice1
choice1 --> B : [x > 0]
choice1 --> C : [x <= 0]
B --> [*]
C --> [*]
"""))

write("state_choice_three_way", diagram("""
state choice1 <<choice>>

[*] --> A
A --> choice1
choice1 --> B : [low]
choice1 --> C : [medium]
choice1 --> D : [high]
B --> [*]
C --> [*]
D --> [*]
"""))

write("state_choice_nested", diagram("""
state c1 <<choice>>
state c2 <<choice>>

[*] --> Start
Start --> c1
c1 --> c2 : [branch1]
c1 --> End1 : [branch2]
c2 --> End2 : [subbranch1]
c2 --> End3 : [subbranch2]
End1 --> [*]
End2 --> [*]
End3 --> [*]
"""))

write("state_history", diagram("""
state Composite {
  state H <<history>>
  [*] --> H
  H --> A
  A --> B
  B --> A
}
[*] --> Composite
Composite --> [*]
"""))

write("state_deep_history", diagram("""
state Composite {
  state H* <<deepHistory>>
  [*] --> H*
  H* --> A
  A --> B
  B --> A
}
[*] --> Composite
Composite --> [*]
"""))

write("state_history_shallow", diagram("""
state S {
  [*] --> s1
  s1 --> s2 : e1
  s2 --> s3 : e2
  state H <<history>>
}
[*] --> S
S --> OutState : pause
OutState --> S.H : resume
"""))

write("state_end_pseudostate", diagram("""
state end1 <<end>>
[*] --> A
A --> B : normal
A --> end1 : terminate
B --> [*]
"""))

# ─── 6. NOTES ─────────────────────────────────────────────────────────────────

write("state_note_left", diagram("""
[*] --> A
A --> B
note left of A : This is a note\non the left
B --> [*]
"""))

write("state_note_right", diagram("""
[*] --> A
A --> B
note right of A : This is a note\non the right
B --> [*]
"""))

write("state_note_on_link", diagram("""
[*] --> A
A --> B
note on link
  This note is on the link
end note
B --> [*]
"""))

write("state_note_multiline", diagram("""
[*] --> A
note right of A
  This is a multi-line note
  with several lines
  of text content
end note
A --> [*]
"""))

write("state_note_floating", diagram("""
note "This is a floating note" as FN
[*] --> A
A --> [*]
"""))

write("state_multiple_notes", diagram("""
[*] --> A
note left of A : Note on A
A --> B : transition
note on link
  Note on transition
end note
note right of B : Note on B
B --> [*]
"""))

write("state_note_html", diagram("""
[*] --> A
note right of A
  <b>Bold</b> text
  <i>Italic</i> text
  <u>Underline</u>
end note
A --> [*]
"""))

write("state_note_composite", diagram("""
state Composite {
  [*] --> Inner
  Inner --> [*]
}
note right of Composite : Note on composite state
[*] --> Composite
Composite --> [*]
"""))

# ─── 7. STATE COLORS AND STYLING ──────────────────────────────────────────────

colors = ["red", "blue", "green", "yellow", "orange", "purple", "pink", "cyan", "lime", "silver"]

for color in colors:
    write(f"state_color_{color}", diagram(f"""
state A ##{color}
[*] --> A
A --> [*]
"""))

for color in colors:
    write(f"state_bg_color_{color}", diagram(f"""
state A #{color}
[*] --> A
A --> [*]
"""))

write("state_color_hex", diagram("""
state A #FF5733
state B #33FF57
state C #3357FF
[*] --> A
A --> B
B --> C
C --> [*]
"""))

write("state_color_gradient", diagram("""
state A #red/blue
[*] --> A
A --> [*]
"""))

write("state_color_line", diagram("""
state A ##red
[*] --> A
A --> [*]
"""))

write("state_color_line_bold", diagram("""
state A ##[bold]blue
[*] --> A
A --> [*]
"""))

write("state_color_line_dashed", diagram("""
state A ##[dashed]red
[*] --> A
A --> [*]
"""))

write("state_color_line_dotted", diagram("""
state A ##[dotted]green
[*] --> A
A --> [*]
"""))

write("state_multiple_colors", diagram("""
state A #red
state B #blue
state C #green
[*] --> A
A --> B
B --> C
C --> [*]
"""))

# Skinparams
write("state_skinparam_basic", diagram("""
[*] --> A
A --> B
B --> [*]
""", skinparams="""skinparam state {
  BackgroundColor LightBlue
  BorderColor DarkBlue
  FontSize 14
}"""))

write("state_skinparam_start_end", diagram("""
[*] --> A
A --> [*]
""", skinparams="""skinparam state {
  StartColor Green
  EndColor Red
}"""))

write("state_skinparam_composite", diagram("""
state Composite {
  [*] --> Inner
  Inner --> [*]
}
[*] --> Composite
Composite --> [*]
""", skinparams="""skinparam state {
  BackgroundColor<<composite>> LightYellow
  BorderColor<<composite>> Orange
}"""))

write("state_hide_empty_description", diagram("""
hide empty description
[*] --> A
A --> B
B --> [*]
"""))

write("state_skinparam_arrow_color", diagram("""
[*] --> A
A --> B
B --> [*]
""", skinparams="skinparam ArrowColor Red"))

write("state_skinparam_font", diagram("""
[*] --> A
A --> B
B --> [*]
""", skinparams="""skinparam state {
  FontName Courier
  FontColor Navy
}"""))

# Stereotypes
write("state_stereotype", diagram("""
state A <<myStereotype>>
[*] --> A
A --> [*]
"""))

write("state_multiple_stereotypes", diagram("""
state A <<stereo1>>
state B <<stereo2>>
state C <<stereo3>>
[*] --> A
A --> B
B --> C
C --> [*]
"""))

write("state_stereotype_with_skinparam", diagram("""
state A <<important>>
[*] --> A
A --> [*]
""", skinparams="""skinparam state<<important>> {
  BackgroundColor OrangeRed
  FontColor White
}"""))

# ─── 8. COMPLEX WORKFLOWS ────────────────────────────────────────────────────

write("state_traffic_light", diagram("""
[*] --> Red
Red --> Green : timer
Green --> Yellow : timer
Yellow --> Red : timer
"""))

write("state_vending_machine", diagram("""
[*] --> Idle
Idle --> HasMoney : insert coin
HasMoney --> HasMoney : insert coin [need more]
HasMoney --> Dispensing : select item [enough money]
HasMoney --> Idle : cancel / return money
Dispensing --> Idle : item dispensed
"""))

write("state_tcp_connection", diagram("""
state "CLOSED" as CLOSED
state "LISTEN" as LISTEN
state "SYN_SENT" as SYN_SENT
state "SYN_RECEIVED" as SYN_RCVD
state "ESTABLISHED" as EST
state "FIN_WAIT_1" as FW1
state "FIN_WAIT_2" as FW2
state "TIME_WAIT" as TW
state "CLOSE_WAIT" as CW
state "LAST_ACK" as LA

[*] --> CLOSED
CLOSED --> LISTEN : passive open
CLOSED --> SYN_SENT : active open / send SYN
LISTEN --> SYN_RCVD : receive SYN / send SYN+ACK
SYN_SENT --> EST : receive SYN+ACK / send ACK
SYN_RCVD --> EST : receive ACK
EST --> FW1 : close / send FIN
EST --> CW : receive FIN / send ACK
FW1 --> FW2 : receive ACK
FW1 --> TW : receive FIN+ACK / send ACK
FW2 --> TW : receive FIN / send ACK
TW --> CLOSED : timeout
CW --> LA : close / send FIN
LA --> CLOSED : receive ACK
"""))

write("state_order_lifecycle", diagram("""
[*] --> Created
Created --> Pending : submit
Pending --> Approved : approve
Pending --> Rejected : reject
Approved --> Processing : start
Processing --> Shipped : ship
Shipped --> Delivered : deliver
Delivered --> [*]
Rejected --> [*]
Approved --> Cancelled : cancel
Processing --> Cancelled : cancel
Cancelled --> [*]
"""))

write("state_atm_machine", diagram("""
[*] --> Idle
Idle --> CardInserted : insert card
CardInserted --> PINEntry : card valid
CardInserted --> Idle : card invalid / eject
PINEntry --> Authenticated : PIN correct
PINEntry --> CardInserted : PIN wrong [attempts < 3]
PINEntry --> CardBlocked : PIN wrong [attempts >= 3]
Authenticated --> MenuShown
MenuShown --> WithdrawAmount : select withdraw
MenuShown --> CheckBalance : select balance
MenuShown --> Idle : cancel / eject card
WithdrawAmount --> Dispensing : amount confirmed
Dispensing --> Idle : cash taken / eject card
CheckBalance --> MenuShown : ok
CardBlocked --> Idle : card confiscated
"""))

write("state_user_session", diagram("""
[*] --> LoggedOut
LoggedOut --> LoggingIn : login attempt
LoggingIn --> LoggedIn : credentials valid
LoggingIn --> LoggedOut : credentials invalid
LoggedIn --> Active : user action
Active --> Idle : no action [timeout < 30min]
Idle --> LoggedOut : no action [timeout >= 30min]
Idle --> Active : user action
Active --> LoggedOut : logout
LoggedIn --> LoggedOut : logout
"""))

write("state_document_workflow", diagram("""
[*] --> Draft
Draft --> Review : submit for review
Review --> Draft : needs revision
Review --> Approved : approve
Review --> Rejected : reject
Approved --> Published : publish
Published --> Archived : archive
Rejected --> [*]
Archived --> [*]
"""))

# ─── 9. UNICODE AND SPECIAL CHARACTERS ────────────────────────────────────────

write("state_unicode_names", diagram("""
state "État Initial" as EI
state "Traitement" as TR
state "Résultat Final" as RF

[*] --> EI
EI --> TR : démarrer
TR --> RF : terminer
RF --> [*]
"""))

write("state_unicode_labels", diagram("""
[*] --> A
A --> B : événement reçu
B --> C : données traitées
C --> [*] : terminé
"""))

write("state_unicode_description", diagram("""
state A : données en cours de traitement
state B : résultat disponible
[*] --> A
A --> B
B --> [*]
"""))

write("state_special_chars_underscore", diagram("""
state my_state_1
state my_state_2
[*] --> my_state_1
my_state_1 --> my_state_2
my_state_2 --> [*]
"""))

write("state_numbers_in_names", diagram("""
state state1
state state2
state state3
[*] --> state1
state1 --> state2
state2 --> state3
state3 --> [*]
"""))

# ─── 10. NEWPAGE ──────────────────────────────────────────────────────────────

write("state_newpage_basic", diagram("""
[*] --> A
A --> B
newpage
B --> C
C --> [*]
"""))

write("state_newpage_titled", diagram("""
[*] --> A
A --> B
newpage Second Page
B --> C
C --> [*]
"""))

write("state_newpage_multiple", diagram("""
[*] --> A
A --> B
newpage
B --> C
C --> D
newpage
D --> E
E --> [*]
"""))

# ─── 11. LARGE DIAGRAMS ───────────────────────────────────────────────────────

def make_large_linear(n):
    lines = ["[*] --> S1"]
    for i in range(1, n):
        lines.append(f"S{i} --> S{i+1}")
    lines.append(f"S{n} --> [*]")
    return "\n".join(lines)

write("state_large_20_states", diagram(make_large_linear(20)))
write("state_large_30_states", diagram(make_large_linear(30)))

def make_large_web(n):
    import random
    random.seed(42)
    lines = []
    lines.append("[*] --> S1")
    for i in range(1, n + 1):
        j = random.randint(1, n)
        if j != i:
            lines.append(f"S{i} --> S{j} : e{i}")
    lines.append(f"S{n} --> [*]")
    return "\n".join(lines)

write("state_large_web_20", diagram(make_large_web(20)))

def make_parallel_chains(n_chains, chain_len):
    lines = ["state fork1 <<fork>>", "state join1 <<join>>", "[*] --> fork1"]
    for c in range(1, n_chains + 1):
        prev = "fork1"
        for s in range(1, chain_len + 1):
            name = f"C{c}S{s}"
            if prev == "fork1":
                lines.append(f"fork1 --> {name}")
            else:
                lines.append(f"{prev} --> {name}")
            prev = name
        lines.append(f"{prev} --> join1")
    lines.append("join1 --> [*]")
    return "\n".join(lines)

write("state_parallel_3_chains_3", diagram(make_parallel_chains(3, 3)))
write("state_parallel_4_chains_4", diagram(make_parallel_chains(4, 4)))
write("state_parallel_5_chains_2", diagram(make_parallel_chains(5, 2)))

# ─── 12. COMPOSITE WITH CONCURRENT REGIONS ────────────────────────────────────

write("state_composite_concurrent", diagram("""
state Composite {
  [*] --> A
  A --> B
  B --> [*]
  --
  [*] --> C
  C --> D
  D --> [*]
}
[*] --> Composite
Composite --> [*]
"""))

write("state_composite_concurrent_three", diagram("""
state Composite {
  [*] --> A1
  A1 --> A2
  A2 --> [*]
  --
  [*] --> B1
  B1 --> B2
  B2 --> [*]
  --
  [*] --> C1
  C1 --> C2
  C2 --> [*]
}
[*] --> Composite
Composite --> [*]
"""))

write("state_nested_concurrent", diagram("""
state Outer {
  state Inner1 {
    [*] --> X
    X --> [*]
    --
    [*] --> Y
    Y --> [*]
  }
  state Inner2 {
    [*] --> P
    P --> Q
    Q --> [*]
    --
    [*] --> R
    R --> [*]
  }
  [*] --> Inner1
  Inner1 --> Inner2
  Inner2 --> [*]
}
[*] --> Outer
Outer --> [*]
"""))

write("state_concurrent_with_history", diagram("""
state Composite {
  state H <<history>>
  [*] --> H
  H --> Active
  Active --> Paused
  Paused --> Active
  --
  [*] --> Logging
  Logging --> [*]
}
[*] --> Composite
Composite --> Done
Done --> [*]
"""))

# ─── 13. MULTIPLE STATE MACHINES ──────────────────────────────────────────────

write("state_two_machines", diagram("""
state Machine1 {
  [*] --> A
  A --> B
  B --> [*]
}

state Machine2 {
  [*] --> X
  X --> Y
  Y --> [*]
}

[*] --> Machine1
Machine1 --> Machine2
Machine2 --> [*]
"""))

write("state_three_machines", diagram("""
state Machine1 {
  [*] --> A
  A --> [*]
}

state Machine2 {
  [*] --> B
  B --> [*]
}

state Machine3 {
  [*] --> C
  C --> [*]
}

[*] --> Machine1
Machine1 --> Machine2
Machine2 --> Machine3
Machine3 --> [*]
"""))

# ─── 14. TRANSITIONS ACROSS COMPOSITE BOUNDARIES ─────────────────────────────

write("state_cross_boundary_in", diagram("""
state Composite {
  [*] --> Inner
  Inner --> Deep
  Deep --> [*]
}
[*] --> Composite
[*] --> Composite.Inner
"""))

write("state_cross_boundary_out", diagram("""
state Composite {
  [*] --> Inner
  Inner --> [*]
}
[*] --> Composite
Composite --> Outside
Outside --> [*]
"""))

# ─── 15. COMBINATORIAL VARIANTS ───────────────────────────────────────────────

# Transitions with all combinations of label/guard/action
label_variants = [
    ("none", ""),
    ("event", "click"),
    ("guard", "[x > 0]"),
    ("action", "/ doSomething()"),
    ("event_guard", "click [x > 0]"),
    ("event_action", "click / doSomething()"),
    ("guard_action", "[x > 0] / doSomething()"),
    ("full", "click [x > 0] / doSomething()"),
]

for name, label in label_variants:
    sep = " : " if label else ""
    write(f"state_transition_variant_{name}", diagram(f"""
[*] --> A
A --> B{sep}{label}
B --> [*]
"""))

# Arrow direction x length
for direction in ["up", "down", "left", "right"]:
    for length in ["-", "--", "---"]:
        dashes = length
        write(f"state_arrow_{direction}_{len(dashes)}", diagram(f"""
[*] --> A
A -{direction}{dashes}> B
B --> [*]
"""))

# Color on states x shapes
state_shapes = [
    ("normal", "state A"),
    ("entry", "state A <<entryPoint>>"),
    ("exit", "state A <<exitPoint>>"),
    ("choice", "state A <<choice>>"),
    ("fork", "state A <<fork>>"),
    ("join", "state A <<join>>"),
]

bg_colors = ["red", "blue", "green", "yellow"]
for shape_name, shape_def in state_shapes:
    for color in bg_colors:
        write(f"state_shape_{shape_name}_color_{color}", diagram(f"""
{shape_def} #{color}
[*] --> A
A --> [*]
"""))

# Note positions x state positions
for note_pos in ["left", "right"]:
    for n_states in [1, 2, 3]:
        states = " --> ".join([f"S{i}" for i in range(1, n_states + 1)])
        write(f"state_note_{note_pos}_{n_states}states", diagram(f"""
[*] --> S1
{states}
S{n_states} --> [*]
note {note_pos} of S1 : Note on S1
"""))

# Skinparam combinations
skinparam_combos = [
    ("bgcolor", "skinparam backgroundColor #F0F0F0"),
    ("handwritten", "skinparam handwritten true"),
    ("monochrome", "skinparam monochrome true"),
    ("shadowing", "skinparam shadowing true"),
    ("no_shadowing", "skinparam shadowing false"),
    ("roundcorner", "skinparam roundCorner 15"),
    ("linetype_ortho", "skinparam linetype ortho"),
    ("linetype_polyline", "skinparam linetype polyline"),
    ("default_font", "skinparam defaultFontSize 16"),
    ("default_font_name", "skinparam defaultFontName Courier"),
]

for name, sp in skinparam_combos:
    write(f"state_skinparam_{name}", diagram("""
[*] --> A
A --> B
B --> [*]
""", skinparams=sp))

# Composite nesting depth x number of inner states
for depth in [1, 2, 3]:
    for inner_count in [1, 2, 3]:
        def make_nested(d, inner):
            if d == 0:
                states = "\n".join([f"  [*] --> I1"] +
                                   [f"  I{i} --> I{i+1}" for i in range(1, inner)] +
                                   [f"  I{inner} --> [*]"])
                return states
            inner_body = make_nested(d - 1, inner)
            return f"  state Level{d} {{\n" + "\n".join("  " + l for l in inner_body.split("\n")) + "\n  }\n  [*] --> Level{d}\n  Level{d} --> [*]"

        body = f"state Outer {{\n{make_nested(depth, inner_count)}\n}}\n[*] --> Outer\nOuter --> [*]"
        write(f"state_nested_depth{depth}_inner{inner_count}", diagram(body))

# ─── 16. CONCURRENT REGION COUNTS ────────────────────────────────────────────

for region_count in range(2, 7):
    regions = []
    for i in range(region_count):
        regions.append(f"  [*] --> R{i}A\n  R{i}A --> R{i}B\n  R{i}B --> [*]")
    sep = "\n  --\n"
    body = f"state Concurrent {{\n{sep.join(regions)}\n}}\n[*] --> Concurrent\nConcurrent --> [*]"
    write(f"state_concurrent_{region_count}_regions", diagram(body))

# ─── 17. FORK/JOIN VARIANT COUNTS ────────────────────────────────────────────

for branch_count in range(2, 8):
    lines = [f"state fork1 <<fork>>", f"state join1 <<join>>", "[*] --> fork1"]
    for i in range(branch_count):
        lines.append(f"fork1 --> Branch{i}")
        lines.append(f"Branch{i} --> join1")
    lines.append("join1 --> [*]")
    write(f"state_fork_join_{branch_count}_branches", diagram("\n".join(lines)))

# ─── 18. CHOICE CHAIN ────────────────────────────────────────────────────────

write("state_choice_chain", diagram("""
state c1 <<choice>>
state c2 <<choice>>
state c3 <<choice>>

[*] --> Start
Start --> c1
c1 --> c2 : [path1]
c1 --> End1 : [else]
c2 --> c3 : [path2]
c2 --> End2 : [else]
c3 --> End3 : [path3]
c3 --> End4 : [else]
End1 --> [*]
End2 --> [*]
End3 --> [*]
End4 --> [*]
"""))

# ─── 19. HISTORY STATE VARIANTS ──────────────────────────────────────────────

for depth in [1, 2, 3]:
    lines = []
    for d in range(depth):
        prefix = "  " * d
        if d == 0:
            lines.append(f"state Outer {{")
        else:
            lines.append(f"{prefix}state Level{d} {{")
    # innermost
    inner_prefix = "  " * depth
    lines.append(f"{inner_prefix}state H <<history>>")
    lines.append(f"{inner_prefix}[*] --> H")
    lines.append(f"{inner_prefix}H --> X")
    lines.append(f"{inner_prefix}X --> Y")
    lines.append(f"{inner_prefix}Y --> X")
    for d in range(depth, 0, -1):
        prefix = "  " * (d - 1)
        lines.append(f"{prefix}}}")
        if d > 1:
            lines.append(f"{prefix}[*] --> Level{d-1}")
            lines.append(f"{prefix}Level{d-1} --> [*]")
    lines.append("[*] --> Outer")
    lines.append("Outer --> [*]")
    write(f"state_history_depth_{depth}", diagram("\n".join(lines)))

# Deep history variants
for depth in [1, 2]:
    lines = []
    for d in range(depth):
        prefix = "  " * d
        if d == 0:
            lines.append(f"state Outer {{")
        else:
            lines.append(f"{prefix}state Level{d} {{")
    inner_prefix = "  " * depth
    lines.append(f"{inner_prefix}state H* <<deepHistory>>")
    lines.append(f"{inner_prefix}[*] --> H*")
    lines.append(f"{inner_prefix}H* --> X")
    lines.append(f"{inner_prefix}X --> Y")
    lines.append(f"{inner_prefix}Y --> X")
    for d in range(depth, 0, -1):
        prefix = "  " * (d - 1)
        lines.append(f"{prefix}}}")
        if d > 1:
            lines.append(f"{prefix}[*] --> Level{d-1}")
            lines.append(f"{prefix}Level{d-1} --> [*]")
    lines.append("[*] --> Outer")
    lines.append("Outer --> [*]")
    write(f"state_deep_history_depth_{depth}", diagram("\n".join(lines)))

# ─── 20. STATE DESCRIPTION VARIANTS ──────────────────────────────────────────

write("state_description_single", diagram("""
state A : single line description
[*] --> A
A --> [*]
"""))

write("state_description_two_lines", diagram("""
state A : first line
state A : second line
[*] --> A
A --> [*]
"""))

write("state_description_three_lines", diagram("""
state A : first line
state A : second line
state A : third line
[*] --> A
A --> [*]
"""))

write("state_description_multiple_states", diagram("""
state A : description of A
state B : description of B
state C : description of C
[*] --> A
A --> B
B --> C
C --> [*]
"""))

write("state_description_long", diagram("""
state A : This is a very long description that goes into detail about what this state represents in the system
[*] --> A
A --> [*]
"""))

write("state_description_with_slashes", diagram("""
state A : entry / do something
state A : do / main activity
state A : exit / cleanup
[*] --> A
A --> [*]
"""))

# ─── 21. TRANSITION DIRECTION + COLOR COMBINATIONS ───────────────────────────

for direction in ["up", "down", "left", "right"]:
    for color in ["red", "blue", "green"]:
        write(f"state_arrow_{direction}_color_{color}", diagram(f"""
[*] --> A
A -{direction}[#{color}]-> B
B --> [*]
"""))

# ─── 22. ENTRY/EXIT POINTS IN VARIOUS CONTEXTS ───────────────────────────────

write("state_entry_in_composite", diagram("""
state Composite {
  state ep <<entryPoint>>
  [*] --> ep
  ep --> A
  A --> B
  B --> [*]
}
[*] --> Composite
Composite --> [*]
"""))

write("state_exit_in_composite", diagram("""
state Composite {
  state xp <<exitPoint>>
  [*] --> A
  A --> B
  B --> xp
  xp --> [*]
}
[*] --> Composite
Composite --> [*]
"""))

write("state_entry_exit_both", diagram("""
state Composite {
  state ep <<entryPoint>>
  state xp <<exitPoint>>
  [*] --> ep
  ep --> A
  A --> B
  B --> xp
}
[*] --> Composite
Composite --> [*]
"""))

write("state_multiple_entry_points", diagram("""
state Composite {
  state ep1 <<entryPoint>>
  state ep2 <<entryPoint>>
  ep1 --> A
  ep2 --> B
  A --> [*]
  B --> [*]
}
[*] --> Composite
"""))

write("state_multiple_exit_points", diagram("""
state Composite {
  state xp1 <<exitPoint>>
  state xp2 <<exitPoint>>
  [*] --> A
  A --> xp1 : success
  A --> xp2 : failure
}
[*] --> Composite
"""))

# ─── 23. VERY LONG STATE NAMES ────────────────────────────────────────────────

write("state_very_long_names", diagram("""
state "This is a very long state name with many words" as VLONG1
state "Another very long state name for testing purposes" as VLONG2
[*] --> VLONG1
VLONG1 --> VLONG2 : some event with a long label too
VLONG2 --> [*]
"""))

write("state_long_description_long_name", diagram("""
state "Very Long State Name Here" as VS : And this is a very long description too
[*] --> VS
VS --> [*]
"""))

# ─── 24. MIXED FEATURES ──────────────────────────────────────────────────────

write("state_mixed_all_features", diagram("""
skinparam state {
  BackgroundColor LightYellow
  BorderColor DarkOrange
}

state "Start Processing" as SP
state "Error State" as ES #red
state choice1 <<choice>>
state fork1 <<fork>>
state join1 <<join>>

note "Complex diagram" as N1

[*] --> SP
SP --> choice1 : analyze
choice1 --> fork1 : [complex]
choice1 --> Simple : [simple]

fork1 --> Branch1 : fork
fork1 --> Branch2 : fork
Branch1 --> join1
Branch2 --> join1
join1 --> Done

Simple --> Done : simplified

SP --> ES : error
ES --> SP : retry [count < 3]
ES --> [*] : give up [count >= 3]

note right of ES
  Error handling state
  with multi-line note
end note

Done --> [*]
"""))

write("state_mixed_concurrent_composite", diagram("""
state "Outer State" as OS {
  state "Region A" {
    [*] --> Idle
    Idle --> Active : start
    Active --> Idle : stop
    --
    [*] --> Monitoring
    Monitoring --> Alert : threshold exceeded
    Alert --> Monitoring : resolved
  }
  [*] --> Region A
}
[*] --> OS
OS --> [*]
"""))

write("state_mixed_history_concurrent", diagram("""
state Application {
  state UI {
    state H <<history>>
    [*] --> H
    H --> Screen1
    Screen1 --> Screen2 : navigate
    Screen2 --> Screen1 : back
  }
  [*] --> UI
  --
  [*] --> Background
  Background --> [*]
}
[*] --> Application
Application --> [*]
"""))

# ─── 25. EDGE CASES ──────────────────────────────────────────────────────────

write("state_single_state_no_transitions", diagram("state A"))

write("state_only_initial_final", diagram("[*] --> [*]"))

write("state_cyclic", diagram("""
[*] --> A
A --> B
B --> C
C --> A
"""))

write("state_diamond", diagram("""
[*] --> A
A --> B
A --> C
B --> D
C --> D
D --> [*]
"""))

write("state_complex_cycles", diagram("""
[*] --> A
A --> B
B --> C
C --> A
C --> D
D --> B
D --> [*]
"""))

write("state_self_loops_multiple", diagram("""
[*] --> A
A --> A : event1
A --> A : event2 [guard1]
A --> A : event3 / action1
A --> A : event4 [guard2] / action2
A --> [*]
"""))

write("state_no_initial_state", diagram("""
A --> B
B --> C
C --> [*]
"""))

write("state_no_final_state", diagram("""
[*] --> A
A --> B
B --> C
"""))

write("state_isolated_states", diagram("""
state Isolated1
state Isolated2
[*] --> Connected
Connected --> [*]
"""))

# ─── 26. TITLE AND HEADER VARIANTS ────────────────────────────────────────────

write("state_with_title", diagram("[*] --> A\nA --> [*]", title="My State Diagram"))

write("state_with_long_title", diagram("[*] --> A\nA --> [*]",
                                        title="A Very Long Title For This State Diagram"))

write("state_title_and_skinparam", diagram("""
[*] --> A
A --> B
B --> [*]
""", title="Styled Diagram", skinparams="skinparam backgroundColor #FFFACD"))

# ─── 27. ADDITIONAL STEREOTYPE COMBINATIONS ──────────────────────────────────

pseudostates = [
    ("choice", "<<choice>>"),
    ("fork", "<<fork>>"),
    ("join", "<<join>>"),
    ("entryPoint", "<<entryPoint>>"),
    ("exitPoint", "<<exitPoint>>"),
    ("inputPin", "<<inputPin>>"),
    ("outputPin", "<<outputPin>>"),
    ("expansionInput", "<<expansionInput>>"),
    ("expansionOutput", "<<expansionOutput>>"),
    ("history", "<<history>>"),
    ("deepHistory", "<<deepHistory>>"),
    ("end", "<<end>>"),
]

for ps_name, stereotype in pseudostates:
    write(f"state_pseudostate_{ps_name}", diagram(f"""
state ps {stereotype}
[*] --> A
A --> ps
ps --> [*]
"""))

# ─── 28. MULTIPLE INITIAL/FINAL WITH LABELS ──────────────────────────────────

write("state_labeled_initial_transitions", diagram("""
[*] --> A : start event
[*] --> B : alternative start
A --> [*]
B --> [*]
"""))

write("state_labeled_final_transitions", diagram("""
[*] --> A
A --> [*] : success
A --> [*] : failure
"""))

# ─── 29. ANNOTATIONS AND EXTRA PUML SYNTAX ───────────────────────────────────

write("state_hide_empty_description_complex", diagram("""
hide empty description
state A : has description
state B
state C : also has description
state D

[*] --> A
A --> B
B --> C
C --> D
D --> [*]
"""))

write("state_show_transition_guards", diagram("""
[*] --> A
A --> B : [guard1]
A --> C : [guard2]
A --> D : [!guard1 && !guard2]
B --> [*]
C --> [*]
D --> [*]
"""))

# ─── 30. COMPOSITE WITH ENTRY/EXIT AND CONCURRENT ────────────────────────────

write("state_composite_entry_exit_concurrent", diagram("""
state Composite {
  state ep <<entryPoint>>
  state xp <<exitPoint>>
  ep --> R1A
  R1A --> R1B
  R1B --> xp
  --
  [*] --> R2A
  R2A --> R2B
  R2B --> [*]
}
[*] --> Composite
Composite --> [*]
"""))

# ─── 31. DOTTED + COLORED ARROWS IN VARIOUS CONTEXTS ─────────────────────────

write("state_dotted_colored", diagram("""
[*] --> A
A ..> B #red : dotted red
B --> C : normal
C ..> [*] #blue : dotted blue
"""))

write("state_mixed_arrow_styles", diagram("""
[*] --> A
A --> B
A ..> C
A -[#red]-> D
A -up-> E
B --> [*]
C --> [*]
D --> [*]
E --> [*]
"""))

# ─── 32. COMPREHENSIVE WORKFLOW DIAGRAMS ─────────────────────────────────────

write("state_bank_transfer", diagram("""
state "Transfer Initiated" as TI
state "Validating" as VAL
state "Fraud Check" as FC
state choice1 <<choice>>
state choice2 <<choice>>
state fork1 <<fork>>
state join1 <<join>>
state "Processing" as PROC
state "Completed" as COMP
state "Failed" as FAIL
state "Reversed" as REV

[*] --> TI
TI --> VAL : submit
VAL --> choice1 : validate
choice1 --> fork1 : [valid]
choice1 --> FAIL : [invalid]
fork1 --> FC
fork1 --> PROC
FC --> join1 : [approved]
FC --> FAIL : [fraud detected]
PROC --> join1
join1 --> COMP
COMP --> REV : reverse [within 24h]
REV --> [*]
COMP --> [*]
FAIL --> [*]
"""))

write("state_ci_cd_pipeline", diagram("""
[*] --> Triggered
Triggered --> Building
Building --> Testing : build success
Building --> Failed : build failure
Testing --> SecurityScan : tests pass
Testing --> Failed : tests fail
SecurityScan --> Staging : scan clean
SecurityScan --> Failed : vulnerabilities found
Staging --> Approval : deploy to staging
Approval --> Production : approved
Approval --> Failed : rejected
Production --> [*]
Failed --> [*]

note right of Staging : Automated smoke tests run here
"""))

write("state_game_character", diagram("""
state Idle
state Moving {
  [*] --> Walking
  Walking --> Running : sprint
  Running --> Walking : release sprint
}
state Jumping
state Attacking {
  [*] --> WindUp
  WindUp --> Strike
  Strike --> Recovery
  Recovery --> [*]
}
state Hurt
state Dead

[*] --> Idle
Idle --> Moving : move input
Idle --> Jumping : jump input
Idle --> Attacking : attack input
Moving --> Idle : no move input
Moving --> Jumping : jump input
Moving --> Attacking : attack input
Jumping --> Idle : land
Attacking --> Idle : attack complete
Idle --> Hurt : take damage [health > 0]
Moving --> Hurt : take damage [health > 0]
Jumping --> Hurt : take damage [health > 0]
Attacking --> Hurt : take damage [health > 0]
Hurt --> Idle : recover [health > 0]
Hurt --> Dead : take damage [health <= 0]
Dead --> [*]
"""))

# ─── 33. EXTRA COMBOS FOR COUNT ──────────────────────────────────────────────

# Colors x nesting
for color in ["LightBlue", "LightGreen", "LightYellow", "LightPink", "LightGray"]:
    write(f"state_composite_color_{color.lower()}", diagram(f"""
state Outer #{color} {{
  [*] --> Inner
  Inner --> [*]
}}
[*] --> Outer
Outer --> [*]
"""))

# Different numbers of states with descriptions
for n in range(1, 11):
    lines = []
    for i in range(1, n + 1):
        lines.append(f"state S{i} : State {i} description")
    lines.append("[*] --> S1")
    for i in range(1, n):
        lines.append(f"S{i} --> S{i+1}")
    lines.append(f"S{n} --> [*]")
    write(f"state_desc_{n}_states", diagram("\n".join(lines)))

# Notes on various state counts
for n in [1, 2, 3, 5, 7]:
    lines = ["[*] --> S1"]
    for i in range(1, n):
        lines.append(f"S{i} --> S{i+1}")
    lines.append(f"S{n} --> [*]")
    lines.append(f"note right of S1 : Note on first state")
    if n > 1:
        lines.append(f"note left of S{n} : Note on last state")
    write(f"state_notes_{n}_states", diagram("\n".join(lines)))

# Transition label lengths
short_labels = ["e", "go", "done", "click", "submit"]
long_labels = [
    "this is a medium label",
    "this is a longer transition label",
    "this is quite a long transition label indeed",
    "an extremely long transition label that goes on and on",
]
for i, label in enumerate(short_labels + long_labels):
    safe = f"len{i+1}"
    write(f"state_label_{safe}", diagram(f"""
[*] --> A
A --> B : {label}
B --> [*]
"""))

# Skinparam state colors
for bg in ["AliceBlue", "AntiqueWhite", "Aquamarine", "Azure", "Beige",
           "Bisque", "BlanchedAlmond", "Cornsilk", "FloralWhite", "GhostWhite"]:
    write(f"state_skinparam_bg_{bg.lower()}", diagram(f"""
[*] --> A
A --> B
B --> [*]
""", skinparams=f"skinparam state {{ BackgroundColor {bg} }}"))

# Arrow direction combos
for d1 in ["up", "down", "left", "right"]:
    for d2 in ["up", "down", "left", "right"]:
        if d1 != d2:
            write(f"state_arrows_{d1}_{d2}", diagram(f"""
[*] --> A
A -{d1}-> B
B -{d2}-> C
C --> [*]
"""))

# Many states with self-loops
for n in [3, 5, 7]:
    lines = ["[*] --> S1"]
    for i in range(1, n + 1):
        lines.append(f"S{i} --> S{i} : retry")
    for i in range(1, n):
        lines.append(f"S{i} --> S{i+1} : next")
    lines.append(f"S{n} --> [*]")
    write(f"state_self_loops_{n}_states", diagram("\n".join(lines)))

# Mixed pseudostates
write("state_fork_choice_join", diagram("""
state fork1 <<fork>>
state c1 <<choice>>
state join1 <<join>>

[*] --> fork1
fork1 --> A
fork1 --> B
A --> c1
c1 --> C : [cond]
c1 --> D : [!cond]
B --> join1
C --> join1
D --> join1
join1 --> [*]
"""))

write("state_choice_fork_join", diagram("""
state c1 <<choice>>
state fork1 <<fork>>
state join1 <<join>>

[*] --> Start
Start --> c1
c1 --> fork1 : [parallel]
c1 --> Sequential : [sequential]
fork1 --> A
fork1 --> B
A --> join1
B --> join1
join1 --> End
Sequential --> End
End --> [*]
"""))

# Concurrent + fork/join
write("state_concurrent_then_fork", diagram("""
state Concurrent {
  [*] --> A
  A --> [*]
  --
  [*] --> B
  B --> [*]
}

state fork1 <<fork>>
state join1 <<join>>

[*] --> Concurrent
Concurrent --> fork1
fork1 --> C
fork1 --> D
C --> join1
D --> join1
join1 --> [*]
"""))

# Deeply nested with descriptions
write("state_nested_descriptions", diagram("""
state "Outer Level" as OL {
  state "Middle Level" as ML : Middle description
  state "Inner Level" as IL : Inner description
  [*] --> ML
  ML --> IL
  IL --> [*]
}
[*] --> OL
OL --> [*]
"""))

# All pseudostate types in one diagram
write("state_all_pseudostates", diagram("""
state ep <<entryPoint>>
state xp <<exitPoint>>
state ip <<inputPin>>
state op <<outputPin>>
state ei <<expansionInput>>
state eo <<expansionOutput>>
state f <<fork>>
state j <<join>>
state c <<choice>>
state H <<history>>

[*] --> ep
ep --> f
f --> A
f --> B
A --> c
c --> H : [cond]
c --> xp : [!cond]
H --> B
B --> j
j --> op
op --> eo
eo --> xp
xp --> [*]
"""))

# Extra diagrams to push past 1000

# State machine variants with increasing complexity
for complexity in range(1, 21):
    n = complexity * 2 + 2
    lines = ["[*] --> S1"]
    for i in range(1, n):
        if i % 3 == 0:
            lines.append(f"S{i} --> S{i+1} : event{i} [guard{i}] / action{i}")
        elif i % 3 == 1:
            lines.append(f"S{i} --> S{i+1} : event{i}")
        else:
            lines.append(f"S{i} --> S{i+1}")
    lines.append(f"S{n} --> [*]")
    write(f"state_complexity_{complexity}", diagram("\n".join(lines)))

# Composite state variants
for outer_states in range(1, 6):
    for inner_states in range(1, 5):
        parts = []
        for o in range(1, outer_states + 1):
            inner_lines = ["  [*] --> IS1"]
            for i in range(1, inner_states):
                inner_lines.append(f"  IS{i} --> IS{i+1}")
            inner_lines.append(f"  IS{inner_states} --> [*]")
            parts.append(f"state Outer{o} {{\n" + "\n".join(inner_lines) + "\n}")
        parts.append("[*] --> Outer1")
        for o in range(1, outer_states):
            parts.append(f"Outer{o} --> Outer{o+1}")
        parts.append(f"Outer{outer_states} --> [*]")
        write(f"state_composite_o{outer_states}_i{inner_states}", diagram("\n".join(parts)))

# Transition style matrix: length x direction x label
for length_n, dashes in enumerate(["-", "--", "---"], 1):
    for direction in ["up", "down", "left", "right"]:
        for has_label in [True, False]:
            label = " : event" if has_label else ""
            lbl_sfx = "labeled" if has_label else "plain"
            write(f"state_trans_l{length_n}_{direction}_{lbl_sfx}", diagram(f"""
[*] --> A
A -{direction}{dashes}>{label}
A --> B{label}
B --> [*]
"""))

# Concurrent regions with different complexities
for region_count in range(2, 6):
    for states_per_region in range(1, 5):
        regions = []
        for r in range(region_count):
            region_lines = [f"  [*] --> R{r}S1"]
            for s in range(1, states_per_region):
                region_lines.append(f"  R{r}S{s} --> R{r}S{s+1}")
            region_lines.append(f"  R{r}S{states_per_region} --> [*]")
            regions.append("\n".join(region_lines))
        body = f"state Concurrent {{\n" + "\n  --\n".join(regions) + "\n}\n[*] --> Concurrent\nConcurrent --> [*]"
        write(f"state_concurrent_r{region_count}_s{states_per_region}", diagram(body))

# Note variants
for n_notes in range(1, 6):
    lines = ["[*] --> A", "A --> B", "B --> [*]"]
    for i in range(n_notes):
        if i % 2 == 0:
            lines.append(f"note right of A : Note {i+1}")
        else:
            lines.append(f"note left of B : Note {i+1}")
    write(f"state_notes_count_{n_notes}", diagram("\n".join(lines)))

# Color gradient variants
color_pairs = [
    ("red", "blue"), ("green", "yellow"), ("orange", "purple"),
    ("cyan", "pink"), ("lime", "silver"), ("navy", "gold"),
]
for c1, c2 in color_pairs:
    write(f"state_gradient_{c1}_{c2}", diagram(f"""
state A #{c1}/{c2}
[*] --> A
A --> [*]
"""))

# Arrow styles combined
arrow_styles = [
    ("--->", "long_plain"),
    ("--[#red]->", "medium_red"),
    ("-up->", "up"),
    ("-down->", "down"),
    ("-left->", "left"),
    ("-right->", "right"),
    ("..>", "dotted"),
    ("-[#blue,dashed]->", "blue_dashed"),
]
for arrow, style_name in arrow_styles:
    write(f"state_arrow_style_{style_name}", diagram(f"""
[*] --> A
A {arrow} B
B --> [*]
"""))

# Skinparam comprehensive
skinparam_state_props = [
    ("BackgroundColor", "LightCyan"),
    ("BorderColor", "DarkBlue"),
    ("FontColor", "DarkRed"),
    ("FontSize", "14"),
    ("FontName", "Arial"),
    ("FontStyle", "bold"),
    ("AttributeFontColor", "Gray"),
    ("AttributeFontSize", "12"),
    ("StartColor", "DarkGreen"),
    ("EndColor", "DarkRed"),
    ("ArrowColor", "DarkOrange"),
    ("ArrowFontColor", "Black"),
    ("ArrowFontSize", "11"),
]
for prop, val in skinparam_state_props:
    write(f"state_skinparam_state_{prop.lower()}", diagram("""
[*] --> A
A --> B : event
B --> [*]
""", skinparams=f"skinparam state {{\n  {prop} {val}\n}}"))

# Entry/exit point combos in composite states
for n_entry in range(1, 4):
    for n_exit in range(1, 4):
        entry_defs = "\n".join([f"  state ep{i} <<entryPoint>>" for i in range(n_entry)])
        exit_defs = "\n".join([f"  state xp{i} <<exitPoint>>" for i in range(n_exit)])
        entry_trans = "\n".join([f"  ep{i} --> State{i}" for i in range(n_entry)])
        exit_trans = "\n".join([f"  State{i} --> xp{i % n_exit}" for i in range(n_entry)])
        body = f"state Composite {{\n{entry_defs}\n{exit_defs}\n{entry_trans}\n{exit_trans}\n}}\n[*] --> Composite\nComposite --> [*]"
        write(f"state_entry{n_entry}_exit{n_exit}", diagram(body))

# Mixed feature diagrams
for variant in range(1, 11):
    notes = variant % 3 == 0
    colors = variant % 2 == 0
    concurrent = variant % 4 == 0
    fork_join = variant % 5 == 0

    lines = []
    if colors:
        lines.append("state A #LightBlue")
        lines.append("state B #LightGreen")
    if concurrent:
        lines.extend([
            "state Conc {",
            "  [*] --> X",
            "  X --> [*]",
            "  --",
            "  [*] --> Y",
            "  Y --> [*]",
            "}",
        ])
    if fork_join:
        lines.extend([
            "state f <<fork>>",
            "state j <<join>>",
        ])

    lines.extend(["[*] --> A", "A --> B", "B --> [*]"])

    if concurrent:
        lines.extend(["A --> Conc", "Conc --> B"])
    if fork_join:
        lines.extend(["A --> f", "f --> F1", "f --> F2", "F1 --> j", "F2 --> j", "j --> B"])
    if notes:
        lines.append("note right of A : variant note")

    write(f"state_mixed_variant_{variant}", diagram("\n".join(lines)))

# ─── 34. TRANSITION LABEL x GUARD x ACTION MATRIX ───────────────────────────

events = ["click", "timeout", "submit", "cancel", "next", "back", "retry", "reset"]
guards = ["[x > 0]", "[ready]", "[count < 3]", "[valid]", "[!error]"]
actions = ["/ log()", "/ update()", "/ notify()", "/ increment()"]

# event only
for ev in events:
    write(f"state_trans_event_{ev}", diagram(f"""
[*] --> A
A --> B : {ev}
B --> [*]
"""))

# guard only
for g in guards:
    safe = g.replace("[", "").replace("]", "").replace(" ", "_").replace("!", "not_").replace("<", "lt").replace(">", "gt")
    write(f"state_trans_guard_{safe}", diagram(f"""
[*] --> A
A --> B : {g}
B --> [*]
"""))

# action only
for a in actions:
    safe = a.replace("/ ", "").replace("()", "")
    write(f"state_trans_action_{safe}", diagram(f"""
[*] --> A
A --> B : {a}
B --> [*]
"""))

# event + guard combos
for ev in events[:4]:
    for g in guards[:3]:
        safe_e = ev
        safe_g = g.replace("[", "").replace("]", "").replace(" ", "_").replace("!", "not_").replace("<", "lt").replace(">", "gt")
        write(f"state_trans_eg_{safe_e}_{safe_g}", diagram(f"""
[*] --> A
A --> B : {ev} {g}
B --> [*]
"""))

# event + action combos
for ev in events[:4]:
    for a in actions[:3]:
        safe_e = ev
        safe_a = a.replace("/ ", "").replace("()", "")
        write(f"state_trans_ea_{safe_e}_{safe_a}", diagram(f"""
[*] --> A
A --> B : {ev} {a}
B --> [*]
"""))

# guard + action combos
for g in guards[:3]:
    for a in actions[:3]:
        safe_g = g.replace("[", "").replace("]", "").replace(" ", "_").replace("!", "not_").replace("<", "lt").replace(">", "gt")
        safe_a = a.replace("/ ", "").replace("()", "")
        write(f"state_trans_ga_{safe_g}_{safe_a}", diagram(f"""
[*] --> A
A --> B : {g} {a}
B --> [*]
"""))

# full combos
for ev in events[:3]:
    for g in guards[:3]:
        for a in actions[:2]:
            safe_e = ev
            safe_g = g.replace("[", "").replace("]", "").replace(" ", "_").replace("!", "not_").replace("<", "lt").replace(">", "gt")
            safe_a = a.replace("/ ", "").replace("()", "")
            write(f"state_trans_full_{safe_e}_{safe_g}_{safe_a}", diagram(f"""
[*] --> A
A --> B : {ev} {g} {a}
B --> [*]
"""))

# ─── 35. N-STATE DIAGRAMS WITH VARIED STRUCTURES ─────────────────────────────

# Star topology: one hub, N spokes
for n_spokes in range(2, 9):
    lines = ["[*] --> Hub"]
    for i in range(n_spokes):
        lines.append(f"Hub --> Spoke{i} : branch{i}")
        lines.append(f"Spoke{i} --> Hub : return{i}")
    lines.append("Hub --> [*]")
    write(f"state_star_{n_spokes}_spokes", diagram("\n".join(lines)))

# Ring topology
for n in range(3, 9):
    lines = [f"[*] --> S0"]
    for i in range(n):
        lines.append(f"S{i} --> S{(i+1) % n}")
    lines.append(f"S{n-1} --> [*]")
    write(f"state_ring_{n}", diagram("\n".join(lines)))

# Complete graph (all-to-all) for small N
for n in range(2, 6):
    lines = ["[*] --> S0"]
    for i in range(n):
        for j in range(n):
            if i != j:
                lines.append(f"S{i} --> S{j} : e{i}{j}")
    lines.append(f"S{n-1} --> [*]")
    write(f"state_complete_{n}", diagram("\n".join(lines)))

# Binary tree of states
for depth in range(1, 5):
    lines = ["[*] --> N1"]
    n = 1
    for d in range(depth):
        for node in range(2**d, 2**(d+1)):
            left = node * 2
            right = node * 2 + 1
            lines.append(f"N{node} --> N{left} : left")
            lines.append(f"N{node} --> N{right} : right")
    # add finals from leaf nodes
    for leaf in range(2**depth, 2**(depth+1)):
        lines.append(f"N{leaf} --> [*]")
    write(f"state_binary_tree_depth_{depth}", diagram("\n".join(lines)))

# ─── 36. COMPOSITE STATE SHAPE VARIANTS ──────────────────────────────────────

for n_composite in range(1, 7):
    parts = []
    parts.append("[*] --> C1")
    for i in range(1, n_composite + 1):
        parts.append(f"state C{i} {{")
        parts.append(f"  [*] --> CS{i}")
        parts.append(f"  CS{i} --> [*]")
        parts.append(f"}}")
    for i in range(1, n_composite):
        parts.append(f"C{i} --> C{i+1}")
    parts.append(f"C{n_composite} --> [*]")
    write(f"state_sequential_composites_{n_composite}", diagram("\n".join(parts)))

# ─── 37. NOTES IN MANY POSITIONS ─────────────────────────────────────────────

states_for_notes = ["A", "B", "C", "D"]
for i, state in enumerate(states_for_notes):
    for pos in ["left", "right"]:
        write(f"state_note_{pos}_of_{state.lower()}", diagram(f"""
[*] --> A
A --> B
B --> C
C --> D
D --> [*]
note {pos} of {state} : Note on {state}
"""))

# Note on link variations
for label in ["simple", "with event : event", "complex [guard] / action"]:
    safe = label.split()[0]
    write(f"state_note_on_link_{safe}", diagram(f"""
[*] --> A
A --> B : {label}
note on link
  This note is on the {safe} transition
end note
B --> [*]
"""))

# Floating notes in various positions
for note_num in range(1, 6):
    notes = "\n".join([f'note "Floating note {i}" as FN{i}' for i in range(1, note_num + 1)])
    write(f"state_floating_notes_{note_num}", diagram(f"""
{notes}
[*] --> A
A --> [*]
"""))

# ─── 38. SKINPARAM GLOBAL VARIANTS ───────────────────────────────────────────

global_skinparams = [
    ("classfont_size", "skinparam classFontSize 14"),
    ("note_bgcolor", "skinparam noteBorderColor DarkBlue\nskinparam noteBackgroundColor LightYellow"),
    ("arrow_thickness", "skinparam ArrowThickness 2"),
    ("default_text_align", "skinparam defaultTextAlignment center"),
    ("padding", "skinparam padding 10"),
    ("nodesep", "skinparam nodesep 50"),
    ("ranksep", "skinparam ranksep 50"),
    ("dpi_72", "skinparam dpi 72"),
    ("dpi_150", "skinparam dpi 150"),
    ("svg_link", "skinparam svgLinkTarget _blank"),
]
for name, sp in global_skinparams:
    write(f"state_global_skinparam_{name}", diagram("""
[*] --> A
A --> B : event
B --> [*]
""", skinparams=sp))

# ─── 39. COMBINATION STRESS TESTS ────────────────────────────────────────────

# Many notes + many states
for n in [3, 5, 7, 10]:
    lines = [f"[*] --> S1"]
    for i in range(1, n):
        lines.append(f"S{i} --> S{i+1} : e{i}")
    lines.append(f"S{n} --> [*]")
    for i in range(1, n + 1, 2):
        lines.append(f"note right of S{i} : State {i} note")
    write(f"state_notes_stress_{n}", diagram("\n".join(lines)))

# Deeply nested composite with concurrent regions
write("state_deep_concurrent", diagram("""
state L1 {
  state L2 {
    [*] --> X
    X --> Y
    Y --> [*]
    --
    [*] --> P
    P --> Q
    Q --> [*]
  }
  [*] --> L2
  L2 --> [*]
  --
  [*] --> Z
  Z --> W
  W --> [*]
}
[*] --> L1
L1 --> [*]
"""))

# Chain of composite states each with concurrent regions
for n in range(2, 5):
    parts = []
    for i in range(1, n + 1):
        parts.append(f"state CS{i} {{")
        parts.append(f"  [*] --> A{i}")
        parts.append(f"  A{i} --> [*]")
        parts.append(f"  --")
        parts.append(f"  [*] --> B{i}")
        parts.append(f"  B{i} --> [*]")
        parts.append(f"}}")
    parts.append("[*] --> CS1")
    for i in range(1, n):
        parts.append(f"CS{i} --> CS{i+1}")
    parts.append(f"CS{n} --> [*]")
    write(f"state_concurrent_composites_chain_{n}", diagram("\n".join(parts)))

# ─── 40. SPECIAL PUML FEATURES ───────────────────────────────────────────────

write("state_url_on_state", diagram("""
state A [[http://example.com]]
[*] --> A
A --> [*]
"""))

write("state_url_on_state_with_tooltip", diagram("""
state A [[http://example.com{tooltip text}]]
[*] --> A
A --> [*]
"""))

write("state_bold_state_name", diagram("""
state "<b>Bold State</b>" as BS
[*] --> BS
BS --> [*]
"""))

write("state_italic_state_name", diagram("""
state "<i>Italic State</i>" as IS
[*] --> IS
IS --> [*]
"""))

write("state_html_in_description", diagram("""
state A : <b>bold</b> and <i>italic</i>
[*] --> A
A --> [*]
"""))

write("state_color_in_alias", diagram("""
state "My State" as MS #cyan
[*] --> MS
MS --> [*]
"""))

write("state_alias_with_stereotype", diagram("""
state "My State" as MS <<mystereo>>
[*] --> MS
MS --> [*]
"""))

write("state_alias_color_stereotype", diagram("""
state "My State" as MS #yellow <<mystereo>>
[*] --> MS
MS --> [*]
"""))

# ─── 41. ARROW STYLE GRID ────────────────────────────────────────────────────

# All combinations: direction (4) x length (3) x color (3) x dotted (2) = 72
colors_grid = ["red", "blue", "green"]
for direction in ["up", "down", "left", "right"]:
    for n_dashes in range(1, 4):
        dashes = "-" * n_dashes
        for color in colors_grid:
            write(f"state_arrow_grid_{direction}_{n_dashes}_{color}", diagram(f"""
[*] --> A
A -{direction}[#{color}]{dashes}> B
B --> [*]
"""))

# ─── 42. MORE WORKFLOW PATTERNS ───────────────────────────────────────────────

write("state_retry_pattern", diagram("""
state choice1 <<choice>>
[*] --> Attempt
Attempt --> choice1 : result
choice1 --> Success : [ok]
choice1 --> Attempt : [failed && retries < 3] / retries++
choice1 --> GiveUp : [failed && retries >= 3]
Success --> [*]
GiveUp --> [*]
"""))

write("state_saga_pattern", diagram("""
state fork1 <<fork>>
state join1 <<join>>

[*] --> fork1
fork1 --> T1
fork1 --> T2
fork1 --> T3
T1 --> join1 : ok
T1 --> Compensate : fail
T2 --> join1 : ok
T2 --> Compensate : fail
T3 --> join1 : ok
T3 --> Compensate : fail
join1 --> Done
Compensate --> [*]
Done --> [*]
"""))

write("state_event_sourcing", diagram("""
[*] --> Created
Created --> Active : Activate
Active --> Suspended : Suspend
Suspended --> Active : Resume
Active --> Closed : Close
Suspended --> Closed : Close
Closed --> [*]

note right of Active : Commands are processed here
note right of Suspended : No new commands accepted
"""))

write("state_saga_compensation", diagram("""
state "Step 1" as S1
state "Step 2" as S2
state "Step 3" as S3
state "Compensate 1" as C1
state "Compensate 2" as C2
state "Compensate 3" as C3

[*] --> S1
S1 --> S2 : success
S2 --> S3 : success
S3 --> Done : success
S3 --> C3 : fail
C3 --> C2
C2 --> C1
C1 --> Failed
S2 --> C2 : fail
S1 --> C1 : fail
Done --> [*]
Failed --> [*]
"""))

# ─── 43. ANNOTATION COMBINATIONS ──────────────────────────────────────────────

for has_title in [True, False]:
    for has_hide in [True, False]:
        for has_skin in [True, False]:
            title = "Test Diagram" if has_title else None
            body = "[*] --> A\nA --> B\nB --> [*]"
            if has_hide:
                body = "hide empty description\n" + body
            skin = "skinparam state { BackgroundColor LightBlue }" if has_skin else None
            sfx = f"{'t' if has_title else 'n'}{'h' if has_hide else 'n'}{'s' if has_skin else 'n'}"
            write(f"state_annotation_{sfx}", diagram(body, title=title, skinparams=skin))

# ─── 44. CONCURRENT REGIONS WITH VARIED COMPLEXITY ────────────────────────────

for n_regions in range(2, 6):
    for complexity in range(1, 4):
        regions = []
        for r in range(n_regions):
            region_lines = [f"  [*] --> R{r}S0"]
            for s in range(complexity):
                region_lines.append(f"  R{r}S{s} --> R{r}S{s+1} : step{s}")
            region_lines.append(f"  R{r}S{complexity} --> [*]")
            regions.append("\n".join(region_lines))
        body = "state Concurrent {\n" + "\n  --\n".join(regions) + "\n}\n[*] --> Concurrent\nConcurrent --> [*]"
        write(f"state_concurrent_r{n_regions}_c{complexity}", diagram(body))

# ─── 45. DEEP NESTING VARIANTS ───────────────────────────────────────────────

for depth in range(1, 7):
    lines = []
    for d in range(depth):
        indent = "  " * d
        if d == 0:
            lines.append("state Root {")
        else:
            lines.append(f"{indent}state Level{d} {{")
    # innermost content
    indent = "  " * depth
    lines.append(f"{indent}[*] --> Leaf")
    lines.append(f"{indent}Leaf --> [*]")
    # close all
    for d in range(depth, 0, -1):
        indent = "  " * (d - 1)
        lines.append(f"{indent}}}")
        if d > 1:
            lines.append(f"{indent}[*] --> Level{d-1}")
            lines.append(f"{indent}Level{d-1} --> [*]")
    lines.append("[*] --> Root")
    lines.append("Root --> [*]")
    write(f"state_depth_{depth}", diagram("\n".join(lines)))

# ─── 46. TRANSITION STYLE COMPREHENSIVE MATRIX ───────────────────────────────

# All arrow direction + dash-count + label combos
all_labels = [
    ("no_label", ""),
    ("event", " : click"),
    ("guard", " : [valid]"),
    ("action", " : / doIt()"),
    ("full", " : click [valid] / doIt()"),
]
all_dirs = ["up", "down", "left", "right", ""]
all_dashes = ["-", "--", "---"]

for dir_name, dir_str in [("up", "up"), ("down", "down"), ("left", "left"), ("right", "right"), ("plain", "")]:
    for dash_n, dashes in enumerate(["-", "--", "---"], 1):
        for label_name, label in all_labels:
            if dir_str:
                arrow = f"-{dir_str}{dashes}>"
            else:
                arrow = f"{dashes}>"
            write(f"state_matrix_{dir_name}_d{dash_n}_{label_name}", diagram(f"""
[*] --> A
A {arrow}{label}
A --> B
B --> [*]
"""))

# ─── 47. STATE COLOR MATRIX ───────────────────────────────────────────────────

named_colors = [
    "AliceBlue", "AntiqueWhite", "Aqua", "Aquamarine", "Azure",
    "Beige", "Bisque", "Black", "BlanchedAlmond", "Blue",
    "BlueViolet", "Brown", "BurlyWood", "CadetBlue", "Chartreuse",
    "Chocolate", "Coral", "CornflowerBlue", "Cornsilk", "Crimson",
    "Cyan", "DarkBlue", "DarkCyan", "DarkGoldenRod", "DarkGray",
    "DarkGreen", "DarkKhaki", "DarkMagenta", "DarkOliveGreen",
    "DarkOrange", "DarkOrchid", "DarkRed", "DarkSalmon",
    "DarkSeaGreen", "DarkSlateBlue", "DarkSlateGray", "DarkTurquoise",
    "DarkViolet", "DeepPink", "DeepSkyBlue", "DimGray", "DodgerBlue",
    "FireBrick", "FloralWhite", "ForestGreen", "Fuchsia", "Gainsboro",
    "GhostWhite", "Gold", "GoldenRod", "Gray", "Green",
]

for color in named_colors:
    write(f"state_named_color_{color.lower()}", diagram(f"""
state A #{color}
[*] --> A
A --> [*]
"""))

# ─── 48. MULTI-TRANSITION CHAIN VARIANTS ──────────────────────────────────────

# Linear chains with different transition styles every step
for chain_len in [3, 5, 7, 10, 15, 20]:
    lines = ["[*] --> S1"]
    for i in range(1, chain_len):
        mod = i % 4
        if mod == 0:
            lines.append(f"S{i} --> S{i+1} : e{i} [g{i}] / a{i}")
        elif mod == 1:
            lines.append(f"S{i} --> S{i+1} : e{i}")
        elif mod == 2:
            lines.append(f"S{i} --> S{i+1} : [g{i}]")
        else:
            lines.append(f"S{i} --> S{i+1}")
    lines.append(f"S{chain_len} --> [*]")
    write(f"state_chain_{chain_len}_mixed", diagram("\n".join(lines)))

# ─── 49. COMPOSITE STATE WITH DESCRIPTIONS ────────────────────────────────────

for has_outer_desc in [True, False]:
    for has_inner_desc in [True, False]:
        outer_desc = "\nstate Outer : outer description" if has_outer_desc else ""
        inner_desc = "\n  state Inner : inner description" if has_inner_desc else ""
        sfx = f"{'od' if has_outer_desc else 'no'}{'id' if has_inner_desc else 'ni'}"
        write(f"state_composite_desc_{sfx}", diagram(f"""
{outer_desc}
state Outer {{
{inner_desc}
  state Inner {{
    [*] --> S
    S --> [*]
  }}
  [*] --> Inner
  Inner --> [*]
}}
[*] --> Outer
Outer --> [*]
"""))

# ─── 50. HISTORY STATE IN CONCURRENT REGIONS ──────────────────────────────────

write("state_history_in_region_1", diagram("""
state Composite {
  state H1 <<history>>
  [*] --> H1
  H1 --> A
  A --> B
  B --> A
  --
  [*] --> X
  X --> Y
  Y --> [*]
}
[*] --> Composite
Composite --> [*]
"""))

write("state_history_in_region_2", diagram("""
state Composite {
  [*] --> P
  P --> Q
  Q --> [*]
  --
  state H2 <<history>>
  [*] --> H2
  H2 --> M
  M --> N
  N --> M
}
[*] --> Composite
Composite --> [*]
"""))

write("state_deep_history_in_nested", diagram("""
state Outer {
  state Inner {
    state DH <<deepHistory>>
    [*] --> DH
    DH --> X
    X --> Y
    Y --> X
  }
  [*] --> Inner
  Inner --> [*]
}
[*] --> Outer
Outer --> [*]
"""))

# ─── 51. ALIAS + PSEUDOSTATE + COLOR MATRIX ──────────────────────────────────

pseudostate_stereos = [
    ("choice", "<<choice>>"),
    ("fork", "<<fork>>"),
    ("join", "<<join>>"),
    ("end", "<<end>>"),
    ("entryPoint", "<<entryPoint>>"),
    ("exitPoint", "<<exitPoint>>"),
]
ps_colors = ["red", "blue", "green", "yellow", "orange"]

for ps_name, stereo in pseudostate_stereos:
    for color in ps_colors:
        write(f"state_ps_{ps_name}_{color}", diagram(f"""
state ps {stereo} #{color}
[*] --> A
A --> ps
ps --> [*]
"""))

# ─── 52. COMPOSITE WITH MULTIPLE NAMED INNER STATES ───────────────────────────

for n_inner in range(2, 8):
    inner_lines = ["  [*] --> I1"]
    for i in range(1, n_inner):
        inner_lines.append(f"  I{i} --> I{i+1} : step{i}")
    inner_lines.append(f"  I{n_inner} --> [*]")
    body = "state Composite {\n" + "\n".join(inner_lines) + "\n}\n[*] --> Composite\nComposite --> [*]"
    write(f"state_composite_{n_inner}_inner_states", diagram(body))

# ─── 53. DIAMOND PATTERNS ────────────────────────────────────────────────────

for width in range(2, 6):
    lines = ["[*] --> Top"]
    for i in range(width):
        lines.append(f"Top --> M{i} : branch{i}")
    for i in range(width):
        lines.append(f"M{i} --> Bottom : merge{i}")
    lines.append("Bottom --> [*]")
    write(f"state_diamond_width_{width}", diagram("\n".join(lines)))

# ─── 54. MIXED ARROW STYLES IN ONE DIAGRAM ───────────────────────────────────

write("state_all_arrow_dirs", diagram("""
[*] --> Center
Center -up-> Up
Center -down-> Down
Center -left-> Left
Center -right-> Right
Up --> [*]
Down --> [*]
Left --> [*]
Right --> [*]
"""))

write("state_all_arrow_colors", diagram("""
[*] --> A
A -[#red]-> B
B -[#blue]-> C
C -[#green]-> D
D -[#orange]-> E
E -[#purple]-> [*]
"""))

write("state_mixed_dotted_solid", diagram("""
[*] --> A
A --> B
B ..> C
C --> D
D ..> E
E --> [*]
"""))

# ─── 55. STATE MACHINES WITH END PSEUDOSTATE ──────────────────────────────────

for n_ends in range(1, 5):
    end_defs = "\n".join([f"state end{i} <<end>>" for i in range(n_ends)])
    lines = [end_defs, "[*] --> Start", "Start --> Middle"]
    for i in range(n_ends):
        lines.append(f"Middle --> end{i} : terminate{i}")
    lines.append("Middle --> Done : complete")
    lines.append("Done --> [*]")
    write(f"state_ends_{n_ends}", diagram("\n".join(lines)))

# ─── 56. SEQUENCE OF PSEUDOSTATES ────────────────────────────────────────────

write("state_seq_fork_choice_join", diagram("""
state f <<fork>>
state c1 <<choice>>
state c2 <<choice>>
state j <<join>>

[*] --> f
f --> A
f --> B
A --> c1
c1 --> A1 : [path1]
c1 --> A2 : [path2]
B --> c2
c2 --> B1 : [pathA]
c2 --> B2 : [pathB]
A1 --> j
A2 --> j
B1 --> j
B2 --> j
j --> [*]
"""))

write("state_seq_choice_fork_choice", diagram("""
state c1 <<choice>>
state f <<fork>>
state c2 <<choice>>
state j <<join>>

[*] --> Start
Start --> c1
c1 --> f : [do parallel]
c1 --> Serial : [do serial]
f --> P1
f --> P2
P1 --> c2
P2 --> c2
c2 --> j : [merge]
c2 --> Error : [error]
j --> Done
Serial --> Done
Done --> [*]
Error --> [*]
"""))

# ─── 57. VARIED NOTE STYLES ──────────────────────────────────────────────────

note_styles = [
    ("plain", 'note right of A : simple note'),
    ("multiline", 'note right of A\n  line 1\n  line 2\nend note'),
    ("html", 'note right of A\n  <b>bold</b> <i>italic</i>\nend note'),
    ("link", 'note on link\n  on link\nend note'),
    ("floating", 'note "floating" as N1'),
]

for style_name, note_body in note_styles:
    write(f"state_note_style_{style_name}", diagram(f"""
[*] --> A
A --> B
{note_body}
B --> [*]
"""))

# Multiple notes combined
for n1, n2 in [("left", "right"), ("right", "right"), ("left", "left")]:
    write(f"state_notes_{n1}_{n2}", diagram(f"""
[*] --> A
A --> B
B --> [*]
note {n1} of A : note on A
note {n2} of B : note on B
"""))

# ─── 58. VARIED COMPOSITE ALIAS STYLES ───────────────────────────────────────

for alias_style in ["plain", "quoted", "with_color", "with_stereo"]:
    if alias_style == "plain":
        decl = "state Outer {"
    elif alias_style == "quoted":
        decl = 'state "Outer State" as Outer {'
    elif alias_style == "with_color":
        decl = "state Outer #LightBlue {"
    else:
        decl = "state Outer <<mytype>> {"
    write(f"state_composite_{alias_style}", diagram(f"""
{decl}
  [*] --> Inner
  Inner --> [*]
}}
[*] --> Outer
Outer --> [*]
"""))

# ─── 59. FORK/JOIN WITH INNER STATE TRANSITIONS ───────────────────────────────

for n_branches in range(2, 7):
    lines = ["state f <<fork>>", "state j <<join>>", "[*] --> f"]
    for i in range(n_branches):
        lines.append(f"f --> Branch{i}Start : start{i}")
        lines.append(f"Branch{i}Start --> Branch{i}Mid")
        lines.append(f"Branch{i}Mid --> Branch{i}End")
        lines.append(f"Branch{i}End --> j : end{i}")
    lines.append("j --> [*]")
    write(f"state_fork_join_detailed_{n_branches}", diagram("\n".join(lines)))

# ─── 60. FINAL BATCH: VARIED DESCRIPTION + TRANSITION COMBOS ─────────────────

desc_texts = [
    "idle",
    "processing data",
    "waiting for user input",
    "sending network request",
    "handling error condition",
    "validating input parameters",
]
transition_texts = [
    "next",
    "trigger",
    "user input",
    "timeout occurred",
    "data received",
    "error detected",
]

for i, (desc, trans) in enumerate(zip(desc_texts, transition_texts)):
    write(f"state_desc_trans_{i}", diagram(f"""
state A : {desc}
state B : next state
[*] --> A
A --> B : {trans}
B --> [*]
"""))

# Grid: all 6 desc x all 6 trans
for di, desc in enumerate(desc_texts):
    for ti, trans in enumerate(transition_texts):
        write(f"state_desc_d{di}_t{ti}", diagram(f"""
state A : {desc}
[*] --> A
A --> B : {trans}
B --> [*]
"""))

# ─── 61. EXPANDED COLOR PALETTE ──────────────────────────────────────────────

more_colors = [
    "GreenYellow", "HoneyDew", "HotPink", "IndianRed", "Indigo",
    "Ivory", "Khaki", "Lavender", "LavenderBlush", "LawnGreen",
    "LemonChiffon", "LightBlue", "LightCoral", "LightCyan",
    "LightGoldenRodYellow", "LightGray", "LightGreen", "LightPink",
    "LightSalmon", "LightSeaGreen", "LightSkyBlue", "LightSlateGray",
    "LightSteelBlue", "LightYellow", "Lime", "LimeGreen", "Linen",
    "Magenta", "Maroon", "MediumAquaMarine", "MediumBlue",
    "MediumOrchid", "MediumPurple", "MediumSeaGreen", "MediumSlateBlue",
    "MediumSpringGreen", "MediumTurquoise", "MediumVioletRed",
    "MintCream", "MistyRose", "Moccasin", "NavajoWhite", "Navy",
    "OldLace", "Olive", "OliveDrab", "Orange", "OrangeRed",
    "Orchid", "PaleGoldenRod", "PaleGreen", "PaleTurquoise",
    "PaleVioletRed", "PapayaWhip", "PeachPuff", "Peru",
]

for color in more_colors:
    write(f"state_color_ext_{color.lower()}", diagram(f"""
state A #{color}
[*] --> A
A --> [*]
"""))

# ─── 62. TRANSITION DIRECTION x COLOR x LABEL MATRIX ─────────────────────────

dir_color_labels = [
    ("up", "red", " : up red"),
    ("up", "blue", " : up blue"),
    ("down", "red", " : down red"),
    ("down", "green", " : down green"),
    ("left", "blue", " : left blue"),
    ("left", "orange", " : left orange"),
    ("right", "green", " : right green"),
    ("right", "purple", " : right purple"),
]

for direction, color, label in dir_color_labels:
    write(f"state_dir_{direction}_{color}", diagram(f"""
[*] --> A
A -{direction}[#{color}]->{label}
A --> B
B --> [*]
"""))

# ─── 63. PARAMETERISED WORKFLOW TEMPLATES ────────────────────────────────────

workflow_templates = [
    ("linear", lambda n: "\n".join(
        ["[*] --> W1"] +
        [f"W{i} --> W{i+1} : step{i}" for i in range(1, n)] +
        [f"W{n} --> [*]"]
    )),
    ("parallel", lambda n: "\n".join(
        ["state f <<fork>>", "state j <<join>>", "[*] --> f"] +
        [f"f --> P{i}" for i in range(n)] +
        [f"P{i} --> j" for i in range(n)] +
        ["j --> [*]"]
    )),
    ("choice", lambda n: "\n".join(
        ["state c <<choice>>", "[*] --> Start", "Start --> c"] +
        [f"c --> End{i} : [path{i}]" for i in range(n)] +
        [f"End{i} --> [*]" for i in range(n)]
    )),
    ("concurrent", lambda n: (
        "state Concurrent {\n" +
        "\n  --\n".join(
            [f"  [*] --> CR{r}A\n  CR{r}A --> CR{r}B\n  CR{r}B --> [*]" for r in range(n)]
        ) +
        "\n}\n[*] --> Concurrent\nConcurrent --> [*]"
    )),
]

for template_name, template_fn in workflow_templates:
    for n in range(2, 7):
        write(f"state_workflow_{template_name}_{n}", diagram(template_fn(n)))

# ─── 64. NOTE CONTENT VARIANTS ────────────────────────────────────────────────

note_contents = [
    ("short", "x"),
    ("word", "hello"),
    ("sentence", "This is a note."),
    ("two_words", "two words"),
    ("numbers", "123 456"),
    ("special_chars", "a & b"),
    ("lt_gt", "x < y"),
    ("dash", "state-name"),
]

for note_name, content in note_contents:
    write(f"state_note_content_{note_name}", diagram(f"""
[*] --> A
note right of A : {content}
A --> [*]
"""))

# Multiline note content variants
for n_lines in range(2, 7):
    content = "\n".join([f"  line {i}" for i in range(1, n_lines + 1)])
    write(f"state_note_multiline_{n_lines}_lines", diagram(f"""
[*] --> A
note right of A
{content}
end note
A --> [*]
"""))

# ─── 65. SKINPARAM ARROW COMBINATIONS ────────────────────────────────────────

arrow_skinparams = [
    ("color_red", "skinparam ArrowColor Red"),
    ("color_blue", "skinparam ArrowColor Blue"),
    ("color_green", "skinparam ArrowColor Green"),
    ("thickness_1", "skinparam ArrowThickness 1"),
    ("thickness_2", "skinparam ArrowThickness 2"),
    ("thickness_3", "skinparam ArrowThickness 3"),
    ("fontsize_10", "skinparam ArrowFontSize 10"),
    ("fontsize_14", "skinparam ArrowFontSize 14"),
    ("fontsize_18", "skinparam ArrowFontSize 18"),
    ("fontcolor_red", "skinparam ArrowFontColor Red"),
    ("fontcolor_blue", "skinparam ArrowFontColor Blue"),
]

for name, sp in arrow_skinparams:
    write(f"state_skinparam_arrow_{name}", diagram("""
[*] --> A
A --> B : event
B --> [*]
""", skinparams=sp))

# ─── 66. COMPOSITE WITH VARIOUS PSEUDOSTATE COMBINATIONS ─────────────────────

pseudostate_combos = [
    ("fork_only", "state f <<fork>>\n  f --> A\n  f --> B\n  A --> [*]\n  B --> [*]\n  [*] --> f"),
    ("join_only", "state j <<join>>\n  [*] --> A\n  [*] --> B\n  A --> j\n  B --> j\n  j --> [*]"),
    ("choice_only", "state c <<choice>>\n  [*] --> c\n  c --> A : [p1]\n  c --> B : [p2]\n  A --> [*]\n  B --> [*]"),
    ("history_only", "state H <<history>>\n  [*] --> H\n  H --> A\n  A --> B\n  B --> A"),
    ("entry_only", "state ep <<entryPoint>>\n  [*] --> ep\n  ep --> A\n  A --> [*]"),
    ("exit_only", "state xp <<exitPoint>>\n  [*] --> A\n  A --> xp\n  xp --> [*]"),
]

for combo_name, combo_body in pseudostate_combos:
    write(f"state_composite_{combo_name}", diagram(f"""
state Composite {{
{combo_body}
}}
[*] --> Composite
Composite --> [*]
"""))

# ─── 67. FINAL PADDING BATCH ──────────────────────────────────────────────────

# Simple named states with varied alias lengths
alias_lengths = [1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 15, 20]
for n in alias_lengths:
    alias = "S" * n
    name = "A" * n + " State"
    write(f"state_alias_len_{n}", diagram(f"""
state "{name}" as {alias}
[*] --> {alias}
{alias} --> [*]
"""))

# Varying number of labeled self-transitions
for n_self in range(1, 8):
    lines = ["[*] --> A"]
    for i in range(n_self):
        lines.append(f"A --> A : self{i}")
    lines.append("A --> [*]")
    write(f"state_self_transitions_{n_self}", diagram("\n".join(lines)))

# Varying number of choice branches
for n_branches in range(2, 9):
    lines = ["state c <<choice>>", "[*] --> S", "S --> c"]
    for i in range(n_branches):
        lines.append(f"c --> End{i} : [branch{i}]")
        lines.append(f"End{i} --> [*]")
    write(f"state_choice_{n_branches}_branches", diagram("\n".join(lines)))

# Composite states with varying description lines
for n_lines in range(1, 6):
    outer_desc = "\n".join([f"state Outer : desc line {i}" for i in range(1, n_lines + 1)])
    write(f"state_composite_desc_{n_lines}_lines", diagram(f"""
{outer_desc}
state Outer {{
  [*] --> S
  S --> [*]
}}
[*] --> Outer
Outer --> [*]
"""))

# ─── FINAL COUNT ─────────────────────────────────────────────────────────────

print(f"Generated {files_written} .puml files in {OUT_DIR}")
