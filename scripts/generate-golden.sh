#!/bin/sh
# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
#
# Generate golden SVG files from PlantUML test inputs.
# Requires PlantUML picoweb server running on port 8787.
#
# Usage: scripts/generate-golden.sh [port]

set -e

PORT="${1:-8787}"
URL="http://127.0.0.1:${PORT}/render"
GOLDEN_DIR="test-fixtures/golden"

echo "Generating golden files from PlantUML server at port ${PORT}..."

generate() {
    local name="$1"
    local source="$2"
    local outfile="${GOLDEN_DIR}/${name}.svg"

    local body=$(printf '{"source":%s,"options":["-tsvg"]}' \
        "$(echo "$source" | python3 -c 'import sys,json; print(json.dumps(sys.stdin.read()))')")

    local svg=$(curl -sf -H 'Content-Type: application/json' -d "$body" "$URL" 2>/dev/null)

    if echo "$svg" | grep -q '<svg'; then
        echo "$svg" > "$outfile"
        echo "  OK: ${name}"
    else
        echo "  SKIP: ${name} (not valid SVG)"
    fi
}

# Sequence diagrams
generate "sequence/simple" "@startuml
Alice -> Bob : hello
@enduml"

generate "sequence/multi_message" "@startuml
Alice -> Bob : message 1
Bob --> Alice : message 2
Alice -> Bob : message 3
@enduml"

generate "sequence/all_arrows" "@startuml
A -> B : solid
A --> B : dotted
A ->> B : open
A -->> B : open dotted
A ->x B : lost
A ->o B : circle
A <-> B : bidirectional
@enduml"

generate "sequence/participant_types" "@startuml
participant P
actor A
boundary B
control C
entity E
database D
collections Co
queue Q
P -> A : msg
@enduml"

generate "sequence/activation" "@startuml
A -> B ++ : activate
B -> C ++ : nested
C --> B -- : return
B --> A -- : return
@enduml"

generate "sequence/notes" "@startuml
A -> B : msg
note left : left note
note right : right note
note over A : over A
note over A, B : over both
@enduml"

generate "sequence/alt_else" "@startuml
A -> B : check
alt success
  B --> A : ok
else failure
  B --> A : error
end
@enduml"

generate "sequence/autonumber" "@startuml
autonumber
A -> B : first
B -> C : second
C --> B : third
@enduml"

# Class diagrams
generate "class/basic" "@startuml
class Animal {
  +name : String
  +makeSound() : void
}
class Dog extends Animal {
  +fetch() : void
}
Animal <|-- Dog
@enduml"

generate "class/relationships" "@startuml
A <|-- B
C ..|> D
E *-- F
G o-- H
I -- J
K ..> L
@enduml"

generate "class/interface" "@startuml
interface Drawable {
  +draw() : void
}
class Shape implements Drawable {
  #color : Color
}
Drawable <|.. Shape
@enduml"

generate "class/enum" "@startuml
enum Color {
  RED
  GREEN
  BLUE
}
@enduml"

# State diagrams
generate "state/basic" "@startuml
[*] --> Active
Active --> Inactive : disable
Inactive --> Active : enable
Active --> [*] : close
@enduml"

generate "state/nested" "@startuml
state Running {
  [*] --> Processing
  Processing --> Waiting : pause
  Waiting --> Processing : resume
}
[*] --> Running
Running --> [*] : shutdown
@enduml"

# Activity diagrams
generate "activity/basic" "@startuml
start
:Step 1;
if (condition?) then (yes)
  :Step 2a;
else (no)
  :Step 2b;
endif
stop
@enduml"

generate "activity/fork" "@startuml
start
:Initialize;
fork
  :Task A;
fork again
  :Task B;
end fork
:Finalize;
stop
@enduml"

# Component diagrams
generate "component/basic" "@startuml
component \"Web Server\" as WS
component \"Database\" as DB
WS --> DB : query
@enduml"

# Use case diagrams
generate "usecase/basic" "@startuml
actor User
usecase \"Login\" as UC1
usecase \"Browse\" as UC2
User --> UC1
User --> UC2
@enduml"

# Deployment diagrams
generate "deployment/basic" "@startuml
node WebServer {
  artifact app.war
}
database DB
cloud Internet
WebServer --> DB
Internet --> WebServer
@enduml"

echo ""
echo "Done. Golden files written to ${GOLDEN_DIR}/"
find "$GOLDEN_DIR" -name "*.svg" | wc -l | xargs echo "Total files:"
