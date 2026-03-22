#!/usr/bin/env python3
"""
Generator for comprehensive PlantUML activity diagram test cases.
Produces ~1500+ .puml files covering every conceivable feature and edge case.
"""

import os
import itertools
from pathlib import Path

OUT_DIR = Path(__file__).parent / "activity"
OUT_DIR.mkdir(parents=True, exist_ok=True)

_written = 0

def write(name: str, content: str):
    global _written
    path = OUT_DIR / f"{name}.puml"
    path.write_text(content)
    _written += 1


def wrap(body: str, title: str = "") -> str:
    title_line = f"title {title}\n" if title else ""
    return f"@startuml\n{title_line}{body.rstrip()}\n@enduml\n"


# ─── 1. BASIC START/STOP ─────────────────────────────────────────────────────

write("act_basic_start_stop", wrap("""
start
:Do something;
stop
"""))

write("act_basic_start_end", wrap("""
start
:Do something;
end
"""))

write("act_basic_start_kill", wrap("""
start
:Do something;
kill
"""))

write("act_basic_start_detach", wrap("""
start
:Do something;
detach
"""))

write("act_empty_diagram", wrap("""
start
stop
"""))

write("act_single_action", wrap("""
start
:Single action;
stop
"""))

write("act_start_no_action", wrap("""
start
end
"""))

# multiple stops
write("act_multiple_stops", wrap("""
start
if (condition?) then (yes)
  :Action A;
  stop
else (no)
  :Action B;
  stop
endif
"""))

# ─── 2. ACTION ENDINGS ───────────────────────────────────────────────────────

endings = [
    ("semicolon",   ";",  "action"),
    ("fork_bar",    "|",  "fork_action"),
    ("receive",     "<",  "receive_action"),
    ("send",        ">",  "send_action"),
    ("input",       "/",  "input_action"),
    ("output",      "\\", "output_action"),
    ("flow_final",  "]",  "flow_final_action"),
]

for ename, esym, etext in endings:
    write(f"act_action_ending_{ename}", wrap(f"""
start
:{etext}{esym}
stop
"""))

# all endings in sequence
body = "start\n"
for ename, esym, etext in endings:
    body += f":{etext}{esym}\n"
body += "stop\n"
write("act_action_all_endings", wrap(body))

# ─── 3. ACTION COLORS ────────────────────────────────────────────────────────

colors = ["#red", "#green", "#blue", "#yellow", "#orange", "#pink",
          "#lightblue", "#lightgreen", "#tomato", "#AABBCC", "#FF5500"]

for c in colors:
    cname = c.lstrip("#").lower()
    write(f"act_action_color_{cname}", wrap(f"""
start
{c}:Colored action;
stop
"""))

# multiple colored actions
body = "start\n"
for i, c in enumerate(colors):
    body += f"{c}:Action {i+1};\n"
body += "stop\n"
write("act_action_multiple_colors", wrap(body))

# ─── 4. SEQUENTIAL ACTIONS ───────────────────────────────────────────────────

for n in [2, 3, 5, 10, 20, 50]:
    body = "start\n"
    for i in range(1, n+1):
        body += f":Action {i};\n"
    body += "stop\n"
    write(f"act_sequential_{n}_actions", wrap(body))

# ─── 5. IF/THEN/ELSE ─────────────────────────────────────────────────────────

write("act_if_simple", wrap("""
start
if (condition?) then (yes)
  :Action A;
else (no)
  :Action B;
endif
stop
"""))

write("act_if_no_else", wrap("""
start
if (condition?) then (yes)
  :Action A;
endif
stop
"""))

write("act_if_empty_then", wrap("""
start
if (condition?) then (yes)
else (no)
  :Action B;
endif
stop
"""))

write("act_if_empty_both", wrap("""
start
if (condition?) then (yes)
else (no)
endif
stop
"""))

write("act_if_multiple_actions", wrap("""
start
if (condition?) then (yes)
  :Step 1;
  :Step 2;
  :Step 3;
else (no)
  :Alt step 1;
  :Alt step 2;
endif
stop
"""))

write("act_if_long_condition", wrap("""
start
if (Is the value of the variable X greater than the threshold Y?) then (yes, it is)
  :Handle above threshold case;
else (no, it is not)
  :Handle below threshold case;
endif
stop
"""))

write("act_if_no_labels", wrap("""
start
if (condition?) then
  :Action A;
else
  :Action B;
endif
stop
"""))

# nested if 2 levels
write("act_if_nested_2_levels", wrap("""
start
if (outer?) then (yes)
  if (inner?) then (yes)
    :Both true;
  else (no)
    :Outer true inner false;
  endif
else (no)
  if (inner?) then (yes)
    :Outer false inner true;
  else (no)
    :Both false;
  endif
endif
stop
"""))

# nested if 3 levels
write("act_if_nested_3_levels", wrap("""
start
if (A?) then (yes)
  if (B?) then (yes)
    if (C?) then (yes)
      :A and B and C;
    else (no)
      :A and B not C;
    endif
  else (no)
    :A not B;
  endif
else (no)
  if (B?) then (yes)
    :Not A but B;
  else (no)
    :Not A not B;
  endif
endif
stop
"""))

# nested if 4 levels
write("act_if_nested_4_levels", wrap("""
start
if (L1?) then (yes)
  if (L2?) then (yes)
    if (L3?) then (yes)
      if (L4?) then (yes)
        :All true;
      else (no)
        :L1 L2 L3 true, L4 false;
      endif
    else (no)
      :L1 L2 true, L3 false;
    endif
  else (no)
    :L1 true, L2 false;
  endif
else (no)
  :L1 false;
endif
stop
"""))

# nested if 5 levels
write("act_if_nested_5_levels", wrap("""
start
if (L1?) then (yes)
  if (L2?) then (yes)
    if (L3?) then (yes)
      if (L4?) then (yes)
        if (L5?) then (yes)
          :All 5 levels true;
        else (no)
          :L1-L4 true, L5 false;
        endif
      else (no)
        :L1-L3 true, L4 false;
      endif
    else (no)
      :L1-L2 true, L3 false;
    endif
  else (no)
    :L1 true, L2 false;
  endif
else (no)
  :All false;
endif
stop
"""))

# ─── 6. ELSEIF ───────────────────────────────────────────────────────────────

write("act_elseif_2_branches", wrap("""
start
if (A?) then (yes)
  :Branch A;
elseif (B?) then (yes)
  :Branch B;
else (no)
  :Default branch;
endif
stop
"""))

write("act_elseif_3_branches", wrap("""
start
if (A?) then (yes)
  :Branch A;
elseif (B?) then (yes)
  :Branch B;
elseif (C?) then (yes)
  :Branch C;
else (otherwise)
  :Default branch;
endif
stop
"""))

write("act_elseif_4_branches", wrap("""
start
if (A?) then (yes)
  :Branch A;
elseif (B?) then (yes)
  :Branch B;
elseif (C?) then (yes)
  :Branch C;
elseif (D?) then (yes)
  :Branch D;
else (none)
  :Default;
endif
stop
"""))

write("act_elseif_5_branches", wrap("""
start
if (color == red?) then (yes)
  :Red path;
elseif (color == green?) then (yes)
  :Green path;
elseif (color == blue?) then (yes)
  :Blue path;
elseif (color == yellow?) then (yes)
  :Yellow path;
elseif (color == purple?) then (yes)
  :Purple path;
else (other)
  :Unknown color;
endif
stop
"""))

write("act_elseif_no_else", wrap("""
start
if (A?) then (yes)
  :Branch A;
elseif (B?) then (yes)
  :Branch B;
elseif (C?) then (yes)
  :Branch C;
endif
stop
"""))

# ─── 7. SWITCH/CASE ──────────────────────────────────────────────────────────

write("act_switch_2_cases", wrap("""
start
switch (value?)
case (1)
  :Case one;
case (2)
  :Case two;
endswitch
stop
"""))

write("act_switch_3_cases", wrap("""
start
switch (color?)
case (red)
  :Handle red;
case (green)
  :Handle green;
case (blue)
  :Handle blue;
endswitch
stop
"""))

write("act_switch_4_cases", wrap("""
start
switch (status?)
case (pending)
  :Process pending;
case (active)
  :Process active;
case (paused)
  :Process paused;
case (done)
  :Process done;
endswitch
stop
"""))

write("act_switch_5_cases", wrap("""
start
switch (day?)
case (Monday)
  :Start of week;
case (Tuesday)
  :Second day;
case (Wednesday)
  :Mid week;
case (Thursday)
  :Almost Friday;
case (Friday)
  :End of week;
endswitch
stop
"""))

write("act_switch_with_default", wrap("""
start
switch (value?)
case (1)
  :One;
case (2)
  :Two;
case (3)
  :Three;
case (other)
  :Default case;
endswitch
stop
"""))

write("act_switch_multiple_actions", wrap("""
start
switch (state?)
case (A)
  :Action A1;
  :Action A2;
  :Action A3;
case (B)
  :Action B1;
  :Action B2;
case (C)
  :Action C1;
endswitch
stop
"""))

# ─── 8. WHILE LOOPS ──────────────────────────────────────────────────────────

write("act_while_basic", wrap("""
start
while (more items?) is (yes)
  :Process item;
endwhile (no)
stop
"""))

write("act_while_no_labels", wrap("""
start
while (condition?)
  :Loop body;
endwhile
stop
"""))

write("act_while_with_label", wrap("""
start
while (not done?) is (continue)
  :Do work;
endwhile (finished)
stop
"""))

write("act_while_multiple_actions", wrap("""
start
while (queue not empty?) is (has items)
  :Dequeue item;
  :Process item;
  :Log result;
endwhile (empty)
stop
"""))

write("act_while_nested", wrap("""
start
while (outer?) is (yes)
  while (inner?) is (yes)
    :Inner action;
  endwhile (no)
  :Outer action;
endwhile (no)
stop
"""))

write("act_while_nested_3_levels", wrap("""
start
while (L1?) is (yes)
  while (L2?) is (yes)
    while (L3?) is (yes)
      :Innermost;
    endwhile
  endwhile
  :After L2;
endwhile
stop
"""))

write("act_while_with_if", wrap("""
start
while (more?) is (yes)
  :Get item;
  if (valid?) then (yes)
    :Process item;
  else (no)
    :Skip item;
  endif
endwhile (no)
stop
"""))

write("act_while_break", wrap("""
start
while (condition?) is (yes)
  :Do work;
  if (done early?) then (yes)
    break
  endif
  :Continue;
endwhile (no)
stop
"""))

# ─── 9. REPEAT/REPEATWHILE ───────────────────────────────────────────────────

write("act_repeat_basic", wrap("""
start
repeat
  :Action;
repeatwhile (again?) is (yes)
stop
"""))

write("act_repeat_no_label", wrap("""
start
repeat
  :Action;
repeatwhile (again?)
stop
"""))

write("act_repeat_multiple_actions", wrap("""
start
repeat
  :Step 1;
  :Step 2;
  :Step 3;
repeatwhile (more steps?) is (yes)
stop
"""))

write("act_repeat_with_backward", wrap("""
start
repeat
  :Do work;
backward :Retry;
repeatwhile (not done?) is (retry)
stop
"""))

write("act_repeat_backward_multi", wrap("""
start
repeat
  :Attempt operation;
  :Check result;
backward
  :Log failure;
  :Reset state;
repeatwhile (failed?) is (retry)
stop
"""))

write("act_repeat_nested", wrap("""
start
repeat
  repeat
    :Inner action;
  repeatwhile (inner?) is (yes)
  :Outer action;
repeatwhile (outer?) is (yes)
stop
"""))

write("act_repeat_with_if", wrap("""
start
repeat
  :Get input;
  if (valid?) then (yes)
    :Process;
  else (no)
    :Show error;
  endif
repeatwhile (continue?) is (yes)
stop
"""))

write("act_repeat_break", wrap("""
start
repeat
  :Try action;
  if (succeeded?) then (yes)
    break
  endif
  :Handle failure;
repeatwhile (retry?) is (yes)
stop
"""))

# ─── 10. FORK/PARALLEL ───────────────────────────────────────────────────────

write("act_fork_2_branches", wrap("""
start
fork
  :Branch 1;
fork again
  :Branch 2;
end fork
stop
"""))

write("act_fork_3_branches", wrap("""
start
fork
  :Branch 1;
fork again
  :Branch 2;
fork again
  :Branch 3;
end fork
stop
"""))

write("act_fork_4_branches", wrap("""
start
fork
  :Branch A;
fork again
  :Branch B;
fork again
  :Branch C;
fork again
  :Branch D;
end fork
stop
"""))

write("act_fork_5_branches", wrap("""
start
fork
  :Branch 1;
fork again
  :Branch 2;
fork again
  :Branch 3;
fork again
  :Branch 4;
fork again
  :Branch 5;
end fork
stop
"""))

write("act_fork_6_branches", wrap("""
start
fork
  :P1;
fork again
  :P2;
fork again
  :P3;
fork again
  :P4;
fork again
  :P5;
fork again
  :P6;
end fork
stop
"""))

write("act_fork_multi_actions", wrap("""
start
fork
  :A1;
  :A2;
  :A3;
fork again
  :B1;
  :B2;
fork again
  :C1;
  :C2;
  :C3;
  :C4;
end fork
stop
"""))

write("act_fork_end_merge", wrap("""
start
fork
  :Branch 1;
fork again
  :Branch 2;
end merge
stop
"""))

write("act_fork_nested", wrap("""
start
fork
  fork
    :Nested 1a;
  fork again
    :Nested 1b;
  end fork
  :After inner fork 1;
fork again
  fork
    :Nested 2a;
  fork again
    :Nested 2b;
  end fork
  :After inner fork 2;
end fork
stop
"""))

write("act_fork_with_if", wrap("""
start
fork
  if (condition?) then (yes)
    :Branch 1 yes;
  else (no)
    :Branch 1 no;
  endif
fork again
  :Branch 2;
end fork
stop
"""))

write("act_fork_with_while", wrap("""
start
fork
  while (items?) is (yes)
    :Process item;
  endwhile (done)
fork again
  :Parallel task;
end fork
stop
"""))

# ─── 11. SWIMLANES ───────────────────────────────────────────────────────────

write("act_swimlane_2_lanes", wrap("""
|Lane A|
start
:Action in A;
|Lane B|
:Action in B;
|Lane A|
:Back in A;
stop
"""))

write("act_swimlane_3_lanes", wrap("""
|Client|
start
:Send request;
|Server|
:Receive request;
:Process;
:Send response;
|Client|
:Receive response;
stop
"""))

write("act_swimlane_4_lanes", wrap("""
|UI|
start
:User clicks;
|Controller|
:Handle event;
|Service|
:Business logic;
|Database|
:Persist data;
|Service|
:Return result;
|Controller|
:Format response;
|UI|
:Update display;
stop
"""))

write("act_swimlane_5_lanes", wrap("""
|A|
start
:Step A1;
|B|
:Step B1;
|C|
:Step C1;
|D|
:Step D1;
|E|
:Step E1;
|A|
:Step A2;
stop
"""))

write("act_swimlane_8_lanes", wrap("""
|L1|
start
:Action 1;
|L2|
:Action 2;
|L3|
:Action 3;
|L4|
:Action 4;
|L5|
:Action 5;
|L6|
:Action 6;
|L7|
:Action 7;
|L8|
:Action 8;
stop
"""))

write("act_swimlane_colored", wrap("""
|#lightblue|Client|
start
:Submit request;
|#lightyellow|Processor|
:Validate input;
:Execute logic;
|#lightgreen|Storage|
:Save result;
|#lightblue|Client|
:Display result;
stop
"""))

write("act_swimlane_with_if", wrap("""
|User|
start
:Submit form;
|System|
if (valid?) then (yes)
  :Process;
  |Database|
  :Save;
  |System|
  :Success response;
else (no)
  :Error response;
endif
|User|
:Show result;
stop
"""))

write("act_swimlane_with_fork", wrap("""
|Coordinator|
start
:Initialize;
fork
  |Worker1|
  :Task 1;
fork again
  |Worker2|
  :Task 2;
fork again
  |Worker3|
  :Task 3;
end fork
|Coordinator|
:Aggregate results;
stop
"""))

write("act_swimlane_with_while", wrap("""
|Producer|
start
while (more data?) is (yes)
  :Produce item;
  |Consumer|
  :Consume item;
  |Producer|
endwhile (done)
|Consumer|
stop
"""))

write("act_swimlane_with_partition", wrap("""
partition "System" {
  |Frontend|
  start
  :Render UI;
  |Backend|
  :Handle request;
  |Frontend|
  stop
}
"""))

# ─── 12. PARTITIONS ──────────────────────────────────────────────────────────

write("act_partition_basic", wrap("""
start
partition "Initialization" {
  :Load config;
  :Connect to DB;
}
partition "Processing" {
  :Read data;
  :Transform data;
  :Write output;
}
stop
"""))

write("act_partition_colored", wrap("""
start
partition #lightblue "Setup" {
  :Initialize;
  :Configure;
}
partition #lightyellow "Execute" {
  :Run;
  :Monitor;
}
partition #lightgreen "Cleanup" {
  :Finalize;
  :Dispose;
}
stop
"""))

write("act_partition_nested", wrap("""
start
partition "Outer" {
  :Outer start;
  partition "Inner" {
    :Inner action 1;
    :Inner action 2;
  }
  :Outer end;
}
stop
"""))

write("act_partition_with_if", wrap("""
start
partition "Validation" {
  :Check input;
  if (valid?) then (yes)
    :Accept;
  else (no)
    :Reject;
  endif
}
partition "Processing" {
  :Process valid input;
}
stop
"""))

write("act_partition_single", wrap("""
start
partition "Single Partition" {
  :Only action;
}
stop
"""))

write("act_partition_empty_name", wrap("""
start
partition "" {
  :Action in unnamed partition;
}
stop
"""))

# ─── 13. NOTES ───────────────────────────────────────────────────────────────

write("act_note_right", wrap("""
start
:Action;
note right
  This is a note on the right
end note
stop
"""))

write("act_note_left", wrap("""
start
:Action;
note left
  This is a note on the left
end note
stop
"""))

write("act_note_floating", wrap("""
start
floating note left: This is a floating note
:Action;
stop
"""))

write("act_note_multiline", wrap("""
start
:Action;
note right
  Line one of the note
  Line two of the note
  Line three of the note
end note
stop
"""))

write("act_note_multiple", wrap("""
start
:Action 1;
note right
  Note after action 1
end note
:Action 2;
note left
  Note after action 2
end note
:Action 3;
stop
"""))

write("act_note_on_if", wrap("""
start
if (condition?) then (yes)
  note right: Yes branch note
  :Yes action;
else (no)
  :No action;
endif
stop
"""))

write("act_note_colored", wrap("""
start
:Action;
note right #lightyellow
  Colored note
end note
stop
"""))

write("act_note_on_start", wrap("""
start
note right: Note on start
:Action;
stop
"""))

# ─── 14. CONNECTORS (GOTO) ───────────────────────────────────────────────────

write("act_connector_basic", wrap("""
start
:Action 1;
(A)
:Action 2;
(B)
:Action 3;
stop
"""))

write("act_connector_goto", wrap("""
start
:Step 1;
if (skip?) then (yes)
  (SKIP)
else (no)
  :Step 2;
  :Step 3;
endif
(SKIP)
:Final step;
stop
"""))

write("act_connector_multiple", wrap("""
start
:Start action;
(1)
:Middle action;
(2)
:End action;
stop
"""))

write("act_connector_named", wrap("""
start
:Process;
(ErrorHandler)
:Handle error;
stop
"""))

# ─── 15. GROUPING ────────────────────────────────────────────────────────────

write("act_group_basic", wrap("""
start
group "My Group" {
  :Action 1;
  :Action 2;
}
stop
"""))

write("act_group_colored", wrap("""
start
group #lightblue "Blue Group" {
  :Action A;
  :Action B;
}
stop
"""))

write("act_group_nested", wrap("""
start
group "Outer Group" {
  :Outer action;
  group "Inner Group" {
    :Inner action 1;
    :Inner action 2;
  }
  :After inner;
}
stop
"""))

write("act_group_with_if", wrap("""
start
group "Conditional Group" {
  if (test?) then (yes)
    :True path;
  else (no)
    :False path;
  endif
}
stop
"""))

# ─── 16. ARROW LABELS AND STYLES ─────────────────────────────────────────────

write("act_arrow_labels", wrap("""
start
:Action 1;
-> labeled arrow;
:Action 2;
--> dashed arrow label;
:Action 3;
stop
"""))

write("act_arrow_color", wrap("""
start
:Action 1;
-[#red]->
:Action 2;
-[#blue]->
:Action 3;
stop
"""))

write("act_arrow_color_labeled", wrap("""
start
:Action 1;
-[#green]-> green path;
:Action 2;
-[#red]-> red path;
:Action 3;
stop
"""))

write("act_arrow_backward_label", wrap("""
start
repeat
  :Try;
backward :Retry after failure;
repeatwhile (failed?) is (yes)
stop
"""))

# ─── 17. SKINPARAM ───────────────────────────────────────────────────────────

write("act_skinparam_basic", wrap("""
skinparam activity {
  BackgroundColor lightyellow
  BorderColor darkblue
  FontName Arial
}
start
:Action;
stop
"""))

write("act_skinparam_diamond", wrap("""
skinparam activityDiamondBackgroundColor lightblue
skinparam activityDiamondBorderColor navy
start
if (condition?) then (yes)
  :Yes;
else (no)
  :No;
endif
stop
"""))

write("act_skinparam_arrow", wrap("""
skinparam activityArrowColor red
skinparam activityArrowFontColor blue
start
:A;
:B;
:C;
stop
"""))

write("act_skinparam_full", wrap("""
skinparam activity {
  BackgroundColor #FFFACD
  BorderColor #8B4513
  FontName Courier
  FontSize 12
  FontStyle bold
  ArrowColor #666666
  StartColor #228B22
  EndColor #8B0000
  DiamondBackgroundColor #E0E0FF
  DiamondBorderColor #0000CD
}
start
:Initialize;
if (ready?) then (yes)
  :Process;
else (no)
  :Wait;
endif
stop
"""))

write("act_skinparam_swimlane", wrap("""
skinparam swimlane {
  BorderColor darkblue
  TitleFontColor white
  TitleBackgroundColor darkblue
  BackgroundColor lightyellow
}
|Lane A|
start
:A action;
|Lane B|
:B action;
stop
"""))

# ─── 18. COMPLEX COMBINATIONS ────────────────────────────────────────────────

write("act_complex_if_in_fork", wrap("""
start
fork
  if (cond1?) then (yes)
    :F1 yes;
  else (no)
    :F1 no;
  endif
fork again
  if (cond2?) then (yes)
    :F2 yes;
  else (no)
    :F2 no;
  endif
fork again
  :F3 simple;
end fork
stop
"""))

write("act_complex_fork_in_if", wrap("""
start
if (parallel?) then (yes)
  fork
    :P1;
  fork again
    :P2;
  end fork
else (no)
  :Sequential;
endif
stop
"""))

write("act_complex_loop_in_fork", wrap("""
start
fork
  while (items?) is (yes)
    :Process;
  endwhile
fork again
  repeat
    :Poll;
  repeatwhile (pending?)
end fork
stop
"""))

write("act_complex_all_in_swimlane", wrap("""
|User|
start
:Login;
|System|
if (authenticated?) then (yes)
  fork
    |Audit|
    :Log access;
    fork again
    |System|
    while (session active?) is (yes)
      :Handle request;
    endwhile
  end fork
else (no)
  :Reject;
endif
|User|
stop
"""))

write("act_complex_deep_nesting", wrap("""
start
partition "Level 1" {
  if (L1?) then (yes)
    while (L2?) is (yes)
      fork
        repeat
          :Deepest action;
        repeatwhile (retry?) is (yes)
      fork again
        :Parallel deep;
      end fork
    endwhile
  else (no)
    :L1 false path;
  endif
}
stop
"""))

write("act_complex_e2e_workflow", wrap("""
title End-to-End Order Processing

|Customer|
start
:Place order;
|Order Service|
:Validate order;
if (valid?) then (yes)
  fork
    |Inventory|
    :Reserve items;
    fork again
    |Payment|
    :Charge customer;
  end fork
  if (all succeeded?) then (yes)
    |Fulfillment|
    :Pack order;
    :Ship order;
    |Customer|
    :Receive confirmation;
    stop
  else (no)
    |Order Service|
    :Rollback;
    |Customer|
    :Notify failure;
    stop
  endif
else (no)
  |Customer|
  :Show validation errors;
  stop
endif
"""))

write("act_complex_retry_pattern", wrap("""
start
:Initialize connection;
repeat
  :Attempt operation;
  if (succeeded?) then (yes)
    :Process result;
    break
  else (no)
    if (retries exceeded?) then (yes)
      :Log fatal error;
      stop
    else (no)
      :Wait before retry;
    endif
  endif
backward :Increment retry count;
repeatwhile (not done?) is (retry)
:Finalize;
stop
"""))

write("act_complex_state_machine_like", wrap("""
start
:IDLE state;
switch (event?)
case (START)
  :RUNNING state;
  while (processing?) is (yes)
    :Handle event;
  endwhile (done)
  :FINISHED state;
case (ABORT)
  :ABORTED state;
case (ERROR)
  :ERROR state;
  :Recover;
endswitch
stop
"""))

# ─── 19. NEWPAGE ─────────────────────────────────────────────────────────────

write("act_newpage_basic", wrap("""
start
:Page 1 action;
newpage
:Page 2 action;
stop
"""))

write("act_newpage_titled", wrap("""
start
:Page 1;
newpage Second Page
:Page 2;
newpage Third Page
:Page 3;
stop
"""))

write("act_newpage_with_if", wrap("""
start
:First action;
if (condition?) then (yes)
  :Yes action;
  newpage
  :Continue yes;
else (no)
  :No action;
endif
stop
"""))

# ─── 20. UNICODE AND SPECIAL CHARS ───────────────────────────────────────────

write("act_unicode_basic", wrap("""
start
:Привет мир;
:你好世界;
:こんにちは;
stop
"""))

write("act_unicode_mixed", wrap("""
start
:Action with émojis and àccents;
:数字 123 and symbols ±≠∞;
stop
"""))

write("act_special_chars", wrap("""
start
:Action with <brackets>;
:Action with (parens);
:Action with [square brackets];
stop
"""))

write("act_long_action_text", wrap("""
start
:This is a very long action text that spans quite a bit of horizontal space to test wrapping and layout behavior in the diagram renderer;
:Another long action: Initialize the system configuration by reading all environment variables and applying defaults where necessary;
stop
"""))

write("act_multiline_action", wrap("""
start
:Line 1\\nLine 2\\nLine 3;
:Action with\\nmultiple lines;
stop
"""))

# ─── 21. LEGACY SYNTAX ───────────────────────────────────────────────────────

write("act_legacy_basic", wrap("""
(*) --> "Action 1"
"Action 1" --> "Action 2"
"Action 2" --> (*)
"""))

write("act_legacy_if", wrap("""
(*) --> "Start"
if "condition" then
  -->[yes] "True branch"
  --> (*)
else
  -->[no] "False branch"
  --> (*)
endif
"""))

write("act_legacy_note", wrap("""
(*) --> "Action"
note right: Legacy note
"Action" --> (*)
"""))

write("act_legacy_partition", wrap("""
partition "Legacy Partition" {
  (*) --> "Action 1"
  "Action 1" --> "Action 2"
  "Action 2" --> (*)
}
"""))

write("act_legacy_colored", wrap("""
(*) --> #red "Red action"
#red "Red action" --> #blue "Blue action"
#blue "Blue action" --> (*)
"""))

write("act_legacy_arrow_labels", wrap("""
(*) --> "Step 1"
"Step 1" -right-> "Step 2"
"Step 2" -down-> "Step 3"
"Step 3" --> (*)
"""))

write("act_legacy_complex", wrap("""
(*) --> "Initialize"
"Initialize" --> "Check input"
if "valid?" then
  -->[yes] "Process"
  --> "Save"
  --> (*)
else
  -->[no] "Show error"
  --> "Check input"
endif
"""))

# ─── 22. DETACH AND KILL ─────────────────────────────────────────────────────

write("act_detach_basic", wrap("""
start
:Action;
detach
"""))

write("act_detach_in_if", wrap("""
start
if (fatal?) then (yes)
  :Log error;
  detach
else (no)
  :Continue;
endif
stop
"""))

write("act_kill_basic", wrap("""
start
:Action;
kill
"""))

write("act_kill_in_if", wrap("""
start
if (abort?) then (yes)
  kill
else (no)
  :Process;
  stop
endif
"""))

# ─── 23. COMBINATORIAL VARIANTS ──────────────────────────────────────────────

# if + while combinations
for i, (if_cond, while_cond) in enumerate([
    ("ready?", "more?"),
    ("valid?", "not done?"),
    ("enabled?", "active?"),
    ("success?", "retry?"),
    ("configured?", "running?"),
]):
    write(f"act_combo_if_while_{i+1}", wrap(f"""
start
if ({if_cond}) then (yes)
  while ({while_cond}) is (yes)
    :Process;
  endwhile (done)
else (no)
  :Skip;
endif
stop
"""))

# if + fork combinations
for i, branches in enumerate([2, 3, 4]):
    fork_body = "\nfork\n  :Branch 1;\n"
    for j in range(2, branches+1):
        fork_body += f"fork again\n  :Branch {j};\n"
    fork_body += "end fork\n"
    write(f"act_combo_if_fork_{i+1}", wrap(f"""
start
if (parallel?) then (yes)
  {fork_body}
else (no)
  :Sequential;
endif
stop
"""))

# while + fork combinations
for i in range(1, 4):
    fork_body = "  fork\n"
    for j in range(1, i+2):
        if j > 1:
            fork_body += "  fork again\n"
        fork_body += f"    :Branch {j};\n"
    fork_body += "  end fork\n"
    write(f"act_combo_while_fork_{i}", wrap(f"""
start
while (iter?) is (yes)
{fork_body}  :After fork;
endwhile
stop
"""))

# swimlane + if combinations
swim_configs = [
    (["User", "System"], "user request?"),
    (["Client", "Server", "DB"], "valid?"),
    (["Frontend", "Backend", "Cache", "DB"], "cache hit?"),
]
for i, (lanes, cond) in enumerate(swim_configs):
    body = f"|{lanes[0]}|\nstart\n:Request;\n|{lanes[1]}|\nif ({cond}) then (yes)\n"
    if len(lanes) > 2:
        body += f"  |{lanes[2]}|\n  :Cached response;\n"
    body += "else (no)\n  :Process;\nendif\n"
    body += f"|{lanes[0]}|\nstop\n"
    write(f"act_combo_swimlane_if_{i+1}", wrap(body))

# swimlane + fork combinations
for n in range(2, 5):
    body = ""
    for lane_i in range(n):
        body += f"|Lane{lane_i+1}|\n"
        if lane_i == 0:
            body += "start\nfork\n"
        else:
            body += "fork again\n"
        body += f"  :Task {lane_i+1};\n"
    body += "end fork\nstop\n"
    write(f"act_combo_swimlane_fork_{n}lanes", wrap(body))

# repeat + switch combinations
for n_cases in range(2, 6):
    cases = ""
    for i in range(1, n_cases+1):
        cases += f"case ({i})\n  :Handle {i};\n"
    write(f"act_combo_repeat_switch_{n_cases}cases", wrap(f"""
start
repeat
  switch (event?)
{cases}  endswitch
repeatwhile (more events?) is (yes)
stop
"""))

# partition + swimlane combinations
for n_parts in range(1, 4):
    body = ""
    for p in range(1, n_parts+1):
        body += f'partition "Partition {p}" {{\n  |Lane A|\n  :Action A{p};\n  |Lane B|\n  :Action B{p};\n}}\n'
    write(f"act_combo_partition_swimlane_{n_parts}parts", wrap(f"start\n{body}stop\n"))

# nested loops with different types
write("act_combo_while_in_repeat", wrap("""
start
repeat
  while (inner?) is (yes)
    :Inner;
  endwhile
  :After while;
repeatwhile (outer?) is (yes)
stop
"""))

write("act_combo_repeat_in_while", wrap("""
start
while (outer?) is (yes)
  repeat
    :Inner;
  repeatwhile (inner?) is (yes)
  :After repeat;
endwhile
stop
"""))

write("act_combo_fork_in_fork", wrap("""
start
fork
  fork
    :A1;
  fork again
    :A2;
  end fork
fork again
  fork
    :B1;
  fork again
    :B2;
  end fork
end fork
stop
"""))

# ─── 24. STYLING VARIANTS ────────────────────────────────────────────────────

# All action types with colors
for color_name, color_val in [("red", "#red"), ("blue", "#blue"), ("green", "#green"),
                               ("orange", "#orange"), ("purple", "#purple")]:
    write(f"act_styled_{color_name}_actions", wrap(f"""
start
{color_val}:First action;
{color_val}:Second action;
{color_val}:Third action;
stop
"""))

# Mixed colors
write("act_styled_rainbow", wrap("""
start
#red:Red action;
#orange:Orange action;
#yellow:Yellow action;
#green:Green action;
#blue:Blue action;
#purple:Purple action;
stop
"""))

# ─── 25. EDGE CASES ──────────────────────────────────────────────────────────

write("act_edge_no_start", wrap("""
:Action without explicit start;
stop
"""))

write("act_edge_deep_if_in_swimlane", wrap("""
|Lane A|
start
if (L1?) then (yes)
  if (L2?) then (yes)
    if (L3?) then (yes)
      |Lane B|
      :Deep in lane B;
      |Lane A|
    else (no)
      :L3 false;
    endif
  else (no)
    :L2 false;
  endif
else (no)
  :L1 false;
endif
stop
"""))

write("act_edge_empty_partition", wrap("""
start
partition "Empty" {
}
:After empty partition;
stop
"""))

write("act_edge_single_swimlane", wrap("""
|Solo|
start
:Action;
stop
"""))

write("act_edge_many_connectors", wrap("""
start
(A)
:Step 1;
(B)
:Step 2;
(C)
:Step 3;
(D)
:Step 4;
(E)
:Step 5;
stop
"""))

write("act_edge_action_immediately_after_fork", wrap("""
start
fork
  :Parallel A;
fork again
  :Parallel B;
end fork
:After fork action;
stop
"""))

write("act_edge_if_then_fork", wrap("""
start
if (split?) then (yes)
  fork
    :P1;
  fork again
    :P2;
  end fork
else (no)
  :Single;
endif
stop
"""))

write("act_edge_break_in_if_in_while", wrap("""
start
while (running?) is (yes)
  :Get next;
  if (done?) then (yes)
    break
  endif
  :Process;
endwhile
stop
"""))

write("act_edge_multiple_newpages", wrap("""
start
:P1 action 1;
:P1 action 2;
newpage
:P2 action 1;
if (split?) then (yes)
  :P2 yes;
else (no)
  :P2 no;
endif
newpage
:P3 final;
stop
"""))

# ─── 26. MANY-STEP VARIATIONS ────────────────────────────────────────────────

# pipeline pattern
write("act_pipeline_10_stages", wrap("""
start
:Stage 1: Input;
:Stage 2: Parse;
:Stage 3: Validate;
:Stage 4: Transform;
:Stage 5: Enrich;
:Stage 6: Filter;
:Stage 7: Aggregate;
:Stage 8: Format;
:Stage 9: Output;
:Stage 10: Confirm;
stop
"""))

write("act_pipeline_20_stages", wrap("""
start
:Stage 1;
:Stage 2;
:Stage 3;
:Stage 4;
:Stage 5;
:Stage 6;
:Stage 7;
:Stage 8;
:Stage 9;
:Stage 10;
:Stage 11;
:Stage 12;
:Stage 13;
:Stage 14;
:Stage 15;
:Stage 16;
:Stage 17;
:Stage 18;
:Stage 19;
:Stage 20;
stop
"""))

# alternating if pattern
body = "start\n"
for i in range(1, 11):
    body += f"if (condition {i}?) then (yes)\n  :Action {i}a;\nelse (no)\n  :Action {i}b;\nendif\n"
body += "stop\n"
write("act_alternating_ifs_10", wrap(body))

# alternating while pattern
body = "start\n"
for i in range(1, 6):
    body += f"while (loop {i}?) is (yes)\n  :Loop {i} body;\nendwhile (done)\n"
body += "stop\n"
write("act_sequential_whiles_5", wrap(body))

# alternating repeat pattern
body = "start\n"
for i in range(1, 5):
    body += f"repeat\n  :Repeat {i} body;\nrepeatwhile (more {i}?) is (yes)\n"
body += "stop\n"
write("act_sequential_repeats_4", wrap(body))

# ─── 27. MIXED FEATURE SHOWCASE ──────────────────────────────────────────────

write("act_showcase_order_system", wrap("""
title Order Processing System

|Customer|
start
:Browse catalog;
:Add items to cart;
:Checkout;

|Order Validation|
if (items in stock?) then (yes)
  if (payment valid?) then (yes)
    note right: Payment validated
    :Confirm order;
  else (no)
    :Payment failed;
    |Customer|
    :Retry payment;
    stop
  endif
else (no)
  :Notify out of stock;
  |Customer|
  stop
endif

|Warehouse|
fork
  :Pick items;
  :Pack items;
fork again
  |Shipping|
  :Generate label;
  :Schedule pickup;
end fork

|Shipping|
repeat
  :Attempt delivery;
backward :Reschedule;
repeatwhile (delivered?) is (no)

|Customer|
:Receive package;
stop
"""))

write("act_showcase_ci_cd_pipeline", wrap("""
title CI/CD Pipeline

|Developer|
start
:Push code;

|CI System|
fork
  :Run unit tests;
fork again
  :Run linter;
fork again
  :Build artifacts;
end fork

if (all checks pass?) then (yes)
  |CD System|
  :Deploy to staging;
  :Run integration tests;
  if (staging ok?) then (yes)
    if (manual approval?) then (yes)
      :Deploy to production;
      :Run smoke tests;
      if (smoke ok?) then (yes)
        :Notify success;
        |Developer|
        stop
      else (no)
        :Rollback;
        |Developer|
        :Notify rollback;
        stop
      endif
    else (no)
      :Queue for later;
      stop
    endif
  else (no)
    :Notify staging failure;
    |Developer|
    stop
  endif
else (no)
  :Notify failure;
  |Developer|
  stop
endif
"""))

write("act_showcase_user_auth", wrap("""
title User Authentication Flow

start
:Present login form;
repeat
  :Enter credentials;
  if (credentials valid?) then (yes)
    if (2FA enabled?) then (yes)
      repeat
        :Enter 2FA code;
        if (code valid?) then (yes)
          break
        else (no)
          :Show error;
        endif
      repeatwhile (attempts < 3?) is (yes)
      if (2FA passed?) then (yes)
        :Grant access;
        stop
      else (no)
        :Lock account;
        :Send unlock email;
        stop
      endif
    else (no)
      :Grant access;
      stop
    endif
  else (no)
    :Show login error;
  endif
backward :Increment failure count;
repeatwhile (failures < 5?) is (yes)
:Lock account temporarily;
stop
"""))

# ─── 28. MORE COMBINATORIAL: COLORS + ENDINGS ────────────────────────────────

for color in ["#red", "#green", "#blue", "#yellow"]:
    for ending_name, ending_sym in [("action", ";"), ("send", ">"), ("receive", "<")]:
        cname = color.lstrip("#")
        write(f"act_color_ending_{cname}_{ending_name}", wrap(f"""
start
{color}:Colored {ending_name}{ending_sym}
stop
"""))

# ─── 29. WHILE WITH VARIOUS BREAK PATTERNS ───────────────────────────────────

write("act_while_break_at_start", wrap("""
start
while (run?) is (yes)
  if (immediate exit?) then (yes)
    break
  endif
  :Long process;
endwhile
stop
"""))

write("act_while_break_at_end", wrap("""
start
while (run?) is (yes)
  :Process;
  :Check;
  if (exit?) then (yes)
    break
  endif
endwhile
stop
"""))

write("act_while_break_nested_if", wrap("""
start
while (outer run?) is (yes)
  if (A?) then (yes)
    if (B?) then (yes)
      break
    else (no)
      :Handle B false;
    endif
  else (no)
    :Handle A false;
  endif
endwhile
stop
"""))

# ─── 30. PARTITION COLORS ────────────────────────────────────────────────────

partition_colors = ["#lightblue", "#lightyellow", "#lightgreen", "#lightsalmon",
                    "#lavender", "#honeydew", "#mistyrose", "#aliceblue"]
for i, pc in enumerate(partition_colors):
    cname = pc.lstrip("#")
    write(f"act_partition_color_{cname}", wrap(f"""
start
partition {pc} "Colored {cname}" {{
  :Action in {cname} partition;
  :Another action;
}}
stop
"""))

# ─── 31. SWIMLANE DIRECTION VARIANTS ─────────────────────────────────────────

write("act_swimlane_left_to_right", wrap("""
left to right direction
|Lane A|
start
:A1;
|Lane B|
:B1;
|Lane A|
stop
"""))

# ─── 32. COMPLEX SWIMLANE SCENARIOS ──────────────────────────────────────────

write("act_swimlane_shopping_cart", wrap("""
title Shopping Cart Checkout

|Customer|
start
:View cart;
:Enter shipping address;

|Payment System|
:Request payment details;
|Customer|
:Provide payment;

|Payment System|
if (payment authorized?) then (yes)
  |Inventory|
  fork
    :Reserve item A;
  fork again
    :Reserve item B;
  fork again
    :Reserve item C;
  end fork
  if (all reserved?) then (yes)
    |Fulfillment|
    :Create shipment;
    |Customer|
    :Send confirmation email;
    stop
  else (no)
    |Payment System|
    :Refund payment;
    |Customer|
    :Notify partial failure;
    stop
  endif
else (no)
  |Customer|
  :Payment declined notice;
  stop
endif
"""))

# ─── 33. ELSEIF + NESTED COMBINATIONS ────────────────────────────────────────

write("act_elseif_with_nested_if", wrap("""
start
if (A?) then (yes)
  if (A1?) then (yes)
    :A1 true;
  else (no)
    :A1 false;
  endif
elseif (B?) then (yes)
  if (B1?) then (yes)
    :B1 true;
  else (no)
    :B1 false;
  endif
else (neither)
  :Default;
endif
stop
"""))

write("act_elseif_with_while", wrap("""
start
if (mode == batch?) then (yes)
  while (items?) is (yes)
    :Process batch item;
  endwhile
elseif (mode == stream?) then (yes)
  repeat
    :Process stream item;
  repeatwhile (streaming?) is (yes)
else (idle)
  :Wait for work;
endif
stop
"""))

write("act_elseif_with_fork", wrap("""
start
if (parallel?) then (yes)
  fork
    :P1;
  fork again
    :P2;
  end fork
elseif (sequential?) then (yes)
  :S1;
  :S2;
else (skip)
  :Skip all;
endif
stop
"""))

# ─── 34. SKINPARAM VARIATIONS ────────────────────────────────────────────────

skinparam_styles = [
    ("monochrome", "skinparam monochrome true\n"),
    ("handwritten", "skinparam handwritten true\n"),
    ("shadowing", "skinparam shadowing true\n"),
    ("no_shadowing", "skinparam shadowing false\n"),
    ("rounded", "skinparam roundcorner 20\n"),
]
for style_name, style_code in skinparam_styles:
    write(f"act_skinparam_{style_name}", wrap(f"""
{style_code}start
:Action A;
if (condition?) then (yes)
  :Yes;
else (no)
  :No;
endif
:Action B;
stop
"""))

# ─── 35. TITLE AND LEGEND ────────────────────────────────────────────────────

write("act_with_title", wrap("""
start
:Action;
stop
""", title="My Activity Diagram"))

write("act_with_header_footer", wrap("""
header
  My Organization - Confidential
end header
footer
  Page %page% of %lastpage%
end footer
start
:Action;
stop
"""))

write("act_with_caption", wrap("""
caption Figure 1: Sample Activity Diagram

start
:Action;
stop
"""))

write("act_with_legend", wrap("""
legend right
  | Symbol | Meaning |
  | :action; | Standard action |
  | diamond | Decision |
end legend
start
:Action;
if (condition?) then (yes)
  :Yes;
else (no)
  :No;
endif
stop
"""))

# ─── 36. MULTI-START SCENARIOS ───────────────────────────────────────────────

write("act_multi_start_via_fork", wrap("""
fork
  start
  :Flow 1 action;
  stop
fork again
  start
  :Flow 2 action;
  stop
end fork
"""))

# ─── 37. MORE EDGE CASES ─────────────────────────────────────────────────────

write("act_edge_if_then_stop", wrap("""
start
:Step 1;
if (done?) then (yes)
  stop
else (no)
  :Step 2;
endif
stop
"""))

write("act_edge_while_empty_body", wrap("""
start
while (condition?) is (yes)
endwhile
stop
"""))

write("act_edge_fork_single_action_per_branch", wrap("""
start
fork
  :A;
fork again
  :B;
fork again
  :C;
fork again
  :D;
fork again
  :E;
fork again
  :F;
end fork
stop
"""))

write("act_edge_all_action_endings_in_if", wrap("""
start
if (type?) then (standard)
  :standard action;
elseif (type?) then (send)
  :send action>
elseif (type?) then (receive)
  :receive action<
elseif (type?) then (input)
  :input action/
else (output)
  :output action\\
endif
stop
"""))

write("act_edge_nested_partitions_3_deep", wrap("""
start
partition "L1" {
  :L1 start;
  partition "L2" {
    :L2 start;
    partition "L3" {
      :L3 action;
    }
    :L2 end;
  }
  :L1 end;
}
stop
"""))

# ─── 38. REALISTIC DOMAIN SCENARIOS ──────────────────────────────────────────

write("act_domain_loan_approval", wrap("""
title Loan Approval Process

|Applicant|
start
:Submit application;

|Loan Officer|
:Review application;
if (complete?) then (yes)
  :Credit check;
  if (credit ok?) then (yes)
    :Calculate terms;
    |Applicant|
    :Review offer;
    if (accept?) then (yes)
      |Loan Officer|
      :Process loan;
      |Applicant|
      :Receive funds;
      stop
    else (no)
      :Decline offer;
      stop
    endif
  else (no)
    :Reject application;
    |Applicant|
    :Receive rejection;
    stop
  endif
else (no)
  :Request more info;
  |Applicant|
  :Provide info;
  |Loan Officer|
  :Review application;
  stop
endif
"""))

write("act_domain_support_ticket", wrap("""
title Support Ticket Workflow

|Customer|
start
:Submit ticket;

|Support Bot|
:Auto-categorize;
if (resolved by FAQ?) then (yes)
  :Send FAQ answer;
  |Customer|
  :Rate answer;
  stop
else (no)
  |Support Agent|
  :Assign ticket;
  repeat
    :Investigate;
    if (need more info?) then (yes)
      :Request info;
      |Customer|
      :Provide info;
      |Support Agent|
    else (no)
      :Prepare solution;
      :Test solution;
    endif
  repeatwhile (solved?) is (no)
  :Send solution;
  |Customer|
  :Test solution;
  if (satisfied?) then (yes)
    :Close ticket;
    stop
  else (no)
    :Escalate;
    |Support Agent|
    :Senior review;
    stop
  endif
endif
"""))

write("act_domain_deploy_pipeline", wrap("""
title Deployment Pipeline

start
partition "Build" {
  :Checkout code;
  :Install dependencies;
  :Compile;
  :Run tests;
  if (build success?) then (yes)
  else (no)
    :Notify build failure;
    stop
  endif
}

partition "Package" {
  :Create artifact;
  :Sign artifact;
  :Push to registry;
}

partition "Deploy Staging" {
  :Deploy to staging;
  :Health check;
  :Integration tests;
  if (staging ok?) then (yes)
  else (no)
    :Rollback staging;
    stop
  endif
}

partition "Deploy Production" {
  :Request approval;
  if (approved?) then (yes)
    :Deploy canary;
    :Monitor metrics;
    if (metrics ok?) then (yes)
      :Full rollout;
    else (no)
      :Rollback canary;
      stop
    endif
  else (no)
    :Cancel deployment;
    stop
  endif
}
stop
"""))

# ─── 39. EXTRA COMBINATORIAL: IF DEPTH × LANE COUNT ──────────────────────────

for depth in range(1, 5):
    for lanes in range(1, 4):
        lane_defs = "".join(f"|Lane{j+1}|\n" for j in range(lanes))
        nested_if = ""
        close = ""
        for d in range(depth):
            nested_if += f"{'  '*d}if (cond{d+1}?) then (yes)\n"
            nested_if += f"{'  '*(d+1)}:Action depth {d+1};\n"
            close = f"{'  '*d}else (no)\n{'  '*(d+1)}:Alt {d+1};\n{'  '*d}endif\n" + close
        body = f"{lane_defs}start\n{nested_if}{close}stop\n"
        write(f"act_depth{depth}_lanes{lanes}", wrap(body))

# ─── 40. FORK × BRANCH COUNT × LANE COUNT ────────────────────────────────────

for branches in range(2, 7):
    for lanes in range(1, 4):
        fork_body = "fork\n"
        for b in range(branches):
            if b > 0:
                fork_body += "fork again\n"
            if lanes > 1:
                lane_idx = b % lanes
                fork_body += f"  |Lane{lane_idx+1}|\n"
            fork_body += f"  :Branch {b+1};\n"
        fork_body += "end fork\n"
        lane_header = "".join(f"|Lane{j+1}|\n" for j in range(lanes)) if lanes > 1 else ""
        write(f"act_fork{branches}br_lanes{lanes}", wrap(f"{lane_header}start\n{fork_body}stop\n"))

# ─── 41. WHILE × REPEAT NESTING COMBOS ───────────────────────────────────────

nesting_combos = [
    ("while_repeat", "while (outer?) is (yes)\n  repeat\n    :action;\n  repeatwhile (inner?)\nendwhile\n"),
    ("repeat_while", "repeat\n  while (inner?) is (yes)\n    :action;\n  endwhile\nrepeatwhile (outer?)\n"),
    ("while_while_while", "while (L1?) is (yes)\n  while (L2?) is (yes)\n    while (L3?) is (yes)\n      :deep;\n    endwhile\n  endwhile\nendwhile\n"),
    ("repeat_repeat", "repeat\n  repeat\n    :inner;\n  repeatwhile (inner?)\n  :outer body;\nrepeatwhile (outer?)\n"),
]
for name, body in nesting_combos:
    write(f"act_nest_{name}", wrap(f"start\n{body}stop\n"))

# ─── 42. ACTION ENDING COMBINATIONS IN SEQUENCE ──────────────────────────────

all_endings = [";", "|", "<", ">", "/", "\\", "]"]
# pairs
for i, e1 in enumerate(all_endings):
    for j, e2 in enumerate(all_endings):
        if i != j:
            n1 = ["semi", "fbar", "recv", "send", "inp", "out", "ffin"][i]
            n2 = ["semi", "fbar", "recv", "send", "inp", "out", "ffin"][j]
            write(f"act_ending_seq_{n1}_{n2}", wrap(f"start\n:action1{e1}\n:action2{e2}\nstop\n"))

# ─── 43. PARTITION + FORK ────────────────────────────────────────────────────

for n in range(2, 5):
    fork_lines = "fork\n"
    for i in range(n):
        if i > 0:
            fork_lines += "fork again\n"
        fork_lines += f'  partition "Part {i+1}" {{\n    :Action {i+1};\n  }}\n'
    fork_lines += "end fork\n"
    write(f"act_fork_partitions_{n}", wrap(f"start\n{fork_lines}stop\n"))

# ─── 44. NOTES IN VARIOUS POSITIONS ──────────────────────────────────────────

positions = ["right", "left"]
for pos in positions:
    for ctx in ["action", "if_branch", "after_start"]:
        if ctx == "action":
            body = f"start\n:Action;\nnote {pos}: Note on action\nstop\n"
        elif ctx == "if_branch":
            body = f"start\nif (cond?) then (yes)\n  note {pos}: Note in branch\n  :Yes;\nelse (no)\n  :No;\nendif\nstop\n"
        else:
            body = f"start\nnote {pos}: Note after start\n:Action;\nstop\n"
        write(f"act_note_{pos}_{ctx}", wrap(body))

# ─── 45. SWITCH DEPTH VARIANTS ───────────────────────────────────────────────

for n in range(2, 9):
    cases = "".join(f"case ({i})\n  :Case {i} action;\n" for i in range(1, n+1))
    write(f"act_switch_{n}cases", wrap(f"start\nswitch (val?)\n{cases}endswitch\nstop\n"))

# switch inside while
for n_cases in range(2, 5):
    cases = "".join(f"case ({i})\n  :Handle {i};\n" for i in range(1, n_cases+1))
    write(f"act_switch_in_while_{n_cases}cases", wrap(f"""
start
while (events?) is (yes)
  switch (event type?)
{cases}  endswitch
endwhile
stop
"""))

# switch inside fork
write("act_switch_in_fork", wrap("""
start
fork
  switch (type A?)
  case (1)
    :A1;
  case (2)
    :A2;
  endswitch
fork again
  switch (type B?)
  case (X)
    :BX;
  case (Y)
    :BY;
  endswitch
end fork
stop
"""))

# ─── 46. CREOLE MARKUP IN ACTIONS ────────────────────────────────────────────

write("act_creole_bold", wrap("""
start
:**Bold action**;
stop
"""))

write("act_creole_italic", wrap("""
start
://Italic action//;
stop
"""))

write("act_creole_mixed_markup", wrap("""
start
:**Bold** and //italic// action;
:__underline__ and --strikethrough--;
stop
"""))

write("act_creole_list", wrap("""
start
:Process items\\n* item1\\n* item2\\n* item3;
stop
"""))

# ─── 47. CONDITIONAL LABELS VARIATIONS ───────────────────────────────────────

label_variations = [
    ("yes_no", "yes", "no"),
    ("true_false", "true", "false"),
    ("ok_fail", "ok", "fail"),
    ("continue_stop", "continue", "stop"),
    ("success_error", "success", "error"),
    ("accept_reject", "accept", "reject"),
    ("empty_labels", "", ""),
]
for name, yes_lbl, no_lbl in label_variations:
    yes_part = f" ({yes_lbl})" if yes_lbl else ""
    no_part = f" ({no_lbl})" if no_lbl else ""
    write(f"act_condition_labels_{name}", wrap(f"""
start
if (condition?){yes_part} then{yes_part}
  :True branch;
else{no_part}
  :False branch;
endif
stop
"""))

# ─── 48. DETACH/KILL IN VARIOUS POSITIONS ────────────────────────────────────

for keyword in ["detach", "kill"]:
    write(f"act_{keyword}_in_fork", wrap(f"""
start
fork
  :Branch A;
  {keyword}
fork again
  :Branch B;
  stop
end fork
"""))

    write(f"act_{keyword}_in_while", wrap(f"""
start
while (run?) is (yes)
  :Work;
  if (abort?) then (yes)
    {keyword}
  endif
endwhile
stop
"""))

    write(f"act_{keyword}_in_repeat", wrap(f"""
start
repeat
  :Try;
  if (fatal?) then (yes)
    {keyword}
  endif
repeatwhile (retry?) is (yes)
stop
"""))

# ─── 49. LONG CONDITION TEXT ─────────────────────────────────────────────────

long_conditions = [
    "Is the total amount greater than the configured threshold value?",
    "Has the user provided all required fields in the registration form?",
    "Does the current system load exceed the maximum allowed percentage?",
    "Is the external API service responding within the timeout window?",
]
for i, cond in enumerate(long_conditions):
    write(f"act_long_condition_{i+1}", wrap(f"""
start
if ({cond}) then (yes)
  :Positive path action;
else (no)
  :Negative path action;
endif
stop
"""))

# ─── 50. GROUPS + OTHER FEATURES ─────────────────────────────────────────────

write("act_group_with_swimlane", wrap("""
|Lane A|
start
group "My Group" {
  :Action in group;
  |Lane B|
  :Another in group;
}
|Lane A|
stop
"""))

write("act_group_with_while", wrap("""
start
group "Processing Loop" {
  while (items?) is (yes)
    :Process;
  endwhile
}
stop
"""))

write("act_group_with_fork", wrap("""
start
group "Parallel Work" {
  fork
    :Worker 1;
  fork again
    :Worker 2;
  end fork
}
stop
"""))

# ─── 51. COMPREHENSIVE SWIMLANE CONFIGURATIONS ───────────────────────────────

# swimlane count × feature type
swimlane_counts = range(2, 9)
for n in swimlane_counts:
    lanes = [f"Lane{i+1}" for i in range(n)]
    body = ""
    for i, lane in enumerate(lanes):
        body += f"|{lane}|\n"
        if i == 0:
            body += "start\n"
        body += f":Action in {lane};\n"
    body += "stop\n"
    write(f"act_swimlane_{n}lanes_basic", wrap(body))

# swimlane with multiple actions per lane
for n in range(2, 6):
    lanes = [f"L{i+1}" for i in range(n)]
    body = ""
    for i, lane in enumerate(lanes):
        body += f"|{lane}|\n"
        if i == 0:
            body += "start\n"
        for j in range(1, 4):
            body += f":{lane} step {j};\n"
    body += "stop\n"
    write(f"act_swimlane_{n}lanes_multi_actions", wrap(body))

# ─── 52. COMPREHENSIVE WHILE VARIANTS ────────────────────────────────────────

while_conditions = [
    "not finished?",
    "queue not empty?",
    "retries remaining?",
    "data available?",
    "connection active?",
    "user wants more?",
    "system running?",
    "items pending?",
]
while_labels = [
    ("yes", "no"),
    ("continue", "done"),
    ("loop", "exit"),
    ("process", "stop"),
    ("keep going", "finished"),
]
for cond in while_conditions:
    for yes_l, no_l in while_labels:
        cname = cond.replace(" ", "_").replace("?", "")
        lname = yes_l.replace(" ", "_")
        write(f"act_while_{cname}_{lname}", wrap(f"""
start
while ({cond}) is ({yes_l})
  :Process iteration;
endwhile ({no_l})
stop
"""))

# ─── 53. COMPREHENSIVE IF CONDITION VARIANTS ─────────────────────────────────

conditions = [
    ("ready", "not ready"),
    ("success", "failure"),
    ("valid input", "invalid input"),
    ("user authenticated", "not authenticated"),
    ("data loaded", "data missing"),
    ("cache hit", "cache miss"),
    ("approved", "rejected"),
    ("enabled", "disabled"),
    ("connected", "disconnected"),
    ("healthy", "unhealthy"),
]
for cond, alt in conditions:
    cname = cond.replace(" ", "_")
    write(f"act_if_cond_{cname}", wrap(f"""
start
:Prepare;
if ({cond}?) then ({cond})
  :Handle {cond};
else ({alt})
  :Handle {alt};
endif
:Continue;
stop
"""))

# ─── 54. COMPREHENSIVE FORK PATTERNS ─────────────────────────────────────────

# Fork with varying action counts per branch
for total_branches in range(2, 7):
    for actions_per_branch in range(1, 5):
        fork_body = "fork\n"
        for b in range(total_branches):
            if b > 0:
                fork_body += "fork again\n"
            for a in range(actions_per_branch):
                fork_body += f"  :Branch {b+1} Action {a+1};\n"
        fork_body += "end fork\n"
        write(f"act_fork_{total_branches}br_{actions_per_branch}actions", wrap(f"start\n{fork_body}stop\n"))

# ─── 55. COMPREHENSIVE REPEAT VARIANTS ───────────────────────────────────────

repeat_conditions = [
    "try again?",
    "not converged?",
    "errors remain?",
    "user wants retry?",
    "deadline not reached?",
]
for cond in repeat_conditions:
    cname = cond.replace(" ", "_").replace("?", "")
    write(f"act_repeat_{cname}", wrap(f"""
start
repeat
  :Attempt action;
  :Check result;
repeatwhile ({cond}) is (yes)
stop
"""))

    write(f"act_repeat_backward_{cname}", wrap(f"""
start
repeat
  :Main action;
backward :Reset and prepare;
repeatwhile ({cond}) is (yes)
stop
"""))

# ─── 56. SWITCH WITH COMPLEX CASE BODIES ─────────────────────────────────────

for n_cases in range(2, 7):
    cases = ""
    for i in range(1, n_cases+1):
        cases += f"case ({i})\n  :Init case {i};\n  :Process case {i};\n  :Finalize case {i};\n"
    write(f"act_switch_{n_cases}cases_multi_action", wrap(f"start\nswitch (val?)\n{cases}endswitch\nstop\n"))

# switch with nested ifs
for n_cases in range(2, 5):
    cases = ""
    for i in range(1, n_cases+1):
        cases += f"case ({i})\n  if (sub-cond {i}?) then (yes)\n    :Case {i} yes;\n  else (no)\n    :Case {i} no;\n  endif\n"
    write(f"act_switch_{n_cases}cases_nested_if", wrap(f"start\nswitch (val?)\n{cases}endswitch\nstop\n"))

# ─── 57. PARTITION FEATURE MATRIX ────────────────────────────────────────────

# partitions × content types
content_types = [
    ("sequential", ":Step 1;\n:Step 2;\n:Step 3;\n"),
    ("if_else", "if (cond?) then (yes)\n  :Yes;\nelse (no)\n  :No;\nendif\n"),
    ("while", "while (loop?) is (yes)\n  :Iter;\nendwhile\n"),
    ("fork", "fork\n  :P1;\nfork again\n  :P2;\nend fork\n"),
]
for n_parts in range(1, 5):
    for ctype, cbody in content_types:
        parts = ""
        for p in range(1, n_parts+1):
            parts += f'partition "Part {p}" {{\n{cbody}}}\n'
        write(f"act_partition_{n_parts}parts_{ctype}", wrap(f"start\n{parts}stop\n"))

# ─── 58. NOTE FEATURE MATRIX ─────────────────────────────────────────────────

note_types = ["right", "left"]
note_contents = [
    ("short", "Short note"),
    ("medium", "This is a medium length note\\nspanning two lines"),
    ("long", "This is a longer note\\nwith three lines\\nof content here"),
]
for pos in note_types:
    for ctype, content in note_contents:
        write(f"act_note_{pos}_{ctype}", wrap(f"""
start
:Action;
note {pos}
  {content}
end note
stop
"""))

# notes in different diagram positions
for pos in note_types:
    write(f"act_note_{pos}_before_if", wrap(f"""
start
:Prepare;
note {pos}: Before condition
if (cond?) then (yes)
  :Yes;
else (no)
  :No;
endif
stop
"""))

    write(f"act_note_{pos}_after_fork", wrap(f"""
start
fork
  :P1;
fork again
  :P2;
end fork
note {pos}: After fork join
:Continue;
stop
"""))

# ─── 59. LEGACY SYNTAX COMPREHENSIVE ─────────────────────────────────────────

# Legacy arrow directions
directions = ["right", "left", "up", "down"]
for d in directions:
    write(f"act_legacy_arrow_{d}", wrap(f"""
(*) --> "Start"
"Start" -{d}-> "End"
"End" --> (*)
"""))

# Legacy with notes
write("act_legacy_note_right", wrap("""
(*) --> "Action 1"
note right: Note on action 1
"Action 1" --> "Action 2"
"Action 2" --> (*)
"""))

write("act_legacy_note_left", wrap("""
(*) --> "Action 1"
note left: Note on action 1
"Action 1" --> (*)
"""))

# Legacy complex flow
write("act_legacy_loop", wrap("""
(*) --> "Initialize"
"Initialize" --> "Check condition"
if "condition ok?" then
  -->[yes] "Process"
  --> "Check condition"
else
  -->[no] "Finalize"
  --> (*)
endif
"""))

write("act_legacy_parallel", wrap("""
(*) --> "Start"
"Start" --> ===FORK===
===FORK=== --> "Branch A"
===FORK=== --> "Branch B"
"Branch A" --> ===JOIN===
"Branch B" --> ===JOIN===
===JOIN=== --> "End"
"End" --> (*)
"""))

# ─── 60. COMPLEX NESTED COMBINATIONS ─────────────────────────────────────────

# if inside while inside fork
for fork_branches in range(2, 5):
    fork_body = "fork\n"
    for b in range(fork_branches):
        if b > 0:
            fork_body += "fork again\n"
        fork_body += f"  while (loop {b+1}?) is (yes)\n"
        fork_body += f"    if (inner {b+1}?) then (yes)\n"
        fork_body += f"      :Work {b+1}a;\n"
        fork_body += f"    else (no)\n"
        fork_body += f"      :Work {b+1}b;\n"
        fork_body += f"    endif\n"
        fork_body += f"  endwhile\n"
    fork_body += "end fork\n"
    write(f"act_complex_fork{fork_branches}_while_if", wrap(f"start\n{fork_body}stop\n"))

# swimlane + complex content
lane_names = ["Frontend", "Backend", "Database", "Cache", "Queue"]
for n_lanes in range(2, 6):
    lanes = lane_names[:n_lanes]
    body = f"|{lanes[0]}|\nstart\n:Start action;\n"
    for i in range(1, len(lanes)):
        body += f"|{lanes[i]}|\n"
        body += f":Action in {lanes[i]};\n"
        if i % 2 == 0:
            body += f"if (cond {i}?) then (yes)\n  :Yes {i};\nelse (no)\n  :No {i};\nendif\n"
    body += f"|{lanes[0]}|\nstop\n"
    write(f"act_complex_swimlane_{n_lanes}lanes_ifs", wrap(body))

# ─── 61. SKINPARAM COMPREHENSIVE MATRIX ──────────────────────────────────────

skinparam_activities = [
    ("background", "BackgroundColor", "lightyellow"),
    ("border", "BorderColor", "darkblue"),
    ("font_bold", "FontStyle", "bold"),
    ("font_italic", "FontStyle", "italic"),
    ("font_arial", "FontName", "Arial"),
    ("font_courier", "FontName", "Courier"),
    ("font_size_10", "FontSize", "10"),
    ("font_size_14", "FontSize", "14"),
    ("font_size_18", "FontSize", "18"),
]
for name, param, val in skinparam_activities:
    write(f"act_skinparam_activity_{name}", wrap(f"""
skinparam activity {{
  {param} {val}
}}
start
:Action A;
if (cond?) then (yes)
  :Yes;
else (no)
  :No;
endif
:Action B;
stop
"""))

# ─── 62. CONNECTOR PATTERNS ──────────────────────────────────────────────────

# goto connector patterns
for n_connectors in range(2, 7):
    labels = [chr(65+i) for i in range(n_connectors)]  # A, B, C, ...
    body = "start\n"
    for i, lbl in enumerate(labels):
        body += f"({lbl})\n:Step {i+1};\n"
    body += "stop\n"
    write(f"act_connectors_{n_connectors}", wrap(body))

# ─── 63. ARROW COLOR/STYLE MATRIX ────────────────────────────────────────────

arrow_colors = ["red", "blue", "green", "orange", "purple", "gray", "darkblue", "darkgreen"]
for color in arrow_colors:
    write(f"act_arrow_color_{color}", wrap(f"""
start
:Action 1;
-[#{color}]->
:Action 2;
-[#{color}]-> {color} path;
:Action 3;
stop
"""))

# ─── 64. COMPREHENSIVE COLORING ──────────────────────────────────────────────

all_colors = [
    "red", "green", "blue", "yellow", "orange", "pink", "purple",
    "lightblue", "lightgreen", "lightyellow", "lightsalmon", "lavender",
    "cyan", "magenta", "lime", "teal", "navy", "maroon", "olive",
    "aqua", "fuchsia", "silver", "gray", "darkgreen", "darkblue",
    "tomato", "coral", "salmon", "gold", "khaki", "plum", "violet",
    "AABBCC", "FF5500", "00AA88", "7744FF",
]

for color in all_colors:
    cname = color.lower()
    write(f"act_color_{cname}_action", wrap(f"""
start
#{color}:Colored action;
:Normal action;
stop
"""))

# partitions with all colors
for color in all_colors[:20]:
    cname = color.lower()
    write(f"act_color_{cname}_partition", wrap(f"""
start
partition #{color} "Colored Partition" {{
  :Action inside;
}}
stop
"""))

# swimlanes with colors
for color in all_colors[:15]:
    cname = color.lower()
    write(f"act_color_{cname}_swimlane", wrap(f"""
|#{color}|Colored Lane|
start
:Action;
stop
"""))

# ─── 65. WHILE WITH ACTION COUNTS ────────────────────────────────────────────

for n_actions in range(1, 8):
    actions = "".join(f"  :Action {i+1};\n" for i in range(n_actions))
    write(f"act_while_{n_actions}actions_body", wrap(f"""
start
while (loop?) is (yes)
{actions}endwhile
stop
"""))

# ─── 66. IF THEN WITH VARYING ACTION COUNTS ──────────────────────────────────

for n_yes in range(0, 5):
    for n_no in range(0, 5):
        yes_actions = "".join(f"  :Yes {i+1};\n" for i in range(n_yes))
        no_actions = "".join(f"  :No {i+1};\n" for i in range(n_no))
        write(f"act_if_{n_yes}yes_{n_no}no_actions", wrap(f"""
start
if (cond?) then (yes)
{yes_actions}else (no)
{no_actions}endif
stop
"""))

# ─── 67. REPEAT WITH ACTION COUNTS ───────────────────────────────────────────

for n_actions in range(1, 7):
    actions = "".join(f"  :Action {i+1};\n" for i in range(n_actions))
    write(f"act_repeat_{n_actions}actions_body", wrap(f"""
start
repeat
{actions}repeatwhile (more?) is (yes)
stop
"""))

# ─── 68. PARTITION WITH ACTION COUNTS ────────────────────────────────────────

for n_parts in range(1, 6):
    for n_actions in range(1, 5):
        parts = ""
        for p in range(1, n_parts+1):
            actions = "".join(f"  :Part {p} Action {i+1};\n" for i in range(n_actions))
            parts += f'partition "Partition {p}" {{\n{actions}}}\n'
        write(f"act_partition_{n_parts}parts_{n_actions}actions", wrap(f"start\n{parts}stop\n"))

# ─── 69. SWIMLANE CROSSING PATTERNS ──────────────────────────────────────────

# Pattern: actions alternate between 2 lanes
for n_crossings in range(1, 9):
    body = "|Lane A|\nstart\n"
    for i in range(n_crossings * 2):
        lane = "Lane A" if i % 2 == 0 else "Lane B"
        if i > 0:
            body += f"|{lane}|\n"
        body += f":Action {i+1};\n"
    body += "stop\n"
    write(f"act_swimlane_crossing_{n_crossings}x", wrap(body))

# ─── 70. FORK THEN JOIN PATTERNS ─────────────────────────────────────────────

for pre_actions in range(0, 4):
    for post_actions in range(0, 4):
        pre = "".join(f":Pre {i+1};\n" for i in range(pre_actions))
        post = "".join(f":Post {i+1};\n" for i in range(post_actions))
        write(f"act_fork_pre{pre_actions}_post{post_actions}", wrap(f"""
start
{pre}fork
  :Branch A;
fork again
  :Branch B;
end fork
{post}stop
"""))

# ─── 71. CONDITION NESTING WITH ACTIONS ──────────────────────────────────────

for pre_if in range(0, 3):
    for in_yes in range(1, 4):
        for in_no in range(1, 4):
            pre = "".join(f":Pre {i+1};\n" for i in range(pre_if))
            yes_body = "".join(f"  :Yes {i+1};\n" for i in range(in_yes))
            no_body = "".join(f"  :No {i+1};\n" for i in range(in_no))
            write(f"act_cond_pre{pre_if}_yes{in_yes}_no{in_no}", wrap(f"""
start
{pre}if (condition?) then (yes)
{yes_body}else (no)
{no_body}endif
stop
"""))

# ─── 72. ELSEIF CHAIN LENGTHS ────────────────────────────────────────────────

for chain_len in range(2, 8):
    branches = "if (cond 1?) then (branch 1)\n  :Action 1;\n"
    for i in range(2, chain_len):
        branches += f"elseif (cond {i}?) then (branch {i})\n  :Action {i};\n"
    branches += f"else (default)\n  :Default action;\nendif\n"
    write(f"act_elseif_chain_{chain_len}", wrap(f"start\n{branches}stop\n"))

# ─── 73. MIXED DIAGRAM PATTERNS ──────────────────────────────────────────────

# ETL-like pattern
write("act_pattern_etl", wrap("""
title ETL Process
start
partition "Extract" {
  :Connect to source;
  while (more records?) is (yes)
    :Read batch;
    :Buffer records;
  endwhile
}
partition "Transform" {
  fork
    :Validate records;
  fork again
    :Normalize fields;
  fork again
    :Apply business rules;
  end fork
}
partition "Load" {
  :Begin transaction;
  repeat
    :Write batch;
  repeatwhile (batches remaining?) is (yes)
  if (success?) then (yes)
    :Commit;
  else (no)
    :Rollback;
  endif
}
stop
"""))

# Event-driven pattern
write("act_pattern_event_driven", wrap("""
title Event-Driven Architecture
|Event Source|
start
:Emit event;
|Event Bus|
:Route event;
fork
  |Consumer A|
  :Handle in A;
  if (ack?) then (yes)
    :Acknowledge;
  else (no)
    :Dead letter;
  endif
fork again
  |Consumer B|
  :Handle in B;
  :Store result;
fork again
  |Consumer C|
  :Handle in C;
  :Notify users;
end fork
|Event Bus|
stop
"""))

# Microservice pattern
write("act_pattern_microservice", wrap("""
title Microservice Request Flow
|API Gateway|
start
:Receive request;
:Authenticate;
if (valid token?) then (yes)
  :Rate check;
  if (within limit?) then (yes)
    fork
      |Auth Service|
      :Get permissions;
      fork again
      |Cache|
      :Check cache;
    end fork
    if (cache hit?) then (yes)
      :Return cached;
    else (no)
      |Business Service|
      :Process request;
      |Cache|
      :Store result;
      |Business Service|
      :Return result;
    endif
  else (no)
    :429 Too Many Requests;
  endif
else (no)
  :401 Unauthorized;
endif
|API Gateway|
:Send response;
stop
"""))

# Saga pattern
write("act_pattern_saga", wrap("""
title Distributed Saga Pattern
start
:Start saga;
repeat
  switch (next step?)
  case (step1)
    :Execute step 1;
    if (step1 ok?) then (no)
      :Compensate step 1;
    endif
  case (step2)
    :Execute step 2;
    if (step2 ok?) then (no)
      :Compensate step 2;
      :Compensate step 1;
    endif
  case (step3)
    :Execute step 3;
    if (step3 ok?) then (no)
      :Compensate all;
    endif
  case (done)
    :Saga complete;
    break
  endswitch
repeatwhile (not complete?) is (yes)
stop
"""))

# ─── 74. MORE SWIMLANE COMPLEX PATTERNS ──────────────────────────────────────

write("act_swimlane_approval_workflow", wrap("""
title Approval Workflow
|Requester|
start
:Create request;
:Submit for approval;

|Manager|
:Review request;
if (approved by manager?) then (yes)
  if (amount > threshold?) then (yes)
    |Director|
    :Review request;
    if (approved by director?) then (yes)
      |Finance|
      :Process payment;
      |Requester|
      :Receive approval;
      stop
    else (no)
      |Requester|
      :Receive rejection;
      stop
    endif
  else (no)
    |Finance|
    :Process payment;
    |Requester|
    :Receive approval;
    stop
  endif
else (no)
  |Requester|
  :Receive rejection;
  if (want to revise?) then (yes)
    :Revise request;
    |Manager|
    :Review request;
    stop
  else (no)
    stop
  endif
endif
"""))

write("act_swimlane_restaurant_order", wrap("""
title Restaurant Order Flow
|Customer|
start
:View menu;
:Place order;

|Waiter|
:Receive order;
:Submit to kitchen;

|Kitchen|
fork
  :Prepare starter;
fork again
  :Prepare main;
fork again
  :Prepare drinks;
end fork
:Notify ready;

|Waiter|
:Serve food;

|Customer|
:Eat;
:Request bill;

|Waiter|
:Prepare bill;

|Customer|
if (pay cash?) then (yes)
  :Pay cash;
  |Waiter|
  :Process cash;
else (card)
  :Pay by card;
  |Waiter|
  :Process card payment;
endif

|Customer|
:Leave;
stop
"""))

# ─── 75. DIAGRAM METADATA VARIANTS ───────────────────────────────────────────

# title variations
titles = [
    "Simple Title",
    "Multi Word Title Example",
    "Title with Numbers 123",
    "Very Long Title That Spans A Considerable Amount Of Text",
]
for i, title in enumerate(titles):
    write(f"act_title_variant_{i+1}", wrap(f"start\n:Action;\nstop\n", title=title))

# title + skinparam combinations
write("act_title_with_skinparam", wrap("""
skinparam titleFontSize 20
skinparam titleFontStyle bold
start
:Action;
stop
""", title="Styled Title"))

# ─── 76. REALISTIC TECHNICAL FLOWS ───────────────────────────────────────────

write("act_technical_http_request", wrap("""
title HTTP Request Lifecycle
start
:Client sends request;
:DNS lookup;
if (DNS cached?) then (yes)
  :Use cached IP;
else (no)
  :Query DNS server;
  :Cache IP;
endif
:TCP handshake;
if (HTTPS?) then (yes)
  :TLS negotiation;
  if (cert valid?) then (yes)
  else (no)
    :Certificate error;
    stop
  endif
endif
:Send HTTP request;
:Server processes request;
:Server sends response;
if (redirect?) then (yes)
  :Follow redirect;
else (no)
  :Parse response;
endif
:Render content;
stop
"""))

write("act_technical_db_transaction", wrap("""
title Database Transaction
start
:Open connection;
:Begin transaction;
repeat
  :Execute query;
  if (query failed?) then (yes)
    :Log error;
    if (retryable?) then (yes)
      :Wait;
    else (no)
      :Rollback;
      :Close connection;
      stop
    endif
  endif
repeatwhile (more queries?) is (yes)
if (all succeeded?) then (yes)
  :Commit transaction;
else (no)
  :Rollback transaction;
endif
:Close connection;
stop
"""))

write("act_technical_cache_aside", wrap("""
title Cache-Aside Pattern
start
:Request data(key);
if (cache hit?) then (yes)
  :Return from cache;
  stop
else (no)
  :Query database;
  if (found?) then (yes)
    fork
      :Store in cache;
    fork again
      :Return to caller;
    end fork
    stop
  else (no)
    :Return null;
    stop
  endif
endif
"""))

# ─── 77. MORE LEGACY SYNTAX VARIANTS ─────────────────────────────────────────

write("act_legacy_full_workflow", wrap("""
(*) --> "Start Process"

"Start Process" --> "Validate Input"
if "Is valid?" then
  -->[yes] "Process Data"
  "Process Data" --> "Generate Output"
  "Generate Output" --> "Send Result"
  "Send Result" --> (*)
else
  -->[no] "Log Error"
  "Log Error" --> "Notify User"
  "Notify User" --> (*)
endif
"""))

write("act_legacy_multipath", wrap("""
(*) --> "Entry"
"Entry" --> "Check A"
if "A?" then
  -->[yes] "Path A"
else
  -->[no] "Check B"
  if "B?" then
    -->[yes] "Path B"
  else
    -->[no] "Default Path"
  endif
endif
"Path A" --> "Exit"
"Path B" --> "Exit"
"Default Path" --> "Exit"
"Exit" --> (*)
"""))

write("act_legacy_colored_actions", wrap("""
(*) --> #lightblue "Init"
#lightblue "Init" --> #lightgreen "Process"
#lightgreen "Process" --> #lightyellow "Review"
#lightyellow "Review" --> #lightpink "Finalize"
#lightpink "Finalize" --> (*)
"""))

# ─── 78. COMPREHENSIVE BREAK PATTERNS ────────────────────────────────────────

for loop_type in ["while", "repeat"]:
    for break_pos in ["early", "mid", "late"]:
        if loop_type == "while":
            if break_pos == "early":
                body = "while (run?) is (yes)\n  if (stop?) then (yes)\n    break\n  endif\n  :Work;\n  :More work;\nendwhile\n"
            elif break_pos == "mid":
                body = "while (run?) is (yes)\n  :Pre work;\n  if (stop?) then (yes)\n    break\n  endif\n  :Post work;\nendwhile\n"
            else:
                body = "while (run?) is (yes)\n  :Work;\n  :More work;\n  if (stop?) then (yes)\n    break\n  endif\nendwhile\n"
        else:
            if break_pos == "early":
                body = "repeat\n  if (stop?) then (yes)\n    break\n  endif\n  :Work;\nrepeatwhile (cont?) is (yes)\n"
            elif break_pos == "mid":
                body = "repeat\n  :Pre;\n  if (stop?) then (yes)\n    break\n  endif\n  :Post;\nrepeatwhile (cont?) is (yes)\n"
            else:
                body = "repeat\n  :Work;\n  :More;\n  if (stop?) then (yes)\n    break\n  endif\nrepeatwhile (cont?) is (yes)\n"
        write(f"act_break_{loop_type}_{break_pos}", wrap(f"start\n{body}stop\n"))

# ─── 79. DETACH/KILL COMPREHENSIVE ───────────────────────────────────────────

for keyword in ["detach", "kill"]:
    for parent in ["if_yes", "if_no", "while_body", "repeat_body",
                   "fork_branch1", "fork_branch2", "switch_case"]:
        if parent == "if_yes":
            body = f"if (cond?) then (yes)\n  :{keyword} trigger;\n  {keyword}\nelse (no)\n  :Normal;\nendif\nstop\n"
        elif parent == "if_no":
            body = f"if (cond?) then (yes)\n  :Normal;\nelse (no)\n  :{keyword} trigger;\n  {keyword}\nendif\nstop\n"
        elif parent == "while_body":
            body = f"while (run?) is (yes)\n  if (abort?) then (yes)\n    {keyword}\n  endif\n  :Work;\nendwhile\nstop\n"
        elif parent == "repeat_body":
            body = f"repeat\n  if (abort?) then (yes)\n    {keyword}\n  endif\n  :Work;\nrepeatwhile (more?) is (yes)\nstop\n"
        elif parent == "fork_branch1":
            body = f"fork\n  :Branch A;\n  {keyword}\nfork again\n  :Branch B;\n  stop\nend fork\n"
        elif parent == "fork_branch2":
            body = f"fork\n  :Branch A;\n  stop\nfork again\n  :Branch B;\n  {keyword}\nend fork\n"
        elif parent == "switch_case":
            body = f"switch (val?)\ncase (1)\n  :Case 1;\n  stop\ncase (abort)\n  :{keyword};\n  {keyword}\nendswitch\n"
        write(f"act_{keyword}_{parent}", wrap(f"start\n{body}"))

# ─── 80. UNICODE COMPREHENSIVE ───────────────────────────────────────────────

unicode_samples = [
    ("arabic", "مرحبا بالعالم"),
    ("chinese", "处理数据"),
    ("japanese", "データ処理"),
    ("korean", "데이터 처리"),
    ("russian", "Обработка данных"),
    ("greek", "Επεξεργασία δεδομένων"),
    ("hebrew", "עיבוד נתונים"),
    ("thai", "การประมวลผลข้อมูล"),
    ("emoji", "Process ✓ Complete ✗"),
    ("math", "Compute x² + y² = z²"),
    ("symbols", "Status: ★★★☆☆"),
    ("mixed", "Step 1: Données → Output"),
]
for lang, text in unicode_samples:
    write(f"act_unicode_{lang}", wrap(f"""
start
:{text};
stop
"""))

# ─── 81. BACKWARD LABEL VARIATIONS ───────────────────────────────────────────

backward_labels = [
    "retry",
    "go back",
    "try again",
    "restart",
    "redo",
    "loop back",
    "reattempt after cooling off",
]
for label in backward_labels:
    lname = label.replace(" ", "_")
    write(f"act_backward_{lname}", wrap(f"""
start
repeat
  :Main action;
backward :{label};
repeatwhile (failed?) is (yes)
stop
"""))

# ─── 82. MIXED ENDING TYPES IN DIFFERENT CONTEXTS ────────────────────────────

contexts = ["if", "while", "repeat", "fork"]
for ctx in contexts:
    for ending, sym in [("send", ">"), ("receive", "<"), ("fork_bar", "|")]:
        if ctx == "if":
            body = f"if (cond?) then (yes)\n  :action {ending}{sym}\nelse (no)\n  :normal;\nendif\n"
        elif ctx == "while":
            body = f"while (loop?) is (yes)\n  :action {ending}{sym}\nendwhile\n"
        elif ctx == "repeat":
            body = f"repeat\n  :action {ending}{sym}\nrepeatwhile (more?) is (yes)\n"
        else:
            body = f"fork\n  :action {ending}{sym}\nfork again\n  :normal;\nend fork\n"
        write(f"act_ending_{ending}_in_{ctx}", wrap(f"start\n{body}stop\n"))

# ─── 83. DEEP SWIMLANE + ALL FEATURES ────────────────────────────────────────

write("act_swimlane_all_features", wrap("""
title Comprehensive Swimlane Example

|Customer|
start
:Initiate request;
note right: Customer starts here

|Validator|
:Receive request;
if (schema valid?) then (yes)
  if (data valid?) then (yes)
  else (no)
    :Return validation error;
    |Customer|
    :Handle error;
    stop
  endif
else (no)
  :Return schema error;
  |Customer|
  :Handle error;
  stop
endif

|Processor|
:Queue request;
while (processing?) is (yes)
  :Dequeue;
  switch (request type?)
  case (create)
    :Handle create;
  case (update)
    :Handle update;
  case (delete)
    :Handle delete;
  endswitch
  :Mark complete;
endwhile

fork
  |Notifier|
  :Send notification;
  fork again
  |Auditor|
  :Write audit log;
  fork again
  |Cache|
  :Invalidate cache;
end fork

|Customer|
:Receive result;
stop
"""))

# ─── 84. NEWPAGE WITH VARIOUS CONTENT ────────────────────────────────────────

for n_pages in range(2, 6):
    body = "start\n"
    for page in range(1, n_pages+1):
        if page > 1:
            body += f"newpage Page {page}\n"
        body += f":Page {page} action 1;\n"
        body += f":Page {page} action 2;\n"
        if page < n_pages:
            body += f"if (continue to page {page+1}?) then (yes)\nelse (no)\n  stop\nendif\n"
    body += "stop\n"
    write(f"act_newpage_{n_pages}pages", wrap(body))

# ─── 85. REALISTIC BUSINESS PROCESS FLOWS ────────────────────────────────────

write("act_business_expense_report", wrap("""
title Expense Report Process

|Employee|
start
:Create expense report;
:Attach receipts;
:Submit report;

|Manager|
:Review report;
if (amount <= limit?) then (yes)
  :Approve;
  |Finance|
  :Process reimbursement;
  |Employee|
  :Receive payment;
  stop
else (no)
  if (needs director?) then (yes)
    |Director|
    :Review large expense;
    if (director approves?) then (yes)
      |Finance|
      :Process reimbursement;
      |Employee|
      :Receive payment;
      stop
    else (no)
      |Employee|
      :Report rejected;
      stop
    endif
  else (no)
    |Employee|
    :Report rejected;
    stop
  endif
endif
"""))

write("act_business_onboarding", wrap("""
title Employee Onboarding

|HR|
start
:Create employee record;
:Send welcome email;

fork
  |IT|
  :Create accounts;
  :Setup workstation;
  :Grant system access;
  fork again
  |Facilities|
  :Assign desk;
  :Order equipment;
  fork again
  |HR|
  :Schedule orientation;
  :Prepare paperwork;
end fork

|New Employee|
:Day 1: Orientation;
:Complete paperwork;

fork
  |IT|
  :Verify access;
  fork again
  |Manager|
  :Team introduction;
  :Assign buddy;
end fork

|New Employee|
while (onboarding tasks?) is (remaining)
  :Complete task;
endwhile (done)

|HR|
:30-day check-in;
:Update employee status;
stop
"""))

# ─── 86. DIAGRAM EDGE CASES AND STRESS TESTS ─────────────────────────────────

# maximum depth nesting
write("act_stress_max_nesting", wrap("""
start
if (L1?) then (yes)
  while (L2?) is (yes)
    fork
      repeat
        if (L4?) then (yes)
          while (L5?) is (yes)
            :Deepest action;
          endwhile
        else (no)
          :L4 false;
        endif
      repeatwhile (L3?) is (yes)
    fork again
      :Parallel deep;
    end fork
  endwhile
else (no)
  :L1 false path;
endif
stop
"""))

# Many ifs in sequence
body = "start\n"
for i in range(15):
    body += f"if (condition {i+1}?) then (yes)\n  :Action {i+1};\nelse (no)\n  :Alt {i+1};\nendif\n"
body += "stop\n"
write("act_stress_15_sequential_ifs", wrap(body))

# Deeply chained elseif
body = "start\nif (v == 1?) then (1)\n  :One;\n"
for i in range(2, 11):
    body += f"elseif (v == {i}?) then ({i})\n  :{['Two','Three','Four','Five','Six','Seven','Eight','Nine','Ten'][i-2]};\n"
body += "else (other)\n  :Other;\nendif\nstop\n"
write("act_stress_10_elseif_chain", wrap(body))

# Long sequential with colors
body = "start\n"
all_c = ["#red", "#orange", "#yellow", "#green", "#blue", "#purple", "#pink"]
for i in range(30):
    c = all_c[i % len(all_c)]
    body += f"{c}:Step {i+1};\n"
body += "stop\n"
write("act_stress_30_colored_actions", wrap(body))

# Fork with 8 branches
fork_body = "fork\n"
for i in range(8):
    if i > 0:
        fork_body += "fork again\n"
    fork_body += f"  :Branch {i+1} action;\n"
fork_body += "end fork\n"
write("act_stress_fork_8_branches", wrap(f"start\n{fork_body}stop\n"))

# ─── 87. ACTION COLORS COMPREHENSIVE ─────────────────────────────────────────

# Every named color on every action ending
named_colors = ["red", "blue", "green", "yellow", "orange", "purple", "pink",
               "cyan", "magenta", "lime", "teal", "navy"]
for color in named_colors:
    for ending_name, ending_sym in [("semi", ";"), ("send", ">"), ("receive", "<"),
                                     ("input", "/"), ("output", "\\")]:
        write(f"act_color_{color}_ending_{ending_name}", wrap(f"""
start
#{color}:Action with {ending_name}{ending_sym}
stop
"""))

# ─── 88. GROUPED SCENARIOS ───────────────────────────────────────────────────

# group + various features
for feature in ["if", "while", "repeat", "fork", "switch"]:
    if feature == "if":
        inner = "if (cond?) then (yes)\n  :Yes;\nelse (no)\n  :No;\nendif\n"
    elif feature == "while":
        inner = "while (loop?) is (yes)\n  :Work;\nendwhile\n"
    elif feature == "repeat":
        inner = "repeat\n  :Try;\nrepeatwhile (more?) is (yes)\n"
    elif feature == "fork":
        inner = "fork\n  :P1;\nfork again\n  :P2;\nend fork\n"
    else:
        inner = "switch (val?)\ncase (1)\n  :One;\ncase (2)\n  :Two;\nendswitch\n"
    write(f"act_group_with_{feature}", wrap(f'start\ngroup "Feature Group" {{\n{inner}}}\nstop\n'))

# multiple groups
for n_groups in range(2, 6):
    body = "start\n"
    for g in range(1, n_groups+1):
        body += f'group "Group {g}" {{\n  :Group {g} action;\n}}\n'
    body += "stop\n"
    write(f"act_group_{n_groups}groups", wrap(body))

# ─── 89. HEADER/FOOTER/CAPTION COMBINATIONS ──────────────────────────────────

header_footer_combos = [
    ("header_only", "header\n  Company Name\nend header\n", ""),
    ("footer_only", "", "footer\n  Page %page%\nend footer\n"),
    ("both", "header\n  Company Name\nend header\n", "footer\n  Confidential\nend footer\n"),
    ("multiline_header", "header\n  Line 1\n  Line 2\nend header\n", ""),
]
for name, hdr, ftr in header_footer_combos:
    write(f"act_metadata_{name}", wrap(f"""
{hdr}{ftr}start
:Action;
if (cond?) then (yes)
  :Yes;
else (no)
  :No;
endif
stop
"""))

# ─── 90. FINAL STRESS TESTS ──────────────────────────────────────────────────

# All features in one diagram
write("act_stress_kitchen_sink", wrap("""
title Kitchen Sink Activity Diagram

skinparam activity {
  BackgroundColor lightyellow
  BorderColor navy
}

header
  Stress Test Diagram
end header

|Lane A|
start
note right: Starting point

:Initialize;
#lightblue:Configure;

partition "Setup Phase" {
  fork
    :Load config;
  fork again
    :Connect DB;
  fork again
    :Init cache;
  end fork
}

if (ready?) then (yes)
  while (items?) is (yes)
    switch (item type?)
    case (type1)
      repeat
        :Process type1;
      backward :Retry type1;
      repeatwhile (failed?) is (yes)
    case (type2)
      if (sub-cond?) then (yes)
        |Lane B|
        :Handle in B;
        |Lane A|
      else (no)
        :Handle in A;
      endif
    case (other)
      :Generic handler;
    endswitch
  endwhile (done)

  partition "Finalize" {
    :Commit results;
    :Close connections;
  }

  (DONE)
  :Final step;
  note left: End of processing
  stop

else (no)
  |Lane B|
  :Not ready - cleanup;
  |Lane A|
  detach
endif
"""))

# Minimal valid diagrams of each type
for diagram_type, content in [
    ("just_start_stop", "start\nstop\n"),
    ("just_start_end", "start\nend\n"),
    ("just_start_kill", "start\nkill\n"),
    ("one_action_stop", "start\n:x;\nstop\n"),
    ("one_if_stop", "start\nif (x?) then (yes)\n  :y;\nendif\nstop\n"),
    ("one_while_stop", "start\nwhile (x?) is (yes)\n  :y;\nendwhile\nstop\n"),
    ("one_fork_stop", "start\nfork\n  :a;\nfork again\n  :b;\nend fork\nstop\n"),
    ("one_repeat_stop", "start\nrepeat\n  :x;\nrepeatwhile (y?)\nstop\n"),
    ("one_switch_stop", "start\nswitch (x?)\ncase (a)\n  :aa;\nendswitch\nstop\n"),
]:
    write(f"act_minimal_{diagram_type}", wrap(content))

# ─── 91. WHILE WITH NESTED IF DEPTH MATRIX ───────────────────────────────────

for if_depth in range(1, 5):
    for while_actions in range(1, 4):
        nested = ""
        close = ""
        for d in range(if_depth):
            nested += f"{'  '*d}if (cond{d+1}?) then (yes)\n"
            close = f"{'  '*d}else (no)\n{'  '*(d+1)}:Alt {d+1};\n{'  '*d}endif\n" + close
        inner_actions = "".join(f"  :Action {i+1};\n" for i in range(while_actions))
        write(f"act_while_ifdepth{if_depth}_acts{while_actions}", wrap(
            f"start\nwhile (loop?) is (yes)\n{nested}{inner_actions}{close}endwhile\nstop\n"
        ))

# ─── 92. FORK BRANCH × IF DEPTH MATRIX ──────────────────────────────────────

for branches in range(2, 6):
    for if_depth in range(1, 4):
        fork_body = "fork\n"
        for b in range(branches):
            if b > 0:
                fork_body += "fork again\n"
            nested = ""
            close = ""
            for d in range(if_depth):
                nested += f"  {'  '*d}if (b{b+1}cond{d+1}?) then (yes)\n"
                close = f"  {'  '*d}else (no)\n  {'  '*(d+1)}:B{b+1} alt {d+1};\n  {'  '*d}endif\n" + close
            fork_body += f"  :Branch {b+1} start;\n{nested}  :Branch {b+1} work;\n{close}"
        fork_body += "end fork\n"
        write(f"act_fork{branches}br_ifdepth{if_depth}", wrap(f"start\n{fork_body}stop\n"))

# ─── 93. SWIMLANE × FEATURE MATRIX ──────────────────────────────────────────

features_by_lane = {
    "sequential": lambda n: "".join(f":Action {i+1};\n" for i in range(n)),
    "if_simple": lambda _: "if (cond?) then (yes)\n  :Yes;\nelse (no)\n  :No;\nendif\n",
    "while_simple": lambda _: "while (loop?) is (yes)\n  :Work;\nendwhile\n",
    "fork_simple": lambda _: "fork\n  :P1;\nfork again\n  :P2;\nend fork\n",
    "repeat_simple": lambda _: "repeat\n  :Try;\nrepeatwhile (more?)\n",
}

for n_lanes in range(2, 6):
    for fname, fgen in features_by_lane.items():
        lanes = [f"L{i+1}" for i in range(n_lanes)]
        body = ""
        for i, lane in enumerate(lanes):
            body += f"|{lane}|\n"
            if i == 0:
                body += "start\n"
            body += fgen(2)
        body += "stop\n"
        write(f"act_swimlane{n_lanes}_{fname}", wrap(body))

# ─── 94. PARTITION × SWIMLANE × ACTION COUNTS ────────────────────────────────

for n_parts in range(1, 4):
    for n_lanes in range(2, 4):
        for n_acts in range(1, 4):
            body = "start\n"
            for p in range(1, n_parts+1):
                lanes_defs = "".join(f"|L{j+1}|\n:P{p}L{j+1} action;\n" for j in range(n_lanes))
                body += f'partition "Part {p}" {{\n{lanes_defs}}}\n'
            body += "stop\n"
            write(f"act_part{n_parts}_swim{n_lanes}_acts{n_acts}", wrap(body))

# ─── 95. CONDITIONAL STYLES MATRIX ───────────────────────────────────────────

cond_types = [
    ("diamonds_both_labeled", "if (cond?) then (yes)\n  :Y;\nelse (no)\n  :N;\nendif"),
    ("diamonds_yes_only", "if (cond?) then (yes)\n  :Y;\nelse\n  :N;\nendif"),
    ("diamonds_then_only", "if (cond?) then\n  :Y;\nelse (no)\n  :N;\nendif"),
    ("diamonds_no_labels", "if (cond?) then\n  :Y;\nelse\n  :N;\nendif"),
    ("diamonds_is", "if (cond?) is (value)\n  :Y;\nelse\n  :N;\nendif"),
]
for name, body in cond_types:
    write(f"act_cond_style_{name}", wrap(f"start\n{body}\nstop\n"))

# ─── 96. ARROW STYLE MATRIX ──────────────────────────────────────────────────

arrow_styles = [
    ("solid", "->"),
    ("dashed", "-->"),
    ("solid_labeled", "-> label;"),
    ("dashed_labeled", "--> label;"),
    ("colored_red", "-[#red]->"),
    ("colored_blue", "-[#blue]->"),
    ("colored_labeled", "-[#green]-> green;"),
    ("dotted", "-[dotted]->"),
    ("bold", "-[bold]->"),
    ("hidden", "-[hidden]->"),
]
for name, arrow in arrow_styles:
    write(f"act_arrow_style_{name}", wrap(f"start\n:A;\n{arrow}\n:B;\nstop\n"))

# ─── 97. SKINPARAM FULL MATRIX ────────────────────────────────────────────────

skinparam_pairs = [
    ("monochrome_true", "skinparam monochrome true"),
    ("monochrome_reverse", "skinparam monochrome reverse"),
    ("handwritten_true", "skinparam handwritten true"),
    ("shadowing_true", "skinparam shadowing true"),
    ("shadowing_false", "skinparam shadowing false"),
    ("default_font_size_8", "skinparam defaultFontSize 8"),
    ("default_font_size_12", "skinparam defaultFontSize 12"),
    ("default_font_size_16", "skinparam defaultFontSize 16"),
    ("default_font_size_20", "skinparam defaultFontSize 20"),
    ("roundcorner_5", "skinparam roundcorner 5"),
    ("roundcorner_10", "skinparam roundcorner 10"),
    ("roundcorner_20", "skinparam roundcorner 20"),
    ("roundcorner_40", "skinparam roundcorner 40"),
    ("linecolor_red", "skinparam activityBorderColor red"),
    ("linecolor_blue", "skinparam activityBorderColor blue"),
    ("linecolor_green", "skinparam activityBorderColor green"),
    ("bgcolor_lightyellow", "skinparam activityBackgroundColor lightyellow"),
    ("bgcolor_lightblue", "skinparam activityBackgroundColor lightblue"),
    ("bgcolor_white", "skinparam activityBackgroundColor white"),
    ("startcolor_green", "skinparam activityStartColor green"),
    ("endcolor_red", "skinparam activityEndColor red"),
    ("barcolor_blue", "skinparam activityBarColor blue"),
    ("diamond_bg", "skinparam activityDiamondBackgroundColor lightyellow"),
    ("diamond_border", "skinparam activityDiamondBorderColor darkblue"),
]
for name, param in skinparam_pairs:
    write(f"act_skinparam2_{name}", wrap(f"""
{param}
start
:Action A;
if (cond?) then (yes)
  :Yes;
else (no)
  :No;
endif
while (loop?) is (yes)
  :Work;
endwhile
stop
"""))

# ─── 98. COMPREHENSIVE SWITCH PATTERNS ───────────────────────────────────────

# switch in various nesting levels
for switch_cases in range(2, 7):
    # switch in if
    cases = "".join(f"case ({i})\n    :C{i};\n" for i in range(1, switch_cases+1))
    write(f"act_switch_in_if_{switch_cases}cases", wrap(f"""
start
if (outer?) then (yes)
  switch (val?)
{cases}  endswitch
else (no)
  :Alt;
endif
stop
"""))

    # switch in while
    write(f"act_switch_in_while2_{switch_cases}cases", wrap(f"""
start
while (go?) is (yes)
  switch (event?)
{cases}  endswitch
endwhile
stop
"""))

    # switch in fork
    write(f"act_switch_in_fork2_{switch_cases}cases", wrap(f"""
start
fork
  switch (val?)
{cases}  endswitch
fork again
  :Other branch;
end fork
stop
"""))

# ─── 99. NOTE POSITION MATRIX ────────────────────────────────────────────────

note_positions = ["right", "left"]
note_anchors = [
    ("after_action", ":Action;\n"),
    ("after_if", "if (c?) then (yes)\n  :Y;\nelse (no)\n  :N;\nendif\n"),
    ("after_while", "while (loop?) is (yes)\n  :W;\nendwhile\n"),
    ("after_fork", "fork\n  :A;\nfork again\n  :B;\nend fork\n"),
    ("after_repeat", "repeat\n  :R;\nrepeatwhile (m?)\n"),
]
for pos in note_positions:
    for anchor_name, anchor_body in note_anchors:
        write(f"act_note_{pos}_anchor_{anchor_name}", wrap(f"""
start
{anchor_body}note {pos}: Note after {anchor_name.replace('_', ' ')}
stop
"""))

# ─── 100. CONNECTOR GOTO PATTERNS ────────────────────────────────────────────

# connector used as error handler
for n_connectors in range(1, 6):
    body = "start\n"
    for i in range(1, n_connectors+1):
        body += f":Step {i};\n"
        body += f"if (error?) then (yes)\n  (ERR{i})\nendif\n"
    body += ":Success;\nstop\n"
    for i in range(1, n_connectors+1):
        body += f"(ERR{i})\n:Handle error {i};\nstop\n"
    write(f"act_connector_error_handler_{n_connectors}", wrap(body))

# ─── 101. CREOLE MARKUP IN VARIOUS POSITIONS ─────────────────────────────────

creole_variants = [
    ("bold_italic", ":**Bold** and //italic//;"),
    ("underline", ":__underlined__ text;"),
    ("strikethrough", ":--strikethrough-- text;"),
    ("monospace", ":<font:monospace>code here</font>;"),
    ("color_text", ":<color:red>red text</color>;"),
    ("size_text", ":<size:16>bigger text</size>;"),
    ("link", ":<u>http://example.com</u>;"),
    ("list_items", ":Items:\\n* one\\n* two\\n* three;"),
    ("table", ":| a | b |\\n| 1 | 2 |;"),
    ("mixed_all", ":**bold** //italic// __under__ <color:blue>blue</color>;"),
]
for name, action in creole_variants:
    write(f"act_creole_{name}", wrap(f"start\n{action}\nstop\n"))

# ─── 102. MULTI-DIAGRAM SCENARIOS (MANY VARIANTS) ────────────────────────────

# Varying depths of if-while nesting
for if_depth in range(1, 6):
    for while_depth in range(1, 4):
        body = "start\n"
        indent = ""
        for d in range(if_depth):
            body += f"{indent}if (L{d+1}?) then (yes)\n"
            indent += "  "
        for d in range(while_depth):
            body += f"{indent}while (W{d+1}?) is (yes)\n"
            indent += "  "
        body += f"{indent}:Innermost action;\n"
        for d in range(while_depth):
            indent = indent[2:]
            body += f"{indent}endwhile\n"
        for d in range(if_depth):
            indent = indent[2:]
            body += f"{indent}else (no)\n{indent}  :Alt {d+1};\n{indent}endif\n"
        body += "stop\n"
        write(f"act_if{if_depth}while{while_depth}_nesting", wrap(body))

# Varying depths of repeat inside if
for if_depth in range(1, 5):
    for repeat_count in range(1, 4):
        body = "start\n"
        indent = ""
        for d in range(if_depth):
            body += f"{indent}if (L{d+1}?) then (yes)\n"
            indent += "  "
        for r in range(repeat_count):
            body += f"{indent}repeat\n{indent}  :Repeat {r+1};\n{indent}repeatwhile (m{r+1}?)\n"
        for d in range(if_depth):
            indent = indent[2:]
            body += f"{indent}else (no)\n{indent}  :Alt;\n{indent}endif\n"
        body += "stop\n"
        write(f"act_if{if_depth}_repeats{repeat_count}", wrap(body))

# ─── 103. EVERY COMBINATION OF START/STOP POSITIONS ─────────────────────────

start_ends = [
    ("start_stop", "start\n:A;\nstop\n"),
    ("start_end", "start\n:A;\nend\n"),
    ("start_kill", "start\n:A;\nkill\n"),
    ("start_detach", "start\n:A;\ndetach\n"),
]
for name, body in start_ends:
    for with_if in [False, True]:
        suffix = "_with_if" if with_if else ""
        if with_if:
            term = body.split("\n")[-2]  # stop/end/kill/detach
            content = f"start\nif (c?) then (yes)\n  :Y;\n  {term}\nelse (no)\n  :N;\n  stop\nendif\n"
        else:
            content = body
        write(f"act_{name}{suffix}", wrap(content))

# ─── 104. COMPLEX FORK PATTERNS ──────────────────────────────────────────────

# fork inside if inside while inside swimlane
for n_lanes in range(2, 4):
    for fork_branches in range(2, 5):
        lane_names_local = [f"SL{i+1}" for i in range(n_lanes)]
        fork_body = "fork\n"
        for b in range(fork_branches):
            if b > 0:
                fork_body += "fork again\n"
            lane = lane_names_local[b % n_lanes]
            fork_body += f"  |{lane}|\n  :Branch {b+1};\n"
        fork_body += f"end fork\n"
        body = f"|{lane_names_local[0]}|\nstart\nif (go?) then (yes)\n  while (loop?) is (yes)\n    {fork_body}  endwhile\nelse (no)\n  :Skip;\nendif\nstop\n"
        write(f"act_complex_swim{n_lanes}_fork{fork_branches}_while_if", wrap(body))

# ─── 105. ALL FEATURES WITH COLORS ───────────────────────────────────────────

feature_colors = ["#lightblue", "#lightyellow", "#lightgreen", "#lavender", "#mistyrose"]
for color in feature_colors:
    cname = color.lstrip("#")
    # colored partition with while
    write(f"act_colored_part_while_{cname}", wrap(f"""
start
partition {color} "Colored Partition" {{
  while (loop?) is (yes)
    :Work;
  endwhile
}}
stop
"""))

    # colored group with if
    write(f"act_colored_group_if_{cname}", wrap(f"""
start
group {color} "Colored Group" {{
  if (cond?) then (yes)
    :Yes;
  else (no)
    :No;
  endif
}}
stop
"""))

    # colored swimlane with fork
    write(f"act_colored_swim_fork_{cname}", wrap(f"""
|{color}|Colored Lane|
start
fork
  :P1;
fork again
  :P2;
end fork
stop
"""))

# ─── 106. SWITCH × WHILE × IF COMBOS ─────────────────────────────────────────

for sw_cases in range(2, 5):
    for while_acts in range(1, 4):
        cases = "".join(f"  case ({i})\n    :Handle {i};\n" for i in range(1, sw_cases+1))
        acts = "".join(f"  :Loop act {i+1};\n" for i in range(while_acts))
        write(f"act_sw{sw_cases}_while{while_acts}", wrap(f"""
start
while (events?) is (yes)
  switch (event?)
{cases}  endswitch
{acts}endwhile
stop
"""))

# ─── 107. ELSEIF × ACTIONS PER BRANCH ────────────────────────────────────────

for chain_len in range(2, 6):
    for acts_per_branch in range(1, 4):
        branches = f"if (cond 1?) then (b1)\n"
        for i in range(acts_per_branch):
            branches += f"  :B1 action {i+1};\n"
        for j in range(2, chain_len):
            branches += f"elseif (cond {j}?) then (b{j})\n"
            for i in range(acts_per_branch):
                branches += f"  :B{j} action {i+1};\n"
        branches += "else (default)\n"
        for i in range(acts_per_branch):
            branches += f"  :Default action {i+1};\n"
        branches += "endif\n"
        write(f"act_elseif_chain{chain_len}_acts{acts_per_branch}", wrap(f"start\n{branches}stop\n"))

# ─── 108. PARTITIONS WITH CONDITIONALS AT EACH DEPTH ─────────────────────────

for part_count in range(1, 5):
    for cond_depth in range(1, 4):
        parts = ""
        for p in range(1, part_count+1):
            nested = ""
            close = ""
            for d in range(cond_depth):
                nested += f"{'  '*d}if (p{p}cond{d+1}?) then (yes)\n"
                close = f"{'  '*d}else (no)\n{'  '*(d+1)}:P{p} alt {d+1};\n{'  '*d}endif\n" + close
            inner_act = f"{'  '*cond_depth}:P{p} inner action;\n"
            parts += f'partition "P{p}" {{\n{nested}{inner_act}{close}}}\n'
        write(f"act_parts{part_count}_conddepth{cond_depth}", wrap(f"start\n{parts}stop\n"))

# ─── 109. PURE FORK-HEAVY DIAGRAMS ───────────────────────────────────────────

# sequential forks
for n_forks in range(2, 6):
    body = "start\n"
    for f in range(n_forks):
        body += f"fork\n  :Fork {f+1} Branch A;\nfork again\n  :Fork {f+1} Branch B;\nend fork\n"
    body += "stop\n"
    write(f"act_sequential_{n_forks}forks", wrap(body))

# ─── 110. REALISTIC ALGORITHM FLOWS ──────────────────────────────────────────

write("act_algo_binary_search", wrap("""
title Binary Search Algorithm
start
:Initialize low = 0, high = n-1;
while (low <= high?) is (yes)
  :mid = (low + high) / 2;
  if (arr[mid] == target?) then (yes)
    :Return mid;
    stop
  elseif (arr[mid] < target?) then (yes)
    :low = mid + 1;
  else (no)
    :high = mid - 1;
  endif
endwhile (no)
:Return -1 (not found);
stop
"""))

write("act_algo_quicksort", wrap("""
title Quicksort Algorithm
start
if (array size <= 1?) then (yes)
  :Return array;
  stop
endif
:Choose pivot;
fork
  :Partition left (< pivot);
fork again
  :Partition right (> pivot);
end fork
fork
  :Recursively sort left;
fork again
  :Recursively sort right;
end fork
:Combine: left + pivot + right;
stop
"""))

write("act_algo_bfs", wrap("""
title Breadth-First Search
start
:Initialize queue with start node;
:Mark start as visited;
while (queue not empty?) is (yes)
  :Dequeue node;
  if (node == target?) then (yes)
    :Return found;
    stop
  endif
  :Get unvisited neighbors;
  while (more neighbors?) is (yes)
    :Enqueue neighbor;
    :Mark as visited;
  endwhile
endwhile (empty)
:Return not found;
stop
"""))

write("act_algo_fibonacci", wrap("""
title Fibonacci with Memoization
start
if (n <= 1?) then (yes)
  :Return n;
  stop
endif
if (memo[n] exists?) then (yes)
  :Return memo[n];
  stop
endif
:result = fib(n-1) + fib(n-2);
:Store memo[n] = result;
:Return result;
stop
"""))

write("act_algo_merge_sort", wrap("""
title Merge Sort
start
if (size <= 1?) then (yes)
  :Return as-is;
  stop
endif
:Find midpoint;
fork
  :Sort left half recursively;
fork again
  :Sort right half recursively;
end fork
:Merge sorted halves;
:i = 0, j = 0, k = 0;
while (i < left.size AND j < right.size?) is (yes)
  if (left[i] <= right[j]?) then (yes)
    :result[k] = left[i]; i++;
  else (no)
    :result[k] = right[j]; j++;
  endif
  :k++;
endwhile
:Copy remaining left elements;
:Copy remaining right elements;
stop
"""))

# ─── 111. MANY SMALL SINGLE-FEATURE DIAGRAMS ─────────────────────────────────

# every skinparam color option
color_options = [
    "red", "blue", "green", "yellow", "orange", "purple", "pink", "cyan",
    "white", "black", "gray", "lightgray", "darkgray", "silver",
    "lightblue", "lightgreen", "lightyellow", "lightcyan", "lavender",
    "navy", "maroon", "olive", "teal", "aqua", "fuchsia", "lime",
    "coral", "salmon", "khaki", "plum", "tomato", "gold",
]
bg_params = ["activityBackgroundColor", "activityBorderColor", "activityArrowColor"]
for param in bg_params:
    for color in color_options[:15]:  # limit to avoid too many
        pname = param.replace("activity", "").lower()
        write(f"act_skinparam3_{pname}_{color}", wrap(f"""
skinparam {param} {color}
start
:Test action;
stop
"""))

# ─── 112. REPEAT × IF DEPTH × BACKWARD ──────────────────────────────────────

for if_depth in range(1, 4):
    for has_backward in [False, True]:
        nested = ""
        close = ""
        for d in range(if_depth):
            nested += f"  {'  '*d}if (cond{d+1}?) then (yes)\n"
            close = f"  {'  '*d}else (no)\n  {'  '*(d+1)}:Alt {d+1};\n  {'  '*d}endif\n" + close
        backward_line = "backward :Retry;\n" if has_backward else ""
        bsuffix = "_backward" if has_backward else ""
        write(f"act_repeat_ifdepth{if_depth}{bsuffix}", wrap(f"""
start
repeat
{nested}  :{'  '*if_depth}Action;
{close}{backward_line}repeatwhile (more?) is (yes)
stop
"""))

# ─── 113. LARGE REALISTIC DIAGRAMS ───────────────────────────────────────────

write("act_large_kubernetes_deploy", wrap("""
title Kubernetes Deployment Process

|Developer|
start
:Write application code;
:Write Dockerfile;
:Write K8s manifests;

|CI Pipeline|
fork
  :Build Docker image;
fork again
  :Run unit tests;
fork again
  :Security scan;
end fork

if (all checks pass?) then (yes)
  :Push image to registry;
else (no)
  |Developer|
  :Fix issues;
  stop
endif

|CD Pipeline|
:Update manifest with new image tag;

|Kubernetes|
:Apply manifests;
:Create new ReplicaSet;

repeat
  :Check pod status;
  if (pods ready?) then (yes)
    break
  endif
  if (timeout?) then (yes)
    :Rollback deployment;
    |Developer|
    :Investigate failure;
    stop
  endif
backward :Wait 10 seconds;
repeatwhile (not done?) is (waiting)

:Update Service to new pods;
:Delete old ReplicaSet;

|Monitoring|
fork
  :Check application metrics;
fork again
  :Check error rates;
fork again
  :Check latency;
end fork

if (metrics healthy?) then (yes)
  |Developer|
  :Deployment successful;
  stop
else (no)
  |CD Pipeline|
  :Trigger rollback;
  |Developer|
  :Investigate;
  stop
endif
"""))

write("act_large_ml_pipeline", wrap("""
title Machine Learning Training Pipeline

start
partition "Data Collection" {
  :Identify data sources;
  while (more sources?) is (yes)
    :Fetch data from source;
    :Validate data format;
    if (valid?) then (yes)
      :Add to dataset;
    else (no)
      :Log invalid data;
      :Skip record;
    endif
  endwhile
}

partition "Preprocessing" {
  fork
    :Handle missing values;
  fork again
    :Normalize features;
  fork again
    :Encode categorical vars;
  end fork
  :Split train/val/test;
}

partition "Training" {
  :Initialize model;
  :Set hyperparameters;
  repeat
    :Train epoch;
    :Evaluate on validation set;
    if (validation improved?) then (yes)
      :Save checkpoint;
    endif
    :Adjust learning rate;
  repeatwhile (not converged?) is (yes)
}

partition "Evaluation" {
  :Load best checkpoint;
  :Evaluate on test set;
  fork
    :Calculate accuracy;
  fork again
    :Calculate F1 score;
  fork again
    :Calculate AUC-ROC;
  end fork
  if (meets threshold?) then (yes)
    :Promote model;
  else (no)
    :Archive results;
    :Schedule next experiment;
  endif
}
stop
"""))

# ─── 114. COMBINATORIAL: ACTIONS BEFORE/AFTER CONTROL STRUCTURES ─────────────

control_structures = [
    ("if", "if (c?) then (yes)\n  :Y;\nelse (no)\n  :N;\nendif\n"),
    ("while", "while (l?) is (yes)\n  :W;\nendwhile\n"),
    ("fork", "fork\n  :A;\nfork again\n  :B;\nend fork\n"),
    ("repeat", "repeat\n  :R;\nrepeatwhile (m?)\n"),
    ("switch", "switch (v?)\ncase (1)\n  :One;\ncase (2)\n  :Two;\nendswitch\n"),
]
for pre in range(0, 4):
    for post in range(0, 4):
        for cs_name, cs_body in control_structures:
            pre_acts = "".join(f":Pre {i+1};\n" for i in range(pre))
            post_acts = "".join(f":Post {i+1};\n" for i in range(post))
            write(f"act_{cs_name}_pre{pre}_post{post}", wrap(f"start\n{pre_acts}{cs_body}{post_acts}stop\n"))

# ─── 115. BACKWARD WITHIN COMPLEX STRUCTURES ─────────────────────────────────

write("act_backward_in_swimlane", wrap("""
|Lane A|
start
repeat
  :Try in A;
  |Lane B|
  :Process in B;
  if (ok?) then (yes)
    break
  endif
  |Lane A|
backward :Reset;
repeatwhile (retry?) is (yes)
stop
"""))

write("act_backward_multiple", wrap("""
start
repeat
  :Step 1;
  :Step 2;
  if (ok?) then (yes)
    break
  endif
backward
  :Cleanup;
  :Log failure;
  :Reset state;
repeatwhile (attempts?) is (yes)
stop
"""))

# ─── 116. EDGE CASES: EMPTY/MINIMAL BRANCHES ─────────────────────────────────

for n_branches in range(2, 6):
    cases = ""
    for i in range(n_branches):
        cases += f"case ({i+1})\n"
        if i % 2 == 0:  # even cases have actions, odd are empty
            cases += f"  :Case {i+1} action;\n"
    write(f"act_switch_mixed_empty_cases_{n_branches}", wrap(f"start\nswitch (v?)\n{cases}endswitch\nstop\n"))

# empty fork branches
for empty_branch in range(2):
    fork_body = "fork\n"
    for b in range(3):
        if b > 0:
            fork_body += "fork again\n"
        if b != empty_branch:
            fork_body += f"  :Branch {b+1};\n"
    fork_body += "end fork\n"
    write(f"act_fork_with_empty_branch_{empty_branch}", wrap(f"start\n{fork_body}stop\n"))

# ─── 117. NEWPAGE IN COMPLEX DIAGRAMS ────────────────────────────────────────

write("act_newpage_in_while", wrap("""
start
:Pre-loop setup;
while (items?) is (yes)
  :Process batch;
  newpage Next Batch
  :Continue processing;
endwhile
:Post-loop cleanup;
stop
"""))

write("act_newpage_in_swimlane", wrap("""
|Lane A|
start
:Page 1 action A;
|Lane B|
:Page 1 action B;
newpage Page 2
|Lane A|
:Page 2 action A;
|Lane B|
:Page 2 action B;
stop
"""))

write("act_newpage_in_partition", wrap("""
start
partition "Part 1" {
  :Action 1;
  newpage
  :Action 2;
}
partition "Part 2" {
  :Action 3;
}
stop
"""))

# ─── 118. COMBINATORIAL: COLORS × FEATURES ───────────────────────────────────

colors_subset = ["#red", "#blue", "#green", "#yellow", "#orange"]
for color in colors_subset:
    cname = color.lstrip("#")

    # colored action before if
    write(f"act_color_{cname}_before_if", wrap(f"""
start
{color}:Colored action;
if (after?) then (yes)
  :Yes;
else (no)
  :No;
endif
stop
"""))

    # colored action in while
    write(f"act_color_{cname}_in_while", wrap(f"""
start
while (loop?) is (yes)
  {color}:Colored in loop;
endwhile
stop
"""))

    # colored action in fork
    write(f"act_color_{cname}_in_fork", wrap(f"""
start
fork
  {color}:Colored branch;
fork again
  :Normal branch;
end fork
stop
"""))

    # colored action in swimlane
    write(f"act_color_{cname}_in_swimlane", wrap(f"""
|Lane1|
start
{color}:Colored in lane;
|Lane2|
:Normal in lane;
stop
"""))

# ─── 119. FLOATING NOTES ─────────────────────────────────────────────────────

note_styles = [
    ("right", "floating note right: Floating right note"),
    ("left", "floating note left: Floating left note"),
    ("right_colored", "floating note right #yellow: Yellow floating note"),
    ("left_colored", "floating note left #lightblue: Blue floating note"),
]
for name, note_line in note_styles:
    write(f"act_floating_note_{name}", wrap(f"""
start
{note_line}
:Action;
stop
"""))

# multi-line floating notes
write("act_floating_note_multiline", wrap("""
start
floating note right
  This is a multiline
  floating note
  with three lines
end note
:Action;
stop
"""))

# ─── 120. MIXED FORMAT LEGACY+NEW ────────────────────────────────────────────

# NOTE: PlantUML doesn't truly mix, but test boundary cases
write("act_legacy_with_colors", wrap("""
(*) --> #lightblue "Colored Start"
#lightblue "Colored Start" --> #lightgreen "Colored Middle"
#lightgreen "Colored Middle" --> #lightyellow "Colored End"
#lightyellow "Colored End" --> (*)
"""))

write("act_legacy_with_notes", wrap("""
(*) --> "Action 1"
note right: First note
"Action 1" --> "Action 2"
note left: Second note
"Action 2" --> (*)
"""))

write("act_legacy_deep_if", wrap("""
(*) --> "Start"
if "outer?" then
  -->[yes] "Outer true"
  if "inner?" then
    -->[yes] "Both true"
    --> (*)
  else
    -->[no] "Outer true inner false"
    --> (*)
  endif
else
  -->[no] "Outer false"
  --> (*)
endif
"""))

# ─── 121. EXHAUSTIVE WHILE CONDITION × BODY × LABEL MATRIX ──────────────────

while_body_variants = [
    ("single", ":Work;\n"),
    ("double", ":Work 1;\n:Work 2;\n"),
    ("triple", ":Work 1;\n:Work 2;\n:Work 3;\n"),
    ("with_if", "if (inner?) then (yes)\n  :Inner yes;\nelse (no)\n  :Inner no;\nendif\n"),
]
while_label_variants = [
    ("labeled", " is (yes)", " (done)"),
    ("unlabeled", "", ""),
    ("custom", " is (continue)", " (finished)"),
]
for bname, body in while_body_variants:
    for lname, is_label, end_label in while_label_variants:
        write(f"act_while_body_{bname}_label_{lname}", wrap(f"""
start
while (condition?){is_label}
{body}endwhile{end_label}
stop
"""))

# ─── 122. FORK BEFORE/AFTER ACTION MATRIX ────────────────────────────────────

for pre in range(0, 4):
    for post in range(0, 4):
        for branches in range(2, 5):
            pre_acts = "".join(f":Pre {i+1};\n" for i in range(pre))
            post_acts = "".join(f":Post {i+1};\n" for i in range(post))
            fork_body = "fork\n"
            for b in range(branches):
                if b > 0:
                    fork_body += "fork again\n"
                fork_body += f"  :Branch {b+1};\n"
            fork_body += "end fork\n"
            write(f"act_fork_pre{pre}_post{post}_br{branches}", wrap(f"start\n{pre_acts}{fork_body}{post_acts}stop\n"))

# ─── 123. COMPREHENSIVE SWIMLANE ACTION DISTRIBUTION ─────────────────────────

# Distribute different numbers of actions across lanes
for n_lanes in range(2, 5):
    for acts_per_lane in range(1, 5):
        body = ""
        for lane_i in range(n_lanes):
            body += f"|Lane{lane_i+1}|\n"
            if lane_i == 0:
                body += "start\n"
            for act_i in range(acts_per_lane):
                body += f":L{lane_i+1} action {act_i+1};\n"
        body += "stop\n"
        write(f"act_swim{n_lanes}_acts{acts_per_lane}", wrap(body))

# ─── 124. PARTITION × BODY VARIANTS ─────────────────────────────────────────

partition_bodies = [
    ("if", "if (c?) then (yes)\n  :Y;\nelse (no)\n  :N;\nendif\n"),
    ("while", "while (l?) is (yes)\n  :W;\nendwhile\n"),
    ("fork2", "fork\n  :A;\nfork again\n  :B;\nend fork\n"),
    ("fork3", "fork\n  :A;\nfork again\n  :B;\nfork again\n  :C;\nend fork\n"),
    ("repeat", "repeat\n  :R;\nrepeatwhile (m?)\n"),
    ("switch2", "switch (v?)\ncase (1)\n  :One;\ncase (2)\n  :Two;\nendswitch\n"),
    ("seq3", ":A;\n:B;\n:C;\n"),
]
for n_parts in range(1, 4):
    for bname, bbody in partition_bodies:
        parts = "".join(f'partition "P{p}" {{\n{bbody}}}\n' for p in range(1, n_parts+1))
        write(f"act_parts{n_parts}_body_{bname}", wrap(f"start\n{parts}stop\n"))

# ─── 125. ELSEIF × SWIMLANE COMBINATIONS ─────────────────────────────────────

for chain_len in range(2, 5):
    for n_lanes in range(2, 4):
        lane_header = f"|Lane1|\n"
        body = lane_header + "start\n"
        body += f"if (cond 1?) then (b1)\n"
        body += f"  :Branch 1;\n"
        for j in range(2, chain_len):
            body += f"elseif (cond {j}?) then (b{j})\n"
            if n_lanes > 1:
                body += f"  |Lane{(j % n_lanes) + 1}|\n"
            body += f"  :Branch {j};\n"
        body += "else (default)\n  :Default;\nendif\n"
        body += "stop\n"
        write(f"act_elseif{chain_len}_swim{n_lanes}", wrap(body))

# ─── 126. COMPREHENSIVE NOTE COLORS ──────────────────────────────────────────

note_colors = ["#yellow", "#lightblue", "#lightgreen", "#pink", "#lavender",
               "#mistyrose", "#honeydew", "#FFFACD", "#E0E0FF", "#FFE4E1"]
for color in note_colors:
    cname = color.lstrip("#").lower()
    for pos in ["right", "left"]:
        write(f"act_note_color_{cname}_{pos}", wrap(f"""
start
:Action;
note {pos} {color}
  Colored note
end note
stop
"""))

# ─── 127. ACTION ENDING × CONTEXT MATRIX ─────────────────────────────────────

endings_short = [
    ("semi", ";"), ("send", ">"), ("recv", "<"),
    ("inp", "/"), ("out", "\\"), ("ffin", "]"), ("fbar", "|"),
]
contexts_simple = ["sequential", "if_yes", "if_no", "while", "repeat", "fork", "partition"]

for ename, esym in endings_short:
    for ctx in contexts_simple:
        if ctx == "sequential":
            body = f":prev action;\n:this action{esym}\n:next action;\n"
        elif ctx == "if_yes":
            body = f"if (c?) then (yes)\n  :action{esym}\nelse (no)\n  :alt;\nendif\n"
        elif ctx == "if_no":
            body = f"if (c?) then (yes)\n  :alt;\nelse (no)\n  :action{esym}\nendif\n"
        elif ctx == "while":
            body = f"while (l?) is (yes)\n  :action{esym}\nendwhile\n"
        elif ctx == "repeat":
            body = f"repeat\n  :action{esym}\nrepeatwhile (m?)\n"
        elif ctx == "fork":
            body = f"fork\n  :action{esym}\nfork again\n  :other;\nend fork\n"
        else:  # partition
            body = f'partition "P" {{\n  :action{esym}\n}}\n'
        write(f"act_ending_{ename}_ctx_{ctx}", wrap(f"start\n{body}stop\n"))

# ─── 128. TITLE VARIATIONS ───────────────────────────────────────────────────

title_variants = [
    ("short", "Title"),
    ("spaces", "Title With Spaces"),
    ("numbers", "Process 42 Steps"),
    ("special_chars", "Process: A → B"),
    ("long", "This Is A Very Long Title For The Activity Diagram That Tests Layout"),
    ("unicode", "Processus de données"),
    ("lowercase", "lowercase title"),
    ("uppercase", "UPPERCASE TITLE"),
    ("mixed_case", "Mixed Case Title Example"),
    ("with_colon", "Section: Subsection"),
]
for tname, title_text in title_variants:
    write(f"act_title_{tname}", wrap(f"start\n:Action;\nstop\n", title=title_text))

# ─── 129. COMPREHENSIVE DETACH/KILL IN ALL CONTROL STRUCTURES ────────────────

for keyword in ["detach", "kill"]:
    # In switch case
    for n_cases in range(2, 5):
        cases = ""
        for i in range(1, n_cases+1):
            cases += f"case ({i})\n"
            if i == n_cases:  # last case uses keyword
                cases += f"  :{keyword} case;\n  {keyword}\n"
            else:
                cases += f"  :Case {i};\n"
        write(f"act_{keyword}_switch_{n_cases}cases", wrap(f"start\nswitch (v?)\n{cases}endswitch\nstop\n"))

    # In nested if
    for depth in range(1, 4):
        nested = ""
        close = ""
        for d in range(depth):
            nested += f"{'  '*d}if (cond{d+1}?) then (yes)\n"
            close = f"{'  '*d}else (no)\n{'  '*(d+1)}:Alt;\n{'  '*d}endif\n" + close
        inner = f"{'  '*depth}:{keyword} trigger;\n{'  '*depth}{keyword}\n"
        write(f"act_{keyword}_nested_if_depth{depth}", wrap(f"start\n{nested}{inner}{close}stop\n"))

# ─── 130. SWIMLANE COLORED COMPREHENSIVE ─────────────────────────────────────

lane_colors = ["#lightblue", "#lightyellow", "#lightgreen", "#lavender",
               "#mistyrose", "#honeydew", "#aliceblue", "#ivory"]
for n_lanes in range(2, 5):
    colors_used = lane_colors[:n_lanes]
    body = ""
    for i, (color) in enumerate(colors_used):
        lane_name = f"Lane{i+1}"
        body += f"|{color}|{lane_name}|\n"
        if i == 0:
            body += "start\n"
        body += f":Action in {lane_name};\n"
    body += "stop\n"
    write(f"act_colored_swimlane_{n_lanes}lanes", wrap(body))

# ─── 131. REPEAT WITH ALL COMBINATIONS ───────────────────────────────────────

for has_backward in [False, True]:
    for has_break in [False, True]:
        for n_body_acts in range(1, 4):
            body_acts = "".join(f"  :Action {i+1};\n" for i in range(n_body_acts))
            backward_line = "backward :Retry;\n" if has_backward else ""
            break_code = "  if (early exit?) then (yes)\n    break\n  endif\n" if has_break else ""
            bwd = "bwd" if has_backward else "nobwd"
            brk = "brk" if has_break else "nobrk"
            write(f"act_repeat_acts{n_body_acts}_{bwd}_{brk}", wrap(f"""
start
repeat
{body_acts}{break_code}{backward_line}repeatwhile (more?) is (yes)
stop
"""))

# ─── 132. ADDITIONAL EDGE CASES AND MISC ─────────────────────────────────────

# split/sync (synonym for fork/end fork in some versions)
write("act_split_sync", wrap("""
start
split
  :Branch A;
split again
  :Branch B;
end split
stop
"""))

write("act_split_3_branches", wrap("""
start
split
  :A;
split again
  :B;
split again
  :C;
end split
stop
"""))

# detach as last action after a full workflow
write("act_detach_after_full_workflow", wrap("""
start
:Initialize;
while (running?) is (yes)
  :Process;
endwhile
fork
  :Log;
fork again
  :Notify;
end fork
detach
"""))

# kill after fork merge
write("act_kill_after_fork_merge", wrap("""
start
:Begin;
fork
  :Branch A;
fork again
  :Branch B;
end fork
if (fatal error?) then (yes)
  kill
else (no)
  :Continue;
  stop
endif
"""))

# test all action endings with colors
for ename, esym in [("semi", ";"), ("fbar", "|"), ("recv", "<"), ("send", ">")]:
    for color in ["#red", "#blue", "#green"]:
        cname = color.lstrip("#")
        write(f"act_colored_ending_{cname}_{ename}", wrap(f"start\n{color}:colored {ename}{esym}\nstop\n"))

# while with break and backward together
write("act_while_break_and_notes", wrap("""
start
while (running?) is (yes)
  :Do work;
  note right: Working...
  if (done?) then (yes)
    break
  endif
  :Continue working;
endwhile
:Post-loop;
stop
"""))

# swimlane with notes
write("act_swimlane_with_notes", wrap("""
|Lane A|
start
:Action A;
note right: Note in lane A
|Lane B|
:Action B;
note left: Note in lane B
|Lane A|
stop
"""))

# partition with notes
write("act_partition_with_notes", wrap("""
start
partition "Noted Partition" {
  :Action 1;
  note right: Note in partition
  :Action 2;
}
stop
"""))

# complex backward pattern
write("act_backward_complex_pattern", wrap("""
start
:Init;
repeat
  :Attempt;
  if (A fails?) then (yes)
    :Handle A failure;
  else (no)
    if (B fails?) then (yes)
      :Handle B failure;
    else (no)
      :Success;
      break
    endif
  endif
backward
  :Increment counter;
  :Log attempt;
  if (max retries?) then (yes)
    :Fatal failure;
    stop
  endif
repeatwhile (not done?) is (retry)
:Finalize;
stop
"""))

# group inside swimlane
write("act_group_inside_swimlane", wrap("""
|Process Lane|
start
group "Phase 1" {
  :Step A;
  :Step B;
}
group "Phase 2" {
  :Step C;
  :Step D;
}
stop
"""))

# note after partition
write("act_note_after_partition", wrap("""
start
partition "Work" {
  :Action;
}
note right: Note after partition
stop
"""))

# deeply nested groups
write("act_group_triple_nested", wrap("""
start
group "Outer" {
  :Outer start;
  group "Middle" {
    :Middle action;
    group "Inner" {
      :Innermost action;
    }
    :After inner;
  }
  :Outer end;
}
stop
"""))

# ─── Final report ────────────────────────────────────────────────────────────

print(f"Generated {_written} .puml files in {OUT_DIR}")
