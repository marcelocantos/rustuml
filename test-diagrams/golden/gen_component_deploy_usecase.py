#!/usr/bin/env python3
"""
Generator for comprehensive PlantUML test cases:
- Component diagrams (~600 files)
- Deployment diagrams (~500 files)
- Use case diagrams (~400 files)
"""

import os
import itertools

BASE = os.path.dirname(os.path.abspath(__file__))
COMP_DIR = os.path.join(BASE, "component")
DEPLOY_DIR = os.path.join(BASE, "deployment")
USECASE_DIR = os.path.join(BASE, "usecase")

for d in [COMP_DIR, DEPLOY_DIR, USECASE_DIR]:
    os.makedirs(d, exist_ok=True)


def write(path, content):
    with open(path, "w") as f:
        f.write(content.strip() + "\n")


# ---------------------------------------------------------------------------
# COMPONENT DIAGRAMS
# ---------------------------------------------------------------------------

def gen_component():
    idx = [0]

    def save(name, content):
        write(os.path.join(COMP_DIR, f"{name}.puml"), content)
        idx[0] += 1

    # --- Basic element types ---
    save("comp_basic_component", """
@startuml
component Foo
component Bar
Foo --> Bar
@enduml
""")

    save("comp_basic_interface", """
@startuml
component Foo
interface IFoo
Foo - IFoo
@enduml
""")

    save("comp_basic_port", """
@startuml
component Foo {
  port p1
  port p2
}
Foo::p1 --> Foo::p2
@enduml
""")

    save("comp_bracket_notation", """
@startuml
[Foo] --> [Bar]
[Baz] ..> [Qux]
@enduml
""")

    save("comp_bracket_with_interface", """
@startuml
[Foo] - IFoo
IFoo - [Bar]
@enduml
""")

    # --- Lollipop / socket ---
    save("comp_lollipop_provided", """
@startuml
component Foo {
  interface IFoo
}
Foo -( IFoo
@enduml
""")

    save("comp_lollipop_required", """
@startuml
component Foo
component Bar
Foo -(0- Bar : uses
@enduml
""")

    save("comp_socket_notation", """
@startuml
component A
component B
A -(0)- B
@enduml
""")

    save("comp_lollipop_complex", """
@startuml
component Server {
  interface HTTP
  interface HTTPS
}
component Client
Client --( HTTP
Client --( HTTPS
@enduml
""")

    # --- Arrow types ---
    for arrow, aname in [
        ("-->", "dep"), ("..>", "use"), ("--", "assoc"), ("..", "real"),
        ("->", "nav"), (".-.", "iface"), ("--->", "long_dep"),
        ("....>", "long_use"),
    ]:
        save(f"comp_arrow_{aname}", f"""
@startuml
component Alpha
component Beta
Alpha {arrow} Beta
@enduml
""")

    # --- Arrow directions ---
    for direction in ["up", "down", "left", "right"]:
        save(f"comp_arrow_dir_{direction}", f"""
@startuml
component A
component B
A -{direction}-> B
@enduml
""")

    # --- Arrow type + direction combos ---
    for arrow, aname in [("-->", "dep"), ("..>", "use"), ("->", "nav"), ("--", "assoc")]:
        for d in ["up", "down", "left", "right"]:
            save(f"comp_arrow_{aname}_dir_{d}", f"""
@startuml
component Source
component Target
Source -{d}{arrow[1:]} Target
@enduml
""")

    # --- Container types ---
    for container in ["package", "node", "folder", "frame", "cloud", "database", "rectangle"]:
        save(f"comp_container_{container}", f"""
@startuml
{container} MyContainer {{
  component Foo
  component Bar
  Foo --> Bar
}}
@enduml
""")

    # --- Containers with colors ---
    for container in ["package", "node", "folder", "frame", "cloud", "database", "rectangle"]:
        for color in ["#LightBlue", "#LightGreen", "#Pink", "#Yellow"]:
            safe_c = color.replace("#", "")
            save(f"comp_container_{container}_{safe_c}", f"""
@startuml
{container} MyContainer {color} {{
  component Foo
  component Bar
  Foo --> Bar
}}
@enduml
""")

    # --- Containers with stereotypes ---
    for container in ["package", "node", "cloud", "rectangle"]:
        for stereo in ["<<external>>", "<<internal>>", "<<service>>"]:
            safe_s = stereo.replace("<<", "").replace(">>", "")
            save(f"comp_container_{container}_stereo_{safe_s}", f"""
@startuml
{container} MyContainer {stereo} {{
  component Foo
  component Bar
  Foo --> Bar
}}
@enduml
""")

    # --- Nested containers 2 levels ---
    for outer, inner in itertools.product(
        ["package", "node", "folder", "frame", "cloud", "rectangle"],
        ["package", "node", "folder", "frame"]
    ):
        save(f"comp_nested2_{outer}_{inner}", f"""
@startuml
{outer} Outer {{
  {inner} Inner {{
    component Alpha
    component Beta
    Alpha --> Beta
  }}
  component Gamma
  Inner --> Gamma
}}
@enduml
""")

    # --- Nested containers 3 levels ---
    for combo in itertools.islice(
        itertools.product(["package", "node", "cloud"], ["frame", "folder"], ["rectangle", "package"]),
        20
    ):
        o, m, i = combo
        save(f"comp_nested3_{o}_{m}_{i}", f"""
@startuml
{o} L1 {{
  {m} L2 {{
    {i} L3 {{
      component DeepComp
    }}
    component MidComp
    L3 --> MidComp
  }}
  component TopComp
  L2 --> TopComp
}}
@enduml
""")

    # --- Nested 4 levels ---
    save("comp_nested4_deep", """
@startuml
package Level1 {
  folder Level2 {
    frame Level3 {
      rectangle Level4 {
        component CoreService
      }
      component L3Component
      Level4 --> L3Component
    }
    component L2Component
    Level3 --> L2Component
  }
  component L1Component
  Level2 --> L1Component
}
@enduml
""")

    # --- Stereotypes ---
    for stereo in ["<<service>>", "<<controller>>", "<<repository>>", "<<facade>>",
                   "<<adapter>>", "<<gateway>>", "<<factory>>", "<<singleton>>"]:
        safe = stereo.replace("<<", "").replace(">>", "")
        save(f"comp_stereotype_{safe}", f"""
@startuml
component Foo {stereo}
component Bar {stereo}
Foo --> Bar
@enduml
""")

    save("comp_stereotype_multiple", """
@startuml
component AuthService <<service>> <<secured>>
component UserRepo <<repository>> <<persistent>>
AuthService --> UserRepo : uses
@enduml
""")

    # --- Component + stereotype + color combos ---
    for stereo in ["<<service>>", "<<repository>>", "<<gateway>>"]:
        for color in ["#LightBlue", "#LightGreen", "#Pink"]:
            safe_s = stereo.replace("<<", "").replace(">>", "")
            safe_c = color.replace("#", "")
            save(f"comp_stereo_{safe_s}_color_{safe_c}", f"""
@startuml
component MyComp {stereo} {color}
component OtherComp {stereo} {color}
MyComp --> OtherComp
@enduml
""")

    # --- Colors on components ---
    colors = ["#Pink", "#LightBlue", "#Yellow", "#LightGreen", "#Orange",
              "#Violet", "#Cyan", "#AAFFAA", "#FF8888", "#FFAAFF",
              "#AAAAFF", "#FFD700", "#90EE90", "#FFA07A", "#87CEEB"]
    for i, color in enumerate(colors):
        safe = color.replace("#", "hex")
        save(f"comp_color_{i:02d}_{safe}", f"""
@startuml
component Foo {color}
component Bar {color}
Foo --> Bar
@enduml
""")

    save("comp_color_mixed", """
@startuml
component A #Pink
component B #LightBlue
component C #Yellow
A --> B
B --> C
C --> A
@enduml
""")

    # --- Notes ---
    save("comp_note_on_component", """
@startuml
component Foo
note right of Foo : This is a note
@enduml
""")

    save("comp_note_on_link", """
@startuml
component Foo
component Bar
Foo --> Bar : uses
note on link : This link note
@enduml
""")

    save("comp_note_floating", """
@startuml
component Foo
component Bar
Foo --> Bar
note "Floating note" as N1
N1 .. Foo
@enduml
""")

    save("comp_note_multiline", """
@startuml
component Server
note right of Server
  This server handles
  multiple requests
  concurrently
end note
@enduml
""")

    # Note position variants
    for pos in ["top", "bottom", "left", "right"]:
        save(f"comp_note_{pos}", f"""
@startuml
component MyComp
note {pos} of MyComp : {pos.capitalize()} note text
@enduml
""")

    # Note on each container type
    for container in ["package", "node", "folder", "cloud"]:
        save(f"comp_note_on_{container}", f"""
@startuml
{container} MyContainer {{
  component Foo
}}
note right of MyContainer : Note on {container}
@enduml
""")

    # --- Required / provided interfaces ---
    save("comp_required_provided", """
@startuml
component WebApp {
  interface [IDatabase] as IDB
  interface [ICache] as ICache
}
component Database
component Cache
WebApp --> Database : IDB
WebApp --> Cache : ICache
@enduml
""")

    save("comp_provided_interface_explicit", """
@startuml
component ServiceA
component ServiceB
interface IService
ServiceA --( IService : provides
ServiceB --> IService : requires
@enduml
""")

    # --- Groups / together ---
    save("comp_together_basic", """
@startuml
together {
  component A
  component B
  component C
}
component D
A --> D
B --> D
C --> D
@enduml
""")

    save("comp_together_multiple", """
@startuml
together {
  component Frontend
  component Gateway
}
together {
  component ServiceA
  component ServiceB
}
Frontend --> ServiceA
Gateway --> ServiceB
@enduml
""")

    # --- Hide / show / remove ---
    save("comp_hide_component", """
@startuml
component A
component B
component C
A --> B
B --> C
hide C
@enduml
""")

    save("comp_remove_component", """
@startuml
component A
component B
component C
A --> B
B --> C
remove C
@enduml
""")

    save("comp_hide_stereotype", """
@startuml
component A <<internal>>
component B <<public>>
hide <<internal>>
@enduml
""")

    # --- Relationship labels ---
    save("comp_arrow_labels", """
@startuml
component A
component B
component C
A --> B : calls
B ..> C : delegates
A -- C : associates
@enduml
""")

    # Label variants
    for label in ["uses", "calls", "delegates", "extends", "implements",
                  "publishes", "subscribes", "depends on", "requires"]:
        safe = label.replace(" ", "_")
        save(f"comp_label_{safe}", f"""
@startuml
component Foo
component Bar
Foo --> Bar : {label}
@enduml
""")

    # --- Skinparam ---
    save("comp_skinparam_basic", """
@startuml
skinparam component {
  BackgroundColor LightBlue
  BorderColor DarkBlue
  ArrowColor Navy
}
component Foo
component Bar
Foo --> Bar
@enduml
""")

    save("comp_skinparam_roundcorner", """
@startuml
skinparam roundcorner 15
skinparam componentStyle uml2
component A
component B
A --> B
@enduml
""")

    save("comp_skinparam_monochrome", """
@startuml
skinparam monochrome true
component Foo
component Bar
Foo --> Bar
@enduml
""")

    save("comp_skinparam_comprehensive", """
@startuml
skinparam component {
  BackgroundColor<<service>> LightGreen
  BorderColor<<service>> DarkGreen
  BackgroundColor<<db>> LightBlue
  BorderColor<<db>> DarkBlue
  FontSize 14
  FontColor Black
}
component AuthService <<service>>
component UserService <<service>>
database UserDB <<db>>
AuthService --> UserService : delegates
UserService --> UserDB : queries
@enduml
""")

    for style in ["uml2", "rectangle"]:
        save(f"comp_style_{style}", f"""
@startuml
skinparam componentStyle {style}
component A
component B
component C
A --> B
B --> C
@enduml
""")

    # --- Edge cases ---
    save("comp_empty", """
@startuml
@enduml
""")

    save("comp_single_element", """
@startuml
component Alone
@enduml
""")

    save("comp_single_no_relations", """
@startuml
component A
component B
component C
@enduml
""")

    save("comp_unicode_names", """
@startuml
component "Composant Français" as CF
component "Deutsche Komponente" as DK
component "日本語コンポーネント" as JC
CF --> DK
DK --> JC
@enduml
""")

    save("comp_special_chars", """
@startuml
component "Service-Alpha" as SA
component "Service_Beta" as SB
component "Service.Gamma" as SG
SA --> SB : "link-1"
SB --> SG : "link_2"
@enduml
""")

    # --- Ring graphs of various sizes ---
    for n in [3, 4, 5, 6, 8, 10, 12, 15, 20]:
        comps = "\n".join([f"component C{i:02d}" for i in range(1, n+1)])
        arrows = "\n".join([f"C{i:02d} --> C{((i) % n) + 1:02d}" for i in range(1, n+1)])
        save(f"comp_ring_{n}", f"""
@startuml
{comps}
{arrows}
@enduml
""")

    # --- Chain graphs ---
    for n in [3, 5, 8, 10]:
        comps = "\n".join([f"component C{i:02d}" for i in range(1, n+1)])
        arrows = "\n".join([f"C{i:02d} --> C{i+1:02d}" for i in range(1, n)])
        save(f"comp_chain_{n}", f"""
@startuml
{comps}
{arrows}
@enduml
""")

    # --- Star graphs ---
    for n in [3, 5, 8, 10]:
        spokes = "\n".join([f"component Spoke{i}" for i in range(1, n+1)])
        arrows = "\n".join([f"Hub --> Spoke{i}" for i in range(1, n+1)])
        save(f"comp_star_{n}", f"""
@startuml
component Hub
{spokes}
{arrows}
@enduml
""")

    # --- Fully connected ---
    save("comp_fully_connected_5", """
@startuml
component A
component B
component C
component D
component E
A --> B
A --> C
A --> D
A --> E
B --> C
B --> D
B --> E
C --> D
C --> E
D --> E
@enduml
""")

    # --- Bidirectional ---
    save("comp_bidirectional", """
@startuml
component A
component B
A <--> B
@enduml
""")

    save("comp_bidirectional_many", """
@startuml
component A
component B
component C
A <--> B
B <--> C
A <..> C
@enduml
""")

    # --- Self link ---
    save("comp_self_link", """
@startuml
component A
A -> A : self-call
@enduml
""")

    # --- Alias ---
    save("comp_alias", """
@startuml
component "Very Long Component Name" as VLCN
component "Another Long Name" as ALN
VLCN --> ALN
@enduml
""")

    # --- Title/header/footer/legend ---
    save("comp_title", """
@startuml
title My Component Diagram
component A
component B
A --> B
@enduml
""")

    save("comp_title_multiline", """
@startuml
title
  My Complex
  Component Diagram
end title
component A
component B
A --> B
@enduml
""")

    save("comp_header_footer", """
@startuml
header Generated by RustUML
footer Page 1
component A
component B
A --> B
@enduml
""")

    save("comp_legend", """
@startuml
component A
component B
A --> B
legend right
  | Arrow | Meaning |
  | --> | depends on |
endlegend
@enduml
""")

    # --- Mixed arrow types ---
    save("comp_mixed_arrows", """
@startuml
component A
component B
component C
component D
component E
A --> B : dependency
B ..> C : usage
C -- D : association
D .. E : realization
@enduml
""")

    # --- Ports ---
    save("comp_ports_complex", """
@startuml
component Router {
  portin pIn1
  portin pIn2
  portout pOut1
  portout pOut2
}
component Sender1
component Sender2
component Receiver1
component Receiver2
Sender1 --> Router::pIn1
Sender2 --> Router::pIn2
Router::pOut1 --> Receiver1
Router::pOut2 --> Receiver2
@enduml
""")

    # --- Multiple packages with cross-package links ---
    save("comp_cross_package", """
@startuml
package P1 {
  component A
  component B
}
package P2 {
  component C
  component D
}
package P3 {
  component E
}
A --> C
B --> D
C --> E
D --> E
@enduml
""")

    # --- Interfaces ---
    for i, style in enumerate(["() IFoo", "interface IFoo", "[IFoo]"]):
        save(f"comp_interface_style_{i}", f"""
@startuml
component MyComp
{style}
MyComp -- IFoo
@enduml
""")

    save("comp_many_interfaces", """
@startuml
component Hub
interface IA
interface IB
interface IC
interface ID
interface IE
Hub - IA
Hub - IB
Hub - IC
Hub - ID
Hub - IE
@enduml
""")

    # --- Package chain variants ---
    for n in range(2, 8):
        components = "\n".join([f"  component Svc{i}" for i in range(1, n+1)])
        arrows = "\n".join([f"Svc{i} --> Svc{i+1}" for i in range(1, n)])
        save(f"comp_package_chain_{n}", f"""
@startuml
package Group1 {{
{components}
}}
{arrows}
@enduml
""")

    # --- Real-world architectures ---
    save("comp_web_architecture", """
@startuml
package "Frontend" {
  component Browser
  component "React App" as ReactApp
}
package "Backend" {
  component "API Gateway" as APIGW
  component "Auth Service" as Auth
  component "User Service" as UserSvc
  component "Product Service" as ProdSvc
}
package "Data Layer" {
  database "User DB" as UserDB
  database "Product DB" as ProdDB
  component "Cache" as Redis
}
Browser --> ReactApp : renders
ReactApp --> APIGW : HTTP
APIGW --> Auth : validates
APIGW --> UserSvc : routes
APIGW --> ProdSvc : routes
UserSvc --> UserDB : CRUD
ProdSvc --> ProdDB : CRUD
UserSvc --> Redis : cache
ProdSvc --> Redis : cache
@enduml
""")

    save("comp_microservices", """
@startuml
cloud "Internet" {
  component Client
}
rectangle "DMZ" {
  component "Load Balancer" as LB
  component "API Gateway" as GW
}
node "Service Mesh" {
  component "Order Service" as OS
  component "Payment Service" as PS
  component "Shipping Service" as SS
  component "Notification Service" as NS
}
database "Event Bus" as EB
Client --> LB
LB --> GW
GW --> OS
GW --> PS
OS --> EB : publishes
EB --> SS : subscribes
EB --> NS : subscribes
PS --> EB : publishes
@enduml
""")

    save("comp_plugin_system", """
@startuml
package "Core" {
  component Engine
  interface IPlugin
  interface IRenderer
}
package "Plugins" {
  component PluginA <<plugin>>
  component PluginB <<plugin>>
  component PluginC <<plugin>>
}
package "Renderers" {
  component SVGRenderer
  component PNGRenderer
}
Engine -( IPlugin
PluginA --> IPlugin : implements
PluginB --> IPlugin : implements
PluginC --> IPlugin : implements
Engine -( IRenderer
SVGRenderer --> IRenderer : implements
PNGRenderer --> IRenderer : implements
@enduml
""")

    save("comp_layered_architecture", """
@startuml
package "Presentation Layer" {
  component UI
  component ViewModel
}
package "Business Layer" {
  component ServiceA
  component ServiceB
  component DomainModel
}
package "Data Layer" {
  component Repository
  database Database
}
UI --> ViewModel
ViewModel --> ServiceA
ViewModel --> ServiceB
ServiceA --> DomainModel
ServiceB --> DomainModel
DomainModel --> Repository
Repository --> Database
@enduml
""")

    save("comp_event_driven", """
@startuml
component EventProducer
queue EventBus
component ConsumerA
component ConsumerB
component ConsumerC
EventProducer --> EventBus : publish
EventBus --> ConsumerA : subscribe
EventBus --> ConsumerB : subscribe
EventBus --> ConsumerC : subscribe
@enduml
""")

    save("comp_payment_system", """
@startuml
package "Payment Gateway" {
  component PaymentAPI
  component FraudDetection
  component PaymentProcessor
}
package "Bank Integration" {
  component BankConnector
  interface IBankAPI
}
package "Notification" {
  component EmailService
  component SMSService
}
database TransactionDB
PaymentAPI --> FraudDetection : validates
FraudDetection --> PaymentProcessor : approves
PaymentProcessor --> BankConnector : transfers
BankConnector - IBankAPI
PaymentProcessor --> TransactionDB : records
PaymentProcessor --> EmailService : notifies
PaymentProcessor --> SMSService : alerts
@enduml
""")

    save("comp_cicd_pipeline", """
@startuml
component "Source Control" as SC
component "CI Server" as CI
component "Test Runner" as TR
component "Build Server" as BS
component "Artifact Store" as AS
component "Deploy Agent" as DA
cloud "Production" as PROD
SC --> CI : webhook
CI --> TR : triggers
TR --> BS : on success
BS --> AS : publishes
AS --> DA : deploys
DA --> PROD : updates
@enduml
""")

    save("comp_cloud_internet", """
@startuml
cloud Internet {
  component DNS
  component CDN
}
rectangle Datacenter {
  component WebServer
  component AppServer
  database MainDB
}
Internet --> WebServer : HTTP
DNS --> CDN
CDN --> WebServer
WebServer --> AppServer
AppServer --> MainDB
@enduml
""")

    save("comp_frame_layout", """
@startuml
frame "Request Processing" {
  component Validator
  component Processor
  component Formatter
  Validator --> Processor
  Processor --> Formatter
}
frame "Data Access" {
  component DAO
  database DB
  DAO --> DB
}
Formatter --> DAO
@enduml
""")

    save("comp_database_container", """
@startuml
database "Main Store" {
  component "Read Replica" as RR
  component "Write Primary" as WP
  component "Backup" as BK
  WP --> RR : replicates
  WP --> BK : backups
}
component Application
Application --> WP : writes
Application --> RR : reads
@enduml
""")

    save("comp_folder_structure", """
@startuml
folder "src" {
  folder "controllers" {
    component UserController
    component ProductController
  }
  folder "services" {
    component UserService
    component ProductService
  }
  folder "models" {
    component User
    component Product
  }
}
UserController --> UserService
ProductController --> ProductService
UserService --> User
ProductService --> Product
@enduml
""")

    save("comp_nested_interfaces", """
@startuml
package Outer {
  interface IOuter
  package Inner {
    interface IInner
    component CoreComp
    CoreComp - IInner
  }
  component OuterComp
  OuterComp - IOuter
  Inner --> OuterComp
}
@enduml
""")

    # Arrow dual label
    save("comp_arrow_dual_label", """
@startuml
component Foo
component Bar
Foo "1" --> "many" Bar : contains
@enduml
""")

    # Arrow with URL
    save("comp_arrow_label_url", """
@startuml
component Frontend
component Backend
Frontend --> Backend : [[http://example.com REST API]]
@enduml
""")

    save("comp_visibility_mix", """
@startuml
component PublicAPI <<public>>
component InternalService <<internal>>
component PrivateImpl <<private>>
PublicAPI --> InternalService
InternalService --> PrivateImpl
@enduml
""")

    save("comp_dashed_border", """
@startuml
skinparam component {
  BorderStyle dashed
}
component A
component B
A --> B
@enduml
""")

    # --- Many combinatorial: n components in containers with arrows ---
    for container in ["package", "node", "cloud", "rectangle", "frame"]:
        for n in [2, 3, 4, 5]:
            comps = "\n".join([f"  component X{i}" for i in range(1, n+1)])
            arrows = "\n".join([f"X{i} --> X{i+1}" for i in range(1, n)])
            save(f"comp_{container}_n{n}_chain", f"""
@startuml
{container} Container {{
{comps}
}}
{arrows}
@enduml
""")

    # --- 2 containers with cross links ---
    for c1 in ["package", "node", "cloud"]:
        for c2 in ["package", "node", "frame", "folder"]:
            save(f"comp_twocont_{c1}_{c2}_cross", f"""
@startuml
{c1} Group1 {{
  component A
  component B
}}
{c2} Group2 {{
  component C
  component D
}}
A --> C
B --> D
A --> D
@enduml
""")

    # --- 3 containers with arrows ---
    for combo in itertools.islice(
        itertools.product(["package", "node", "cloud", "frame"],
                          ["package", "node", "folder"],
                          ["rectangle", "package", "node"]), 30
    ):
        c1, c2, c3 = combo
        save(f"comp_3cont_{c1}_{c2}_{c3}", f"""
@startuml
{c1} G1 {{
  component AA
}}
{c2} G2 {{
  component BB
}}
{c3} G3 {{
  component CC
}}
AA --> BB
BB --> CC
@enduml
""")

    # --- Arrow type x container combos ---
    for arrow, aname in [("-->", "dep"), ("..>", "use"), ("--", "assoc"), ("->", "nav")]:
        for container in ["package", "node", "cloud"]:
            save(f"comp_{container}_arrow_{aname}", f"""
@startuml
{container} Group {{
  component A
  component B
  A {arrow} B
}}
@enduml
""")

    # --- Color x arrow type ---
    for color in ["#LightBlue", "#Pink", "#LightGreen", "#Yellow"]:
        for arrow, aname in [("-->", "dep"), ("..>", "use"), ("->", "nav")]:
            safe_c = color.replace("#", "")
            save(f"comp_color_{safe_c}_arrow_{aname}", f"""
@startuml
component Foo {color}
component Bar {color}
Foo {arrow} Bar
@enduml
""")

    # --- Stereotype x arrow x color ---
    for stereo, sname in [("<<service>>", "svc"), ("<<repository>>", "repo"), ("<<gateway>>", "gw")]:
        for arrow, aname in [("-->", "dep"), ("..>", "use")]:
            for color in ["#LightBlue", "#Pink"]:
                safe_c = color.replace("#", "")
                save(f"comp_{sname}_{aname}_{safe_c}", f"""
@startuml
component Foo {stereo} {color}
component Bar {stereo} {color}
Foo {arrow} Bar
@enduml
""")

    # --- Note + color + stereotype ---
    for pos in ["top", "bottom", "left", "right"]:
        for stereo in ["<<service>>", "<<gateway>>"]:
            safe_s = stereo.replace("<<", "").replace(">>", "")
            save(f"comp_note_{pos}_stereo_{safe_s}", f"""
@startuml
component MyComp {stereo} #LightBlue
note {pos} of MyComp : Note on {safe_s}
@enduml
""")

    # --- Dense diagrams with increasing size ---
    for n in [5, 8, 10, 15, 20, 25, 30]:
        comps = "\n".join([f"component C{i:03d}" for i in range(1, n+1)])
        arrows = "\n".join([f"C{i:03d} --> C{(i % n) + 1:03d}" for i in range(1, n+1)])
        save(f"comp_dense_ring_{n}", f"""
@startuml
{comps}
{arrows}
@enduml
""")

    # --- Skinparam: many font/background variants ---
    for bg in ["LightBlue", "LightGreen", "White", "LightYellow"]:
        for border in ["DarkBlue", "DarkGreen", "Black", "Gray"]:
            save(f"comp_skin_bg_{bg}_border_{border}", f"""
@startuml
skinparam component {{
  BackgroundColor {bg}
  BorderColor {border}
}}
component Foo
component Bar
Foo --> Bar
@enduml
""")

    # --- Direction combos with containers ---
    for d in ["up", "down", "left", "right"]:
        for container in ["package", "node", "cloud"]:
            save(f"comp_dir_{d}_{container}", f"""
@startuml
{container} GroupA {{
  component A
}}
{container} GroupB {{
  component B
}}
A -{d}-> B
@enduml
""")

    # --- Together + colors ---
    for n in [2, 3, 4]:
        comps = "\n".join([f"  component G{i}" for i in range(1, n+1)])
        arrows = "\n".join([f"G{i} --> G{i+1}" for i in range(1, n)])
        save(f"comp_together_{n}", f"""
@startuml
together {{
{comps}
}}
component Z
G1 --> Z
@enduml
""")

    # --- Mixed container + together ---
    save("comp_mixed_together_container", """
@startuml
package PkgA {
  together {
    component A1
    component A2
  }
  component A3
}
package PkgB {
  component B1
  component B2
}
A1 --> B1
A2 --> B2
A3 --> B1
@enduml
""")

    # --- Parameterized: all element types as sources/targets ---
    for elem in ["component", "interface", "database", "queue", "storage"]:
        save(f"comp_elem_{elem}_as_node", f"""
@startuml
{elem} Source
component Target
Source --> Target
@enduml
""")

    # --- Complex notes ---
    save("comp_note_on_link_complex", """
@startuml
component A
component B
A --> B : main path
note on link
  This is the critical
  path for performance
end note
@enduml
""")

    save("comp_multiple_notes", """
@startuml
component A
component B
component C
note right of A : Note A
note top of B : Note B
note left of C : Note C
A --> B
B --> C
@enduml
""")

    # --- Aliased components in containers ---
    save("comp_aliased_in_container", """
@startuml
package "My Package" {
  component "Long Component Name A" as LCNA
  component "Long Component Name B" as LCNB
  component "Long Component Name C" as LCNC
  LCNA --> LCNB
  LCNB --> LCNC
}
@enduml
""")

    # --- Mixed component/interface/database in one diagram ---
    save("comp_mixed_element_types", """
@startuml
component WebServer
component AppServer
database MainDB
database CacheDB
interface IAuth
interface IData
WebServer --> AppServer : HTTP
AppServer --> MainDB : SQL
AppServer --> CacheDB : Redis
AppServer - IAuth
AppServer - IData
@enduml
""")

    # --- Component with ports and interfaces together ---
    save("comp_ports_and_interfaces", """
@startuml
component Server {
  portin HTTP_IN
  portout HTTP_OUT
  interface IAdminAPI
}
component Client
component AdminTool
Client --> Server::HTTP_IN
Server::HTTP_OUT --> Client
AdminTool --> Server::IAdminAPI
@enduml
""")

    # --- Lollipop variations ---
    for i, notation in enumerate(["-(", "-(0-", "-(0)-", "-(0"]):
        save(f"comp_lollipop_variant_{i}", f"""
@startuml
component A
component B
A {notation} B
@enduml
""")

    # --- Large realistic diagrams ---
    save("comp_enterprise_arch", """
@startuml
package "Client Layer" {
  component "Web Browser" as WB
  component "Mobile App" as MA
  component "Desktop App" as DA
}
package "API Layer" {
  component "API Gateway" as GW
  component "Auth Service" as AS
  component "Rate Limiter" as RL
}
package "Business Logic" {
  component "User Service" as US
  component "Order Service" as OS
  component "Payment Service" as PS
  component "Product Service" as PRS
  component "Notification Service" as NS
}
package "Data Layer" {
  database "User DB" as UDB
  database "Order DB" as ODB
  database "Product DB" as PRDB
  queue "Message Queue" as MQ
  component "Cache" as CA
}
WB --> GW
MA --> GW
DA --> GW
GW --> AS
GW --> RL
RL --> US
RL --> OS
RL --> PS
RL --> PRS
US --> UDB
OS --> ODB
PS --> MQ
PRS --> PRDB
MQ --> NS
US --> CA
PRS --> CA
@enduml
""")

    save("comp_compiler_pipeline", """
@startuml
package "Frontend" {
  component Lexer
  component Parser
  component "AST Builder" as ASTB
}
package "Middle End" {
  component "Semantic Analyzer" as SA
  component "Type Checker" as TC
  component "IR Generator" as IRG
  component "Optimizer" as OPT
}
package "Backend" {
  component "Code Generator" as CG
  component "Register Allocator" as RA
  component "Assembler" as ASM
  component "Linker" as LNK
}
Lexer --> Parser
Parser --> ASTB
ASTB --> SA
SA --> TC
TC --> IRG
IRG --> OPT
OPT --> CG
CG --> RA
RA --> ASM
ASM --> LNK
@enduml
""")

    save("comp_game_engine", """
@startuml
package "Core Engine" {
  component "Game Loop" as GL
  component "Event System" as ES
  component "Resource Manager" as RM
}
package "Rendering" {
  component "Renderer" as R
  component "Shader Manager" as SM
  component "Texture Manager" as TM
}
package "Physics" {
  component "Physics Engine" as PE
  component "Collision Detection" as CD
}
package "Audio" {
  component "Audio Engine" as AE
  component "Sound Manager" as SOM
}
package "Input" {
  component "Input Handler" as IH
  component "Controller Support" as CS
}
GL --> ES
GL --> R
GL --> PE
ES --> IH
IH --> CS
R --> SM
R --> TM
PE --> CD
GL --> AE
AE --> SOM
GL --> RM
RM --> TM
RM --> SOM
@enduml
""")

    # --- Component with unicode in stereotype ---
    save("comp_unicode_stereotypes", """
@startuml
component "Service α" <<αservice>>
component "Service β" <<βservice>>
"Service α" --> "Service β"
@enduml
""")

    # --- Skinparam with stereo colors ---
    for stereo, sname in [("service", "svc"), ("db", "db"), ("cache", "cache")]:
        save(f"comp_skinparam_stereo_{sname}", f"""
@startuml
skinparam component {{
  BackgroundColor<<{stereo}>> LightGreen
  BorderColor<<{stereo}>> DarkGreen
}}
component MyComp <<{stereo}>>
component OtherComp <<{stereo}>>
MyComp --> OtherComp
@enduml
""")

    # More realistic systems
    save("comp_iot_system", """
@startuml
node "IoT Device" {
  component Sensor
  component Firmware
}
cloud "MQTT Broker" {
  component Broker
}
package "Backend" {
  component "Data Ingestion" as DI
  component "Stream Processor" as SP
  database "Time Series DB" as TSDB
}
package "Frontend" {
  component Dashboard
  component AlertManager
}
Firmware --> Broker : MQTT
Broker --> DI : stream
DI --> SP
SP --> TSDB
Dashboard --> TSDB : query
AlertManager --> TSDB : monitor
@enduml
""")

    save("comp_search_engine", """
@startuml
package "Indexing Pipeline" {
  component Crawler
  component "HTML Parser" as HP
  component Tokenizer
  component "Index Writer" as IW
}
package "Query Processing" {
  component "Query Parser" as QP
  component "Query Planner" as QPL
  component Ranker
  component "Result Formatter" as RF
}
database "Inverted Index" as II
database "Document Store" as DS
Crawler --> HP
HP --> Tokenizer
Tokenizer --> IW
IW --> II
IW --> DS
QP --> QPL
QPL --> II
QPL --> Ranker
Ranker --> DS
Ranker --> RF
@enduml
""")

    print(f"  Component: {idx[0]} files")


# ---------------------------------------------------------------------------
# DEPLOYMENT DIAGRAMS
# ---------------------------------------------------------------------------

def gen_deployment():
    idx = [0]

    def save(name, content):
        write(os.path.join(DEPLOY_DIR, f"{name}.puml"), content)
        idx[0] += 1

    # --- Basic element types ---
    for elem in ["node", "artifact", "component", "cloud", "database", "storage",
                 "file", "folder", "frame", "package", "queue", "stack",
                 "rectangle", "card", "agent", "boundary", "control",
                 "entity", "collections"]:
        save(f"deploy_element_{elem}", f"""
@startuml
{elem} MyElement
@enduml
""")

    # --- Element pairs ---
    element_types = ["node", "artifact", "cloud", "database", "storage",
                     "file", "folder", "queue", "stack", "rectangle", "card",
                     "agent", "boundary", "control", "entity", "collections"]
    for e1 in element_types:
        for e2 in ["node", "database", "cloud", "queue"]:
            if e1 != e2:
                save(f"deploy_pair_{e1}_{e2}", f"""
@startuml
{e1} Source
{e2} Target
Source --> Target
@enduml
""")

    # --- Basic node relationships ---
    save("deploy_node_basic", """
@startuml
node AppServer
node DatabaseServer
AppServer --> DatabaseServer : JDBC
@enduml
""")

    save("deploy_artifact_in_node", """
@startuml
node AppServer {
  artifact app.war
  artifact config.xml
}
@enduml
""")

    save("deploy_component_in_node", """
@startuml
node Server {
  component WebApp
  component Cache
  WebApp --> Cache
}
@enduml
""")

    # --- Nesting nodes within nodes ---
    save("deploy_nested_nodes_2", """
@startuml
node DataCenter {
  node ServerRack {
    node Server1
    node Server2
  }
}
@enduml
""")

    save("deploy_nested_nodes_3", """
@startuml
node Region {
  node AvailabilityZone {
    node Cluster {
      node Instance
    }
  }
}
@enduml
""")

    save("deploy_nested_nodes_4", """
@startuml
node PhysicalDC {
  node VirtualDC {
    node K8sCluster {
      node Pod {
        component Container
      }
    }
  }
}
@enduml
""")

    # --- Nesting: outer x inner combos ---
    for outer in ["node", "cloud", "rectangle", "frame", "folder"]:
        for inner in ["node", "artifact", "component", "database", "queue"]:
            save(f"deploy_nest_{outer}_{inner}", f"""
@startuml
{outer} Outer {{
  {inner} Inner
}}
@enduml
""")

    # --- All relationship types ---
    for arrow, aname in [
        ("-->", "dep"), ("..>", "use"), ("--", "assoc"), ("..", "real"),
        ("->", "nav"), ("--->", "long_dep"), ("....>", "long_use"),
        ("<->", "bidi"), ("<-->", "bidi_long"),
    ]:
        save(f"deploy_rel_{aname}", f"""
@startuml
node NodeA
node NodeB
NodeA {arrow} NodeB
@enduml
""")

    # Arrow direction variants
    for arrow, aname in [("-->", "dep"), ("..>", "use"), ("->", "nav")]:
        for d in ["up", "down", "left", "right"]:
            save(f"deploy_arrow_{aname}_dir_{d}", f"""
@startuml
node NodeA
node NodeB
NodeA -{d}{arrow[1:]} NodeB
@enduml
""")

    # --- Relationship labels ---
    for protocol in ["HTTP", "HTTPS", "TCP/IP", "JDBC", "AMQP", "gRPC",
                     "WebSocket", "REST", "SOAP", "UDP", "SSH", "FTP",
                     "SMTP", "LDAP", "DNS"]:
        safe = protocol.replace("/", "_").replace(" ", "_")
        save(f"deploy_rel_label_{safe}", f"""
@startuml
node Client
node Server
Client --> Server : {protocol}
@enduml
""")

    # --- Colored elements ---
    colors = ["#Pink", "#LightBlue", "#Yellow", "#LightGreen", "#Orange",
              "#Violet", "#Cyan", "#AAFFAA", "#FF8888", "#FFAAFF",
              "#AAAAFF", "#FFD700", "#90EE90"]
    for i, color in enumerate(colors):
        safe = color.replace("#", "hex")
        save(f"deploy_color_{i}_{safe}", f"""
@startuml
node AppNode {color}
node DBNode {color}
AppNode --> DBNode
@enduml
""")

    # Color on multiple element types
    for elem in ["node", "database", "cloud", "queue", "storage", "artifact"]:
        for color in ["#LightBlue", "#LightGreen", "#Pink"]:
            safe_c = color.replace("#", "")
            save(f"deploy_elem_{elem}_color_{safe_c}", f"""
@startuml
{elem} MyElem {color}
@enduml
""")

    save("deploy_color_mixed_types", """
@startuml
node AppServer #LightBlue
database MainDB #LightGreen
cloud Internet #LightGray
artifact deploy.zip #Yellow
Internet --> AppServer : HTTP
AppServer --> MainDB : JDBC
artifact deploy.zip --> AppServer : deployed to
@enduml
""")

    # --- Stereotypes ---
    for stereo in ["<<server>>", "<<vm>>", "<<container>>", "<<cloud>>",
                   "<<physical>>", "<<virtual>>", "<<embedded>>", "<<cluster>>"]:
        safe = stereo.replace("<<", "").replace(">>", "")
        save(f"deploy_stereo_{safe}", f"""
@startuml
node MyNode {stereo}
artifact app.jar
app.jar --> MyNode : deployed on
@enduml
""")

    # Stereotype + color combos
    for stereo in ["<<vm>>", "<<container>>", "<<physical>>"]:
        for color in ["#LightBlue", "#LightGreen", "#Pink"]:
            safe_s = stereo.replace("<<", "").replace(">>", "")
            safe_c = color.replace("#", "")
            save(f"deploy_stereo_{safe_s}_color_{safe_c}", f"""
@startuml
node MyServer {color} {stereo}
note right of MyServer : Tagged server
@enduml
""")

    # --- Notes ---
    save("deploy_note_on_node", """
@startuml
node AppServer
note right of AppServer : 16 cores, 64GB RAM
@enduml
""")

    save("deploy_note_on_artifact", """
@startuml
artifact "app.war"
note bottom of "app.war" : Version 1.2.3
@enduml
""")

    save("deploy_note_floating", """
@startuml
node Server
note "Primary server" as N1
N1 .. Server
@enduml
""")

    for pos in ["top", "bottom", "left", "right"]:
        save(f"deploy_note_{pos}", f"""
@startuml
node CentralNode
note {pos} of CentralNode : {pos.capitalize()} note
@enduml
""")

    save("deploy_note_multiline", """
@startuml
node ProductionServer
note right of ProductionServer
  OS: Ubuntu 22.04
  CPU: 32 cores
  RAM: 128 GB
  Storage: 2 TB SSD
end note
@enduml
""")

    save("deploy_note_all_sides", """
@startuml
node CentralNode
note top of CentralNode : top
note bottom of CentralNode : bottom
note left of CentralNode : left
note right of CentralNode : right
@enduml
""")

    # Notes on each element type
    for elem in ["node", "database", "cloud", "queue", "artifact", "storage"]:
        save(f"deploy_note_elem_{elem}", f"""
@startuml
{elem} MyElem
note right of MyElem : Note on {elem}
@enduml
""")

    # --- Deep nesting 3 levels ---
    for combo in itertools.islice(
        itertools.product(["node", "cloud", "rectangle"], ["node", "folder"], ["node", "frame"]),
        15
    ):
        o, m, i = combo
        save(f"deploy_deep3_{o}_{m}_{i}", f"""
@startuml
{o} Level1 {{
  {m} Level2 {{
    {i} Level3 {{
      artifact "app.jar"
      database "LocalDB"
    }}
    node Worker
    Level3 --> Worker
  }}
  node Manager
  Level2 --> Manager
}}
@enduml
""")

    # --- Deep nesting 4 levels ---
    save("deploy_deep4_aws", """
@startuml
cloud "AWS" {
  rectangle "us-east-1" {
    rectangle "vpc-123" {
      node "ec2-instance" {
        component "nginx"
        component "app"
        nginx --> app
      }
    }
  }
}
@enduml
""")

    save("deploy_deep4_k8s", """
@startuml
node "Kubernetes Cluster" {
  node "Worker Node" {
    node "Pod" {
      component "Container A"
      component "Container B"
      "Container A" --> "Container B"
    }
  }
}
@enduml
""")

    # --- Multiple isolated clusters ---
    for n in [2, 3, 4, 5]:
        clusters = "\n".join([
            f"node Cluster{i} {{\n  node Node{i}a\n  node Node{i}b\n  Node{i}a --> Node{i}b\n}}"
            for i in range(1, n+1)
        ])
        save(f"deploy_isolated_clusters_{n}", f"""
@startuml
{clusters}
@enduml
""")

    # --- Long names and aliases ---
    save("deploy_long_names", """
@startuml
node "Very Long Application Server Name" as VLASN
node "Another Extremely Long Database Server" as AELDS
artifact "super-long-artifact-name-v1.2.3-release.tar.gz" as SLA
VLASN --> AELDS : JDBC
SLA --> VLASN : deployed
@enduml
""")

    save("deploy_aliases", """
@startuml
node n1 as "Primary Web Server"
node n2 as "Secondary Web Server"
node n3 as "Load Balancer"
n3 --> n1
n3 --> n2
@enduml
""")

    # --- Edge cases ---
    save("deploy_empty", """
@startuml
@enduml
""")

    save("deploy_single_node", """
@startuml
node Alone
@enduml
""")

    save("deploy_single_artifact", """
@startuml
artifact "standalone.jar"
@enduml
""")

    # Ring, chain, star topologies
    for n in [3, 4, 5, 6, 8, 10]:
        nodes = "\n".join([f"node N{i:02d}" for i in range(1, n+1)])
        arrows = "\n".join([f"N{i:02d} --> N{(i % n) + 1:02d}" for i in range(1, n+1)])
        save(f"deploy_ring_{n}", f"""
@startuml
{nodes}
{arrows}
@enduml
""")

    for n in [3, 5, 8, 10, 15, 20]:
        nodes = "\n".join([f"node N{i:02d}" for i in range(1, n+1)])
        arrows = "\n".join([f"N{i:02d} --> N{i+1:02d}" for i in range(1, n)])
        save(f"deploy_chain_{n}", f"""
@startuml
{nodes}
{arrows}
@enduml
""")

    for n in [3, 4, 5, 6, 8]:
        spokes = "\n".join([f"node Spoke{i}" for i in range(1, n+1)])
        arrows = "\n".join([f"Hub --> Spoke{i}" for i in range(1, n+1)])
        save(f"deploy_star_{n}", f"""
@startuml
node Hub
{spokes}
{arrows}
@enduml
""")

    save("deploy_large_20_nodes", """
@startuml
node N01
node N02
node N03
node N04
node N05
node N06
node N07
node N08
node N09
node N10
node N11
node N12
node N13
node N14
node N15
node N16
node N17
node N18
node N19
node N20
N01 --> N02
N02 --> N03
N03 --> N04
N04 --> N05
N05 --> N06
N06 --> N07
N07 --> N08
N08 --> N09
N09 --> N10
N10 --> N11
N11 --> N12
N12 --> N13
N13 --> N14
N14 --> N15
N15 --> N16
N16 --> N17
N17 --> N18
N18 --> N19
N19 --> N20
@enduml
""")

    save("deploy_unicode", """
@startuml
node "Сервер приложений" as SA
database "База данных" as BD
node "Веб-сервер" as WS
WS --> SA : HTTP
SA --> BD : SQL
@enduml
""")

    save("deploy_special_chars", """
@startuml
node "server-01.example.com"
node "db_primary"
artifact "app-v1.0.0.war"
"server-01.example.com" --> "db_primary" : port 5432
artifact "app-v1.0.0.war" --> "server-01.example.com" : deploy
@enduml
""")

    # --- Realistic deployments ---
    save("deploy_three_tier", """
@startuml
node "Web Tier" {
  node WebServer1 <<Apache>>
  node WebServer2 <<Apache>>
}
node "App Tier" {
  node AppServer1 <<Tomcat>>
  node AppServer2 <<Tomcat>>
}
node "Data Tier" {
  database PrimaryDB <<MySQL>>
  database ReplicaDB <<MySQL>>
}
WebServer1 --> AppServer1 : HTTP
WebServer2 --> AppServer2 : HTTP
AppServer1 --> PrimaryDB : JDBC
AppServer2 --> PrimaryDB : JDBC
PrimaryDB --> ReplicaDB : replication
@enduml
""")

    save("deploy_kubernetes", """
@startuml
cloud "Kubernetes Cluster" {
  node "Master Node" {
    component "API Server"
    component "Scheduler"
    component "Controller Manager"
    database "etcd"
    "API Server" --> "Scheduler"
    "API Server" --> "Controller Manager"
    "API Server" --> "etcd"
  }
  node "Worker Node 1" {
    component "kubelet"
    component "kube-proxy"
    component "Pod: Frontend"
    component "Pod: Backend"
  }
  node "Worker Node 2" {
    component "kubelet"
    component "kube-proxy"
    component "Pod: Backend"
    component "Pod: Database"
  }
}
@enduml
""")

    save("deploy_aws_vpc", """
@startuml
cloud "AWS" {
  rectangle "VPC" {
    rectangle "Public Subnet" {
      node "NAT Gateway"
      node "Bastion Host"
      node "Load Balancer"
    }
    rectangle "Private Subnet" {
      node "App Server 1"
      node "App Server 2"
    }
    rectangle "Data Subnet" {
      database "RDS Primary"
      database "RDS Replica"
      storage "S3 Bucket"
    }
  }
}
"Load Balancer" --> "App Server 1"
"Load Balancer" --> "App Server 2"
"App Server 1" --> "RDS Primary"
"App Server 2" --> "RDS Primary"
"RDS Primary" --> "RDS Replica"
@enduml
""")

    save("deploy_microservices_docker", """
@startuml
node "Docker Host" {
  component "nginx" <<container>>
  component "auth-service" <<container>>
  component "user-service" <<container>>
  component "product-service" <<container>>
  component "order-service" <<container>>
  database "postgres" <<container>>
  queue "rabbitmq" <<container>>
  component "redis" <<container>>
  nginx --> "auth-service"
  nginx --> "user-service"
  nginx --> "product-service"
  nginx --> "order-service"
  "user-service" --> "postgres"
  "product-service" --> "postgres"
  "order-service" --> "postgres"
  "order-service" --> "rabbitmq"
  "user-service" --> "redis"
}
@enduml
""")

    save("deploy_mobile_backend", """
@startuml
node "Mobile Device" {
  component "iOS App"
  component "Android App"
}
cloud "CDN" {
  component "Static Assets"
}
node "API Gateway" {
  component "Rate Limiter"
  component "Auth Middleware"
  component "Router"
}
node "Backend Services" {
  component "User API"
  component "Content API"
  component "Push Notification"
}
database "MongoDB"
database "Redis Cache"
"iOS App" --> "API Gateway" : HTTPS
"Android App" --> "API Gateway" : HTTPS
"API Gateway" --> "User API"
"API Gateway" --> "Content API"
"User API" --> "MongoDB"
"Content API" --> "MongoDB"
"User API" --> "Redis Cache"
"Push Notification" --> "iOS App"
"Push Notification" --> "Android App"
@enduml
""")

    save("deploy_ci_cd", """
@startuml
node "Developer Machine" {
  component "IDE"
  component "Local Git"
}
cloud "GitHub" {
  component "Repository"
  component "Actions Runner"
}
node "Build Server" {
  component "Maven Build"
  component "Unit Tests"
  component "SonarQube"
}
node "Artifact Repository" {
  storage "Nexus"
}
node "Staging Environment" {
  node "App Server"
  database "Test DB"
}
node "Production Environment" {
  node "Prod App Server"
  database "Prod DB"
}
"IDE" --> "Local Git" : commit
"Local Git" --> "Repository" : push
"Repository" --> "Actions Runner" : trigger
"Actions Runner" --> "Maven Build"
"Maven Build" --> "Unit Tests"
"Unit Tests" --> "SonarQube"
"SonarQube" --> "Nexus" : publish artifact
"Nexus" --> "App Server" : deploy
"App Server" --> "Prod App Server" : promote
@enduml
""")

    save("deploy_iot_architecture", """
@startuml
node "IoT Device" <<embedded>> {
  component "Sensor"
  component "Firmware"
  Sensor --> Firmware
}
cloud "AWS IoT Core" {
  component "MQTT Broker"
  component "Rules Engine"
  component "Device Shadow"
}
node "Processing" {
  component "Lambda Function"
  database "DynamoDB"
  queue "SQS Queue"
}
node "Analytics" {
  component "Kinesis"
  database "S3 Data Lake"
  component "Athena"
}
node "Dashboard" {
  component "Grafana"
  component "Alert Manager"
}
Firmware --> "MQTT Broker" : MQTT/TLS
"Rules Engine" --> "Lambda Function"
"Lambda Function" --> "DynamoDB"
"Lambda Function" --> "SQS Queue"
"SQS Queue" --> "Kinesis"
"Kinesis" --> "S3 Data Lake"
"Athena" --> "S3 Data Lake"
"Grafana" --> "Athena"
@enduml
""")

    save("deploy_blockchain_network", """
@startuml
node "Validator Node 1" {
  component "Consensus Engine"
  database "Ledger"
  component "Smart Contract VM"
}
node "Validator Node 2" {
  component "Consensus Engine"
  database "Ledger"
  component "Smart Contract VM"
}
node "Validator Node 3" {
  component "Consensus Engine"
  database "Ledger"
  component "Smart Contract VM"
}
node "Client Application" {
  component "Wallet"
  component "DApp"
}
"Validator Node 1" <--> "Validator Node 2" : P2P
"Validator Node 2" <--> "Validator Node 3" : P2P
"Validator Node 1" <--> "Validator Node 3" : P2P
"Client Application" --> "Validator Node 1" : RPC
@enduml
""")

    # --- Skinparam variations ---
    save("deploy_skinparam_basic", """
@startuml
skinparam node {
  BackgroundColor LightBlue
  BorderColor DarkBlue
}
skinparam database {
  BackgroundColor LightGreen
  BorderColor DarkGreen
}
node AppServer
database MainDB
AppServer --> MainDB
@enduml
""")

    save("deploy_skinparam_monochrome", """
@startuml
skinparam monochrome true
node Server
database DB
artifact app.war
Server --> DB
app.war --> Server
@enduml
""")

    save("deploy_skinparam_handwritten", """
@startuml
skinparam handwritten true
node Server
database DB
Server --> DB : queries
@enduml
""")

    # Skinparam bg/border combos
    for bg in ["LightBlue", "LightGreen", "White", "LightYellow"]:
        for border in ["DarkBlue", "DarkGreen", "Black"]:
            save(f"deploy_skin_bg_{bg}_border_{border}", f"""
@startuml
skinparam node {{
  BackgroundColor {bg}
  BorderColor {border}
}}
node AppServer
database MainDB
AppServer --> MainDB
@enduml
""")

    # --- Queue and messaging ---
    save("deploy_messaging", """
@startuml
node Producer
queue "Message Queue" as MQ
node Consumer1
node Consumer2
node Consumer3
Producer --> MQ : publish
MQ --> Consumer1 : subscribe
MQ --> Consumer2 : subscribe
MQ --> Consumer3 : subscribe
@enduml
""")

    save("deploy_pub_sub", """
@startuml
node EventSource
queue "Topic A" as TA
queue "Topic B" as TB
queue "Dead Letter Queue" as DLQ
node HandlerA1
node HandlerA2
node HandlerB1
EventSource --> TA
EventSource --> TB
TA --> HandlerA1
TA --> HandlerA2
TB --> HandlerB1
HandlerA1 --> DLQ : on error
@enduml
""")

    # --- Storage types ---
    save("deploy_storage_types", """
@startuml
node AppServer
storage "Block Storage" as BS
storage "Object Storage" as OS
database "Relational DB" as RDB
database "NoSQL DB" as NDB
AppServer --> BS : mount
AppServer --> OS : S3 API
AppServer --> RDB : JDBC
AppServer --> NDB : MongoDB driver
@enduml
""")

    # --- Cross-region deployment ---
    save("deploy_multi_region", """
@startuml
cloud "US-East" {
  node "Primary App Server"
  database "Primary DB"
  "Primary App Server" --> "Primary DB"
}
cloud "EU-West" {
  node "Secondary App Server"
  database "Replica DB"
  "Secondary App Server" --> "Replica DB"
}
cloud "AP-Southeast" {
  node "Edge App Server"
  database "Local Cache DB"
  "Edge App Server" --> "Local Cache DB"
}
"Primary DB" --> "Replica DB" : replication
"Primary DB" --> "Local Cache DB" : replication
@enduml
""")

    # --- Mixed UML elements ---
    save("deploy_mixed_elements", """
@startuml
agent ClientAgent
boundary "API Boundary"
control "Request Controller"
entity "User Entity"
collections "Log Collection"
ClientAgent --> "API Boundary" : HTTP
"API Boundary" --> "Request Controller"
"Request Controller" --> "User Entity"
"Request Controller" --> "Log Collection" : logs
@enduml
""")

    # --- Title and header ---
    save("deploy_with_title", """
@startuml
title Production Deployment Architecture
node AppServer
database MainDB
AppServer --> MainDB
@enduml
""")

    save("deploy_with_header_footer", """
@startuml
header Confidential - Internal Use Only
footer Generated by CI/CD Pipeline
node Server
database DB
Server --> DB
@enduml
""")

    save("deploy_with_legend", """
@startuml
node AppServer
database MainDB
cloud CDN
AppServer --> MainDB : JDBC
CDN --> AppServer : HTTP
legend right
  | Symbol | Meaning |
  | node | Physical/virtual server |
  | database | Data store |
  | cloud | External service |
endlegend
@enduml
""")

    # --- Node with all sub-elements ---
    save("deploy_node_full", """
@startuml
node "Production Server" {
  component "Web App" <<spring>>
  component "Cache Layer" <<redis>>
  database "Local DB" <<h2>>
  artifact "app.jar"
  queue "Task Queue"
  "Web App" --> "Cache Layer"
  "Web App" --> "Local DB"
  "Web App" --> "Task Queue"
}
@enduml
""")

    # Agent/boundary/control/entity combo
    save("deploy_uml_elements", """
@startuml
agent MobileApp
boundary APIGateway
control BusinessLogic
entity DataStore
collections EventLog
MobileApp --> APIGateway : HTTPS
APIGateway --> BusinessLogic : internal
BusinessLogic --> DataStore : persist
BusinessLogic --> EventLog : audit
@enduml
""")

    # Card elements
    save("deploy_cards", """
@startuml
card "Service Card A" {
  component ServiceA
}
card "Service Card B" {
  component ServiceB
}
card "Service Card C" {
  component ServiceC
}
ServiceA --> ServiceB
ServiceB --> ServiceC
@enduml
""")

    # Stack elements
    save("deploy_stacks", """
@startuml
stack "Tech Stack A" {
  component Frontend
  component Backend
  database DB
  Frontend --> Backend
  Backend --> DB
}
stack "Tech Stack B" {
  component APILayer
  component LogicLayer
  APILayer --> LogicLayer
}
@enduml
""")

    # Artifacts in node
    save("deploy_artifacts_in_node", """
@startuml
node AppServer {
  artifact "web.war"
  artifact "api.jar"
  artifact "config.properties"
  file "logback.xml"
  folder "logs"
}
@enduml
""")

    # Multiple nodes with artifacts
    for n in [2, 3, 4]:
        nodes = "\n".join([
            f"node Server{i} {{\n  artifact app{i}.jar\n}}"
            for i in range(1, n+1)
        ])
        arrows = "\n".join([f"Server{i} --> Server{i+1} : sync" for i in range(1, n)])
        save(f"deploy_nodes_with_artifacts_{n}", f"""
@startuml
{nodes}
{arrows}
@enduml
""")

    # --- 2-node combos with protocol labels ---
    for e1 in ["node", "cloud", "database"]:
        for e2 in ["node", "database", "queue"]:
            for protocol in ["HTTP", "JDBC", "AMQP"]:
                save(f"deploy_{e1}_{e2}_{protocol}", f"""
@startuml
{e1} Source
{e2} Target
Source --> Target : {protocol}
@enduml
""")

    # --- Various nesting patterns ---
    for outer in ["cloud", "node", "rectangle"]:
        for n in [2, 3, 4]:
            inner_nodes = "\n".join([f"  node Inner{i}" for i in range(1, n+1)])
            inner_arrows = "\n".join([f"Inner{i} --> Inner{i+1}" for i in range(1, n)])
            save(f"deploy_{outer}_with_{n}_inner_nodes", f"""
@startuml
{outer} Outer {{
{inner_nodes}
}}
{inner_arrows}
@enduml
""")

    # --- Artifact deployment patterns ---
    for ext in ["war", "jar", "ear", "zip", "tar.gz", "deb", "rpm"]:
        safe = ext.replace(".", "_")
        save(f"deploy_artifact_{safe}", f"""
@startuml
artifact "application.{ext}"
node "Application Server"
"application.{ext}" --> "Application Server" : deploy
@enduml
""")

    # --- Database replication patterns ---
    for n in [2, 3, 4]:
        primary = "database Primary"
        replicas = "\n".join([f"database Replica{i}" for i in range(1, n+1)])
        arrows = "\n".join([f"Primary --> Replica{i} : replication" for i in range(1, n+1)])
        save(f"deploy_db_replication_{n}", f"""
@startuml
{primary}
{replicas}
{arrows}
@enduml
""")

    # --- Load balancer patterns ---
    for n in [2, 3, 4, 5]:
        backends = "\n".join([f"node Backend{i}" for i in range(1, n+1)])
        arrows = "\n".join([f"LB --> Backend{i}" for i in range(1, n+1)])
        save(f"deploy_load_balancer_{n}", f"""
@startuml
node LB <<LoadBalancer>>
{backends}
{arrows}
@enduml
""")

    # --- Service mesh patterns ---
    for n in [3, 4, 5]:
        services = "\n".join([f"node Service{i}" for i in range(1, n+1)])
        arrows = "\n".join([
            f"Service{i} --> Service{j}"
            for i in range(1, n+1)
            for j in range(i+1, n+1)
            if j - i <= 2
        ])
        save(f"deploy_service_mesh_{n}", f"""
@startuml
{services}
{arrows}
@enduml
""")

    # --- More realistic systems ---
    save("deploy_saas_platform", """
@startuml
cloud "SaaS Platform" {
  rectangle "Tenant A" {
    node "App Instance A"
    database "DB A"
    "App Instance A" --> "DB A"
  }
  rectangle "Tenant B" {
    node "App Instance B"
    database "DB B"
    "App Instance B" --> "DB B"
  }
  rectangle "Shared Services" {
    node "Auth Service"
    node "Billing Service"
    queue "Event Bus"
  }
}
"App Instance A" --> "Auth Service"
"App Instance B" --> "Auth Service"
"App Instance A" --> "Event Bus"
"App Instance B" --> "Event Bus"
"Event Bus" --> "Billing Service"
@enduml
""")

    save("deploy_edge_computing", """
@startuml
node "Edge Device 1" <<embedded>> {
  component "Edge Runtime"
  database "Local Store"
}
node "Edge Device 2" <<embedded>> {
  component "Edge Runtime"
  database "Local Store"
}
cloud "Fog Layer" {
  node "Fog Gateway"
  database "Aggregated Store"
}
cloud "Cloud Core" {
  node "Cloud Backend"
  database "Central DB"
  component "Analytics Engine"
}
"Edge Device 1" --> "Fog Gateway" : MQTT
"Edge Device 2" --> "Fog Gateway" : MQTT
"Fog Gateway" --> "Aggregated Store"
"Fog Gateway" --> "Cloud Backend" : sync
"Cloud Backend" --> "Central DB"
"Analytics Engine" --> "Central DB"
@enduml
""")

    save("deploy_disaster_recovery", """
@startuml
rectangle "Primary Site" {
  node "Primary Web Server"
  node "Primary App Server"
  database "Primary DB"
  storage "Primary Storage"
  "Primary Web Server" --> "Primary App Server"
  "Primary App Server" --> "Primary DB"
  "Primary App Server" --> "Primary Storage"
}
rectangle "DR Site" {
  node "DR Web Server"
  node "DR App Server"
  database "DR DB"
  storage "DR Storage"
}
"Primary DB" --> "DR DB" : async replication
"Primary Storage" --> "DR Storage" : rsync
node "DNS Failover"
"DNS Failover" --> "Primary Web Server" : active
"DNS Failover" --> "DR Web Server" : standby
@enduml
""")

    save("deploy_cdn_architecture", """
@startuml
cloud "Origin" {
  node "Origin Server"
  storage "Asset Storage"
  "Origin Server" --> "Asset Storage"
}
cloud "CDN Network" {
  node "PoP US-East"
  node "PoP EU-West"
  node "PoP AP-South"
  node "PoP US-West"
}
"Origin Server" --> "PoP US-East" : push
"Origin Server" --> "PoP EU-West" : push
"Origin Server" --> "PoP AP-South" : push
"Origin Server" --> "PoP US-West" : push
node "End User US"
node "End User EU"
node "End User AP"
"End User US" --> "PoP US-East" : request
"End User EU" --> "PoP EU-West" : request
"End User AP" --> "PoP AP-South" : request
@enduml
""")

    # UML stereotyped node grid
    for stereo in ["server", "vm", "container", "device", "cluster"]:
        for n in [1, 2, 3]:
            nodes = "\n".join([f"node Node{i} <<{stereo}>>" for i in range(1, n+1)])
            arrows = "\n".join([f"Node{i} --> Node{i+1}" for i in range(1, n)])
            save(f"deploy_stereo_{stereo}_n{n}", f"""
@startuml
{nodes}
{arrows}
@enduml
""")

    # Relation label x color combos
    for color in ["#LightBlue", "#LightGreen", "#Pink"]:
        for label in ["HTTP", "JDBC", "TCP"]:
            safe_c = color.replace("#", "")
            save(f"deploy_color_{safe_c}_label_{label}", f"""
@startuml
node Source {color}
node Target {color}
Source --> Target : {label}
@enduml
""")

    print(f"  Deployment: {idx[0]} files")


# ---------------------------------------------------------------------------
# USE CASE DIAGRAMS
# ---------------------------------------------------------------------------

def gen_usecase():
    idx = [0]

    def save(name, content):
        write(os.path.join(USECASE_DIR, f"{name}.puml"), content)
        idx[0] += 1

    # --- Basic elements ---
    save("usecase_basic_actor_usecase", """
@startuml
actor User
usecase "Log In" as UC1
User --> UC1
@enduml
""")

    save("usecase_basic_multiple", """
@startuml
actor User
usecase "Log In" as UC1
usecase "View Dashboard" as UC2
usecase "Log Out" as UC3
User --> UC1
User --> UC2
User --> UC3
@enduml
""")

    # --- Actor types ---
    save("usecase_actor_stick", """
@startuml
actor "Regular User" as RU
actor "Admin User" as AU
usecase "Manage Users" as MU
usecase "View Reports" as VR
RU --> VR
AU --> MU
AU --> VR
@enduml
""")

    save("usecase_actor_rectangle", """
@startuml
actor User << Human >>
actor System << System >>
usecase "Process Data" as PD
User --> PD
System --> PD
@enduml
""")

    save("usecase_actor_no_stereotype", """
@startuml
:User:
:Admin:
(Login)
(Manage)
User --> (Login)
Admin --> (Manage)
@enduml
""")

    # --- System boundaries ---
    save("usecase_system_boundary", """
@startuml
actor User
rectangle "Banking System" {
  usecase "Check Balance" as CB
  usecase "Transfer Funds" as TF
  usecase "Pay Bill" as PB
}
User --> CB
User --> TF
User --> PB
@enduml
""")

    save("usecase_multiple_boundaries", """
@startuml
actor User
actor Admin
rectangle "Frontend System" {
  usecase "Browse Products" as BP
  usecase "Add to Cart" as AC
  usecase "Checkout" as CO
}
rectangle "Admin System" {
  usecase "Manage Products" as MP
  usecase "View Orders" as VO
  usecase "Generate Reports" as GR
}
User --> BP
User --> AC
User --> CO
Admin --> MP
Admin --> VO
Admin --> GR
@enduml
""")

    save("usecase_three_boundaries", """
@startuml
actor User
actor Staff
actor System
rectangle "Public Portal" {
  usecase "Browse Catalog" as BC
  usecase "Search" as S
}
rectangle "User Account" {
  usecase "Login" as L
  usecase "View Orders" as VO
}
rectangle "Back Office" {
  usecase "Manage Inventory" as MI
  usecase "Process Orders" as PO
}
User --> BC
User --> S
User --> L
User --> VO
Staff --> MI
Staff --> PO
System --> PO
@enduml
""")

    # --- Include relationship ---
    save("usecase_include_basic", """
@startuml
actor User
usecase "Transfer Funds" as TF
usecase "Authenticate" as AUTH
TF .> AUTH : <<include>>
User --> TF
@enduml
""")

    save("usecase_include_chain", """
@startuml
actor Customer
usecase "Place Order" as PO
usecase "Validate Cart" as VC
usecase "Check Inventory" as CI
usecase "Process Payment" as PP
PO .> VC : <<include>>
VC .> CI : <<include>>
PO .> PP : <<include>>
Customer --> PO
@enduml
""")

    save("usecase_include_multiple", """
@startuml
actor User
usecase "Submit Form" as SF
usecase "Validate Input" as VI
usecase "Log Activity" as LA
usecase "Send Notification" as SN
SF .> VI : <<include>>
SF .> LA : <<include>>
SF .> SN : <<include>>
User --> SF
@enduml
""")

    save("usecase_include_chain_long", """
@startuml
actor User
usecase UC1
usecase UC2
usecase UC3
usecase UC4
usecase UC5
UC1 .> UC2 : <<include>>
UC2 .> UC3 : <<include>>
UC3 .> UC4 : <<include>>
UC4 .> UC5 : <<include>>
User --> UC1
@enduml
""")

    # Include chain of various lengths
    for n in [2, 3, 4, 5, 6]:
        ucs = "\n".join([f"usecase UC{i}" for i in range(1, n+1)])
        includes = "\n".join([f"UC{i} .> UC{i+1} : <<include>>" for i in range(1, n)])
        save(f"usecase_include_chain_{n}", f"""
@startuml
actor User
{ucs}
{includes}
User --> UC1
@enduml
""")

    # --- Extend relationship ---
    save("usecase_extend_basic", """
@startuml
actor User
usecase "Login" as L
usecase "Login with 2FA" as L2FA
L2FA .> L : <<extend>>
User --> L
@enduml
""")

    save("usecase_extend_multiple", """
@startuml
actor User
usecase "Search Products" as SP
usecase "Filter by Category" as FC
usecase "Filter by Price" as FP
usecase "Sort Results" as SR
FC .> SP : <<extend>>
FP .> SP : <<extend>>
SR .> SP : <<extend>>
User --> SP
@enduml
""")

    # Extend fan-out of various sizes
    for n in [2, 3, 4, 5]:
        extensions = "\n".join([f"usecase Extension{i}" for i in range(1, n+1)])
        extends = "\n".join([f"Extension{i} .> BaseUC : <<extend>>" for i in range(1, n+1)])
        save(f"usecase_extend_fan_{n}", f"""
@startuml
actor User
usecase BaseUC as "Base Use Case"
{extensions}
{extends}
User --> BaseUC
@enduml
""")

    save("usecase_include_and_extend", """
@startuml
actor Customer
rectangle "E-Commerce" {
  usecase "Purchase Product" as PP
  usecase "Authenticate" as AUTH
  usecase "Apply Coupon" as AC
  usecase "Express Checkout" as EC
}
PP .> AUTH : <<include>>
AC .> PP : <<extend>>
EC .> PP : <<extend>>
Customer --> PP
@enduml
""")

    # --- Generalization ---
    save("usecase_actor_generalization", """
@startuml
actor Person
actor Employee
actor Manager
Employee --|> Person
Manager --|> Employee
usecase "Work" as W
usecase "Manage Team" as MT
usecase "Approve Budget" as AB
Employee --> W
Manager --> MT
Manager --> AB
@enduml
""")

    save("usecase_usecase_generalization", """
@startuml
actor User
usecase "Search" as S
usecase "Advanced Search" as AS
usecase "Simple Search" as SS
AS --|> S
SS --|> S
User --> AS
User --> SS
@enduml
""")

    save("usecase_deep_generalization", """
@startuml
actor Entity
actor LegalEntity
actor Person
actor Organization
actor Company
actor NonProfit
LegalEntity --|> Entity
Person --|> LegalEntity
Organization --|> LegalEntity
Company --|> Organization
NonProfit --|> Organization
usecase "File Taxes" as FT
usecase "Register" as REG
Person --> FT
Company --> FT
NonProfit --> REG
@enduml
""")

    # Generalization chains of various depths
    for n in [2, 3, 4, 5]:
        actors = "\n".join([f"actor Actor{i}" for i in range(1, n+1)])
        gen = "\n".join([f"Actor{i+1} --|> Actor{i}" for i in range(1, n)])
        save(f"usecase_actor_gen_chain_{n}", f"""
@startuml
{actors}
Actor{n} --> (Use Case)
usecase "Use Case"
{gen}
@enduml
""")

    # --- Direction hints ---
    save("usecase_left_to_right", """
@startuml
left to right direction
actor User
rectangle System {
  usecase UC1
  usecase UC2
  usecase UC3
}
User --> UC1
User --> UC2
User --> UC3
@enduml
""")

    save("usecase_top_to_bottom", """
@startuml
top to bottom direction
actor User
rectangle System {
  usecase UC1
  usecase UC2
}
User --> UC1
UC1 --> UC2
@enduml
""")

    # Direction x n-usecase combos
    for direction in ["left to right", "top to bottom"]:
        for n in [2, 3, 4, 5]:
            safe_d = direction.replace(" ", "_")
            ucs = "\n".join([f"  usecase UC{i:02d}" for i in range(1, n+1)])
            links = "\n".join([f"User --> UC{i:02d}" for i in range(1, n+1)])
            save(f"usecase_{safe_d}_{n}ucs", f"""
@startuml
{direction} direction
actor User
rectangle System {{
{ucs}
}}
{links}
@enduml
""")

    # --- Notes ---
    save("usecase_note_on_actor", """
@startuml
actor Customer
note right of Customer : External user
usecase "Buy Product" as BP
Customer --> BP
@enduml
""")

    save("usecase_note_on_usecase", """
@startuml
actor User
usecase "Generate Report" as GR
note right of GR : May take several minutes
User --> GR
@enduml
""")

    save("usecase_note_on_link", """
@startuml
actor Admin
usecase "Delete User" as DU
Admin --> DU
note on link : Requires confirmation
@enduml
""")

    save("usecase_note_floating", """
@startuml
actor User
usecase "Login" as L
note "Entry point" as N1
N1 .. L
User --> L
@enduml
""")

    save("usecase_note_multiline", """
@startuml
actor Customer
usecase "Checkout" as CO
note right of CO
  Steps:
  1. Review cart
  2. Enter payment
  3. Confirm order
end note
Customer --> CO
@enduml
""")

    for pos in ["top", "bottom", "left", "right"]:
        save(f"usecase_note_{pos}_actor", f"""
@startuml
actor User
note {pos} of User : {pos.capitalize()} note on actor
usecase UC
User --> UC
@enduml
""")

        save(f"usecase_note_{pos}_usecase", f"""
@startuml
actor User
usecase UC
note {pos} of UC : {pos.capitalize()} note on use case
User --> UC
@enduml
""")

    # --- Stereotypes ---
    for stereo in ["<<primary>>", "<<optional>>", "<<automated>>",
                   "<<manual>>", "<<external>>", "<<internal>>"]:
        safe = stereo.replace("<<", "").replace(">>", "")
        save(f"usecase_stereo_{safe}", f"""
@startuml
actor User
usecase "Action" {stereo}
User --> (Action)
@enduml
""")

    save("usecase_stereo_actor", """
@startuml
actor "External System" <<system>>
actor "Human User" <<human>>
usecase "Process Data"
"External System" --> (Process Data)
"Human User" --> (Process Data)
@enduml
""")

    # Stereo on actor and usecase combos
    for actor_stereo in ["<<human>>", "<<system>>", "<<external>>"]:
        for uc_stereo in ["<<primary>>", "<<optional>>", "<<automated>>"]:
            safe_a = actor_stereo.replace("<<", "").replace(">>", "")
            safe_u = uc_stereo.replace("<<", "").replace(">>", "")
            save(f"usecase_stereo_a_{safe_a}_uc_{safe_u}", f"""
@startuml
actor MyActor {actor_stereo}
usecase MyUC {uc_stereo}
MyActor --> MyUC
@enduml
""")

    # --- Colors ---
    colors = ["#Pink", "#LightBlue", "#Yellow", "#LightGreen", "#Orange",
              "#Violet", "#Cyan", "#AAFFAA"]
    for color in colors:
        safe = color.replace("#", "hex")
        save(f"usecase_color_actor_{safe}", f"""
@startuml
actor User {color}
usecase "Action"
User --> (Action)
@enduml
""")
        save(f"usecase_color_uc_{safe}", f"""
@startuml
actor User
usecase "Action" {color}
User --> (Action)
@enduml
""")

    save("usecase_color_boundary", """
@startuml
actor User
rectangle "System" #LightBlue {
  usecase "Action A" #Pink
  usecase "Action B" #Yellow
}
User --> (Action A)
User --> (Action B)
@enduml
""")

    # Color x boundary combos
    for color in ["#LightBlue", "#LightGreen", "#Pink", "#Yellow"]:
        safe_c = color.replace("#", "")
        save(f"usecase_boundary_color_{safe_c}", f"""
@startuml
actor User
rectangle "System" {color} {{
  usecase UC1
  usecase UC2
}}
User --> UC1
User --> UC2
@enduml
""")

    # --- Package grouping ---
    save("usecase_package_grouping", """
@startuml
actor User
actor Admin
package "User Operations" {
  usecase "Login"
  usecase "View Profile"
  usecase "Update Profile"
}
package "Admin Operations" {
  usecase "Manage Users"
  usecase "View Logs"
  usecase "Configure System"
}
User --> (Login)
User --> (View Profile)
User --> (Update Profile)
Admin --> (Manage Users)
Admin --> (View Logs)
Admin --> (Configure System)
@enduml
""")

    # Package grouping with n packages
    for n in [2, 3, 4]:
        packages = []
        for i in range(1, n+1):
            pkg = f"""package "Package{i}" {{
  usecase "Action{i}A"
  usecase "Action{i}B"
}}"""
            packages.append(pkg)
        links = "\n".join([
            f"Actor --> (Action{i}A)\nActor --> (Action{i}B)"
            for i in range(1, n+1)
        ])
        save(f"usecase_packages_{n}", f"""
@startuml
actor Actor
{chr(10).join(packages)}
{links}
@enduml
""")

    # --- Long use case names ---
    save("usecase_long_names", """
@startuml
actor "External Payment Provider" as EPP
usecase "Process Credit Card Transaction" as PCCT
usecase "Validate Payment Information and Fraud Check" as VPIFC
usecase "Generate Transaction Receipt and Send Confirmation Email" as GTRSCE
EPP --> PCCT
PCCT .> VPIFC : <<include>>
PCCT .> GTRSCE : <<include>>
@enduml
""")

    # --- Dense diagrams ---
    save("usecase_dense_small", """
@startuml
actor A1
actor A2
actor A3
usecase UC1
usecase UC2
usecase UC3
usecase UC4
usecase UC5
A1 --> UC1
A1 --> UC2
A2 --> UC2
A2 --> UC3
A3 --> UC3
A3 --> UC4
A1 --> UC5
A2 --> UC5
A3 --> UC5
UC1 .> UC5 : <<include>>
UC4 .> UC5 : <<include>>
@enduml
""")

    save("usecase_dense_large", """
@startuml
left to right direction
actor Customer
actor Employee
actor Manager
actor System
rectangle "CRM System" {
  usecase "Create Customer" as CC
  usecase "View Customer" as VC
  usecase "Update Customer" as UC
  usecase "Delete Customer" as DC
  usecase "Create Order" as CO
  usecase "View Orders" as VO
  usecase "Process Order" as PO
  usecase "Cancel Order" as CAO
  usecase "Generate Invoice" as GI
  usecase "Send Email" as SE
  usecase "Authenticate" as AUTH
  usecase "Authorize" as AUTHZ
}
Customer --> CC
Customer --> VC
Customer --> CO
Customer --> VO
Employee --> CC
Employee --> VC
Employee --> UC
Employee --> CO
Employee --> VO
Employee --> PO
Manager --> DC
Manager --> CAO
Manager --> GI
System --> SE
CC .> AUTH : <<include>>
CO .> AUTH : <<include>>
DC .> AUTHZ : <<include>>
GI .> SE : <<include>>
PO .> GI : <<include>>
@enduml
""")

    # Dense: n actors x m usecases
    for n_actors in [2, 3, 4, 5]:
        for n_ucs in [3, 4, 5, 6, 8, 10]:
            actors = "\n".join([f"actor A{i}" for i in range(1, n_actors+1)])
            ucs = "\n".join([f"  usecase \"UC{i:02d}\" as UC{i:02d}" for i in range(1, n_ucs+1)])
            links = "\n".join([f"A{(i % n_actors) + 1} --> UC{i:02d}" for i in range(1, n_ucs+1)])
            save(f"usecase_dense_a{n_actors}_uc{n_ucs}", f"""
@startuml
{actors}
rectangle System {{
{ucs}
}}
{links}
@enduml
""")

    # --- Use case with description ---
    save("usecase_with_description", """
@startuml
usecase UC1 as "
  Login
  --
  User enters credentials
  System validates
  Session is created
"
actor User
User --> UC1
@enduml
""")

    save("usecase_with_description2", """
@startuml
usecase (Place Order) as UC1
note right of UC1
  Pre: User is logged in
  Post: Order is created
  Main flow:
    1. Select items
    2. Enter address
    3. Pay
end note
actor Customer
Customer --> UC1
@enduml
""")

    # Description with separator
    for sep_style in ["--", "==", ".."]:
        safe = sep_style.replace("-", "dash").replace("=", "eq").replace(".", "dot")
        save(f"usecase_desc_sep_{safe}", f"""
@startuml
usecase UC1 as "
  Title
  {sep_style}
  Description text here
  Multiple lines allowed
"
actor User
User --> UC1
@enduml
""")

    # --- Abstract actors ---
    save("usecase_abstract_actor", """
@startuml
actor AbstractUser <<abstract>>
actor RegularUser
actor PremiumUser
RegularUser --|> AbstractUser
PremiumUser --|> AbstractUser
usecase "Basic Feature" as BF
usecase "Premium Feature" as PF
AbstractUser --> BF
PremiumUser --> PF
@enduml
""")

    # --- Edge cases ---
    save("usecase_empty", """
@startuml
@enduml
""")

    save("usecase_single_actor", """
@startuml
actor Alone
@enduml
""")

    save("usecase_single_usecase", """
@startuml
usecase "Just Me"
@enduml
""")

    save("usecase_actor_no_usecase", """
@startuml
actor A1
actor A2
actor A3
@enduml
""")

    save("usecase_no_actors", """
@startuml
usecase UC1
usecase UC2
usecase UC3
UC1 .> UC2 : <<include>>
UC2 .> UC3 : <<extend>>
@enduml
""")

    save("usecase_unicode", """
@startuml
actor "Utilisateur" as U
rectangle "Système" {
  usecase "Se connecter" as SC
  usecase "Voir le tableau de bord" as VTB
}
U --> SC
U --> VTB
@enduml
""")

    save("usecase_special_chars", """
@startuml
actor "User-123" as U
usecase "Process & Validate" as PV
usecase "Save/Export Data" as SED
U --> PV
U --> SED
@enduml
""")

    save("usecase_unicode_japanese", """
@startuml
actor "ユーザー" as U
rectangle "システム" {
  usecase "ログイン" as L
  usecase "データ閲覧" as D
}
U --> L
U --> D
@enduml
""")

    # --- Skinparam ---
    save("usecase_skinparam_basic", """
@startuml
skinparam usecase {
  BackgroundColor LightBlue
  BorderColor DarkBlue
  ArrowColor Navy
}
skinparam actor {
  BackgroundColor LightGreen
  BorderColor DarkGreen
}
actor User
usecase "Action"
User --> (Action)
@enduml
""")

    save("usecase_skinparam_stereotype", """
@startuml
skinparam usecase {
  BackgroundColor<<primary>> LightBlue
  BackgroundColor<<secondary>> LightGreen
}
actor User
usecase "Primary Action" <<primary>>
usecase "Secondary Action" <<secondary>>
User --> (Primary Action)
User --> (Secondary Action)
@enduml
""")

    for bg in ["LightBlue", "LightGreen", "White", "LightYellow", "Pink"]:
        for border in ["DarkBlue", "DarkGreen", "Black"]:
            save(f"usecase_skin_bg_{bg}_border_{border}", f"""
@startuml
skinparam usecase {{
  BackgroundColor {bg}
  BorderColor {border}
}}
actor User
usecase "Action"
User --> (Action)
@enduml
""")

    # --- Title/header/footer ---
    save("usecase_with_title", """
@startuml
title User Authentication Use Cases
actor User
rectangle System {
  usecase "Login" as L
  usecase "Logout" as LO
  usecase "Reset Password" as RP
}
User --> L
User --> LO
User --> RP
@enduml
""")

    save("usecase_with_header", """
@startuml
header My Application - Use Cases v1.0
actor User
usecase UC1
User --> UC1
@enduml
""")

    save("usecase_with_footer", """
@startuml
footer Confidential - Internal Use Only
actor User
usecase UC1
User --> UC1
@enduml
""")

    # --- Actor on both sides ---
    save("usecase_actor_both_sides", """
@startuml
left to right direction
actor "System A" as SA
rectangle "Middleware" {
  usecase "Transform Data" as TD
  usecase "Route Message" as RM
}
actor "System B" as SB
SA --> TD
TD .> RM : <<include>>
RM --> SB
@enduml
""")

    # --- Complex real-world scenarios ---
    save("usecase_online_banking", """
@startuml
left to right direction
actor Customer
actor Teller
actor "Fraud System" as FS

rectangle "Online Banking" {
  usecase "Login" as L
  usecase "View Balance" as VB
  usecase "Transfer Money" as TM
  usecase "Pay Bill" as PB
  usecase "Download Statement" as DS
  usecase "Change Password" as CP
  usecase "2FA Verification" as TFA
  usecase "Fraud Check" as FC
  usecase "Log Transaction" as LT
}

Customer --> L
Customer --> VB
Customer --> TM
Customer --> PB
Customer --> DS
Customer --> CP
Teller --> VB
Teller --> TM
FS --> FC

L .> TFA : <<include>>
TM .> FC : <<include>>
TM .> LT : <<include>>
PB .> FC : <<include>>
PB .> LT : <<include>>
TFA .> L : <<extend>>
@enduml
""")

    save("usecase_hospital_system", """
@startuml
top to bottom direction
actor Patient
actor Doctor
actor Nurse
actor "Lab Technician" as Lab
actor Administrator

rectangle "Hospital Management System" {
  usecase "Register Patient" as RP
  usecase "Book Appointment" as BA
  usecase "View Medical History" as VMH
  usecase "Prescribe Medication" as PM
  usecase "Order Lab Test" as OLT
  usecase "Enter Lab Results" as ELR
  usecase "Generate Bill" as GB
  usecase "Process Payment" as PP
  usecase "Discharge Patient" as DP
  usecase "Authenticate" as AUTH
}

Patient --> RP
Patient --> BA
Doctor --> VMH
Doctor --> PM
Doctor --> OLT
Nurse --> VMH
Lab --> ELR
Administrator --> GB
Administrator --> DP
PP .> GB : <<extend>>
VMH .> AUTH : <<include>>
PM .> AUTH : <<include>>
OLT .> ELR : <<include>>
@enduml
""")

    save("usecase_library_system", """
@startuml
actor Member
actor Librarian
actor "Library System" as LS

rectangle "Library Management" {
  usecase "Search Catalog" as SC
  usecase "Borrow Book" as BB
  usecase "Return Book" as RB
  usecase "Reserve Book" as RESB
  usecase "Pay Fine" as PF
  usecase "Renew Membership" as RM
  usecase "Add Book" as AB
  usecase "Remove Book" as RemB
  usecase "Send Reminder" as SR
  usecase "Calculate Fine" as CF
}

Member --> SC
Member --> BB
Member --> RB
Member --> RESB
Member --> PF
Member --> RM
Librarian --> AB
Librarian --> RemB
LS --> SR
BB .> SC : <<include>>
PF .> CF : <<include>>
SR .> CF : <<include>>
RESB .> SC : <<include>>
@enduml
""")

    save("usecase_atm_system", """
@startuml
actor Customer
actor "Bank Server" as BS
actor "Security System" as SS

rectangle "ATM System" {
  usecase "Insert Card" as IC
  usecase "Enter PIN" as EP
  usecase "Withdraw Cash" as WC
  usecase "Check Balance" as CB
  usecase "Transfer Funds" as TF
  usecase "Change PIN" as CP
  usecase "Validate Card" as VC
  usecase "Verify PIN" as VP
  usecase "Dispense Cash" as DC
  usecase "Print Receipt" as PR
}

Customer --> IC
Customer --> EP
Customer --> WC
Customer --> CB
Customer --> TF
Customer --> CP
BS --> VC
SS --> VP

IC .> VC : <<include>>
EP .> VP : <<include>>
WC .> DC : <<include>>
WC .> PR : <<include>>
CB .> PR : <<extend>>
TF .> PR : <<include>>
@enduml
""")

    save("usecase_social_media", """
@startuml
left to right direction
actor "Registered User" as RU
actor "Guest" as G
actor "Admin" as ADM

rectangle "Social Media Platform" {
  usecase "Browse Feed" as BF
  usecase "Create Post" as CP
  usecase "Like Post" as LP
  usecase "Comment" as CM
  usecase "Follow User" as FU
  usecase "Direct Message" as DM
  usecase "Search" as S
  usecase "Register" as REG
  usecase "Login" as L
  usecase "Moderate Content" as MC
  usecase "Ban User" as BU
  usecase "View Analytics" as VA
  usecase "Upload Media" as UM
  usecase "Validate Media" as VM
}

G --> BF
G --> S
G --> REG
RU --> BF
RU --> CP
RU --> LP
RU --> CM
RU --> FU
RU --> DM
RU --> S
ADM --> MC
ADM --> BU
ADM --> VA
CP .> UM : <<include>>
UM .> VM : <<include>>
L .> REG : <<extend>>
@enduml
""")

    save("usecase_elearning", """
@startuml
actor Student
actor Instructor
actor "Platform Admin" as PA

rectangle "E-Learning Platform" {
  usecase "Enroll Course" as EC
  usecase "Watch Lecture" as WL
  usecase "Submit Assignment" as SA
  usecase "Take Quiz" as TQ
  usecase "View Grade" as VG
  usecase "Create Course" as CC
  usecase "Upload Content" as UC
  usecase "Grade Submission" as GS
  usecase "Manage Platform" as MP
  usecase "Generate Certificate" as GCert
  usecase "Authenticate" as AUTH
}

Student --> EC
Student --> WL
Student --> SA
Student --> TQ
Student --> VG
Instructor --> CC
Instructor --> UC
Instructor --> GS
PA --> MP
EC .> AUTH : <<include>>
SA .> AUTH : <<include>>
GS .> VG : <<include>>
GCert .> VG : <<extend>>
@enduml
""")

    # --- Arrow variations ---
    for arrow in ["-->", "->", "--", "-", "..>", ".."]:
        safe = arrow.replace(">", "r").replace(".", "d").replace("-", "l")
        save(f"usecase_arrow_{safe}", f"""
@startuml
actor User
usecase "Action"
User {arrow} (Action)
@enduml
""")

    # --- Mixed include/extend/generalize ---
    save("usecase_mixed_relations", """
@startuml
actor BaseActor
actor SpecialActor
SpecialActor --|> BaseActor
usecase BaseUC
usecase SpecialUC
usecase IncludedUC
usecase OptionalUC
SpecialUC --|> BaseUC
BaseUC .> IncludedUC : <<include>>
OptionalUC .> BaseUC : <<extend>>
BaseActor --> BaseUC
SpecialActor --> SpecialUC
@enduml
""")

    save("usecase_extend_tree", """
@startuml
actor User
usecase "Base Action" as BA
usecase "Extension A" as EA
usecase "Extension B" as EB
usecase "Extension C" as EC
usecase "Extension D" as ED
EA .> BA : <<extend>>
EB .> BA : <<extend>>
EC .> BA : <<extend>>
ED .> BA : <<extend>>
User --> BA
@enduml
""")

    # --- More realistic systems ---
    save("usecase_hotel_reservation", """
@startuml
left to right direction
actor Guest
actor Receptionist
actor Manager
actor "Payment System" as PS

rectangle "Hotel Reservation System" {
  usecase "Search Rooms" as SR
  usecase "Make Reservation" as MR
  usecase "Cancel Reservation" as CR
  usecase "Check In" as CI
  usecase "Check Out" as CO
  usecase "Process Payment" as PP
  usecase "Generate Invoice" as GI
  usecase "View Reservations" as VR
  usecase "Update Room Status" as URS
  usecase "Generate Reports" as GR
  usecase "Apply Discount" as AD
  usecase "Authenticate" as AUTH
}

Guest --> SR
Guest --> MR
Guest --> CR
Receptionist --> CI
Receptionist --> CO
Receptionist --> VR
Receptionist --> URS
Manager --> GR
Manager --> AD
PS --> PP

MR .> AUTH : <<include>>
MR .> PP : <<include>>
CO .> PP : <<include>>
PP .> GI : <<include>>
AD .> MR : <<extend>>
@enduml
""")

    save("usecase_university_system", """
@startuml
actor Student
actor Professor
actor Administrator
actor "Academic System" as AS

rectangle "University Information System" {
  usecase "Register for Course" as RFC
  usecase "View Schedule" as VS
  usecase "Submit Assignment" as SA
  usecase "View Grades" as VG
  usecase "Create Course" as CC
  usecase "Publish Grades" as PG
  usecase "Manage Enrollment" as ME
  usecase "Generate Transcripts" as GT
  usecase "Check Prerequisites" as CP
  usecase "Send Notifications" as SN
}

Student --> RFC
Student --> VS
Student --> SA
Student --> VG
Professor --> CC
Professor --> PG
Administrator --> ME
Administrator --> GT
AS --> SN

RFC .> CP : <<include>>
PG .> SN : <<include>>
GT .> SN : <<include>>
ME .> RFC : <<extend>>
@enduml
""")

    # --- Note + include/extend combos ---
    for rel in [("include", ".>"), ("extend", ".>")]:
        rel_name, arrow = rel
        save(f"usecase_note_on_{rel_name}", f"""
@startuml
actor User
usecase BaseUC
usecase ChildUC
ChildUC {arrow} BaseUC : <<{rel_name}>>
note on link : This is a {rel_name} relationship
User --> BaseUC
@enduml
""")

    # Actor stereotype x color combos
    for stereo in ["<<human>>", "<<system>>", "<<external>>"]:
        for color in ["#LightBlue", "#Pink", "#LightGreen"]:
            safe_s = stereo.replace("<<", "").replace(">>", "")
            safe_c = color.replace("#", "")
            save(f"usecase_actor_{safe_s}_{safe_c}", f"""
@startuml
actor MyActor {stereo} {color}
usecase UC
MyActor --> UC
@enduml
""")

    # Large use case count
    for n in [10, 15, 20]:
        ucs = "\n".join([f"  usecase \"UC{i:02d}\" as UC{i:02d}" for i in range(1, n+1)])
        links = "\n".join([f"User --> UC{i:02d}" for i in range(1, n+1)])
        save(f"usecase_large_{n}", f"""
@startuml
actor User
rectangle "Large System" {{
{ucs}
}}
{links}
@enduml
""")

    # Multiple actors sharing use cases
    for n_shared in [2, 3, 4]:
        actors = "\n".join([f"actor Actor{i}" for i in range(1, 4)])
        ucs = "\n".join([f"usecase SharedUC{i}" for i in range(1, n_shared+1)])
        links = "\n".join([
            f"Actor{a} --> SharedUC{u}"
            for a in range(1, 4)
            for u in range(1, n_shared+1)
        ])
        save(f"usecase_shared_ucs_{n_shared}", f"""
@startuml
{actors}
{ucs}
{links}
@enduml
""")

    print(f"  Use case: {idx[0]} files")


# ---------------------------------------------------------------------------
# EXTRA COMBINATORIAL VARIANTS
# ---------------------------------------------------------------------------

def gen_extra_component():
    """Additional combinatorial component diagrams to reach ~600 total."""
    idx = [0]

    def save(name, content):
        path = os.path.join(COMP_DIR, f"{name}.puml")
        if not os.path.exists(path):
            write(path, content)
            idx[0] += 1

    # Arrow type x label x color
    for arrow, aname in [("-->", "dep"), ("..>", "use"), ("->", "nav"), ("--", "assoc")]:
        for label in ["calls", "uses", "extends", "implements", "publishes"]:
            for color in ["#LightBlue", "#Pink", "#LightGreen", "#Yellow"]:
                safe_c = color.replace("#", "")
                save(f"comp_extra_{aname}_{label}_{safe_c}", f"""
@startuml
component Foo {color}
component Bar {color}
Foo {arrow} Bar : {label}
@enduml
""")

    # Container + nested container + color + stereotype
    for outer in ["package", "node", "cloud", "rectangle", "frame", "folder"]:
        for inner in ["package", "node", "frame", "folder"]:
            for color in ["#LightBlue", "#LightGreen"]:
                safe_c = color.replace("#", "")
                save(f"comp_extra_nest_{outer}_{inner}_{safe_c}", f"""
@startuml
{outer} Outer {color} {{
  {inner} Inner {{
    component X
    component Y
    X --> Y
  }}
}}
@enduml
""")

    # Skinparam + arrow direction
    for style in ["uml2", "rectangle"]:
        for d in ["up", "down", "left", "right"]:
            save(f"comp_extra_style_{style}_dir_{d}", f"""
@startuml
skinparam componentStyle {style}
component A
component B
A -{d}-> B
@enduml
""")

    # Multi-note diagrams
    for n in [2, 3, 4]:
        comps = "\n".join([f"component C{i}" for i in range(1, n+1)])
        notes = "\n".join([f"note right of C{i} : Note {i}" for i in range(1, n+1)])
        arrows = "\n".join([f"C{i} --> C{i+1}" for i in range(1, n)])
        save(f"comp_extra_multinote_{n}", f"""
@startuml
{comps}
{notes}
{arrows}
@enduml
""")

    # Interface + lollipop + color
    for color in ["#LightBlue", "#Pink", "#LightGreen", "#Yellow", "#Orange"]:
        safe_c = color.replace("#", "")
        save(f"comp_extra_lollipop_{safe_c}", f"""
@startuml
component Provider {color}
component Consumer {color}
interface IService
Provider -( IService
Consumer --> IService : requires
@enduml
""")

    # Together + container + color
    for container in ["package", "node", "cloud"]:
        for color in ["#LightBlue", "#Pink", "#Yellow"]:
            safe_c = color.replace("#", "")
            save(f"comp_extra_together_{container}_{safe_c}", f"""
@startuml
{container} Outer {color} {{
  together {{
    component A
    component B
  }}
  component C
  A --> C
  B --> C
}}
@enduml
""")

    # Chain in container with arrow types
    for arrow, aname in [("-->", "dep"), ("..>", "use"), ("->", "nav")]:
        for container in ["package", "node", "cloud", "rectangle"]:
            for n in [3, 4, 5]:
                comps = "\n".join([f"  component X{i}" for i in range(1, n+1)])
                arrows = "\n".join([f"X{i} {arrow} X{i+1}" for i in range(1, n)])
                save(f"comp_extra_{container}_{aname}_chain{n}", f"""
@startuml
{container} Grp {{
{comps}
}}
{arrows}
@enduml
""")

    # Dense mesh in package
    for n in [4, 5, 6]:
        comps = "\n".join([f"  component M{i}" for i in range(1, n+1)])
        arrows = "\n".join([
            f"M{i} --> M{j}"
            for i in range(1, n+1)
            for j in range(i+1, n+1)
        ])
        save(f"comp_extra_mesh_{n}", f"""
@startuml
package Dense {{
{comps}
}}
{arrows}
@enduml
""")

    # Stereotype + note + color triple
    for stereo, sname in [("<<service>>", "svc"), ("<<repository>>", "repo"), ("<<facade>>", "fac")]:
        for color in ["#LightBlue", "#Pink", "#LightGreen"]:
            for pos in ["top", "right"]:
                safe_c = color.replace("#", "")
                save(f"comp_extra_{sname}_{safe_c}_note_{pos}", f"""
@startuml
component MyComp {stereo} {color}
note {pos} of MyComp : Tagged component
@enduml
""")

    # Bidirectional + color
    for arrow in ["<-->", "<..>"]:
        safe = arrow.replace("<", "l").replace(">", "r").replace(".", "d").replace("-", "s")
        for color in ["#LightBlue", "#Pink"]:
            safe_c = color.replace("#", "")
            save(f"comp_extra_bidi_{safe}_{safe_c}", f"""
@startuml
component A {color}
component B {color}
A {arrow} B
@enduml
""")

    # Cross-container arrows with labels
    for c1 in ["package", "node", "cloud"]:
        for c2 in ["package", "node", "frame"]:
            for label in ["HTTP", "JDBC", "TCP"]:
                save(f"comp_extra_cross_{c1}_{c2}_{label}", f"""
@startuml
{c1} Group1 {{
  component A
}}
{c2} Group2 {{
  component B
}}
A --> B : {label}
@enduml
""")

    print(f"  Component extra: {idx[0]} files")


def gen_extra_deployment():
    """Additional combinatorial deployment diagrams to reach ~500 total."""
    idx = [0]

    def save(name, content):
        path = os.path.join(DEPLOY_DIR, f"{name}.puml")
        if not os.path.exists(path):
            write(path, content)
            idx[0] += 1

    # Node + element + protocol combos
    for node_stereo in ["<<server>>", "<<vm>>", "<<container>>", "<<embedded>>"]:
        for db_type in ["database", "storage", "queue"]:
            for protocol in ["JDBC", "HTTP", "AMQP", "TCP"]:
                safe_s = node_stereo.replace("<<", "").replace(">>", "")
                save(f"deploy_extra_{safe_s}_{db_type}_{protocol}", f"""
@startuml
node AppNode {node_stereo}
{db_type} DataStore
AppNode --> DataStore : {protocol}
@enduml
""")

    # Nesting depth + color combos
    for outer in ["cloud", "node", "rectangle"]:
        for color in ["#LightBlue", "#LightGreen", "#Pink", "#Yellow"]:
            safe_c = color.replace("#", "")
            save(f"deploy_extra_{outer}_color_{safe_c}_nested", f"""
@startuml
{outer} L1 {color} {{
  node L2 {{
    component App
    database DB
    App --> DB
  }}
}}
@enduml
""")

    # Multiple artifacts per node + stereo
    for stereo in ["<<tomcat>>", "<<nginx>>", "<<spring>>"]:
        safe_s = stereo.replace("<<", "").replace(">>", "")
        save(f"deploy_extra_artifacts_{safe_s}", f"""
@startuml
node Server {stereo} {{
  artifact "app.war" {stereo}
  artifact "config.xml"
  file "logback.xml"
}}
@enduml
""")

    # Load balancer + backends with colors
    for n in [2, 3, 4, 5]:
        for color in ["#LightBlue", "#LightGreen"]:
            safe_c = color.replace("#", "")
            backends = "\n".join([f"node Backend{i} {color}" for i in range(1, n+1)])
            arrows = "\n".join([f"LB --> Backend{i}" for i in range(1, n+1)])
            save(f"deploy_extra_lb_{n}_{safe_c}", f"""
@startuml
node LB #Orange
{backends}
{arrows}
@enduml
""")

    # Messaging topologies
    for n_producers in [1, 2, 3]:
        for n_consumers in [2, 3, 4]:
            producers = "\n".join([f"node P{i}" for i in range(1, n_producers+1)])
            consumers = "\n".join([f"node C{i}" for i in range(1, n_consumers+1)])
            prod_arrows = "\n".join([f"P{i} --> Q" for i in range(1, n_producers+1)])
            cons_arrows = "\n".join([f"Q --> C{i}" for i in range(1, n_consumers+1)])
            save(f"deploy_extra_mq_{n_producers}prod_{n_consumers}cons", f"""
@startuml
{producers}
queue Q
{consumers}
{prod_arrows}
{cons_arrows}
@enduml
""")

    # Cloud + node + db + artifact combos
    for cloud_name in ["AWS", "Azure", "GCP"]:
        safe = cloud_name.replace(" ", "_")
        save(f"deploy_extra_cloud_{safe}", f"""
@startuml
cloud "{cloud_name}" {{
  node "Compute Instance" <<vm>> {{
    component "App Server"
    component "Cache"
  }}
  database "Managed DB"
  storage "Object Storage"
  queue "Message Queue"
  "App Server" --> "Managed DB"
  "App Server" --> "Cache"
  "App Server" --> "Message Queue"
  "App Server" --> "Object Storage"
}}
@enduml
""")

    # Database HA patterns
    for n in [1, 2, 3]:
        replicas = "\n".join([f"database Replica{i}" for i in range(1, n+1)])
        arrows = "\n".join([f"Primary --> Replica{i} : async" for i in range(1, n+1)])
        save(f"deploy_extra_db_ha_primary_{n}replicas", f"""
@startuml
node AppServer
database Primary
{replicas}
node "Failover Monitor"
AppServer --> Primary : read/write
{arrows}
"Failover Monitor" --> Primary : health check
@enduml
""")

    # K8s patterns with varying pod counts
    for n_pods in [1, 2, 3, 4]:
        pods = "\n".join([
            f"    node \"Pod-{i}\" {{\n      component \"Container-{i}\"\n    }}"
            for i in range(1, n_pods+1)
        ])
        save(f"deploy_extra_k8s_{n_pods}pods", f"""
@startuml
node "K8s Cluster" {{
  node "Worker Node" {{
{pods}
  }}
  node "Master" {{
    component "API Server"
    database "etcd"
  }}
}}
@enduml
""")

    # Regional deployment variants
    for n_regions in [2, 3, 4]:
        regions = []
        for i in range(1, n_regions+1):
            regions.append(f"""cloud "Region {i}" {{
  node "App-{i}"
  database "DB-{i}"
  "App-{i}" --> "DB-{i}"
}}""")
        replications = "\n".join([
            f'"DB-1" --> "DB-{i}" : replication'
            for i in range(2, n_regions+1)
        ])
        save(f"deploy_extra_multi_region_{n_regions}", f"""
@startuml
{chr(10).join(regions)}
{replications}
@enduml
""")

    # Skinparam variations with multiple element types
    for bg in ["LightBlue", "LightGreen", "White"]:
        for elem_type in ["node", "database", "queue", "cloud"]:
            save(f"deploy_extra_skin_{elem_type}_bg_{bg}", f"""
@startuml
skinparam {elem_type} {{
  BackgroundColor {bg}
}}
{elem_type} MyElem
node AppServer
AppServer --> MyElem
@enduml
""")

    # Agent/boundary/control patterns
    for combo in itertools.combinations(
        ["agent", "boundary", "control", "entity", "collections"], 3
    ):
        e1, e2, e3 = combo
        save(f"deploy_extra_{e1}_{e2}_{e3}", f"""
@startuml
{e1} E1
{e2} E2
{e3} E3
E1 --> E2
E2 --> E3
@enduml
""")

    # Multi-tier with n servers per tier
    for n_web in [1, 2, 3]:
        for n_app in [1, 2, 3]:
            web_nodes = "\n".join([f"  node Web{i}" for i in range(1, n_web+1)])
            app_nodes = "\n".join([f"  node App{i}" for i in range(1, n_app+1)])
            arrows = "\n".join([
                f"Web{i} --> App{j}"
                for i in range(1, n_web+1)
                for j in range(1, n_app+1)
            ])
            save(f"deploy_extra_tier_web{n_web}_app{n_app}", f"""
@startuml
node "Web Tier" {{
{web_nodes}
}}
node "App Tier" {{
{app_nodes}
}}
database DB
{arrows}
App1 --> DB
@enduml
""")

    print(f"  Deployment extra: {idx[0]} files")


def gen_extra_usecase():
    """Additional combinatorial use case diagrams to reach ~400 total."""
    idx = [0]

    def save(name, content):
        path = os.path.join(USECASE_DIR, f"{name}.puml")
        if not os.path.exists(path):
            write(path, content)
            idx[0] += 1

    # Include/extend + direction + n-actors combos
    for direction in ["left to right", "top to bottom"]:
        safe_d = direction.replace(" ", "_")
        for n_actors in [1, 2, 3]:
            for rel in ["include", "extend"]:
                actors = "\n".join([f"actor Actor{i}" for i in range(1, n_actors+1)])
                links = "\n".join([f"Actor{i} --> BaseUC" for i in range(1, n_actors+1)])
                arrow = ".>" if rel == "include" else ".>"
                save(f"usecase_extra_{safe_d}_{n_actors}a_{rel}", f"""
@startuml
{direction} direction
{actors}
usecase BaseUC
usecase SubUC
SubUC {arrow} BaseUC : <<{rel}>>
{links}
@enduml
""")

    # System boundary + direction + color combos
    for direction in ["left to right", "top to bottom"]:
        safe_d = direction.replace(" ", "_")
        for color in ["#LightBlue", "#LightGreen", "#Pink", "#Yellow"]:
            safe_c = color.replace("#", "")
            save(f"usecase_extra_{safe_d}_boundary_{safe_c}", f"""
@startuml
{direction} direction
actor User
rectangle "System" {color} {{
  usecase UC1
  usecase UC2
  usecase UC3
  UC1 .> UC2 : <<include>>
}}
User --> UC1
User --> UC3
@enduml
""")

    # Stereotype + direction + n-usecases
    for stereo in ["<<primary>>", "<<optional>>", "<<automated>>"]:
        safe_s = stereo.replace("<<", "").replace(">>", "")
        for direction in ["left to right", "top to bottom"]:
            safe_d = direction.replace(" ", "_")
            for n in [2, 3, 4]:
                ucs = "\n".join([f"  usecase UC{i} {stereo}" for i in range(1, n+1)])
                links = "\n".join([f"User --> UC{i}" for i in range(1, n+1)])
                save(f"usecase_extra_{safe_d}_{safe_s}_{n}ucs", f"""
@startuml
{direction} direction
actor User
rectangle System {{
{ucs}
}}
{links}
@enduml
""")

    # Multiple actors + generalization + include
    for n_actors in [2, 3, 4]:
        actors = "\n".join([f"actor Child{i}" for i in range(1, n_actors+1)])
        gen = "\n".join([f"Child{i} --|> Parent" for i in range(1, n_actors+1)])
        links = "\n".join([f"Child{i} --> SpecialUC" for i in range(1, n_actors+1)])
        save(f"usecase_extra_gen_{n_actors}children", f"""
@startuml
actor Parent
{actors}
usecase BaseUC
usecase SpecialUC
usecase CommonUC
{gen}
Parent --> BaseUC
{links}
BaseUC .> CommonUC : <<include>>
SpecialUC .> CommonUC : <<include>>
@enduml
""")

    # Color + note + direction combos
    for color in ["#LightBlue", "#Pink", "#Yellow", "#LightGreen"]:
        safe_c = color.replace("#", "")
        for pos in ["top", "right", "bottom", "left"]:
            save(f"usecase_extra_color_{safe_c}_note_{pos}", f"""
@startuml
actor User {color}
usecase "Main Action" {color}
note {pos} of "Main Action" : Description
User --> (Main Action)
@enduml
""")

    # Dense include/extend web
    for n_base in [2, 3]:
        for n_sub in [2, 3, 4]:
            base_ucs = "\n".join([f"usecase Base{i}" for i in range(1, n_base+1)])
            sub_ucs = "\n".join([f"usecase Sub{i}" for i in range(1, n_sub+1)])
            includes = "\n".join([
                f"Sub{j} .> Base{(j % n_base) + 1} : <<include>>"
                for j in range(1, n_sub+1)
            ])
            links = "\n".join([f"User --> Base{i}" for i in range(1, n_base+1)])
            save(f"usecase_extra_dense_base{n_base}_sub{n_sub}", f"""
@startuml
actor User
{base_ucs}
{sub_ucs}
{includes}
{links}
@enduml
""")

    # Package + direction + n-actors
    for direction in ["left to right", "top to bottom"]:
        safe_d = direction.replace(" ", "_")
        for n_pkg in [2, 3]:
            for n_actors in [2, 3]:
                packages = []
                links = []
                for i in range(1, n_pkg+1):
                    packages.append(f"""package "Pkg{i}" {{
  usecase "UC{i}A"
  usecase "UC{i}B"
}}""")
                    for a in range(1, n_actors+1):
                        links.append(f"A{a} --> (UC{i}A)")
                actors = "\n".join([f"actor A{a}" for a in range(1, n_actors+1)])
                save(f"usecase_extra_{safe_d}_pkg{n_pkg}_act{n_actors}", f"""
@startuml
{direction} direction
{actors}
{chr(10).join(packages)}
{chr(10).join(links)}
@enduml
""")

    # System-to-system use cases (no human actors)
    for n in [2, 3, 4]:
        systems = "\n".join([f"actor \"System {i}\" <<system>>" for i in range(1, n+1)])
        ucs = "\n".join([f"usecase UC{i}" for i in range(1, n+1)])
        links = "\n".join([f"\"System {i}\" --> UC{i}" for i in range(1, n+1)])
        save(f"deploy_extra_sys2sys_{n}", f"""
@startuml
{systems}
{ucs}
{links}
@enduml
""")

    # Skinparam + stereo + direction combos
    for stereo, sname in [("primary", "pri"), ("optional", "opt"), ("automated", "auto")]:
        for bg in ["LightBlue", "LightGreen", "Pink"]:
            for direction in ["left to right", "top to bottom"]:
                safe_d = direction.replace(" ", "_")
                save(f"usecase_extra_skin_{sname}_{bg}_{safe_d}", f"""
@startuml
{direction} direction
skinparam usecase {{
  BackgroundColor<<{stereo}>> {bg}
}}
actor User
usecase UC1 <<{stereo}>>
usecase UC2 <<{stereo}>>
User --> UC1
User --> UC2
UC1 .> UC2 : <<include>>
@enduml
""")

    # Abstract actor hierarchies
    for depth in [2, 3, 4]:
        actors = "\n".join([f"actor L{i}" for i in range(1, depth+1)])
        gen = "\n".join([f"L{i+1} --|> L{i}" for i in range(1, depth)])
        save(f"usecase_extra_hierarchy_depth{depth}", f"""
@startuml
{actors}
usecase UC
L{depth} --> UC
{gen}
@enduml
""")

    # Multi-boundary + direction + actors
    for n_bounds in [2, 3]:
        for direction in ["left to right", "top to bottom"]:
            safe_d = direction.replace(" ", "_")
            bounds = []
            links = []
            for i in range(1, n_bounds+1):
                bounds.append(f"""rectangle "Boundary{i}" {{
  usecase "Action{i}A"
  usecase "Action{i}B"
}}""")
                links.append(f"User --> (Action{i}A)")
                links.append(f"User --> (Action{i}B)")
            save(f"usecase_extra_{safe_d}_{n_bounds}bounds", f"""
@startuml
{direction} direction
actor User
{chr(10).join(bounds)}
{chr(10).join(links)}
@enduml
""")

    # Use cases with descriptions (separator styles) + colors
    for sep in ["--", "==", ".."]:
        for color in ["#LightBlue", "#Pink"]:
            safe_sep = sep.replace("-", "d").replace("=", "e").replace(".", "p")
            safe_c = color.replace("#", "")
            save(f"usecase_extra_desc_sep_{safe_sep}_color_{safe_c}", f"""
@startuml
actor User
usecase UC1 {color} as "
  Title
  {sep}
  Details here
"
User --> UC1
@enduml
""")

    # Large system + all relationship types
    save("usecase_extra_all_rels", """
@startuml
actor BaseActor
actor DerivedActor
DerivedActor --|> BaseActor
usecase Base
usecase Derived
usecase Included
usecase Extended
usecase Optional
Derived --|> Base
Base .> Included : <<include>>
Extended .> Base : <<extend>>
Optional .> Base : <<extend>>
BaseActor --> Base
DerivedActor --> Derived
@enduml
""")

    # Notes + colors + stereotypes all together
    for color in ["#LightBlue", "#LightGreen", "#Pink"]:
        for stereo in ["<<primary>>", "<<optional>>"]:
            safe_c = color.replace("#", "")
            safe_s = stereo.replace("<<", "").replace(">>", "")
            save(f"usecase_extra_all_{safe_c}_{safe_s}", f"""
@startuml
actor User #Orange <<human>>
usecase UC {color} {stereo}
note right of UC : Tagged and colored
note right of User : Human actor
User --> UC
@enduml
""")

    # Rectangular actor style
    for n in [1, 2, 3, 4, 5]:
        actors = "\n".join([f"actor A{i} <<system>>" for i in range(1, n+1)])
        ucs = "\n".join([f"usecase UC{i}" for i in range(1, n+1)])
        links = "\n".join([f"A{i} --> UC{i}" for i in range(1, n+1)])
        save(f"usecase_extra_system_actors_{n}", f"""
@startuml
{actors}
{ucs}
{links}
@enduml
""")

    # Association with direction hints
    for arrow in ["-->", "->", "--"]:
        for direction in ["left to right", "top to bottom"]:
            safe_a = arrow.replace(">", "r").replace("-", "l")
            safe_d = direction.replace(" ", "_")
            save(f"usecase_extra_arrow_{safe_a}_{safe_d}", f"""
@startuml
{direction} direction
actor User
usecase UC1
usecase UC2
User {arrow} UC1
User {arrow} UC2
UC1 .> UC2 : <<include>>
@enduml
""")

    print(f"  Use case extra: {idx[0]} files")


if __name__ == "__main__":
    print("Generating PlantUML test cases...")
    gen_component()
    gen_deployment()
    gen_usecase()
    gen_extra_component()
    gen_extra_deployment()
    gen_extra_usecase()
    total = (
        len(os.listdir(COMP_DIR)) +
        len(os.listdir(DEPLOY_DIR)) +
        len(os.listdir(USECASE_DIR))
    )
    print(f"  Total: {total} files")
    print("Done.")
