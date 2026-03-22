#!/usr/bin/env python3
"""Generate comprehensive PlantUML test cases for object, timing, mindmap, wbs,
json-yaml, gantt, salt, and nwdiag diagram types."""

import os
from pathlib import Path

BASE = Path("/Users/marcelo/work/github.com/marcelocantos/rustuml/test-diagrams/golden")

DIRS = {
    "object": BASE / "object",
    "timing": BASE / "timing",
    "mindmap": BASE / "mindmap",
    "wbs": BASE / "wbs",
    "json-yaml": BASE / "json-yaml",
    "gantt": BASE / "gantt",
    "salt": BASE / "salt",
    "nwdiag": BASE / "nwdiag",
}

for d in DIRS.values():
    d.mkdir(parents=True, exist_ok=True)

def write(path: Path, content: str):
    path.write_text(content.strip() + "\n")

counts = {k: 0 for k in DIRS}

def w(category: str, name: str, content: str):
    write(DIRS[category] / f"{name}.puml", content)
    counts[category] += 1

# ─────────────────────────────────────────────
# OBJECT DIAGRAMS (~200)
# ─────────────────────────────────────────────

# Basic objects
w("object", "obj_basic_empty", """
@startuml
object Car
object Bike
@enduml
""")

w("object", "obj_basic_fields", """
@startuml
object Car {
  make = "Toyota"
  model = "Camry"
  year = 2023
}
@enduml
""")

w("object", "obj_basic_link", """
@startuml
object Person {
  name = "Alice"
}
object Address {
  city = "Wonderland"
}
Person --> Address
@enduml
""")

w("object", "obj_multiple_objects", """
@startuml
object Dog {
  name = "Rex"
  breed = "Labrador"
}
object Cat {
  name = "Whiskers"
  breed = "Persian"
}
object Owner {
  name = "Bob"
}
Owner --> Dog
Owner --> Cat
@enduml
""")

# Links with labels
for link_type, sym in [("association","-->"),("composition","*-->"),("aggregation","o-->"),
                        ("dependency","..>"),("realization","..|>"),("extension","--|>")]:
    w("object", f"obj_link_{link_type}", f"""
@startuml
object A {{
  value = 1
}}
object B {{
  value = 2
}}
A {sym} B : "{link_type}"
@enduml
""")

# Link directions
for direction in ["-->", "<--", "<-->", "--"]:
    safe = direction.replace("-","dash").replace("<","lt").replace(">","gt")
    w("object", f"obj_link_dir_{safe}", f"""
@startuml
object Source
object Target
Source {direction} Target
@enduml
""")

# Colors
COLORS = ["red", "blue", "green", "yellow", "orange", "pink", "cyan", "violet", "lightblue", "lightgreen"]
for i, color in enumerate(COLORS):
    w("object", f"obj_color_{color}", f"""
@startuml
object MyObject #{color} {{
  field = "value"
}}
@enduml
""")

# Stereotypes
for stereo in ["entity", "boundary", "control", "database", "actor"]:
    w("object", f"obj_stereo_{stereo}", f"""
@startuml
object MyObject <<{stereo}>> {{
  id = 1
}}
@enduml
""")

# Notes
w("object", "obj_note_basic", """
@startuml
object Car {
  make = "Ford"
}
note right of Car : This is a car
@enduml
""")

w("object", "obj_note_multiple", """
@startuml
object A {
  x = 1
}
object B {
  y = 2
}
note top of A : Note on A
note bottom of B : Note on B
A --> B
@enduml
""")

w("object", "obj_note_floating", """
@startuml
object Server {
  ip = "192.168.1.1"
}
note "This is a server\\nwith multiple lines" as N1
Server .. N1
@enduml
""")

# Map syntax
w("object", "obj_map_basic", """
@startuml
map "Config" as cfg {
  host => localhost
  port => 8080
  debug => true
}
@enduml
""")

w("object", "obj_map_linked", """
@startuml
map "User" as u {
  name => Alice
  role => admin
}
map "Permissions" as p {
  read => true
  write => true
  delete => false
}
u::role --> p
@enduml
""")

w("object", "obj_map_multiple", """
@startuml
map "Database" as db {
  host => db.example.com
  port => 5432
  name => mydb
}
map "Cache" as c {
  host => cache.example.com
  port => 6379
  ttl => 3600
}
map "App" as app {
  db_url => db.example.com
  cache_url => cache.example.com
}
app::db_url --> db
app::cache_url --> c
@enduml
""")

w("object", "obj_map_nested_ref", """
@startuml
map "Order" as o {
  id => 12345
  customer => Alice
  total => 99.99
}
map "Customer" as c {
  name => Alice
  email => alice@example.com
}
map "Payment" as p {
  method => credit_card
  status => approved
}
o::customer --> c
o --> p
@enduml
""")

# JSON-like values
w("object", "obj_json_values", """
@startuml
object Config {
  name = "myapp"
  version = "1.0.0"
  debug = false
  port = 3000
  tags = ["web", "api"]
}
@enduml
""")

# Complex object graphs
w("object", "obj_graph_chain", """
@startuml
object A { v = 1 }
object B { v = 2 }
object C { v = 3 }
object D { v = 4 }
A --> B
B --> C
C --> D
@enduml
""")

w("object", "obj_graph_star", """
@startuml
object Hub { id = 0 }
object N1 { id = 1 }
object N2 { id = 2 }
object N3 { id = 3 }
object N4 { id = 4 }
Hub --> N1
Hub --> N2
Hub --> N3
Hub --> N4
@enduml
""")

w("object", "obj_graph_bidirectional", """
@startuml
object Parent {
  name = "parent"
}
object Child {
  name = "child"
}
Parent "1" --> "0..*" Child : has
Child "0..*" --> "1" Parent : belongs to
@enduml
""")

w("object", "obj_graph_complex", """
@startuml
object Company {
  name = "Acme Corp"
  founded = 1990
}
object Department {
  name = "Engineering"
  budget = 1000000
}
object Employee {
  name = "Alice"
  role = "Developer"
}
object Project {
  name = "Project X"
  deadline = "2024-12-31"
}
Company "1" --> "0..*" Department : has
Department "1" --> "0..*" Employee : employs
Employee "0..*" --> "0..*" Project : works on
@enduml
""")

# Nested object references
w("object", "obj_nested_refs", """
@startuml
object Root {
  id = 1
}
object Child1 {
  parent_id = 1
}
object Child2 {
  parent_id = 1
}
object GrandChild {
  parent_id = 2
}
Root --> Child1
Root --> Child2
Child1 --> GrandChild
@enduml
""")

# Object with all field types
w("object", "obj_field_types", """
@startuml
object AllTypes {
  string_field = "hello"
  int_field = 42
  float_field = 3.14
  bool_field = true
  null_field = null
  empty_field = ""
}
@enduml
""")

# Multiple links between same objects
w("object", "obj_multi_link", """
@startuml
object A
object B
A --> B : link1
A ..> B : link2
A --* B : link3
@enduml
""")

# Object with stereotype and color
w("object", "obj_stereo_color", """
@startuml
object Server <<infrastructure>> #lightblue {
  hostname = "web01"
  ip = "10.0.0.1"
}
object Database <<storage>> #lightyellow {
  type = "PostgreSQL"
  version = "14"
}
Server --> Database : connects
@enduml
""")

# Large object diagram
w("object", "obj_large_10nodes", """
@startuml
object O1 { v = 1 }
object O2 { v = 2 }
object O3 { v = 3 }
object O4 { v = 4 }
object O5 { v = 5 }
object O6 { v = 6 }
object O7 { v = 7 }
object O8 { v = 8 }
object O9 { v = 9 }
object O10 { v = 10 }
O1 --> O2
O2 --> O3
O3 --> O4
O4 --> O5
O5 --> O6
O6 --> O7
O7 --> O8
O8 --> O9
O9 --> O10
O10 --> O1
@enduml
""")

# Combinatorial: fields + colors + stereo + links
for i in range(20):
    color = COLORS[i % len(COLORS)]
    stereo = ["entity","boundary","control","database","actor"][i % 5]
    w("object", f"obj_combo_{i:02d}", f"""
@startuml
object Obj{i}A <<{stereo}>> #{color} {{
  id = {i}
  name = "obj{i}a"
  active = true
}}
object Obj{i}B #{COLORS[(i+1)%len(COLORS)]} {{
  ref = {i}
  label = "obj{i}b"
}}
Obj{i}A --> Obj{i}B : "ref_{i}"
note right of Obj{i}A : Object variant {i}
@enduml
""")

# Map combos
for i in range(15):
    entries = "\n  ".join([f"key{j} => value{j}" for j in range(i % 5 + 2)])
    w("object", f"obj_map_combo_{i:02d}", f"""
@startuml
map "Map{i}" as m{i} {{
  {entries}
}}
@enduml
""")

# Object with package
w("object", "obj_package", """
@startuml
package "Domain" {
  object User {
    id = 1
    name = "Alice"
  }
  object Role {
    name = "admin"
  }
}
package "Infrastructure" {
  object Database {
    host = "localhost"
  }
}
User --> Role
User --> Database
@enduml
""")

w("object", "obj_package_multi", """
@startuml
package "Frontend" {
  object Browser {
    type = "Chrome"
  }
}
package "Backend" {
  object Server {
    port = 8080
  }
  object Cache {
    type = "Redis"
  }
}
package "Data" {
  object DB {
    engine = "Postgres"
  }
}
Browser --> Server : HTTP
Server --> Cache : read/write
Server --> DB : query
@enduml
""")

# skinparam on objects
w("object", "obj_skinparam", """
@startuml
skinparam object {
  BackgroundColor LightYellow
  BorderColor DarkOrange
  FontSize 14
}
object Vehicle {
  type = "car"
  speed = 120
}
object Driver {
  name = "Bob"
  license = "valid"
}
Driver --> Vehicle : drives
@enduml
""")

# ─────────────────────────────────────────────
# TIMING DIAGRAMS (~250)
# ─────────────────────────────────────────────

# Robust syntax basics
w("timing", "tim_robust_basic", """
@startuml
robust "Signal" as sig
@0
sig is idle
@100
sig is active
@200
sig is idle
@enduml
""")

w("timing", "tim_robust_two_signals", """
@startuml
robust "Clock" as clk
robust "Data" as dat
@0
clk is low
dat is unknown
@50
clk is high
dat is valid
@100
clk is low
dat is valid
@150
clk is high
dat is unknown
@enduml
""")

# Concise syntax
w("timing", "tim_concise_basic", """
@startuml
concise "User" as U
@0
U is Idle
@100
U is Active
@200
U is Idle
@enduml
""")

w("timing", "tim_concise_multiple", """
@startuml
concise "Alice" as A
concise "Bob" as B
concise "Charlie" as C
@0
A is Idle
B is Idle
C is Idle
@50
A is Sending
@100
B is Receiving
A is Idle
@150
B is Processing
@200
B is Idle
C is Notified
@250
C is Idle
@enduml
""")

# Binary/clock
w("timing", "tim_binary_clock", """
@startuml
binary "CLK" as clk
@0
clk is low
@25
clk is high
@50
clk is low
@75
clk is high
@100
clk is low
@enduml
""")

w("timing", "tim_binary_multiple", """
@startuml
binary "CLK" as clk
binary "CS" as cs
binary "WR" as wr
@0
clk is low
cs is high
wr is high
@10
clk is high
@20
clk is low
cs is low
@30
clk is high
wr is low
@40
clk is low
@50
clk is high
@60
clk is low
cs is high
wr is high
@enduml
""")

# Clock keyword
w("timing", "tim_clock_signal", """
@startuml
clock "SysClk" as sclk with period 50
binary "Reset" as rst
@0
rst is high
@100
rst is low
@300
rst is high
@enduml
""")

# Annotations
w("timing", "tim_annotations", """
@startuml
robust "Network" as net
@0
net is idle
@50
net is transmitting
@100
net is idle
@0 <-> @50 : setup time
@50 <-> @100 : transfer
@enduml
""")

w("timing", "tim_annotations_multiple", """
@startuml
concise "CPU" as cpu
concise "Memory" as mem
@0
cpu is fetch
mem is idle
@10
cpu is decode
mem is read
@20
cpu is execute
@30
cpu is writeback
mem is write
@40
cpu is fetch
mem is idle
@0 <-> @40 : 1 pipeline stage
@enduml
""")

# Highlighted periods
w("timing", "tim_highlight", """
@startuml
robust "Bus" as bus
highlight 50 to 150 : critical section
@0
bus is idle
@50
bus is busy
@150
bus is idle
@200
bus is busy
@250
bus is idle
@enduml
""")

w("timing", "tim_highlight_multiple", """
@startuml
robust "Signal" as sig
highlight 0 to 50 #pink : startup
highlight 100 to 200 #lightblue : operation
highlight 250 to 300 #lightyellow : shutdown
@0
sig is init
@50
sig is running
@200
sig is stopping
@300
sig is stopped
@enduml
""")

# Colors on states
w("timing", "tim_state_colors", """
@startuml
robust "Process" as proc
@0
proc is #green : running
@100
proc is #red : error
@150
proc is #yellow : recovering
@200
proc is #green : running
@enduml
""")

# Mixed robust and concise
w("timing", "tim_mixed", """
@startuml
robust "Hardware" as hw
concise "Software" as sw
@0
hw is off
sw is stopped
@100
hw is on
@200
sw is starting
@300
sw is running
hw is active
@500
sw is stopping
@600
sw is stopped
hw is idle
@enduml
""")

# Multiple time axes
w("timing", "tim_title_scale", """
@startuml
title System Boot Sequence
scale 1 as 50 pixels
robust "Power" as pwr
robust "Clock" as clk
concise "Firmware" as fw
@0
pwr is off
clk is stopped
fw is unknown
@100
pwr is on
@150
clk is running
@200
fw is loading
@400
fw is running
@enduml
""")

# Complex timing
w("timing", "tim_complex_protocol", """
@startuml
robust "Master" as m
robust "Slave" as s
binary "ACK" as ack
@0
m is idle
s is idle
ack is low
@50
m is sending
@100
s is receiving
@150
ack is high
m is waiting
@200
ack is low
m is idle
s is processing
@300
s is idle
@enduml
""")

# Concise with many states
for n_states in [3, 5, 8]:
    states = ["idle", "active", "waiting", "processing", "error", "recovering", "complete", "resetting"][:n_states]
    timeline = ""
    for i, state in enumerate(states):
        timeline += f"@{i*100}\nproc is {state}\n"
    w("timing", f"tim_concise_{n_states}states", f"""
@startuml
concise "Process" as proc
{timeline}
@enduml
""")

# Robust with many participants
for n in [3, 5]:
    participants = "\n".join([f'robust "Signal{i}" as s{i}' for i in range(n)])
    timeline = ""
    for t in range(0, 300, 50):
        timeline += f"@{t}\n"
        for i in range(n):
            state = "active" if (t // 50 + i) % 2 == 0 else "idle"
            timeline += f"s{i} is {state}\n"
    w("timing", f"tim_robust_{n}signals", f"""
@startuml
{participants}
{timeline}
@enduml
""")

# Constraint annotations
w("timing", "tim_constraints", """
@startuml
robust "TX" as tx
robust "RX" as rx
@0
tx is idle
rx is idle
@50
tx is transmitting
@100
rx is receiving
tx is idle
@150
rx is idle
@50 <-> @100 : propagation delay
@100 <-> @150 : processing time
@enduml
""")

# Long timing diagram
w("timing", "tim_long_timeline", """
@startuml
robust "Service" as svc
concise "Client" as cli
@0
svc is ready
cli is idle
@100
cli is connecting
@200
svc is connected
cli is connected
@300
cli is requesting
@400
svc is processing
@500
svc is responding
@600
cli is receiving
@700
cli is processing
@800
cli is idle
svc is ready
@900
cli is connecting
@1000
svc is connected
cli is connected
@1100
cli is disconnecting
@1200
cli is idle
svc is ready
@enduml
""")

# Combinatorial variants
states_sets = [
    ["idle", "active", "error"],
    ["off", "on", "standby"],
    ["stopped", "starting", "running", "stopping"],
    ["init", "ready", "busy", "done"],
    ["low", "medium", "high"],
]
for i, states in enumerate(states_sets):
    timeline = ""
    for j, state in enumerate(states):
        timeline += f"@{j*100}\nsig is {state}\n"
    timeline += f"@{len(states)*100}\nsig is {states[0]}\n"
    w("timing", f"tim_states_set_{i}", f"""
@startuml
robust "Signal" as sig
{timeline}
@enduml
""")

# Named states with colors combos
for i, color in enumerate(COLORS[:8]):
    w("timing", f"tim_color_{color}", f"""
@startuml
robust "Process" as proc
@0
proc is #lightgrey : idle
@100
proc is #{color} : active
@200
proc is #lightgrey : idle
@enduml
""")

# Binary signal patterns
for pattern_name, values in [
    ("pulse", [0,1,0,1,0]),
    ("burst", [0,1,1,1,0]),
    ("alternating", [0,1,0,1,0,1]),
]:
    timeline = ""
    for i, v in enumerate(values):
        state = "high" if v else "low"
        timeline += f"@{i*50}\nsig is {state}\n"
    w("timing", f"tim_binary_{pattern_name}", f"""
@startuml
binary "Signal" as sig
{timeline}
@enduml
""")

# Note on timing
w("timing", "tim_note", """
@startuml
robust "Channel" as ch
note top of ch : Communication channel
@0
ch is idle
@100
ch is active
@200
ch is idle
@enduml
""")

# Participant with long names and spaces
w("timing", "tim_long_names", """
@startuml
robust "HTTP Request Handler" as hrh
concise "Database Connection Pool" as dcp
concise "Cache Layer" as cache
@0
hrh is waiting
dcp is idle
cache is empty
@100
hrh is processing
dcp is connected
cache is warming
@200
hrh is responding
dcp is idle
cache is full
@300
hrh is waiting
@enduml
""")

# At-relative timing
w("timing", "tim_relative_at", """
@startuml
concise "Task" as task
@0
task is pending
@+50
task is running
@+100
task is paused
@+50
task is running
@+100
task is done
@enduml
""")

# Scale variations
for scale in [1, 2, 5]:
    w("timing", f"tim_scale_{scale}", f"""
@startuml
scale {scale} as 100 pixels
robust "Signal" as sig
@0
sig is low
@50
sig is high
@100
sig is low
@enduml
""")

# Title and header/footer
w("timing", "tim_with_header_footer", """
@startuml
header Protocol Timing v1.0
footer Page 1
title I2C Bus Timing Diagram
robust "SCL" as scl
robust "SDA" as sda
@0
scl is high
sda is high
@50
sda is low
@100
scl is low
@150
sda is high
@200
scl is high
@enduml
""")

# Combinatorial: different diagram types with varying sizes
for i in range(30):
    n_sigs = (i % 4) + 2
    sig_type = ["robust", "concise", "binary"][i % 3]
    signals = "\n".join([f'{sig_type} "Sig{j}" as s{j}' for j in range(n_sigs)])
    timeline = ""
    for t in range(0, 400, 100):
        timeline += f"@{t}\n"
        for j in range(n_sigs):
            if sig_type == "binary":
                state = "high" if (t//100 + j) % 2 == 0 else "low"
            else:
                states = ["idle","active","waiting","error"]
                state = states[(t//100 + j) % len(states)]
            timeline += f"s{j} is {state}\n"
    w("timing", f"tim_combo_{i:02d}", f"""
@startuml
{signals}
{timeline}
@enduml
""")

# ─────────────────────────────────────────────
# MINDMAP DIAGRAMS (~200)
# ─────────────────────────────────────────────

w("mindmap", "mm_basic", """
@startmindmap
* Root
@endmindmap
""")

w("mindmap", "mm_two_levels", """
@startmindmap
* Root
** Branch A
** Branch B
** Branch C
@endmindmap
""")

w("mindmap", "mm_three_levels", """
@startmindmap
* Root
** Branch A
*** Leaf A1
*** Leaf A2
** Branch B
*** Leaf B1
@endmindmap
""")

w("mindmap", "mm_left_right", """
@startmindmap
* Central Idea
** Right Branch 1
*** Sub Right 1
** Right Branch 2
-- Left Branch 1
--- Sub Left 1
-- Left Branch 2
@endmindmap
""")

w("mindmap", "mm_deep_6_levels", """
@startmindmap
* L1
** L2
*** L3
**** L4
***** L5
****** L6
@endmindmap
""")

w("mindmap", "mm_balanced", """
@startmindmap
* Project
** Planning
*** Requirements
*** Schedule
*** Budget
** Execution
*** Development
*** Testing
*** Deployment
-- Analysis
--- Market
--- Competition
-- Research
--- Technology
--- Trends
@endmindmap
""")

# Colors
for color in COLORS[:8]:
    w("mindmap", f"mm_color_{color}", f"""
@startmindmap
*[#{color}] Root
**[#{color}] Child A
**[#white] Child B
*** Grandchild
@endmindmap
""")

# Box/no-box styles
w("mindmap", "mm_no_box", """
@startmindmap
* Root
** Normal box
**_ No box node
*** Child of no-box
@endmindmap
""")

w("mindmap", "mm_underscore_style", """
@startmindmap
*_ Root without box
**_ Level 2 no box
***_ Level 3 no box
** Level 2 with box
@endmindmap
""")

# Creole markup
w("mindmap", "mm_creole_bold", """
@startmindmap
* **Bold Root**
** //Italic Branch//
*** __Underlined Leaf__
** --Strikethrough--
@endmindmap
""")

w("mindmap", "mm_creole_mixed", """
@startmindmap
* Root Node
** **Important** feature
*** This is //very// important
** Simple node
*** With **bold** and //italic// text
@endmindmap
""")

# Multiline nodes
w("mindmap", "mm_multiline", """
@startmindmap
* Root
** First node
with second line
** Another node
-- Left node
with continuation
@endmindmap
""")

# Icons/stereotypes
w("mindmap", "mm_stereotypes", """
@startmindmap
* <<cloud>> Main Topic
** <<database>> Data
*** <<table>> Users
*** <<table>> Orders
** <<server>> Services
*** <<api>> REST API
@endmindmap
""")

# Right-only tree
w("mindmap", "mm_right_only", """
@startmindmap
* Center
** A
*** A1
*** A2
** B
*** B1
**** B1a
** C
@endmindmap
""")

# Left-only tree
w("mindmap", "mm_left_only", """
@startmindmap
* Center
-- A
--- A1
--- A2
-- B
--- B1
---- B1a
-- C
@endmindmap
""")

# Large mindmap
w("mindmap", "mm_large", """
@startmindmap
* Company Strategy
** Products
*** Product A
**** Features
***** Feature 1
***** Feature 2
**** Roadmap
*** Product B
**** MVP
**** Version 2
** Markets
*** North America
**** USA
**** Canada
*** Europe
**** UK
**** Germany
**** France
-- Operations
--- Engineering
---- Frontend
---- Backend
---- DevOps
--- Finance
---- Accounting
---- Budgeting
-- HR
--- Recruiting
--- Training
--- Benefits
@endmindmap
""")

# Skinparam mindmap
w("mindmap", "mm_skinparam", """
@startmindmap
skinparam mindmapBorderColor darkblue
skinparam mindmapBackgroundColor lightyellow
* Root
** Node A
*** Leaf 1
** Node B
*** Leaf 2
@endmindmap
""")

# Caption
w("mindmap", "mm_with_title", """
@startmindmap
title My Mind Map
* Central Topic
** Idea 1
*** Detail 1a
*** Detail 1b
** Idea 2
*** Detail 2a
@endmindmap
""")

# Numeric depth variations
for depth in range(2, 7):
    stars = "\n".join([("*" * d) + f" Level {d}" for d in range(1, depth+1)])
    w("mindmap", f"mm_depth_{depth}", f"""
@startmindmap
{stars}
@endmindmap
""")

# Wide trees
for width in [3, 5, 8, 10]:
    branches = "\n".join([f"** Branch {i}" for i in range(1, width+1)])
    w("mindmap", f"mm_wide_{width}", f"""
@startmindmap
* Root
{branches}
@endmindmap
""")

# Combinatorial color + depth
for i in range(25):
    depth = (i % 4) + 2
    color = COLORS[i % len(COLORS)]
    content = f"*[#{color}] Root\n"
    for d in range(2, depth+1):
        stars = "*" * d
        content += f"{stars} Level {d} node\n"
    w("mindmap", f"mm_colordepth_{i:02d}", f"""
@startmindmap
{content}
@endmindmap
""")

# Mixed left/right with colors
for i in range(15):
    w("mindmap", f"mm_mixed_{i:02d}", f"""
@startmindmap
*[#{COLORS[i%len(COLORS)]}] Topic {i}
**[#{COLORS[(i+1)%len(COLORS)]}] Right {i}.1
***[#white] Sub {i}.1.1
**[#{COLORS[(i+2)%len(COLORS)]}] Right {i}.2
--[#{COLORS[(i+3)%len(COLORS)]}] Left {i}.1
---[#white] Sub {i}.L.1
--[#{COLORS[(i+4)%len(COLORS)]}] Left {i}.2
@endmindmap
""")

# Markdown syntax
w("mindmap", "mm_markdown_style", """
@startmindmap
# Root
## Branch 1
### Leaf 1.1
### Leaf 1.2
## Branch 2
### Leaf 2.1
#### Deep leaf
@endmindmap
""")

# ─────────────────────────────────────────────
# WBS DIAGRAMS (~150)
# ─────────────────────────────────────────────

w("wbs", "wbs_basic", """
@startwbs
* Project
** Phase 1
** Phase 2
** Phase 3
@endwbs
""")

w("wbs", "wbs_three_levels", """
@startwbs
* Project Alpha
** Planning
*** Requirements gathering
*** Feasibility study
** Development
*** Frontend
*** Backend
*** Database
** Testing
*** Unit tests
*** Integration tests
@endwbs
""")

w("wbs", "wbs_deep_levels", """
@startwbs
* Root
** L2
*** L3
**** L4
***** L5
****** L6
@endwbs
""")

# Colors
for color in COLORS[:8]:
    w("wbs", f"wbs_color_{color}", f"""
@startwbs
*[#{color}] Root Task
** Subtask A
**[#{color}] Subtask B
*** Detail B1
** Subtask C
@endwbs
""")

# Wide trees
for width in [3, 5, 8]:
    children = "\n".join([f"** Task {i}" for i in range(1, width+1)])
    w("wbs", f"wbs_wide_{width}", f"""
@startwbs
* Project
{children}
@endwbs
""")

# Deep tree
w("wbs", "wbs_deep_unbalanced", """
@startwbs
* Root
** A
*** A1
**** A1a
***** A1a-i
** B
** C
*** C1
*** C2
@endwbs
""")

# Multi-line entries
w("wbs", "wbs_multiline", """
@startwbs
* Project Root
with description
** Phase One
details here
*** Task 1.1
*** Task 1.2
** Phase Two
@endwbs
""")

# Left branches
w("wbs", "wbs_left_branches", """
@startwbs
* Central
** Right 1
*** R1.1
** Right 2
-- Left 1
--- L1.1
-- Left 2
@endwbs
""")

# Skinparam
w("wbs", "wbs_skinparam", """
@startwbs
skinparam wbsBorderColor darkred
skinparam wbsBackgroundColor lightyellow
* Work Breakdown
** Task Group A
*** Task A1
*** Task A2
** Task Group B
*** Task B1
@endwbs
""")

# Large WBS
w("wbs", "wbs_large_software", """
@startwbs
* Software Development Project
** Requirements
*** Business Requirements
**** Stakeholder Interviews
**** Document Analysis
*** Technical Requirements
**** Architecture Design
**** API Specification
** Design
*** UI/UX Design
**** Wireframes
**** Mockups
**** Prototypes
*** System Design
**** Database Schema
**** Service Architecture
** Implementation
*** Frontend Development
**** Component Library
**** Page Development
**** Integration
*** Backend Development
**** API Development
**** Business Logic
**** Database Layer
*** Infrastructure
**** CI/CD Pipeline
**** Cloud Setup
**** Monitoring
** Testing
*** Unit Testing
*** Integration Testing
*** Performance Testing
*** Security Testing
** Deployment
*** Staging Deployment
*** Production Deployment
*** Post-deployment Verification
@endwbs
""")

# Combos
for i in range(25):
    depth = (i % 4) + 2
    width = (i % 5) + 2
    color = COLORS[i % len(COLORS)]
    children = "\n".join([f"** Child {j}" for j in range(1, width+1)])
    deep = "\n".join(["*" * d + f" Level {d}" for d in range(3, depth+1)])
    w("wbs", f"wbs_combo_{i:02d}", f"""
@startwbs
*[#{color}] Root {i}
{children}
{deep}
@endwbs
""")

# Completion markers (using colors to indicate status)
w("wbs", "wbs_status_colors", """
@startwbs
*[#white] Project
**[#green] Completed Task
***[#green] Done subtask 1
***[#green] Done subtask 2
**[#yellow] In Progress
***[#green] Done part
***[#red] Blocked part
**[#red] Not Started
*** Pending item
@endwbs
""")

w("wbs", "wbs_project_phases", """
@startwbs
*[#lightblue] Q1 Release
**[#green] Sprint 1
***[#green] User auth
***[#green] Login page
**[#yellow] Sprint 2
***[#green] Dashboard
***[#yellow] Reports
***[#red] Export feature
**[#red] Sprint 3
*** Notifications
*** Settings
*** Mobile app
@endwbs
""")

# ─────────────────────────────────────────────
# JSON/YAML DIAGRAMS (~200)
# ─────────────────────────────────────────────

# JSON basics
w("json-yaml", "json_basic_flat", """
@startjson
{
  "name": "Alice",
  "age": 30,
  "active": true
}
@endjson
""")

w("json-yaml", "json_nested_object", """
@startjson
{
  "user": {
    "id": 1,
    "name": "Bob",
    "address": {
      "street": "123 Main St",
      "city": "Springfield",
      "zip": "12345"
    }
  }
}
@endjson
""")

w("json-yaml", "json_array", """
@startjson
{
  "fruits": ["apple", "banana", "cherry"],
  "numbers": [1, 2, 3, 4, 5],
  "mixed": [1, "two", true, null]
}
@endjson
""")

w("json-yaml", "json_array_of_objects", """
@startjson
{
  "users": [
    {"id": 1, "name": "Alice"},
    {"id": 2, "name": "Bob"},
    {"id": 3, "name": "Charlie"}
  ]
}
@endjson
""")

w("json-yaml", "json_deep_nested", """
@startjson
{
  "level1": {
    "level2": {
      "level3": {
        "level4": {
          "level5": {
            "value": "deep"
          }
        }
      }
    }
  }
}
@endjson
""")

w("json-yaml", "json_all_types", """
@startjson
{
  "string": "hello world",
  "integer": 42,
  "float": 3.14159,
  "boolean_true": true,
  "boolean_false": false,
  "null_value": null,
  "empty_string": "",
  "empty_object": {},
  "empty_array": []
}
@endjson
""")

w("json-yaml", "json_config_example", """
@startjson
{
  "app": {
    "name": "MyApp",
    "version": "1.2.3",
    "debug": false,
    "port": 8080,
    "database": {
      "host": "localhost",
      "port": 5432,
      "name": "mydb",
      "ssl": true
    },
    "cache": {
      "driver": "redis",
      "host": "127.0.0.1",
      "ttl": 3600
    },
    "features": ["auth", "api", "admin"]
  }
}
@endjson
""")

w("json-yaml", "json_highlight", """
@startjson
#highlight "name"
#highlight "address" / "city"
{
  "name": "Alice",
  "age": 30,
  "address": {
    "city": "Paris",
    "country": "France"
  }
}
@endjson
""")

w("json-yaml", "json_unicode", """
@startjson
{
  "greeting_en": "Hello",
  "greeting_es": "Hola",
  "greeting_fr": "Bonjour",
  "greeting_de": "Hallo",
  "greeting_jp": "Konnichiwa",
  "symbol": "© 2024",
  "emoji_text": "Stars"
}
@endjson
""")

w("json-yaml", "json_large_array", """
@startjson
{
  "items": [
    "item_01", "item_02", "item_03", "item_04", "item_05",
    "item_06", "item_07", "item_08", "item_09", "item_10",
    "item_11", "item_12", "item_13", "item_14", "item_15"
  ]
}
@endjson
""")

w("json-yaml", "json_api_response", """
@startjson
{
  "status": "success",
  "code": 200,
  "data": {
    "users": [
      {
        "id": 1,
        "username": "alice",
        "email": "alice@example.com",
        "roles": ["user", "admin"]
      },
      {
        "id": 2,
        "username": "bob",
        "email": "bob@example.com",
        "roles": ["user"]
      }
    ],
    "pagination": {
      "page": 1,
      "per_page": 10,
      "total": 2
    }
  },
  "errors": []
}
@endjson
""")

w("json-yaml", "json_package_json", """
@startjson
{
  "name": "my-package",
  "version": "1.0.0",
  "description": "A sample package",
  "main": "index.js",
  "scripts": {
    "start": "node index.js",
    "test": "jest",
    "build": "webpack"
  },
  "dependencies": {
    "express": "^4.18.0",
    "lodash": "^4.17.21"
  },
  "devDependencies": {
    "jest": "^29.0.0",
    "webpack": "^5.0.0"
  }
}
@endjson
""")

# YAML basics
w("json-yaml", "yaml_basic", """
@startyaml
name: Alice
age: 30
active: true
@endyaml
""")

w("json-yaml", "yaml_nested", """
@startyaml
user:
  id: 1
  name: Bob
  address:
    street: 123 Main St
    city: Springfield
@endyaml
""")

w("json-yaml", "yaml_list", """
@startyaml
fruits:
  - apple
  - banana
  - cherry
numbers:
  - 1
  - 2
  - 3
@endyaml
""")

w("json-yaml", "yaml_list_of_maps", """
@startyaml
users:
  - id: 1
    name: Alice
    role: admin
  - id: 2
    name: Bob
    role: user
  - id: 3
    name: Charlie
    role: moderator
@endyaml
""")

w("json-yaml", "yaml_config", """
@startyaml
app:
  name: MyService
  version: 2.0.0
  debug: false
database:
  host: localhost
  port: 5432
  credentials:
    user: admin
    password: secret
logging:
  level: info
  file: /var/log/app.log
@endyaml
""")

w("json-yaml", "yaml_deep_nested", """
@startyaml
level1:
  level2:
    level3:
      level4:
        level5:
          value: deep
          another: also deep
@endyaml
""")

w("json-yaml", "yaml_kubernetes", """
@startyaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app
  labels:
    app: my-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: my-app
  template:
    metadata:
      labels:
        app: my-app
    spec:
      containers:
        - name: app
          image: my-app:latest
          ports:
            - containerPort: 8080
          env:
            - name: DB_HOST
              value: postgres
@endyaml
""")

w("json-yaml", "yaml_anchors", """
@startyaml
defaults: &defaults
  timeout: 30
  retries: 3
  log_level: info

production:
  <<: *defaults
  host: prod.example.com
  debug: false

staging:
  <<: *defaults
  host: staging.example.com
  debug: true
@endyaml
""")

w("json-yaml", "yaml_multiline_strings", """
@startyaml
literal_block: |
  This is line one
  This is line two
  This is line three
folded_block: >
  This long sentence will be
  folded into a single line
plain: just a plain string
@endyaml
""")

# Combinatorial JSON variants
json_structures = [
    ('{"key": "value"}', "flat"),
    ('{"a": {"b": {"c": 1}}}', "nested3"),
    ('{"arr": [1,2,3]}', "array"),
    ('{"n": null, "b": true, "i": 42, "f": 1.5}', "types"),
    ('{"x": {}, "y": []}', "empty"),
]
for i, (struct, name) in enumerate(json_structures):
    w("json-yaml", f"json_variant_{name}_{i}", f"""
@startjson
{struct}
@endjson
""")

# Combinatorial YAML variants
for i in range(20):
    n_keys = (i % 5) + 2
    entries = "\n".join([f"key{j}: value{j}" for j in range(n_keys)])
    w("json-yaml", f"yaml_flat_{i:02d}", f"""
@startyaml
{entries}
@endyaml
""")

for i in range(15):
    n_items = (i % 6) + 2
    items = "\n".join([f"  - item{j}" for j in range(n_items)])
    w("json-yaml", f"yaml_list_{i:02d}", f"""
@startyaml
mylist:
{items}
count: {n_items}
@endyaml
""")

# Mixed JSON with highlights
for i, field in enumerate(["name", "id", "status", "type", "version"]):
    w("json-yaml", f"json_highlight_{field}", f"""
@startjson
#highlight "{field}"
{{
  "id": {i+1},
  "name": "item_{i}",
  "status": "active",
  "type": "example",
  "version": "{i+1}.0"
}}
@endjson
""")

# ─────────────────────────────────────────────
# GANTT CHARTS (~200)
# ─────────────────────────────────────────────

w("gantt", "gantt_basic", """
@startgantt
[Task A] lasts 5 days
[Task B] lasts 3 days
[Task C] lasts 4 days
@endgantt
""")

w("gantt", "gantt_with_dates", """
@startgantt
Project starts 2024-01-01
[Requirements] lasts 10 days
[Design] lasts 7 days
[Development] lasts 20 days
[Testing] lasts 10 days
[Deployment] lasts 3 days
@endgantt
""")

w("gantt", "gantt_dependencies", """
@startgantt
[Analysis] lasts 5 days
[Design] lasts 7 days
[Design] starts at [Analysis]'s end
[Development] lasts 14 days
[Development] starts at [Design]'s end
[Testing] lasts 5 days
[Testing] starts at [Development]'s end
@endgantt
""")

w("gantt", "gantt_milestones", """
@startgantt
[Phase 1] lasts 10 days
[Milestone 1] happens at [Phase 1]'s end
[Phase 2] lasts 10 days
[Phase 2] starts at [Milestone 1]'s end
[Milestone 2] happens at [Phase 2]'s end
[Phase 3] lasts 5 days
[Phase 3] starts at [Milestone 2]'s end
[Project Complete] happens at [Phase 3]'s end
@endgantt
""")

w("gantt", "gantt_colors", """
@startgantt
[Task A] lasts 5 days
[Task A] is colored in Coral
[Task B] lasts 7 days
[Task B] is colored in LightBlue
[Task C] lasts 4 days
[Task C] is colored in LightGreen
[Task D] lasts 6 days
[Task D] is colored in Gold
@endgantt
""")

w("gantt", "gantt_completion", """
@startgantt
[Task 1] lasts 10 days
[Task 1] is 100% completed
[Task 2] lasts 10 days
[Task 2] is 75% completed
[Task 3] lasts 10 days
[Task 3] is 50% completed
[Task 4] lasts 10 days
[Task 4] is 25% completed
[Task 5] lasts 10 days
[Task 5] is 0% completed
@endgantt
""")

w("gantt", "gantt_closed_days", """
@startgantt
Project starts 2024-01-01
saturday are closed
sunday are closed
[Task 1] lasts 5 days
[Task 2] lasts 5 days
[Task 2] starts at [Task 1]'s end
[Task 3] lasts 5 days
[Task 3] starts at [Task 2]'s end
@endgantt
""")

w("gantt", "gantt_separator", """
@startgantt
[T1] lasts 3 days
[T2] lasts 4 days
-- Milestone --
[T3] lasts 5 days
[T3] starts at [T2]'s end
-- Final Phase --
[T4] lasts 2 days
[T4] starts at [T3]'s end
@endgantt
""")

w("gantt", "gantt_sections", """
@startgantt
Project starts 2024-03-01

[Milestone A] happens 2024-03-15

section Planning
[Requirements] lasts 7 days
[Architecture] lasts 5 days
[Architecture] starts at [Requirements]'s end

section Development
[Backend] lasts 14 days
[Backend] starts at [Architecture]'s end
[Frontend] lasts 14 days
[Frontend] starts at [Architecture]'s end

section Release
[Integration] lasts 5 days
[Integration] starts at [Backend]'s end
[Deploy] lasts 2 days
[Deploy] starts at [Integration]'s end
@endgantt
""")

w("gantt", "gantt_parallel_tasks", """
@startgantt
[Task A] lasts 5 days
[Task B] lasts 3 days
[Task B] starts at [Task A]'s start
[Task C] lasts 7 days
[Task D] lasts 4 days
[Task D] starts at [Task C]'s start
@endgantt
""")

w("gantt", "gantt_zoom", """
@startgantt
printscale weekly
Project starts 2024-01-01
[Phase 1] lasts 14 days
[Phase 2] lasts 21 days
[Phase 2] starts at [Phase 1]'s end
[Phase 3] lasts 10 days
[Phase 3] starts at [Phase 2]'s end
@endgantt
""")

w("gantt", "gantt_printscale_daily", """
@startgantt
printscale daily
Project starts 2024-06-01
[Sprint 1 Day 1] lasts 1 day
[Sprint 1 Day 2] lasts 1 day
[Sprint 1 Day 2] starts at [Sprint 1 Day 1]'s end
[Sprint 1 Day 3] lasts 1 day
[Sprint 1 Day 3] starts at [Sprint 1 Day 2]'s end
@endgantt
""")

w("gantt", "gantt_resources", """
@startgantt
[Task 1] on {Alice} lasts 5 days
[Task 2] on {Bob} lasts 5 days
[Task 3] on {Alice} lasts 3 days
[Task 3] starts at [Task 1]'s end
[Task 4] on {Bob} lasts 4 days
[Task 4] starts at [Task 2]'s end
@endgantt
""")

w("gantt", "gantt_then_syntax", """
@startgantt
[Analysis] lasts 5 days
then [Design] lasts 7 days
then [Development] lasts 14 days
then [Testing] lasts 5 days
then [Release] lasts 1 day
@endgantt
""")

w("gantt", "gantt_long_project", """
@startgantt
printscale monthly
Project starts 2024-01-01
[Q1 Planning] lasts 30 days
[Q1 Development] lasts 60 days
[Q1 Development] starts at [Q1 Planning]'s end
[Q1 Testing] lasts 30 days
[Q1 Testing] starts at [Q1 Development]'s end
[Q2 Planning] lasts 30 days
[Q2 Planning] starts at [Q1 Testing]'s end
[Q2 Development] lasts 60 days
[Q2 Development] starts at [Q2 Planning]'s end
[Q2 Testing] lasts 30 days
[Q2 Testing] starts at [Q2 Development]'s end
@endgantt
""")

w("gantt", "gantt_with_title", """
@startgantt
title Project Timeline 2024
Project starts 2024-01-15
[Kickoff] lasts 2 days
[Sprint 1] lasts 14 days
[Sprint 1] starts at [Kickoff]'s end
[Sprint 2] lasts 14 days
[Sprint 2] starts at [Sprint 1]'s end
[Sprint 3] lasts 14 days
[Sprint 3] starts at [Sprint 2]'s end
[Release] lasts 3 days
[Release] starts at [Sprint 3]'s end
@endgantt
""")

w("gantt", "gantt_note", """
@startgantt
[Task 1] lasts 5 days
note bottom
  This task is critical
  path item
end note
[Task 2] lasts 3 days
[Task 2] starts at [Task 1]'s end
@endgantt
""")

# Combinatorial: durations, colors, completion
task_colors = ["Coral", "LightBlue", "LightGreen", "Gold", "Plum", "LightSalmon"]
for i in range(30):
    n_tasks = (i % 5) + 3
    tasks = []
    for j in range(n_tasks):
        dur = (j * 3 + i + 2) % 10 + 2
        color = task_colors[j % len(task_colors)]
        pct = (j * 25) % 125
        pct = min(pct, 100)
        tasks.append(f"[Task {j+1}] lasts {dur} days")
        tasks.append(f"[Task {j+1}] is colored in {color}")
        tasks.append(f"[Task {j+1}] is {pct}% completed")
        if j > 0:
            tasks.append(f"[Task {j+1}] starts at [Task {j}]'s end")
    w("gantt", f"gantt_combo_{i:02d}", f"""
@startgantt
{chr(10).join(tasks)}
@endgantt
""")

# Multiple sections
for n_sections in [2, 3, 4]:
    content = "Project starts 2024-01-01\n"
    prev_task = None
    for s in range(n_sections):
        content += f"\nsection Phase {s+1}\n"
        for t in range(3):
            tname = f"Task S{s+1}T{t+1}"
            dur = (s + t + 2) * 3
            content += f"[{tname}] lasts {dur} days\n"
            if prev_task:
                content += f"[{tname}] starts at [{prev_task}]'s end\n"
            prev_task = tname
    w("gantt", f"gantt_sections_{n_sections}", f"""
@startgantt
{content}
@endgantt
""")

# Gantt with holidays
w("gantt", "gantt_holidays", """
@startgantt
Project starts 2024-12-20
2024-12-25 is closed
2024-12-26 is closed
2025-01-01 is closed
[Task 1] lasts 5 days
[Task 2] lasts 5 days
[Task 2] starts at [Task 1]'s end
[Task 3] lasts 3 days
[Task 3] starts at [Task 2]'s end
@endgantt
""")

# ─────────────────────────────────────────────
# SALT WIREFRAMES (~100)
# ─────────────────────────────────────────────

w("salt", "salt_basic_form", """
@startsalt
{
  Name: | "Alice       "
  Age:  | "30          "
  [Submit] | [Cancel]
}
@endsalt
""")

w("salt", "salt_button_row", """
@startsalt
{
  [OK] | [Cancel] | [Help] | [Reset]
}
@endsalt
""")

w("salt", "salt_checkbox", """
@startsalt
{
  [X] Option A
  [ ] Option B
  [X] Option C
  [ ] Option D
}
@endsalt
""")

w("salt", "salt_radio", """
@startsalt
{
  (X) Choice 1
  ( ) Choice 2
  ( ) Choice 3
}
@endsalt
""")

w("salt", "salt_dropdown", """
@startsalt
{
  Language: | ^English^
  Country:  | ^United States^
  Timezone: | ^UTC-5^
}
@endsalt
""")

w("salt", "salt_table", """
@startsalt
{#
  Name        | Age | City
  Alice       | 30  | Paris
  Bob         | 25  | London
  Charlie     | 35  | New York
}
@endsalt
""")

w("salt", "salt_table_header", """
@startsalt
{+
  ID   | Name    | Email              | Role
  1    | Alice   | alice@example.com  | Admin
  2    | Bob     | bob@example.com    | User
  3    | Charlie | charlie@example.com| Mod
}
@endsalt
""")

w("salt", "salt_tree", """
@startsalt
{T
  + Root
  ++ Branch A
  +++ Leaf 1
  +++ Leaf 2
  ++ Branch B
  +++ Leaf 3
  + Another Root
  ++ Single child
}
@endsalt
""")

w("salt", "salt_tabs", """
@startsalt
{/
  General | Advanced | Security | About
  {
    Name: | "My App"
    Version: | "1.0.0"
    [Save]
  }
}
@endsalt
""")

w("salt", "salt_separator", """
@startsalt
{
  First Name: | "John "
  Last Name:  | "Doe  "
  ..
  Email: | "john@example.com"
  Phone: | "+1-555-0100    "
  ..
  [Save Profile] | [Cancel]
}
@endsalt
""")

w("salt", "salt_grouping", """
@startsalt
{
  {^Personal Details
    Name:    | "Alice"
    Surname: | "Smith"
    DOB:     | "1990-01-15"
  }
  {^Contact
    Email:  | "alice@example.com"
    Phone:  | "555-1234"
  }
  [Submit]
}
@endsalt
""")

w("salt", "salt_nested", """
@startsalt
{
  {
    [Button 1]
    [Button 2]
  } |
  {
    "Text field    "
    "Another field "
  }
}
@endsalt
""")

w("salt", "salt_menu_bar", """
@startsalt
{+
  File | Edit | View | Help
  {
    "Document content area"
    "More content here..."
  }
}
@endsalt
""")

w("salt", "salt_scrollbar", """
@startsalt
{S
  Item 1
  Item 2
  Item 3
  Item 4
  Item 5
  Item 6
  Item 7
  Item 8
}
@endsalt
""")

w("salt", "salt_login_form", """
@startsalt
{
  Login
  ===
  Username: | "          "
  Password: | "          "
  [X] Remember me
  ===
  [  Login  ] | [ Cancel ]
  ---
  Forgot password?
}
@endsalt
""")

w("salt", "salt_settings_dialog", """
@startsalt
{^Settings
  {/
    General | Display | Network | Privacy
    {
      Theme:    | ^Light^
      Language: | ^English^
      Font size:| ^Medium^
      [X] Auto-save
      [ ] Spell check
      ..
      [Apply] | [Reset to defaults]
    }
  }
}
@endsalt
""")

w("salt", "salt_search_bar", """
@startsalt
{
  Search: | "                    " | [Search] | [Clear]
  ---
  {#
    Result 1 | Description 1 | 2024-01-01
    Result 2 | Description 2 | 2024-01-02
    Result 3 | Description 3 | 2024-01-03
  }
  ---
  Showing 1-3 of 10 | [< Prev] | [Next >]
}
@endsalt
""")

w("salt", "salt_grid_layout", """
@startsalt
{
  {
    [A] | [B] | [C]
    [D] | [E] | [F]
    [G] | [H] | [I]
  }
}
@endsalt
""")

w("salt", "salt_bold_italic", """
@startsalt
{
  <b>Bold text</b>
  <i>Italic text</i>
  <u>Underlined text</u>
  <b><i>Bold italic</i></b>
  Regular text
}
@endsalt
""")

w("salt", "salt_complex_dialog", """
@startsalt
{^Create New Project
  Project Name:    | "                    "
  Description:     | {SI
    Enter description
    here
  }
  ..
  Template:        | ^Blank^
  (X) Private project
  ( ) Public project
  ..
  Collaborators:
  {T
    + Team Members
    ++ Alice (owner)
    ++ Bob
    ++ Charlie
  }
  ..
  [  Create  ] | [ Cancel ]
}
@endsalt
""")

# Combinatorial salt variants
form_fields = [
    ("Username", "alice"),
    ("Email", "alice@example.com"),
    ("Phone", "555-1234"),
    ("Address", "123 Main St"),
]
for i in range(20):
    n_fields = (i % 4) + 1
    fields = "\n  ".join([f'{label}: | "{value}"' for label, value in form_fields[:n_fields]])
    buttons = " | ".join([f"[Button {j+1}]" for j in range((i % 3) + 1)])
    w("salt", f"salt_form_{i:02d}", f"""
@startsalt
{{
  {fields}
  ..
  {buttons}
}}
@endsalt
""")

# Salt with various table sizes
for rows, cols in [(3,2),(4,3),(5,4),(6,3),(2,5)]:
    header = " | ".join([f"Col{c}" for c in range(cols)])
    data_rows = "\n  ".join([" | ".join([f"R{r}C{c}" for c in range(cols)]) for r in range(rows)])
    w("salt", f"salt_table_{rows}x{cols}", f"""
@startsalt
{{#
  {header}
  {data_rows}
}}
@endsalt
""")

# ─────────────────────────────────────────────
# NWDIAG (~100)
# ─────────────────────────────────────────────

w("nwdiag", "nw_basic", """
@startnwdiag
nwdiag {
  network internet {
    web01
    web02
  }
}
@endnwdiag
""")

w("nwdiag", "nw_two_networks", """
@startnwdiag
nwdiag {
  network internet {
    address = "203.0.113.0/24"
    web01 [address = "203.0.113.10"]
    web02 [address = "203.0.113.11"]
  }
  network internal {
    address = "10.0.0.0/24"
    web01 [address = "10.0.0.10"]
    web02 [address = "10.0.0.11"]
    db01  [address = "10.0.0.20"]
  }
}
@endnwdiag
""")

w("nwdiag", "nw_three_networks", """
@startnwdiag
nwdiag {
  network dmz {
    address = "172.16.0.0/24"
    lb01  [address = "172.16.0.1"]
    web01 [address = "172.16.0.10"]
    web02 [address = "172.16.0.11"]
  }
  network app {
    address = "10.1.0.0/24"
    web01   [address = "10.1.0.10"]
    web02   [address = "10.1.0.11"]
    app01   [address = "10.1.0.20"]
    app02   [address = "10.1.0.21"]
  }
  network db {
    address = "10.2.0.0/24"
    app01   [address = "10.2.0.20"]
    app02   [address = "10.2.0.21"]
    db01    [address = "10.2.0.30"]
    db02    [address = "10.2.0.31"]
  }
}
@endnwdiag
""")

w("nwdiag", "nw_with_groups", """
@startnwdiag
nwdiag {
  group web {
    color = "#FFD700"
    web01
    web02
  }
  group db {
    color = "#87CEEB"
    db01
    db02
  }
  network frontend {
    web01
    web02
  }
  network backend {
    web01
    web02
    db01
    db02
  }
}
@endnwdiag
""")

w("nwdiag", "nw_with_descriptions", """
@startnwdiag
nwdiag {
  network internet {
    web01 [description = "Primary web server"]
    web02 [description = "Secondary web server"]
  }
  network internal {
    web01
    db01  [description = "Database master"]
    db02  [description = "Database replica"]
  }
}
@endnwdiag
""")

w("nwdiag", "nw_dmz_pattern", """
@startnwdiag
nwdiag {
  network internet {
    address = "0.0.0.0/0"
    firewall [address = "1.2.3.4"]
  }
  network dmz {
    address = "192.168.1.0/24"
    firewall [address = "192.168.1.1"]
    web01    [address = "192.168.1.10"]
    web02    [address = "192.168.1.11"]
    mail01   [address = "192.168.1.20"]
  }
  network internal {
    address = "10.0.0.0/24"
    firewall [address = "10.0.0.1"]
    web01    [address = "10.0.0.10"]
    app01    [address = "10.0.0.20"]
    db01     [address = "10.0.0.30"]
  }
}
@endnwdiag
""")

w("nwdiag", "nw_colors", """
@startnwdiag
nwdiag {
  network internet {
    color = "#FF6B6B"
    router
  }
  network lan {
    color = "#4ECDC4"
    router
    pc01
    pc02
    printer
  }
  network wifi {
    color = "#45B7D1"
    router
    laptop01
    phone01
    phone02
  }
}
@endnwdiag
""")

w("nwdiag", "nw_five_servers", """
@startnwdiag
nwdiag {
  network frontend {
    address = "10.0.1.0/24"
    lb      [address = "10.0.1.1", description = "Load Balancer"]
    web01   [address = "10.0.1.10"]
    web02   [address = "10.0.1.11"]
    web03   [address = "10.0.1.12"]
  }
  network backend {
    address = "10.0.2.0/24"
    web01   [address = "10.0.2.10"]
    web02   [address = "10.0.2.11"]
    web03   [address = "10.0.2.12"]
    api01   [address = "10.0.2.20"]
    api02   [address = "10.0.2.21"]
  }
  network data {
    address = "10.0.3.0/24"
    api01   [address = "10.0.3.20"]
    api02   [address = "10.0.3.21"]
    db01    [address = "10.0.3.30"]
    cache01 [address = "10.0.3.40"]
  }
}
@endnwdiag
""")

w("nwdiag", "nw_peer", """
@startnwdiag
nwdiag {
  network peering {
    as1_router [address = "192.0.2.1"]
    as2_router [address = "192.0.2.2"]
  }
  network as1 {
    address = "10.1.0.0/16"
    as1_router [address = "10.1.0.1"]
    as1_web    [address = "10.1.0.10"]
  }
  network as2 {
    address = "10.2.0.0/16"
    as2_router [address = "10.2.0.1"]
    as2_web    [address = "10.2.0.10"]
  }
}
@endnwdiag
""")

w("nwdiag", "nw_cloud_vpc", """
@startnwdiag
nwdiag {
  network internet {
    nat_gateway [address = "52.0.0.1"]
  }
  network public_subnet {
    address = "10.0.1.0/24"
    nat_gateway    [address = "10.0.1.5"]
    bastion        [address = "10.0.1.10"]
    alb            [address = "10.0.1.20"]
  }
  network private_subnet {
    address = "10.0.2.0/24"
    alb     [address = "10.0.2.20"]
    app01   [address = "10.0.2.30"]
    app02   [address = "10.0.2.31"]
  }
  network data_subnet {
    address = "10.0.3.0/24"
    app01   [address = "10.0.3.30"]
    app02   [address = "10.0.3.31"]
    rds     [address = "10.0.3.40"]
    elasticache [address = "10.0.3.50"]
  }
}
@endnwdiag
""")

# Combinatorial nwdiag
for i in range(20):
    n_nets = (i % 3) + 2
    n_servers = (i % 4) + 2
    networks = []
    all_servers = [f"srv{j:02d}" for j in range(n_servers)]
    for n in range(n_nets):
        net_servers = all_servers[n:] + all_servers[:n]  # rotate
        net_servers = net_servers[:max(2, n_servers - n)]
        addrs = "\n    ".join([f'{s} [address = "10.{n}.{j}.{i}"]' for j, s in enumerate(net_servers)])
        networks.append(f"""  network net{n} {{
    address = "10.{n}.{i}.0/24"
    {addrs}
  }}""")
    w("nwdiag", f"nw_combo_{i:02d}", f"""
@startnwdiag
nwdiag {{
{chr(10).join(networks)}
}}
@endnwdiag
""")

# Simple nwdiag variations
for n_servers in [2, 3, 5, 8]:
    servers = "\n    ".join([f"server{j:02d}" for j in range(n_servers)])
    w("nwdiag", f"nw_single_net_{n_servers}servers", f"""
@startnwdiag
nwdiag {{
  network lan {{
    address = "192.168.0.0/24"
    {servers}
  }}
}}
@endnwdiag
""")

# ─────────────────────────────────────────────
# EXTRA OBJECT DIAGRAMS
# ─────────────────────────────────────────────

# More map combos with links
for i in range(30):
    n = (i % 4) + 2
    maps_content = ""
    for k in range(n):
        entries = "\n  ".join([f"field{j} => val{k}_{j}" for j in range(3)])
        maps_content += f'map "Map{k}_{i}" as m{k}_{i} {{\n  {entries}\n}}\n'
    links = "\n".join([f"m{k}_{i} --> m{(k+1)%n}_{i}" for k in range(n-1)])
    w("object", f"obj_mapchain_{i:02d}", f"""
@startuml
{maps_content}
{links}
@enduml
""")

# Objects with all relationship types and notes
rel_styles = [
    ("extends", "--|>"),
    ("uses", "..>"),
    ("owns", "*--"),
    ("has", "o--"),
    ("refs", "-->"),
    ("knows", "--"),
]
for i, (name, sym) in enumerate(rel_styles):
    color_a = COLORS[i % len(COLORS)]
    color_b = COLORS[(i+3) % len(COLORS)]
    w("object", f"obj_rel_{name}", f"""
@startuml
object Source #{color_a} {{
  id = {i}
  type = "source"
}}
object Target #{color_b} {{
  id = {i+100}
  type = "target"
}}
Source {sym} Target : "{name}"
note right of Source : Source note
note left of Target : Target note
@enduml
""")

# 3-object compositions
compositions = [
    ("Controller", "Service", "Repository"),
    ("Facade", "Manager", "Helper"),
    ("Client", "Server", "Database"),
    ("Browser", "API", "Storage"),
    ("Producer", "Queue", "Consumer"),
]
for i, (a, b, c) in enumerate(compositions):
    w("object", f"obj_trio_{i:02d}", f"""
@startuml
object {a} {{
  name = "{a.lower()}"
}}
object {b} {{
  name = "{b.lower()}"
}}
object {c} {{
  name = "{c.lower()}"
}}
{a} --> {b}
{b} --> {c}
@enduml
""")

# Object grid (NxM)
for rows, cols in [(2,2),(2,3),(3,3),(3,4),(4,4)]:
    objs = "\n".join([f"object O{r}_{c} {{ v = {r*10+c} }}" for r in range(rows) for c in range(cols)])
    links = "\n".join([f"O{r}_{c} --> O{r}_{c+1}" for r in range(rows) for c in range(cols-1)] +
                      [f"O{r}_{c} --> O{r+1}_{c}" for r in range(rows-1) for c in range(cols)])
    w("object", f"obj_grid_{rows}x{cols}", f"""
@startuml
{objs}
{links}
@enduml
""")

# Objects with skinparam combos
for bg, border in [("LightYellow","DarkOrange"),("LightBlue","DarkBlue"),("LightGreen","DarkGreen"),
                    ("LightPink","DarkRed"),("Lavender","Purple")]:
    safe = bg.lower()
    w("object", f"obj_skin_{safe}", f"""
@startuml
skinparam object {{
  BackgroundColor {bg}
  BorderColor {border}
  FontSize 12
}}
object Alpha {{
  x = 1
  y = 2
}}
object Beta {{
  x = 3
  y = 4
}}
Alpha --> Beta
@enduml
""")

# Objects with packages and namespaces
namespaces = ["com.example.model", "com.example.service", "com.example.repo"]
for i, ns in enumerate(namespaces):
    w("object", f"obj_ns_{i:02d}", f"""
@startuml
namespace {ns} {{
  object Entity{i} {{
    id = {i}
    name = "entity{i}"
  }}
}}
@enduml
""")

# More combinatorial objects
for i in range(40):
    n_objs = (i % 6) + 2
    objs = "\n".join([f'object Obj{j}_{i} {{ val = {i*10+j} }}' for j in range(n_objs)])
    links = "\n".join([f"Obj{j}_{i} --> Obj{(j+1)%n_objs}_{i}" for j in range(n_objs)])
    w("object", f"obj_ring_{i:02d}", f"""
@startuml
{objs}
{links}
@enduml
""")

# ─────────────────────────────────────────────
# EXTRA TIMING DIAGRAMS
# ─────────────────────────────────────────────

# Fine-grained time steps
for step in [5, 10, 25, 50]:
    timeline = "\n".join([f"@{t}\nsig is {'high' if (t//step)%2==0 else 'low'}" for t in range(0, step*8, step)])
    w("timing", f"tim_step_{step}", f"""
@startuml
binary "Signal" as sig
{timeline}
@enduml
""")

# Timing with title and notes
titles = ["SPI Protocol", "I2C Bus", "UART Framing", "CAN Frame", "USB Transaction"]
for i, title in enumerate(titles):
    w("timing", f"tim_proto_{i:02d}", f"""
@startuml
title {title}
robust "Line A" as la
robust "Line B" as lb
binary "Enable" as en
@0
la is idle
lb is idle
en is low
@50
en is high
la is sending
@100
lb is receiving
@150
la is idle
@200
en is low
lb is idle
@enduml
""")

# Multi-signal timing with highlights
for i in range(15):
    n = (i % 4) + 2
    sigs = "\n".join([f'robust "Sig{j}" as s{j}' for j in range(n)])
    timeline = ""
    for t in range(0, 500, 100):
        timeline += f"@{t}\n" + "\n".join([f"s{j} is {'active' if (t//100+j)%3!=0 else 'idle'}" for j in range(n)]) + "\n"
    hl_start = 100 + (i % 3) * 100
    hl_end = hl_start + 100
    w("timing", f"tim_multi_hl_{i:02d}", f"""
@startuml
{sigs}
highlight {hl_start} to {hl_end} #lightyellow : phase {i}
{timeline}
@enduml
""")

# Concise with colors
color_states = [
    ("#green","running"), ("#red","error"), ("#yellow","warning"),
    ("#blue","info"), ("#grey","disabled"),
]
for i in range(10):
    states = color_states[:((i%4)+2)]
    timeline = ""
    for j, (color, state) in enumerate(states):
        timeline += f"@{j*100}\nproc is {color} : {state}\n"
    timeline += f"@{len(states)*100}\nproc is #lightgrey : idle\n"
    w("timing", f"tim_colored_states_{i:02d}", f"""
@startuml
robust "Process" as proc
{timeline}
@enduml
""")

# Timing diagrams with constraints annotations
for i in range(15):
    w("timing", f"tim_annotated_{i:02d}", f"""
@startuml
robust "TX" as tx
robust "RX" as rx
@0
tx is idle
rx is idle
@{50+i*5}
tx is active
@{100+i*5}
rx is active
tx is idle
@{150+i*5}
rx is idle
@{50+i*5} <-> @{100+i*5} : propagation {i}ms
@enduml
""")

# Many-participant timing
for n in [6, 8, 10]:
    sigs = "\n".join([f'concise "P{j}" as p{j}' for j in range(n)])
    timeline = ""
    for t in range(0, 400, 100):
        timeline += f"@{t}\n" + "\n".join([f"p{j} is {'active' if j==(t//100)%n else 'idle'}" for j in range(n)]) + "\n"
    w("timing", f"tim_npart_{n}", f"""
@startuml
{sigs}
{timeline}
@enduml
""")

# ─────────────────────────────────────────────
# EXTRA MINDMAP DIAGRAMS
# ─────────────────────────────────────────────

# Full topic trees for different subjects
topics = [
    ("Programming Languages", ["Python","Java","Rust","Go","JavaScript"], ["Scripting","Enterprise","Systems","Cloud","Web"]),
    ("Cloud Providers", ["AWS","Azure","GCP","Oracle","IBM"], ["S3","Blob","GCS","OCI","COS"]),
    ("Databases", ["PostgreSQL","MySQL","MongoDB","Redis","Cassandra"], ["SQL","SQL","NoSQL","Cache","Wide"]),
    ("Frontend Frameworks", ["React","Vue","Angular","Svelte","Ember"], ["SPA","SPA","Full","Compiler","MVC"]),
    ("Testing Types", ["Unit","Integration","E2E","Performance","Security"], ["Fast","Medium","Slow","JMeter","SAST"]),
]
for i, (root, branches, subbranches) in enumerate(topics):
    content = f"* {root}\n"
    for j, (b, sb) in enumerate(zip(branches, subbranches)):
        if j < 3:
            content += f"** {b}\n*** {sb}\n"
        else:
            content += f"-- {b}\n--- {sb}\n"
    w("mindmap", f"mm_topic_{i:02d}", f"""
@startmindmap
{content}
@endmindmap
""")

# Mindmap with mixed node styles
for i in range(20):
    color1 = COLORS[i % len(COLORS)]
    color2 = COLORS[(i+2) % len(COLORS)]
    color3 = COLORS[(i+4) % len(COLORS)]
    w("mindmap", f"mm_style_{i:02d}", f"""
@startmindmap
*[#{color1}] Root {i}
**[#{color2}] Branch A
***[#{color3}] Leaf A1
***_ No box leaf
**[#{color2}] Branch B
*** Leaf B1
**_ Branch no box
*** Sub leaf
-- Left {i}
---[#{color3}] Left child
@endmindmap
""")

# Mindmap with many siblings (wide)
for width in [6, 8, 10, 12, 15]:
    branches = "\n".join([f"** Topic {j}: description here" for j in range(width)])
    w("mindmap", f"mm_wide_siblings_{width}", f"""
@startmindmap
* Main Topic
{branches}
@endmindmap
""")

# Deep mindmaps
for depth in range(3, 9):
    content = ""
    for d in range(1, depth+1):
        stars = "*" * d
        content += f"{stars} Depth {d} node\n"
        if d == 2:
            content += f"** Sibling at depth 2\n"
    w("mindmap", f"mm_depth_full_{depth}", f"""
@startmindmap
{content}
@endmindmap
""")

# Mindmap with creole formatting combos
creole_items = [
    "**Bold**", "//Italic//", "__Underline__", "--Strike--",
    "**Bold** and //italic//", "~~Monospace~~",
]
for i, item in enumerate(creole_items):
    w("mindmap", f"mm_creole_{i:02d}", f"""
@startmindmap
* Root with {item}
** Child node
*** {item} in leaf
-- Left branch
--- {item} left
@endmindmap
""")

# Large balanced mindmaps
for branches, leaves in [(3,3),(4,3),(3,4),(4,4),(5,3),(3,5)]:
    right_content = ""
    for b in range(branches):
        right_content += f"** Branch {b}\n"
        for l in range(leaves):
            right_content += f"*** Leaf {b}.{l}\n"
    left_count = max(2, branches // 2)
    left_content = ""
    for b in range(left_count):
        left_content += f"-- Left {b}\n"
        for l in range(max(2, leaves//2)):
            left_content += f"--- Sub {b}.{l}\n"
    w("mindmap", f"mm_balanced_{branches}b{leaves}l", f"""
@startmindmap
* Central
{right_content}
{left_content}
@endmindmap
""")

# ─────────────────────────────────────────────
# EXTRA WBS DIAGRAMS
# ─────────────────────────────────────────────

# WBS for various project types
project_types = [
    ("Website Launch", [("Design",["Wireframes","Mockups","Style Guide"]),
                        ("Development",["Frontend","Backend","Database"]),
                        ("Launch",["Testing","Staging","Production"])]),
    ("Mobile App", [("iOS",["UI","Logic","API"]),
                    ("Android",["UI","Logic","API"]),
                    ("Backend",["Auth","Data","Push"])]),
    ("Data Pipeline", [("Ingestion",["Sources","ETL","Validation"]),
                       ("Processing",["Transform","Aggregate","Store"]),
                       ("Output",["Reports","APIs","Exports"])]),
]
for i, (proj, phases) in enumerate(project_types):
    content = f"* {proj}\n"
    for phase, tasks in phases:
        content += f"** {phase}\n"
        for task in tasks:
            content += f"*** {task}\n"
    w("wbs", f"wbs_project_{i:02d}", f"""
@startwbs
{content}
@endwbs
""")

# WBS with color-coded status (more variants)
statuses = [
    ("#green", "Done"),
    ("#yellow", "In Progress"),
    ("#red", "Blocked"),
    ("#white", "Pending"),
    ("#lightblue", "In Review"),
]
for i in range(20):
    n_tasks = (i % 5) + 3
    tasks = ""
    for j in range(n_tasks):
        status_color, status_name = statuses[j % len(statuses)]
        tasks += f"**[{status_color}] Task {j}: {status_name}\n"
        if j % 3 == 0:
            tasks += f"***[#white] Subtask {j}.1\n"
    w("wbs", f"wbs_status_{i:02d}", f"""
@startwbs
*[#lightblue] Project {i}
{tasks}
@endwbs
""")

# WBS with left branches
for i in range(10):
    n_right = (i % 4) + 2
    n_left = (i % 3) + 1
    right = "\n".join([f"** Right Task {j}" for j in range(n_right)])
    left = "\n".join([f"-- Left Task {j}" for j in range(n_left)])
    w("wbs", f"wbs_leftright_{i:02d}", f"""
@startwbs
* Project {i}
{right}
{left}
@endwbs
""")

# WBS extra deep combos
for i in range(20):
    depth = (i % 5) + 2
    width = (i % 4) + 2
    color = COLORS[i % len(COLORS)]
    lines = [f"*[#{color}] Root {i}"]
    for d in range(2, depth+1):
        stars = "*" * d
        for w_idx in range(min(width, 4) if d < depth else 1):
            lines.append(f"{stars} Node d{d} w{w_idx}")
    w("wbs", f"wbs_deep_{i:02d}", f"""
@startwbs
{chr(10).join(lines)}
@endwbs
""")

# ─────────────────────────────────────────────
# EXTRA JSON/YAML DIAGRAMS
# ─────────────────────────────────────────────

# JSON configs for various tools
json_configs = [
    ("tsconfig", '{"compilerOptions":{"target":"ES2020","module":"commonjs","strict":true,"esModuleInterop":true},"include":["src/**/*"],"exclude":["node_modules"]}'),
    ("eslint", '{"env":{"browser":true,"es2021":true},"extends":["eslint:recommended"],"rules":{"no-console":"warn","semi":"error"}}'),
    ("prettier", '{"semi":true,"singleQuote":true,"tabWidth":2,"trailingComma":"es5","printWidth":100}'),
    ("jest", '{"testEnvironment":"node","coverageThreshold":{"global":{"branches":80,"functions":80,"lines":80}}}'),
    ("babel", '{"presets":["@babel/preset-env","@babel/preset-typescript"],"plugins":["@babel/plugin-transform-runtime"]}'),
]
for i, (name, config) in enumerate(json_configs):
    w("json-yaml", f"json_toolconfig_{name}", f"""
@startjson
{config}
@endjson
""")

# Nested JSON combos (depth 3-5)
for depth in range(3, 6):
    def make_nested(d, val):
        if d == 0:
            return f'"{val}"'
        return '{"key": ' + make_nested(d-1, val) + ', "level": ' + str(d) + '}'
    content = make_nested(depth, "leaf")
    w("json-yaml", f"json_nested_depth_{depth}", f"""
@startjson
{content}
@endjson
""")

# JSON arrays of varying size
for size in [3, 5, 8, 10, 15, 20]:
    items = ", ".join([f'"item_{i:02d}"' for i in range(size)])
    w("json-yaml", f"json_array_size_{size}", f"""
@startjson
{{"items": [{items}], "count": {size}}}
@endjson
""")

# JSON with numeric arrays
for size in [3, 5, 10]:
    nums = ", ".join([str(i*i) for i in range(1, size+1)])
    w("json-yaml", f"json_numarray_{size}", f"""
@startjson
{{"squares": [{nums}], "length": {size}}}
@endjson
""")

# YAML service configs
yaml_services = [
    ("nginx", "server:\n  listen: 80\n  server_name: example.com\n  root: /var/www/html\n  index: index.html"),
    ("redis", "bind: 127.0.0.1\nport: 6379\nmaxmemory: 256mb\nmaxmemory-policy: allkeys-lru\nsave:\n  - 900 1\n  - 300 10"),
    ("postgres", "host: localhost\nport: 5432\ndatabase: mydb\nuser: postgres\npassword: secret\npool:\n  min: 2\n  max: 10"),
    ("rabbitmq", "host: localhost\nport: 5672\nvhost: /\ncredentials:\n  user: guest\n  pass: guest\nqueues:\n  - tasks\n  - notifications"),
]
for i, (name, config) in enumerate(yaml_services):
    w("json-yaml", f"yaml_service_{name}", f"""
@startyaml
{config}
@endyaml
""")

# YAML with deep nesting combos
for depth in range(3, 7):
    lines = []
    indent = ""
    for d in range(depth):
        lines.append(f"{indent}level{d+1}:")
        indent += "  "
    lines.append(f"{indent}value: leaf_{depth}")
    w("json-yaml", f"yaml_nested_depth_{depth}", f"""
@startyaml
{chr(10).join(lines)}
@endyaml
""")

# More YAML list combos
categories = ["fruits","colors","countries","languages","frameworks"]
items_pool = [
    ["apple","banana","cherry","date","elderberry","fig","grape","honeydew"],
    ["red","green","blue","yellow","orange","purple","cyan","magenta"],
    ["USA","UK","France","Germany","Japan","Brazil","Canada","Australia"],
    ["Python","Rust","Go","Java","JavaScript","TypeScript","Ruby","Elixir"],
    ["Django","FastAPI","Axum","Gin","Spring","Express","Rails","Phoenix"],
]
for i, (cat, items) in enumerate(zip(categories, items_pool)):
    for size in [3, 5, 8]:
        subset = items[:size]
        yaml_items = "\n".join([f"  - {item}" for item in subset])
        w("json-yaml", f"yaml_list_{cat}_{size}", f"""
@startyaml
{cat}:
{yaml_items}
count: {size}
@endyaml
""")

# JSON highlights combos
for i in range(20):
    fields = [f"field{j}" for j in range(4)]
    highlight_field = fields[i % len(fields)]
    obj_fields = "\n  ".join([f'"{f}": "value_{i}_{j}"' for j, f in enumerate(fields)])
    w("json-yaml", f"json_hl_combo_{i:02d}", f"""
@startjson
#highlight "{highlight_field}"
{{
  {obj_fields}
}}
@endjson
""")

# ─────────────────────────────────────────────
# EXTRA GANTT DIAGRAMS
# ─────────────────────────────────────────────

# Sprint planning Gantts
for sprint_len in [7, 10, 14]:
    content = f"Project starts 2024-01-01\nsaturday are closed\nsunday are closed\n\n"
    n_tasks = sprint_len // 2
    for j in range(n_tasks):
        dur = max(1, sprint_len // n_tasks)
        content += f"[Story {j+1}] lasts {dur} days\n"
        if j > 0:
            content += f"[Story {j+1}] starts at [Story {j}]'s end\n"
    w("gantt", f"gantt_sprint_{sprint_len}d", f"""
@startgantt
{content}
@endgantt
""")

# Gantt with many resources
resources = ["Alice", "Bob", "Charlie", "Dave", "Eve"]
for i in range(15):
    n_res = (i % 4) + 2
    res_subset = resources[:n_res]
    tasks = []
    for j, res in enumerate(res_subset):
        dur = (j + i + 2) % 8 + 2
        tasks.append(f"[Task {res}] on {{{res}}} lasts {dur} days")
        if j > 0:
            tasks.append(f"[Task {res}] starts at [Task {res_subset[j-1]}]'s end")
    w("gantt", f"gantt_resources_{i:02d}", f"""
@startgantt
{chr(10).join(tasks)}
@endgantt
""")

# Gantt with milestones only
for n_milestones in [3, 5, 8]:
    content = "Project starts 2024-01-01\n"
    prev = None
    for k in range(n_milestones):
        if k == 0:
            content += f"[M{k}] happens 2024-01-{k*7+1:02d}\n"
        else:
            content += f"[Phase{k}] lasts {(k+1)*5} days\n"
            if prev:
                content += f"[Phase{k}] starts at [{prev}]'s end\n"
            content += f"[M{k}] happens at [Phase{k}]'s end\n"
            prev = f"Phase{k}"
    w("gantt", f"gantt_milestones_{n_milestones}", f"""
@startgantt
{content}
@endgantt
""")

# Gantt with completion percentages combos
for i in range(20):
    n_tasks = (i % 5) + 3
    tasks = []
    for j in range(n_tasks):
        dur = (j + 2) * 3
        pct = min(100, j * (100 // n_tasks))
        color = ["Coral","LightGreen","Gold","Plum","LightBlue"][j % 5]
        tasks.append(f"[T{j+1}] lasts {dur} days")
        tasks.append(f"[T{j+1}] is {pct}% completed")
        tasks.append(f"[T{j+1}] is colored in {color}")
        if j > 0:
            tasks.append(f"[T{j+1}] starts at [T{j}]'s end")
    w("gantt", f"gantt_progress_{i:02d}", f"""
@startgantt
{chr(10).join(tasks)}
@endgantt
""")

# Gantt with separators and sections
for i in range(10):
    n_sections = (i % 3) + 2
    content = "Project starts 2024-01-01\n"
    prev = None
    for s in range(n_sections):
        content += f"\nsection Section {s+1}\n"
        for t in range((i % 3) + 2):
            tname = f"T{s}_{t}"
            dur = (s + t + 1) * 4
            content += f"[{tname}] lasts {dur} days\n"
            if prev:
                content += f"[{tname}] starts at [{prev}]'s end\n"
            prev = tname
        content += "-- Checkpoint --\n"
    w("gantt", f"gantt_sections_combo_{i:02d}", f"""
@startgantt
{content}
@endgantt
""")

# ─────────────────────────────────────────────
# EXTRA SALT WIREFRAMES
# ─────────────────────────────────────────────

# Dashboard layouts
w("salt", "salt_dashboard", """
@startsalt
{+
  File | Edit | View | Reports | Help
  ---
  {
    {^Navigation
      [X] Dashboard
      [ ] Users
      [ ] Settings
      [ ] Reports
    } |
    {^Main Content
      {#
        Metric      | Value | Change
        Active Users| 1234  | +5%
        Revenue     | $9.8K | +12%
        Orders      | 456   | -2%
      }
      ..
      [Refresh] | [Export]
    }
  }
}
@endsalt
""")

# Form variants with different field types
form_combos = [
    ("registration", ["Username","Email","Password","Confirm Password"], True),
    ("payment", ["Card Number","Expiry","CVV","Name on Card"], False),
    ("address", ["Street","City","State","ZIP","Country"], True),
    ("profile", ["Full Name","Bio","Website","Twitter","GitHub"], False),
    ("feedback", ["Subject","Priority","Category","Description"], True),
]
for i, (name, fields, has_checkboxes) in enumerate(form_combos):
    field_lines = "\n  ".join([f'{f}: | "{"":20}"' for f in fields])
    extras = ""
    if has_checkboxes:
        extras = "  ..\n  [X] I agree to terms\n  [ ] Subscribe to newsletter"
    w("salt", f"salt_form_{name}", f"""
@startsalt
{{^{name.title()} Form
  {field_lines}
  {extras}
  ..
  [Submit] | [Reset] | [Cancel]
}}
@endsalt
""")

# Salt tab combos
tab_sets = [
    ["Home","Profile","Settings","Logout"],
    ["Overview","Details","History","Comments","Attachments"],
    ["Summary","Tasks","Files","Wiki","Activity"],
    ["Code","Issues","PRs","Actions","Security"],
]
for i, tabs in enumerate(tab_sets):
    tab_str = " | ".join(tabs)
    w("salt", f"salt_tabs_{i:02d}", f"""
@startsalt
{{/
  {tab_str}
  {{
    "Content area for {tabs[0]}"
    "Additional information"
    [Action Button]
  }}
}}
@endsalt
""")

# Salt tree views
tree_structures = [
    ("Files", [("src",["main.rs","lib.rs","utils.rs"]), ("tests",["unit.rs","integration.rs"])]),
    ("Org", [("Engineering",["Alice","Bob","Charlie"]), ("Design",["Dave","Eve"])]),
    ("Menu", [("File",["New","Open","Save","Exit"]), ("Edit",["Cut","Copy","Paste"])]),
]
for i, (root, branches) in enumerate(tree_structures):
    tree_lines = f"+ {root}\n"
    for branch, leaves in branches:
        tree_lines += f"++ {branch}\n"
        for leaf in leaves:
            tree_lines += f"+++ {leaf}\n"
    w("salt", f"salt_tree_{i:02d}", f"""
@startsalt
{{T
  {tree_lines}
}}
@endsalt
""")

# Salt tables of different sizes
for rows in [3, 5, 8]:
    for cols in [2, 3, 4]:
        header = " | ".join([f"Header {c}" for c in range(cols)])
        data = "\n  ".join([" | ".join([f"R{r}C{c}" for c in range(cols)]) for r in range(rows)])
        w("salt", f"salt_table_r{rows}c{cols}", f"""
@startsalt
{{#
  {header}
  {data}
}}
@endsalt
""")

# Salt with scrollbars and multi-select
w("salt", "salt_multiselect", """
@startsalt
{S
  [X] Item 1
  [ ] Item 2
  [X] Item 3
  [ ] Item 4
  [X] Item 5
  [ ] Item 6
  [ ] Item 7
  [X] Item 8
}
@endsalt
""")

# More salt combo forms
for i in range(20):
    n_fields = (i % 5) + 2
    fields = "\n  ".join([f"Field{j}: | \"{'':15}\"" for j in range(n_fields)])
    n_buttons = (i % 3) + 1
    buttons = " | ".join([f"[Btn{j}]" for j in range(n_buttons)])
    w("salt", f"salt_combo_{i:02d}", f"""
@startsalt
{{
  {fields}
  ..
  {buttons}
}}
@endsalt
""")

# ─────────────────────────────────────────────
# EXTRA NWDIAG DIAGRAMS
# ─────────────────────────────────────────────

# Office network patterns
w("nwdiag", "nw_office", """
@startnwdiag
nwdiag {
  network internet {
    address = "0.0.0.0/0"
    router [address = "203.0.113.1"]
  }
  network office_lan {
    address = "192.168.0.0/24"
    router      [address = "192.168.0.1"]
    workstation1 [address = "192.168.0.10"]
    workstation2 [address = "192.168.0.11"]
    workstation3 [address = "192.168.0.12"]
    printer      [address = "192.168.0.50"]
    nas          [address = "192.168.0.100"]
  }
  network wifi {
    address = "192.168.1.0/24"
    router   [address = "192.168.1.1"]
    laptop1  [address = "192.168.1.10"]
    phone1   [address = "192.168.1.20"]
    tablet1  [address = "192.168.1.30"]
  }
}
@endnwdiag
""")

# Data center patterns
w("nwdiag", "nw_datacenter", """
@startnwdiag
nwdiag {
  group cluster_a {
    color = "#E8F4FD"
    app01
    app02
    app03
  }
  group db_cluster {
    color = "#FDE8E8"
    db01
    db02
  }
  network external {
    address = "10.0.0.0/24"
    firewall [address = "10.0.0.1"]
    lb01     [address = "10.0.0.10"]
    lb02     [address = "10.0.0.11"]
  }
  network app_tier {
    address = "10.1.0.0/24"
    lb01  [address = "10.1.0.10"]
    lb02  [address = "10.1.0.11"]
    app01 [address = "10.1.0.20"]
    app02 [address = "10.1.0.21"]
    app03 [address = "10.1.0.22"]
  }
  network db_tier {
    address = "10.2.0.0/24"
    app01 [address = "10.2.0.20"]
    app02 [address = "10.2.0.21"]
    app03 [address = "10.2.0.22"]
    db01  [address = "10.2.0.30"]
    db02  [address = "10.2.0.31"]
  }
}
@endnwdiag
""")

# Microservices network
w("nwdiag", "nw_microservices", """
@startnwdiag
nwdiag {
  network ingress {
    address = "10.0.1.0/24"
    gateway   [address = "10.0.1.1"]
    auth_svc  [address = "10.0.1.10"]
  }
  network services {
    address = "10.0.2.0/24"
    gateway   [address = "10.0.2.1"]
    user_svc  [address = "10.0.2.10"]
    order_svc [address = "10.0.2.20"]
    product_svc [address = "10.0.2.30"]
    notif_svc [address = "10.0.2.40"]
  }
  network data {
    address = "10.0.3.0/24"
    user_svc    [address = "10.0.3.10"]
    order_svc   [address = "10.0.3.20"]
    product_svc [address = "10.0.3.30"]
    user_db     [address = "10.0.3.100"]
    order_db    [address = "10.0.3.101"]
    product_db  [address = "10.0.3.102"]
    message_bus [address = "10.0.3.200"]
  }
}
@endnwdiag
""")

# VPN topology
w("nwdiag", "nw_vpn", """
@startnwdiag
nwdiag {
  network internet {
    address = "0.0.0.0/0"
    hq_vpn  [address = "1.2.3.4"]
    br1_vpn [address = "5.6.7.8"]
    br2_vpn [address = "9.10.11.12"]
  }
  network hq_lan {
    address = "10.1.0.0/24"
    hq_vpn   [address = "10.1.0.1"]
    hq_srv1  [address = "10.1.0.10"]
    hq_srv2  [address = "10.1.0.11"]
    hq_print [address = "10.1.0.50"]
  }
  network branch1 {
    address = "10.2.0.0/24"
    br1_vpn [address = "10.2.0.1"]
    br1_pc1 [address = "10.2.0.10"]
    br1_pc2 [address = "10.2.0.11"]
  }
  network branch2 {
    address = "10.3.0.0/24"
    br2_vpn [address = "10.3.0.1"]
    br2_pc1 [address = "10.3.0.10"]
  }
}
@endnwdiag
""")

# Kubernetes network analogy
w("nwdiag", "nw_kubernetes", """
@startnwdiag
nwdiag {
  group control_plane {
    color = "#F0E6FF"
    apiserver
    scheduler
    etcd
  }
  group worker_nodes {
    color = "#E6F0FF"
    node1
    node2
    node3
  }
  network cluster_network {
    address = "10.96.0.0/12"
    apiserver  [address = "10.96.0.1"]
    scheduler  [address = "10.96.0.2"]
    etcd       [address = "10.96.0.3"]
    node1      [address = "10.96.1.1"]
    node2      [address = "10.96.1.2"]
    node3      [address = "10.96.1.3"]
  }
  network pod_network {
    address = "192.168.0.0/16"
    node1  [address = "192.168.1.0"]
    node2  [address = "192.168.2.0"]
    node3  [address = "192.168.3.0"]
  }
}
@endnwdiag
""")

# More combinatorial nwdiag
for i in range(25):
    n_nets = (i % 4) + 2
    servers_per_net = (i % 3) + 2
    nets = []
    all_nodes = set()
    for n in range(n_nets):
        nodes = [f"node{n}_{j}_{i}" for j in range(servers_per_net)]
        # share some nodes between adjacent networks
        if n > 0:
            shared = [f"node{n-1}_{servers_per_net-1}_{i}"]
            nodes = shared + nodes[:-1]
        all_nodes.update(nodes)
        node_lines = "\n    ".join([f"{nd} [address = \"172.{16+n}.{i}.{j+1}\"]" for j, nd in enumerate(nodes)])
        nets.append(f"""  network net{n}_{i} {{
    address = "172.{16+n}.{i}.0/24"
    {node_lines}
  }}""")
    w("nwdiag", f"nw_extra_{i:02d}", f"""
@startnwdiag
nwdiag {{
{chr(10).join(nets)}
}}
@endnwdiag
""")

# ─────────────────────────────────────────────
# BATCH EXTRA: push all categories well past targets
# ─────────────────────────────────────────────

# --- OBJECT: 50 more ---
# Diamond inheritance pattern
for i in range(10):
    w("object", f"obj_diamond_{i:02d}", f"""
@startuml
object Base{i} {{ id = {i} }}
object Left{i} {{ l = {i} }}
object Right{i} {{ r = {i} }}
object Bottom{i} {{ b = {i} }}
Base{i} --> Left{i}
Base{i} --> Right{i}
Left{i} --> Bottom{i}
Right{i} --> Bottom{i}
@enduml
""")

# Object with many fields
field_counts = [4, 6, 8, 10, 12]
for fc in field_counts:
    fields = "\n  ".join([f"field{j} = {j*11}" for j in range(fc)])
    w("object", f"obj_fields_{fc}", f"""
@startuml
object BigObject {{
  {fields}
}}
@enduml
""")

# Object with all stereotypes and colors combos
all_stereos = ["entity","boundary","control","database","actor","service","repository","factory","observer","strategy"]
for i, stereo in enumerate(all_stereos):
    color = COLORS[i % len(COLORS)]
    w("object", f"obj_stereo_full_{i:02d}", f"""
@startuml
object MyObj <<{stereo}>> #{color} {{
  id = {i}
  type = "{stereo}"
  active = true
}}
note right : Stereotype: {stereo}
@enduml
""")

# Linked object chains
for chain_len in [4, 6, 8, 10]:
    objs = "\n".join([f'object Node{j} {{ v = {j} }}' for j in range(chain_len)])
    links = "\n".join([f"Node{j} --> Node{j+1}" for j in range(chain_len-1)])
    w("object", f"obj_chain_{chain_len}", f"""
@startuml
{objs}
{links}
@enduml
""")

# Map with pointer syntax
for i in range(10):
    n = (i % 4) + 3
    entries = "\n  ".join([f"entry{j} => data{i}_{j}" for j in range(n)])
    target_entries = "\n  ".join([f"x{j} => y{i}_{j}" for j in range(2)])
    w("object", f"obj_map_ptr_{i:02d}", f"""
@startuml
map "Source{i}" as src{i} {{
  {entries}
}}
map "Target{i}" as tgt{i} {{
  {target_entries}
}}
src{i}::entry0 --> tgt{i}
@enduml
""")

# --- TIMING: 50 more ---
# All three signal types together
for i in range(15):
    n_robust = (i % 3) + 1
    n_concise = (i % 2) + 1
    n_binary = (i % 2) + 1
    robust_sigs = "\n".join([f'robust "R{j}_{i}" as r{j}_{i}' for j in range(n_robust)])
    concise_sigs = "\n".join([f'concise "C{j}_{i}" as c{j}_{i}' for j in range(n_concise)])
    binary_sigs = "\n".join([f'binary "B{j}_{i}" as b{j}_{i}' for j in range(n_binary)])
    tl = ""
    for t in range(0, 400, 100):
        tl += f"@{t}\n"
        for j in range(n_robust):
            tl += f"r{j}_{i} is {'active' if (t//100+j)%2==0 else 'idle'}\n"
        for j in range(n_concise):
            tl += f"c{j}_{i} is {'busy' if (t//100+j)%3!=0 else 'free'}\n"
        for j in range(n_binary):
            tl += f"b{j}_{i} is {'high' if (t//100+j)%2==0 else 'low'}\n"
    w("timing", f"tim_allthree_{i:02d}", f"""
@startuml
{robust_sigs}
{concise_sigs}
{binary_sigs}
{tl}
@enduml
""")

# Clock with period + binary combos
for period in [10, 20, 50, 100]:
    w("timing", f"tim_clkperiod_{period}", f"""
@startuml
clock "CLK{period}" as clk with period {period}
binary "DATA" as dat
@0
dat is low
@{period*2}
dat is high
@{period*5}
dat is low
@enduml
""")

# Timing with long participant names and highlights
long_names = [
    ("HTTP Request Handler", "hrh"),
    ("Authentication Service", "auth"),
    ("Database Connection", "db"),
    ("Cache Layer", "cache"),
    ("Message Queue", "mq"),
]
for i in range(10):
    idx = i % len(long_names)
    name, alias = long_names[idx]
    w("timing", f"tim_longname_{i:02d}", f"""
@startuml
robust "{name}" as {alias}
binary "Status" as st
@0
{alias} is idle
st is low
@{50+i*10}
{alias} is processing
st is high
@{100+i*10}
{alias} is responding
@{150+i*10}
{alias} is idle
st is low
@enduml
""")

# Timing with scale + title combos
for scale in [1, 2, 5, 10]:
    w("timing", f"tim_scaled_{scale}px", f"""
@startuml
scale {scale} as 50 pixels
title Scaled Timing (scale={scale})
robust "Signal" as sig
concise "State" as st
@0
sig is low
st is idle
@100
sig is high
st is active
@200
sig is low
st is idle
@enduml
""")

# --- MINDMAP: 50 more ---
# Subject-specific mindmaps
subjects = [
    "Machine Learning", "Web Development", "DevOps", "Cybersecurity",
    "Data Science", "Mobile Development", "Cloud Computing", "Blockchain",
    "IoT", "Game Development",
]
subtopics = [
    ["Supervised","Unsupervised","Reinforcement","Deep Learning","NLP"],
    ["HTML","CSS","JavaScript","Frameworks","APIs"],
    ["CI/CD","Docker","Kubernetes","Monitoring","IaC"],
    ["Auth","Encryption","Firewall","Pentesting","Compliance"],
    ["Statistics","Visualization","ML","BigData","Reporting"],
    ["iOS","Android","React Native","Flutter","PWA"],
    ["AWS","Azure","GCP","Serverless","CDN"],
    ["Bitcoin","Ethereum","Smart Contracts","DeFi","NFT"],
    ["Sensors","Protocols","Edge","Platforms","Security"],
    ["Engines","Graphics","Physics","Audio","Networking"],
]
for i, (subj, subs) in enumerate(zip(subjects, subtopics)):
    content = f"* {subj}\n"
    for j, s in enumerate(subs):
        if j < 3:
            content += f"** {s}\n"
        else:
            content += f"-- {s}\n"
    w("mindmap", f"mm_subject_{i:02d}", f"""
@startmindmap
{content}
@endmindmap
""")

# Mindmaps with notes
for i in range(10):
    w("mindmap", f"mm_note_{i:02d}", f"""
@startmindmap
* Root Topic {i}
** Branch A
*** Leaf A1
*** Leaf A2
** Branch B
*** Leaf B1
-- Left A
--- Left A1
@endmindmap
""")

# Mindmap with all features combined
for i in range(15):
    color1 = COLORS[i % len(COLORS)]
    color2 = COLORS[(i+3) % len(COLORS)]
    depth = (i % 3) + 3
    extras = "\n".join(["*" * d + f"[#{COLORS[(i+d)%len(COLORS)]}] Node d{d}" for d in range(2, depth+1)])
    w("mindmap", f"mm_full_{i:02d}", f"""
@startmindmap
*[#{color1}] Root {i}
{extras}
** Extra right
***_ No box
-- Left branch
---[#{color2}] Colored left
@endmindmap
""")

# --- WBS: 40 more ---
# WBS for engineering processes
engineering_wbs = [
    ("Software Release", [("Planning",["Scope","Timeline","Resources"]),
                           ("Engineering",["Design","Code","Review"]),
                           ("QA",["Unit","Integration","UAT"]),
                           ("Operations",["Deploy","Monitor","Rollback"])]),
    ("Hardware Design", [("Schematic",["Requirements","Design","Review"]),
                          ("Layout",["Placement","Routing","DRC"]),
                          ("Fabrication",["Gerber","Order","Assembly"]),
                          ("Testing",["Continuity","Functional","EMC"])]),
]
for i, (proj, phases) in enumerate(engineering_wbs):
    content = f"* {proj}\n"
    for phase, tasks in phases:
        content += f"** {phase}\n"
        for task in tasks:
            content += f"*** {task}\n"
    w("wbs", f"wbs_engineering_{i:02d}", f"""
@startwbs
{content}
@endwbs
""")

# WBS color combos wide+deep
for i in range(25):
    n_wide = (i % 5) + 3
    n_deep = (i % 3) + 2
    color = COLORS[i % len(COLORS)]
    children = "\n".join([f"** Task {j}" for j in range(n_wide)])
    deep = "\n".join(["*" * (d+2) + f" Sublevel {d}" for d in range(n_deep)])
    w("wbs", f"wbs_widedeep_{i:02d}", f"""
@startwbs
*[#{color}] Root {i}
{children}
** Deep branch
{deep}
@endwbs
""")

# WBS left+right combos
for i in range(13):
    n_right = (i % 4) + 2
    n_left = (i % 3) + 1
    right = "\n".join([f"**[#{COLORS[(i+j)%len(COLORS)]}] Right {j}" for j in range(n_right)])
    left = "\n".join([f"--[#{COLORS[(i+j+5)%len(COLORS)]}] Left {j}" for j in range(n_left)])
    w("wbs", f"wbs_lr_{i:02d}", f"""
@startwbs
* Root {i}
{right}
{left}
@endwbs
""")

# --- JSON/YAML: 50 more ---
# JSON for REST API schemas
api_schemas = [
    ("user_create", '{"username":{"type":"string","minLength":3},"email":{"type":"string","format":"email"},"password":{"type":"string","minLength":8}}'),
    ("order_create", '{"items":[{"product_id":"string","quantity":"integer"}],"shipping":{"address":"string","method":"string"},"payment":{"method":"string"}}'),
    ("search_params", '{"query":"string","filters":{"category":"string","min_price":"number","max_price":"number"},"sort":"string","page":"integer","per_page":"integer"}'),
]
for i, (name, schema) in enumerate(api_schemas):
    w("json-yaml", f"json_schema_{name}", f"""
@startjson
{schema}
@endjson
""")

# JSON matrix/grid data
for size in [3, 4, 5]:
    matrix = {"matrix": [[i*size+j for j in range(size)] for i in range(size)], "size": size}
    import json
    w("json-yaml", f"json_matrix_{size}x{size}", f"""
@startjson
{json.dumps(matrix, indent=2)}
@endjson
""")

# JSON with boolean arrays
for i in range(10):
    n = (i % 6) + 3
    bools = [str(j % 3 != 0).lower() for j in range(n)]
    w("json-yaml", f"json_boolarray_{i:02d}", f"""
@startjson
{{"flags": [{", ".join(bools)}], "count": {n}}}
@endjson
""")

# YAML CI/CD configs
yaml_ci = [
    ("github_actions", """name: CI
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build
        run: make build
      - name: Test
        run: make test"""),
    ("gitlab_ci", """stages:
  - build
  - test
  - deploy
build_job:
  stage: build
  script:
    - make build
test_job:
  stage: test
  script:
    - make test
deploy_job:
  stage: deploy
  script:
    - make deploy
  only:
    - main"""),
]
for i, (name, config) in enumerate(yaml_ci):
    w("json-yaml", f"yaml_ci_{name}", f"""
@startyaml
{config}
@endyaml
""")

# JSON and YAML mixed bulk
for i in range(25):
    n_keys = (i % 6) + 3
    if i % 2 == 0:
        # JSON
        pairs = ", ".join([f'"k{j}": {j + i}' for j in range(n_keys)])
        w("json-yaml", f"json_bulk_{i:02d}", f"""
@startjson
{{{pairs}}}
@endjson
""")
    else:
        # YAML
        pairs = "\n".join([f"k{j}: {j + i}" for j in range(n_keys)])
        w("json-yaml", f"yaml_bulk_{i:02d}", f"""
@startyaml
{pairs}
@endyaml
""")

# --- GANTT: 50 more ---
# Feature development gantts
feature_names = ["Auth","Dashboard","Reports","Search","Notifications","Settings","API","Billing","Admin","Profile"]
for i in range(20):
    n_features = (i % 5) + 3
    features = feature_names[:n_features]
    tasks = []
    for j, feat in enumerate(features):
        dur = (j + 2) * 3 + (i % 5)
        color = ["Coral","LightBlue","LightGreen","Gold","Plum"][j % 5]
        tasks.append(f"[{feat}] lasts {dur} days")
        tasks.append(f"[{feat}] is colored in {color}")
        if j > 0:
            tasks.append(f"[{feat}] starts at [{features[j-1]}]'s end")
    w("gantt", f"gantt_features_{i:02d}", f"""
@startgantt
Project starts 2024-01-01
saturday are closed
sunday are closed
{chr(10).join(tasks)}
@endgantt
""")

# Gantt with both milestones and sections
for i in range(15):
    n_sections = (i % 3) + 2
    content = f"Project starts 2024-0{(i%9)+1}-01\n"
    prev = None
    for s in range(n_sections):
        content += f"\nsection Phase {s+1}\n"
        for t in range(2):
            tname = f"P{s}T{t}"
            dur = (s + t + i % 5 + 1) * 4
            content += f"[{tname}] lasts {dur} days\n"
            if prev:
                content += f"[{tname}] starts at [{prev}]'s end\n"
            prev = tname
        content += f"[Milestone {s+1}] happens at [{prev}]'s end\n"
    w("gantt", f"gantt_ms_sections_{i:02d}", f"""
@startgantt
{content}
@endgantt
""")

# Gantt with then-syntax chains of varying lengths
for chain_len in [3, 4, 5, 6, 8, 10]:
    tasks = [f"[Step {j+1}] lasts {(j+1)*3} days" for j in range(chain_len)]
    # Use then for all but first
    content = tasks[0] + "\n" + "\n".join([f"then [Step {j+2}] lasts {(j+2)*3} days" for j in range(chain_len-1)])
    w("gantt", f"gantt_then_{chain_len}", f"""
@startgantt
{content}
@endgantt
""")

# --- SALT: 30 more ---
# Settings panels for various apps
settings_panels = [
    ("Audio", ["Volume", "Balance", "Bass", "Treble"]),
    ("Video", ["Resolution", "Brightness", "Contrast", "Saturation"]),
    ("Network", ["Hostname", "DNS Server", "Gateway", "Subnet Mask"]),
    ("Security", ["Firewall", "VPN", "Proxy", "Certificate"]),
    ("Email", ["SMTP Host", "SMTP Port", "IMAP Host", "Username"]),
]
for i, (panel, fields) in enumerate(settings_panels):
    field_lines = "\n  ".join([f"{f}: | \"{'':15}\"" for f in fields])
    w("salt", f"salt_settings_{panel.lower()}", f"""
@startsalt
{{^{panel} Settings
  {field_lines}
  ..
  [Save] | [Reset] | [Cancel]
}}
@endsalt
""")

# Salt wizard steps
for n_steps in [3, 4, 5]:
    tabs = " | ".join([f"Step {j+1}" for j in range(n_steps)])
    w("salt", f"salt_wizard_{n_steps}steps", f"""
@startsalt
{{/
  {tabs}
  {{
    Complete the current step
    "Field 1: " | "          "
    "Field 2: " | "          "
    ..
    [< Back] | [Next >] | [Cancel]
  }}
}}
@endsalt
""")

# Salt modal dialogs
modal_types = [
    ("confirm", "Are you sure you want to delete this item?", ["Yes, Delete", "Cancel"]),
    ("alert", "An error occurred while saving.", ["OK"]),
    ("prompt", "Enter new name:", ["OK", "Cancel"]),
    ("warning", "This action cannot be undone.", ["Proceed", "Go Back"]),
]
for i, (mtype, msg, buttons) in enumerate(modal_types):
    btn_row = " | ".join([f"[{b}]" for b in buttons])
    w("salt", f"salt_modal_{mtype}", f"""
@startsalt
{{^{mtype.title()} Dialog
  {msg}
  ..
  {btn_row}
}}
@endsalt
""")

# Salt data entry forms (more combos)
for i in range(15):
    n_text = (i % 4) + 2
    n_check = (i % 3) + 1
    n_radio = (i % 2) + 2
    text_fields = "\n  ".join([f"Label{j}: | \"{'':12}\"" for j in range(n_text)])
    checks = "\n  ".join([f"{'[X]' if j%2==0 else '[ ]'} Check option {j}" for j in range(n_check)])
    radios = "\n  ".join([f"{'(X)' if j==0 else '( )'} Radio {j}" for j in range(n_radio)])
    w("salt", f"salt_entry_{i:02d}", f"""
@startsalt
{{
  {text_fields}
  ..
  {checks}
  ..
  {radios}
  ..
  [Submit]
}}
@endsalt
""")

# --- NWDIAG: 50 more ---
# Home network
w("nwdiag", "nw_home", """
@startnwdiag
nwdiag {
  network internet {
    address = "0.0.0.0/0"
    modem [address = "1.2.3.4"]
  }
  network home {
    address = "192.168.1.0/24"
    modem    [address = "192.168.1.1"]
    desktop  [address = "192.168.1.10"]
    laptop   [address = "192.168.1.11"]
    smart_tv [address = "192.168.1.20"]
    nas      [address = "192.168.1.30"]
  }
  network iot {
    address = "192.168.2.0/24"
    modem        [address = "192.168.2.1"]
    thermostat   [address = "192.168.2.10"]
    light_bridge [address = "192.168.2.11"]
    camera1      [address = "192.168.2.20"]
    camera2      [address = "192.168.2.21"]
  }
}
@endnwdiag
""")

# Hospital/enterprise network patterns
for i in range(15):
    n_vlans = (i % 3) + 2
    n_hosts = (i % 4) + 2
    nets = []
    for v in range(n_vlans):
        hosts = [f"host{v}_{h}_{i}" for h in range(n_hosts)]
        if v > 0:
            hosts[0] = f"router{v-1}_{i}"
        host_lines = "\n    ".join([f"{h} [address = \"10.{v+1}.{i%10}.{h_idx+1}\"]"
                                     for h_idx, h in enumerate(hosts)])
        nets.append(f"""  network vlan{v}_{i} {{
    address = "10.{v+1}.{i%10}.0/24"
    {host_lines}
  }}""")
    w("nwdiag", f"nw_vlan_{i:02d}", f"""
@startnwdiag
nwdiag {{
{chr(10).join(nets)}
}}
@endnwdiag
""")

# SD-WAN topology
w("nwdiag", "nw_sdwan", """
@startnwdiag
nwdiag {
  network mpls {
    address = "10.0.0.0/8"
    hq_router    [address = "10.0.0.1"]
    site_a_router [address = "10.0.1.1"]
    site_b_router [address = "10.0.2.1"]
  }
  network internet {
    address = "0.0.0.0/0"
    hq_router    [address = "1.1.1.1"]
    site_a_router [address = "2.2.2.2"]
    site_b_router [address = "3.3.3.3"]
  }
  network hq_lan {
    address = "192.168.0.0/24"
    hq_router [address = "192.168.0.1"]
    hq_pc1    [address = "192.168.0.10"]
    hq_pc2    [address = "192.168.0.11"]
    hq_server [address = "192.168.0.100"]
  }
  network site_a {
    address = "192.168.1.0/24"
    site_a_router [address = "192.168.1.1"]
    site_a_pc1    [address = "192.168.1.10"]
  }
  network site_b {
    address = "192.168.2.0/24"
    site_b_router [address = "192.168.2.1"]
    site_b_pc1    [address = "192.168.2.10"]
  }
}
@endnwdiag
""")

# Simple one-network diagrams with different sizes
for n_hosts in [3, 4, 5, 6, 7, 8, 10, 12]:
    hosts = "\n    ".join([f"host{j} [address = \"192.168.0.{10+j}\"]" for j in range(n_hosts)])
    w("nwdiag", f"nw_flat_{n_hosts}hosts", f"""
@startnwdiag
nwdiag {{
  network lan {{
    address = "192.168.0.0/24"
    gateway [address = "192.168.0.1"]
    {hosts}
  }}
}}
@endnwdiag
""")

# Network with groups and colors
group_colors = ["#FFE4E1","#E1F5FE","#E8F5E9","#FFF9C4","#F3E5F5"]
for i in range(10):
    n_groups = (i % 3) + 2
    groups = []
    for g in range(n_groups):
        color = group_colors[g % len(group_colors)]
        members = " ".join([f"srv{g}_{m}_{i}" for m in range(2)])
        groups.append(f'  group grp{g}_{i} {{\n    color = "{color}"\n    {members}\n  }}')
    net_hosts = "\n    ".join([f"srv{g}_{m}_{i}" for g in range(n_groups) for m in range(2)])
    w("nwdiag", f"nw_groups_{i:02d}", f"""
@startnwdiag
nwdiag {{
{chr(10).join(groups)}
  network net_{i} {{
    address = "10.{i}.0.0/24"
    {net_hosts}
  }}
}}
@endnwdiag
""")

# ─────────────────────────────────────────────
# Print summary
# ─────────────────────────────────────────────
print("Generated files:")
total = 0
for cat, count in counts.items():
    print(f"  {cat:12s}: {count:4d} files")
    total += count
print(f"  {'TOTAL':12s}: {total:4d} files")
