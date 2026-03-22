#!/usr/bin/env python3
"""
Generator for PlantUML rendering quality golden files.
Tests professional rendering features: splines/routing, layout, text, and nesting.
Generates .puml + .svg pairs by querying the PlantUML server at http://127.0.0.1:8787.
"""

import json
import os
import time
import urllib.request

BASE_DIR = os.path.dirname(os.path.abspath(__file__))

CATEGORIES = {
    "splines": os.path.join(BASE_DIR, "rendering", "splines"),
    "layout":  os.path.join(BASE_DIR, "rendering", "layout"),
    "text":    os.path.join(BASE_DIR, "rendering", "text"),
    "nesting": os.path.join(BASE_DIR, "rendering", "nesting"),
}

for d in CATEGORIES.values():
    os.makedirs(d, exist_ok=True)

counts = {k: 0 for k in CATEGORIES}
render_calls = 0


def render(source: str):
    global render_calls
    body = json.dumps({"source": source, "options": ["-tsvg"]}).encode()
    req = urllib.request.Request(
        "http://127.0.0.1:8787/render",
        body,
        {"Content-Type": "application/json"},
    )
    try:
        result = urllib.request.urlopen(req, timeout=30).read().decode()
        render_calls += 1
        if render_calls % 20 == 0:
            time.sleep(0.5)
        return result
    except Exception as e:
        print(f"  [warn] render failed: {e}")
        return None


def save(category: str, name: str, source: str) -> bool:
    directory = CATEGORIES[category]
    puml_path = os.path.join(directory, f"{name}.puml")
    svg_path  = os.path.join(directory, f"{name}.svg")
    with open(puml_path, "w", encoding="utf-8") as f:
        f.write(source)
    svg = render(source)
    if svg and "<svg" in svg:
        with open(svg_path, "w", encoding="utf-8") as f:
            f.write(svg)
        counts[category] += 1
        return True
    return False


def wrap(body: str) -> str:
    return f"@startuml\n{body}\n@enduml\n"


# =============================================================================
# 1. SPLINES (25 cases)
# =============================================================================

# 1.1 linetype ortho — orthogonal connectors
save("splines", "splines_linetype_ortho_basic", wrap("""\
skinparam linetype ortho
class A
class B
class C
A --> B
B --> C
A --> C
"""))

# 1.2 linetype polyline — polyline connectors
save("splines", "splines_linetype_polyline_basic", wrap("""\
skinparam linetype polyline
class A
class B
class C
A --> B
B --> C
A --> C
"""))

# 1.3 Crossing edges, ortho
save("splines", "splines_ortho_crossing_edges", wrap("""\
skinparam linetype ortho
class A
class B
class C
class D
A --> D
B --> C
A --> C
B --> D
"""))

# 1.4 Crossing edges, polyline
save("splines", "splines_polyline_crossing_edges", wrap("""\
skinparam linetype polyline
class A
class B
class C
class D
A --> D
B --> C
A --> C
B --> D
"""))

# 1.5 Diamond inheritance — default linetype
save("splines", "splines_diamond_inheritance_default", wrap("""\
class A
class B
class C
class D
A <|-- B
A <|-- C
B <|-- D
C <|-- D
"""))

# 1.6 Diamond inheritance — ortho
save("splines", "splines_diamond_inheritance_ortho", wrap("""\
skinparam linetype ortho
class A
class B
class C
class D
A <|-- B
A <|-- C
B <|-- D
C <|-- D
"""))

# 1.7 Diamond inheritance — polyline
save("splines", "splines_diamond_inheritance_polyline", wrap("""\
skinparam linetype polyline
class A
class B
class C
class D
A <|-- B
A <|-- C
B <|-- D
C <|-- D
"""))

# 1.8 Star topology — default
save("splines", "splines_star_topology_default", wrap("""\
class Hub
class A
class B
class C
class D
class E
class F
Hub --> A
Hub --> B
Hub --> C
Hub --> D
Hub --> E
Hub --> F
"""))

# 1.9 Star topology — ortho
save("splines", "splines_star_topology_ortho", wrap("""\
skinparam linetype ortho
class Hub
class A
class B
class C
class D
class E
class F
Hub --> A
Hub --> B
Hub --> C
Hub --> D
Hub --> E
Hub --> F
"""))

# 1.10 Chain topology A->B->C->D->E — default
save("splines", "splines_chain_topology_default", wrap("""\
class A
class B
class C
class D
class E
A --> B
B --> C
C --> D
D --> E
"""))

# 1.11 Chain topology — ortho
save("splines", "splines_chain_topology_ortho", wrap("""\
skinparam linetype ortho
class A
class B
class C
class D
class E
A --> B
B --> C
C --> D
D --> E
"""))

# 1.12 Chain topology — polyline
save("splines", "splines_chain_topology_polyline", wrap("""\
skinparam linetype polyline
class A
class B
class C
class D
class E
A --> B
B --> C
C --> D
D --> E
"""))

# 1.13 Bidirectional edges
save("splines", "splines_bidirectional_edges", wrap("""\
class A
class B
class C
A <--> B
B <--> C
A <--> C
"""))

# 1.14 Bidirectional edges — ortho
save("splines", "splines_bidirectional_ortho", wrap("""\
skinparam linetype ortho
class A
class B
class C
A <--> B
B <--> C
"""))

# 1.15 Self-referencing edges
save("splines", "splines_self_reference", wrap("""\
class TreeNode {
  +TreeNode parent
  +List~TreeNode~ children
}
TreeNode --> TreeNode : contains
"""))

# 1.16 Self-reference — ortho
save("splines", "splines_self_reference_ortho", wrap("""\
skinparam linetype ortho
class Node
Node --> Node : next
"""))

# 1.17 Short connector (--)
save("splines", "splines_short_connector", wrap("""\
class A
class B
A -- B : short
"""))

# 1.18 Medium connector (----)
save("splines", "splines_medium_connector", wrap("""\
class A
class B
A ---- B : medium
"""))

# 1.19 Long connector (------)
save("splines", "splines_long_connector", wrap("""\
class A
class B
A ------ B : long
"""))

# 1.20 Mixed connector lengths
save("splines", "splines_mixed_lengths", wrap("""\
class A
class B
class C
class D
A -- B
B ---- C
C ------ D
"""))

# 1.21 Complex graph — ortho
save("splines", "splines_complex_ortho", wrap("""\
skinparam linetype ortho
class Controller
class Service
class Repository
class Model
class View
class DTO
Controller --> Service
Controller --> View
Service --> Repository
Service --> DTO
Repository --> Model
DTO --> Model
"""))

# 1.22 Complex graph — polyline
save("splines", "splines_complex_polyline", wrap("""\
skinparam linetype polyline
class Controller
class Service
class Repository
class Model
class View
class DTO
Controller --> Service
Controller --> View
Service --> Repository
Service --> DTO
Repository --> Model
DTO --> Model
"""))

# 1.23 Mixed arrow types — ortho
save("splines", "splines_mixed_arrows_ortho", wrap("""\
skinparam linetype ortho
class A
class B
class C
class D
A <|-- B
C ..|> A
D --> A
D *-- C
"""))

# 1.24 Sequence of inheritance levels with crossing
save("splines", "splines_multilevel_inheritance", wrap("""\
skinparam linetype ortho
abstract class Animal
class Mammal
class Bird
class Dog
class Cat
class Eagle
Animal <|-- Mammal
Animal <|-- Bird
Mammal <|-- Dog
Mammal <|-- Cat
Bird <|-- Eagle
Dog --> Cat : chases
"""))

# 1.25 Dense graph (many edges between few nodes)
save("splines", "splines_dense_graph", wrap("""\
skinparam linetype ortho
class A
class B
class C
class D
A --> B
A --> C
A --> D
B --> C
B --> D
C --> D
D --> A
"""))


# =============================================================================
# 2. LAYOUT (30 cases)
# =============================================================================

# 2.1 left to right direction — class diagram
save("layout", "layout_ltr_class", wrap("""\
left to right direction
class A
class B
class C
A --> B
B --> C
"""))

# 2.2 left to right direction — component diagram
save("layout", "layout_ltr_component", wrap("""\
left to right direction
component A
component B
component C
A --> B
B --> C
"""))

# 2.3 left to right direction — deployment diagram
save("layout", "layout_ltr_deployment", wrap("""\
left to right direction
node Server {
  artifact App
}
node DB
Server --> DB
"""))

# 2.4 left to right direction — usecase diagram
save("layout", "layout_ltr_usecase", wrap("""\
left to right direction
actor User
usecase UC1 as "Login"
usecase UC2 as "View Dashboard"
User --> UC1
User --> UC2
"""))

# 2.5 top to bottom direction — class
save("layout", "layout_ttb_class", wrap("""\
top to bottom direction
class A
class B
class C
A --> B
B --> C
"""))

# 2.6 top to bottom direction — component
save("layout", "layout_ttb_component", wrap("""\
top to bottom direction
component Frontend
component Backend
component Database
Frontend --> Backend
Backend --> Database
"""))

# 2.7 together grouping — basic
save("layout", "layout_together_basic", wrap("""\
together {
  class A
  class B
}
class C
A --> C
B --> C
"""))

# 2.8 together grouping — multiple groups
save("layout", "layout_together_multiple", wrap("""\
together {
  class A
  class B
  class C
}
together {
  class X
  class Y
}
class Hub
A --> Hub
X --> Hub
"""))

# 2.9 nodesep — tight spacing
save("layout", "layout_nodesep_tight", wrap("""\
skinparam nodesep 10
class A
class B
class C
class D
A --> B
C --> D
A --> C
"""))

# 2.10 nodesep — wide spacing
save("layout", "layout_nodesep_wide", wrap("""\
skinparam nodesep 80
class A
class B
class C
A --> B
B --> C
"""))

# 2.11 ranksep — tight spacing
save("layout", "layout_ranksep_tight", wrap("""\
skinparam ranksep 10
class A
class B
class C
A --> B
B --> C
"""))

# 2.12 ranksep — wide spacing
save("layout", "layout_ranksep_wide", wrap("""\
skinparam ranksep 80
class A
class B
class C
A --> B
B --> C
"""))

# 2.13 nodesep + ranksep combined
save("layout", "layout_nodesep_ranksep_combined", wrap("""\
skinparam nodesep 40
skinparam ranksep 60
class A
class B
class C
class D
A --> B
A --> C
B --> D
C --> D
"""))

# 2.14 Large diagram: 10 classes flat
_classes_10 = "\n".join(f"class C{i}" for i in range(10))
_arrows_10  = "\n".join(f"C{i} --> C{i+1}" for i in range(9))
save("layout", "layout_large_10_classes_chain", wrap(f"{_classes_10}\n{_arrows_10}"))

# 2.15 Large diagram: 10 classes with inheritance tree
save("layout", "layout_large_10_classes_tree", wrap("""\
class Root
class L1A
class L1B
class L2A
class L2B
class L2C
class L2D
class L3A
class L3B
class L3C
Root <|-- L1A
Root <|-- L1B
L1A <|-- L2A
L1A <|-- L2B
L1B <|-- L2C
L1B <|-- L2D
L2A <|-- L3A
L2B <|-- L3B
L2C <|-- L3C
"""))

# 2.16 Large diagram: 20 classes
_classes_20 = "\n".join(f"class Node{i}" for i in range(20))
_arrows_20  = "\n".join(f"Node{i} --> Node{i+1}" for i in range(19))
save("layout", "layout_large_20_classes_chain", wrap(f"{_classes_20}\n{_arrows_20}"))

# 2.17 Large diagram: 20 classes with star hub
_classes_20s = "\n".join(f"class S{i}" for i in range(20))
_arrows_20s  = "class Hub\n" + "\n".join(f"Hub --> S{i}" for i in range(20))
save("layout", "layout_large_20_classes_star", wrap(f"{_classes_20s}\n{_arrows_20s}"))

# 2.18 Large diagram: 50 classes flat inheritance
_classes_50 = "\n".join(f"class Item{i}" for i in range(50))
_arrows_50  = "\n".join(f"Item{i} <|-- Item{i+1}" for i in range(49))
save("layout", "layout_large_50_classes_chain", wrap(f"{_classes_50}\n{_arrows_50}"))

# 2.19 Wide-and-shallow hierarchy
save("layout", "layout_wide_shallow", wrap("""\
class Root
class A1
class A2
class A3
class A4
class A5
class A6
class A7
class A8
class A9
class A10
Root <|-- A1
Root <|-- A2
Root <|-- A3
Root <|-- A4
Root <|-- A5
Root <|-- A6
Root <|-- A7
Root <|-- A8
Root <|-- A9
Root <|-- A10
"""))

# 2.20 Deep-and-narrow hierarchy
save("layout", "layout_deep_narrow", wrap("""\
class L0
class L1
class L2
class L3
class L4
class L5
class L6
class L7
class L8
class L9
L0 <|-- L1
L1 <|-- L2
L2 <|-- L3
L3 <|-- L4
L4 <|-- L5
L5 <|-- L6
L6 <|-- L7
L7 <|-- L8
L8 <|-- L9
"""))

# 2.21 Package layout — classes in separate packages connected
save("layout", "layout_package_cross_connect", wrap("""\
package "UI Layer" {
  class View
  class ViewModel
}
package "Business Layer" {
  class Service
  class UseCase
}
package "Data Layer" {
  class Repository
  class Entity
}
ViewModel --> Service
Service --> UseCase
UseCase --> Repository
Repository --> Entity
"""))

# 2.22 Package layout — ltr
save("layout", "layout_package_ltr", wrap("""\
left to right direction
package Frontend {
  class Controller
  class View
}
package Backend {
  class API
  class Logic
}
package Database {
  class DAO
  class Model
}
Controller --> API
API --> Logic
Logic --> DAO
DAO --> Model
"""))

# 2.23 State diagram — horizontal transitions
save("layout", "layout_state_horizontal", wrap("""\
left to right direction
[*] --> Idle
Idle --> Processing : start
Processing --> Done : finish
Processing --> Error : fail
Error --> Idle : reset
Done --> [*]
"""))

# 2.24 State diagram — vertical transitions
save("layout", "layout_state_vertical", wrap("""\
top to bottom direction
[*] --> Init
Init --> Ready : setup complete
Ready --> Running : execute
Running --> Paused : pause
Paused --> Running : resume
Running --> Stopped : stop
Stopped --> [*]
"""))

# 2.25 Mixed spacing in LTR class
save("layout", "layout_ltr_nodesep_ranksep", wrap("""\
left to right direction
skinparam nodesep 30
skinparam ranksep 50
class A
class B
class C
class D
A --> B
A --> C
B --> D
C --> D
"""))

# 2.26 Complex class with ltr + together
save("layout", "layout_ltr_together", wrap("""\
left to right direction
together {
  class Service
  class Repository
}
class Controller
class Model
Controller --> Service
Service --> Repository
Repository --> Model
"""))

# 2.27 Deployment ltr with multiple nodes
save("layout", "layout_deployment_ltr_complex", wrap("""\
left to right direction
node "Web Server" {
  artifact "Frontend App"
}
node "App Server" {
  artifact "Backend API"
}
node "DB Server" {
  database "PostgreSQL"
}
"Frontend App" --> "Backend API" : HTTP
"Backend API" --> "PostgreSQL" : SQL
"""))

# 2.28 Component ltr with interfaces
save("layout", "layout_component_ltr_interfaces", wrap("""\
left to right direction
component A
component B
component C
interface IFoo
interface IBar
A -( IFoo
B -- IFoo
B -( IBar
C -- IBar
"""))

# 2.29 Usecase ttb with includes/extends
save("layout", "layout_usecase_ttb_include_extend", wrap("""\
top to bottom direction
actor User
actor Admin
usecase Login
usecase ViewDash as "View Dashboard"
usecase ManageUsers as "Manage Users"
usecase Logout
User --> Login
User --> ViewDash
Admin --> ManageUsers
ViewDash .> Login : <<include>>
ManageUsers .> Login : <<include>>
Logout .> Login : <<extend>>
"""))

# 2.30 Large ltr with nodesep/ranksep
_large_ltr = "left to right direction\nskinparam nodesep 20\nskinparam ranksep 40\n"
_large_ltr += "\n".join(f"class M{i}" for i in range(15))
_large_ltr += "\n" + "\n".join(f"M{i} --> M{i+1}" for i in range(14))
save("layout", "layout_large_ltr_spacing", wrap(_large_ltr))


# =============================================================================
# 3. TEXT (20 cases)
# =============================================================================

# 3.1 Very long class name
save("text", "text_long_class_name", wrap("""\
class VeryLongClassNameThatExceedsThirtyCharactersEasily {
  +String value
}
"""))

# 3.2 Very long method signature
save("text", "text_long_method_signature", wrap("""\
class MyClass {
  +ResultType veryLongMethodNameWithManyParameters(FirstParameterType firstParam, SecondParameterType secondParam, ThirdParameterType thirdParam)
}
"""))

# 3.3 Very long message label in sequence diagram
save("text", "text_long_sequence_label", wrap("""\
participant Client
participant Server
Client -> Server : This is a very long message label that describes in detail what operation is being invoked here
Server --> Client : Response with equally long description of what was returned by the server
"""))

# 3.4 Labels with special chars — ampersand
save("text", "text_special_chars_ampersand", wrap("""\
class "Fish & Chips" {
  +String name
}
class "Bread & Butter"
"Fish & Chips" --> "Bread & Butter"
"""))

# 3.5 Labels with angle brackets
save("text", "text_special_chars_angle_brackets", wrap("""\
class MyClass {
  +List<String> items
  +Map<String, Integer> counts
  +Optional<List<String>> nested
}
"""))

# 3.6 Labels with quotes
save("text", "text_special_chars_quotes", wrap("""\
class Config {
  +String getValue(String "key")
}
note right : Use \\"quoted\\" keys
"""))

# 3.7 Multi-line note
save("text", "text_multiline_note", wrap("""\
class Service
note right of Service
  This is line one of the note.
  This is line two of the note.
  This is line three of the note.
  And this is line four.
end note
"""))

# 3.8 Unicode class names
save("text", "text_unicode_class_names", wrap("""\
class Ärger
class Übung
class Öffnung
Ärger --> Übung
Übung --> Öffnung
"""))

# 3.9 Unicode in labels and notes
save("text", "text_unicode_labels_notes", wrap("""\
class Customer {
  +String name
  +String 電話番号
}
note right : 顧客エンティティ
"""))

# 3.10 Unicode in sequence diagram
save("text", "text_unicode_sequence", wrap("""\
participant "Пользователь" as User
participant "Сервер" as Server
User -> Server : Привет, мир!
Server --> User : Ответ
"""))

# 3.11 Empty labels
save("text", "text_empty_labels", wrap("""\
class A
class B
class C
A --> B :
B --> C
A .> C : " "
"""))

# 3.12 Bold/italic in notes via creole
save("text", "text_creole_bold_italic_note", wrap("""\
class MyClass
note right : This is **bold** and this is //italic// and this is **//both//**
"""))

# 3.13 Mixed creole in class body
save("text", "text_creole_in_class", wrap("""\
class FormattedClass {
  +**String** boldField
  +//int// italicField
  +__double__ underlineField
}
"""))

# 3.14 Very long note
save("text", "text_very_long_note", wrap("""\
class DocumentedClass
note right of DocumentedClass
  This note contains a very long description that goes into\n  significant detail about the class. It spans multiple lines\n  and tests how the renderer handles large text blocks in notes.\n  Additional lines keep adding content to stress the layout.\n  Final line of the note.
end note
"""))

# 3.15 Long label in sequence across multiple participants
save("text", "text_long_sequence_multiparty", wrap("""\
participant A
participant B
participant C
participant D
A -> B : Initialize with extremely long parameter list: param1=value1, param2=value2, param3=value3
B -> C : Forward request with context: ctx={id:123, user:"admin", timestamp:"2024-01-01T00:00:00Z"}
C -> D : Process and persist
D --> C : Acknowledgement
C --> B : Result
B --> A : Final response
"""))

# 3.16 Special chars in sequence labels
save("text", "text_sequence_special_chars", wrap("""\
participant Client
participant API
Client -> API : GET /api/v1/resource?filter=foo&sort=asc&limit=10
API --> Client : 200 OK { "status": "success", "data": [...] }
"""))

# 3.17 All-Unicode sequence diagram
save("text", "text_unicode_all_sequence", wrap("""\
participant 客户端
participant 服务器
participant 数据库
客户端 -> 服务器 : 请求数据
服务器 -> 数据库 : 查询
数据库 --> 服务器 : 结果集
服务器 --> 客户端 : JSON响应
"""))

# 3.18 Long class with many methods
save("text", "text_many_long_methods", wrap("""\
class VerboseService {
  +ResponseObject processComplexRequestWithMultipleParameters(RequestObject request, ConfigObject config, ContextObject context)
  +ValidationResult validateAndTransformInput(InputObject rawInput, SchemaObject schema)
  +CompletableFuture<ResultObject> executeAsynchronouslyWithRetry(TaskObject task, RetryPolicy policy)
  +List<ReportObject> generateComprehensiveReport(DateRange range, FilterCriteria criteria)
}
"""))

# 3.19 Newlines in notes
save("text", "text_notes_with_newlines", wrap("""\
class A
class B
A --> B : connects to
note on link
  First line
  Second line
  Third line
end note
"""))

# 3.20 Mixed lengths of labels in one diagram
save("text", "text_mixed_label_lengths", wrap("""\
class Short
class AVeryMuchLongerClassName
class Medium123
Short --> AVeryMuchLongerClassName : a very long connector label that describes the relationship
Short --> Medium123 : short
AVeryMuchLongerClassName --> Medium123 :
"""))


# =============================================================================
# 4. NESTING (25 cases)
# =============================================================================

# 4.1 2-level package nesting
save("nesting", "nesting_package_2level", wrap("""\
package Outer {
  package Inner {
    class A
    class B
  }
  class C
}
"""))

# 4.2 3-level package nesting
save("nesting", "nesting_package_3level", wrap("""\
package L1 {
  package L2 {
    package L3 {
      class Deep
    }
    class Mid
  }
  class Top
}
"""))

# 4.3 5-level package nesting
save("nesting", "nesting_package_5level", wrap("""\
package P1 {
  package P2 {
    package P3 {
      package P4 {
        package P5 {
          class VeryDeep
        }
        class L4
      }
      class L3
    }
    class L2
  }
  class L1
}
"""))

# 4.4 Nested packages with cross-package connections
save("nesting", "nesting_package_cross_connections", wrap("""\
package Frontend {
  package Components {
    class Button
    class Input
  }
  class Page
}
package Backend {
  package Services {
    class UserService
    class AuthService
  }
  class API
}
Button --> API
Page --> UserService
AuthService --> UserService
"""))

# 4.5 2-level package with ltr
save("nesting", "nesting_package_2level_ltr", wrap("""\
left to right direction
package Outer {
  package Inner {
    class X
    class Y
  }
  class Z
}
X --> Z
"""))

# 4.6 Composite state — 2 inner states
save("nesting", "nesting_composite_state_2inner", wrap("""\
state Outer {
  [*] --> S1
  S1 --> S2
  S2 --> [*]
}
"""))

# 4.7 Composite state — 3 inner states
save("nesting", "nesting_composite_state_3inner", wrap("""\
state Processing {
  [*] --> Validate
  Validate --> Transform : valid
  Transform --> Persist
  Persist --> [*]
  Validate --> [*] : invalid
}
"""))

# 4.8 Composite state — 5 inner states
save("nesting", "nesting_composite_state_5inner", wrap("""\
state Pipeline {
  [*] --> Ingest
  Ingest --> Parse
  Parse --> Enrich
  Enrich --> Validate
  Validate --> Output
  Output --> [*]
  Validate --> Ingest : retry
}
"""))

# 4.9 Concurrent state regions (-- separator)
save("nesting", "nesting_concurrent_regions", wrap("""\
state Concurrent {
  state "Region A" as RA {
    [*] --> A1
    A1 --> A2
    A2 --> [*]
  }
  --
  state "Region B" as RB {
    [*] --> B1
    B1 --> [*]
  }
}
"""))

# 4.10 Concurrent regions with 3 regions
save("nesting", "nesting_concurrent_3regions", wrap("""\
state ThreeWay {
  state "Audio" as Audio {
    [*] --> Playing
    Playing --> Paused
    Paused --> Playing
    Playing --> [*]
  }
  --
  state "Video" as Video {
    [*] --> Rendering
    Rendering --> [*]
  }
  --
  state "Network" as Network {
    [*] --> Connecting
    Connecting --> Connected
    Connected --> [*]
  }
}
"""))

# 4.11 Nested activity partitions
save("nesting", "nesting_activity_partitions", wrap("""\
|User|
start
:Submit Request;
|System|
:Validate Input;
if (Valid?) then (yes)
  |Backend|
  :Process;
  :Save;
  |System|
  :Send Confirmation;
else (no)
  :Return Error;
endif
|User|
stop
"""))

# 4.12 Nested component packages
save("nesting", "nesting_component_packages", wrap("""\
package UI {
  component WebApp
  component MobileApp
}
package API {
  component Gateway
  component Router
}
package Core {
  component BusinessLogic
  component DataAccess
}
WebApp --> Gateway
MobileApp --> Gateway
Gateway --> Router
Router --> BusinessLogic
BusinessLogic --> DataAccess
"""))

# 4.13 Deployment nodes containing artifacts containing components
save("nesting", "nesting_deployment_nodes_artifacts", wrap("""\
node "Production Server" {
  artifact "web-app.war" {
    component Frontend
    component API
  }
  artifact "backend.jar" {
    component Service
    component Repository
  }
}
node "Database Server" {
  artifact "postgres" {
    database MainDB
  }
}
Repository --> MainDB
"""))

# 4.14 Deep deployment nesting
save("nesting", "nesting_deployment_deep", wrap("""\
node Datacenter {
  node Rack {
    node Server {
      node VM {
        artifact Container {
          component App
        }
      }
    }
  }
}
"""))

# 4.15 Composite state — nested composites
save("nesting", "nesting_composite_state_nested", wrap("""\
state Outer {
  state Inner {
    [*] --> A
    A --> B
    B --> [*]
  }
  [*] --> Inner
  Inner --> Done
  Done --> [*]
}
"""))

# 4.16 Package with deeply nested connections
save("nesting", "nesting_package_deep_connections", wrap("""\
package A {
  package B {
    class X
  }
}
package C {
  package D {
    class Y
  }
}
X --> Y
"""))

# 4.17 3-level nesting with ltr
save("nesting", "nesting_package_3level_ltr", wrap("""\
left to right direction
package P1 {
  package P2 {
    package P3 {
      class Core
    }
    class Middle
  }
  class Outer
}
Core --> Middle
Middle --> Outer
"""))

# 4.18 Component inside deployment with connections
save("nesting", "nesting_component_in_deployment", wrap("""\
node AppServer {
  component WebApp
  component Cache
}
node DBServer {
  component Database
}
WebApp --> Cache : reads
WebApp --> Database : persists
"""))

# 4.19 Namespace nesting (class diagram)
save("nesting", "nesting_namespace", wrap("""\
namespace com.example.ui {
  class View
  class Controller
}
namespace com.example.service {
  class UserService
  class AuthService
}
namespace com.example.data {
  class UserRepo
}
Controller --> UserService
UserService --> UserRepo
AuthService --> UserRepo
"""))

# 4.20 State with entry/exit in composite
save("nesting", "nesting_state_entry_exit", wrap("""\
state Active {
  state Idle
  state Working {
    entry : begin task
    exit : cleanup
  }
  [*] --> Idle
  Idle --> Working : trigger
  Working --> Idle : done
}
[*] --> Active
Active --> [*]
"""))

# 4.21 Nested partitions in activity
save("nesting", "nesting_activity_nested_partitions", wrap("""\
|#LightBlue|CustomerDept|
start
:Submit Order;
|#LightGreen|SalesDept|
:Review Order;
if (Approve?) then (yes)
  |WarehouseDept|
  :Pick Items;
  :Pack;
  |ShippingDept|
  :Ship;
else (no)
  |CustomerDept|
  :Notify Rejection;
endif
stop
"""))

# 4.22 Multiple nested composite states
save("nesting", "nesting_multiple_composites", wrap("""\
state System {
  state Auth {
    [*] --> LoggedOut
    LoggedOut --> LoggedIn : login
    LoggedIn --> LoggedOut : logout
  }
  state Session {
    [*] --> Active
    Active --> Expired : timeout
    Expired --> [*]
  }
  Auth --> Session : authenticated
}
"""))

# 4.23 2-level package with multiple nested sub-packages
save("nesting", "nesting_package_multiple_subpackages", wrap("""\
package Application {
  package Controllers {
    class UserCtrl
    class ProductCtrl
    class OrderCtrl
  }
  package Services {
    class UserSvc
    class ProductSvc
    class OrderSvc
  }
  package Repositories {
    class UserRepo
    class ProductRepo
    class OrderRepo
  }
}
UserCtrl --> UserSvc
ProductCtrl --> ProductSvc
OrderCtrl --> OrderSvc
UserSvc --> UserRepo
ProductSvc --> ProductRepo
OrderSvc --> OrderRepo
"""))

# 4.24 Component packages with interfaces and nesting
save("nesting", "nesting_component_interfaces", wrap("""\
package Platform {
  package Core {
    interface IStorage
    interface IMessaging
    component StorageImpl
    component MessagingImpl
  }
  package Applications {
    component App1
    component App2
  }
}
App1 --> IStorage
App2 --> IStorage
App1 --> IMessaging
StorageImpl ..|> IStorage
MessagingImpl ..|> IMessaging
"""))

# 4.25 Concurrent state with nested composites
save("nesting", "nesting_concurrent_with_composites", wrap("""\
state ComplexSystem {
  state "Module A" as MA {
    [*] --> A_Idle
    A_Idle --> A_Working : start
    A_Working --> A_Idle : done
  }
  --
  state "Module B" as MB {
    state "B_Sub" as BS {
      [*] --> B1
      B1 --> B2
      B2 --> [*]
    }
    [*] --> BS
    BS --> B_Done
    B_Done --> [*]
  }
}
"""))


# =============================================================================
# Summary
# =============================================================================

print("Rendering quality golden files generated:")
for cat, count in counts.items():
    print(f"  {cat:10s}: {count} pairs")
print(f"  {'TOTAL':10s}: {sum(counts.values())} pairs")
