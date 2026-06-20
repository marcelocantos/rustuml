// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Write;

/// Wraps diagram body in @startuml/@enduml.
fn wrap(body: &str) -> String {
    format!("@startuml\n{body}\n@enduml\n")
}

/// Wraps diagram body in custom start/end tags (for non-UML diagram types).
fn wrap_custom(start: &str, body: &str, end: &str) -> String {
    format!("@{start}\n{body}\n@{end}\n")
}

// ---------------------------------------------------------------------------
// Sequence diagrams
// ---------------------------------------------------------------------------

/// Generates a simple two-participant sequence diagram.
pub fn simple_sequence() -> String {
    wrap("Alice -> Bob : hello")
}

/// Generates a sequence diagram with N messages between alternating participants.
pub fn multi_message_sequence(n: usize) -> String {
    let participants = ["Alice", "Bob"];
    let mut body = String::new();
    for i in 0..n {
        let from = participants[i % 2];
        let to = participants[(i + 1) % 2];
        writeln!(body, "{from} -> {to} : message {}", i + 1).unwrap();
    }
    wrap(&body)
}

/// Options for generating sequence diagrams with various features.
#[derive(Default)]
pub struct SequenceOptions {
    pub reply_arrows: bool,
    pub notes: bool,
    pub groups: bool,
    pub participant_declarations: bool,
}

/// Generates a sequence diagram exercising the requested features.
pub fn sequence_with_features(opts: &SequenceOptions) -> String {
    let mut body = String::new();

    if opts.participant_declarations {
        writeln!(body, "participant Alice").unwrap();
        writeln!(body, "participant Bob").unwrap();
    }

    writeln!(body, "Alice -> Bob : request").unwrap();

    if opts.notes {
        writeln!(body, "note right of Bob : Processing...").unwrap();
    }

    if opts.reply_arrows {
        writeln!(body, "Bob --> Alice : response").unwrap();
    }

    if opts.groups {
        writeln!(body, "group Transaction").unwrap();
        writeln!(body, "  Alice -> Bob : commit").unwrap();
        writeln!(body, "  Bob --> Alice : ack").unwrap();
        writeln!(body, "end").unwrap();
    }

    wrap(&body)
}

/// Generates a sequence diagram using all common arrow types.
pub fn sequence_arrow_types() -> String {
    wrap(
        "Alice -> Bob : sync\n\
         Alice --> Bob : dashed\n\
         Alice ->> Bob : async\n\
         Alice -->> Bob : dashed async\n\
         Alice ->x Bob : lost\n\
         Alice -\\ Bob : half solid\n\
         Alice -\\\\ Bob : half dashed\n\
         Alice -/ Bob : half right\n\
         Alice -// Bob : half right dashed\n\
         Alice o-> Bob : circle\n\
         Alice <-> Bob : bidirectional",
    )
}

/// Generates a sequence diagram with all participant types.
pub fn sequence_participant_types() -> String {
    wrap(
        "actor User\n\
         boundary Frontend\n\
         control Controller\n\
         entity Entity\n\
         database DB\n\
         collections Cache\n\
         queue MQ\n\
         User -> Frontend : request\n\
         Frontend -> Controller : forward\n\
         Controller -> Entity : load\n\
         Entity -> DB : query\n\
         Controller -> Cache : read\n\
         Controller -> MQ : publish",
    )
}

/// Generates a sequence diagram with participant aliases and creation.
pub fn sequence_participant_aliases() -> String {
    wrap(
        "participant \"Long Name\" as LN\n\
         participant \"Another Long Name\" as ALN\n\
         create participant \"Created Later\" as C\n\
         LN -> ALN : first\n\
         ALN -> C : create\n\
         C --> ALN : created",
    )
}

/// Generates a sequence diagram with lifeline activation and deactivation.
pub fn sequence_activation() -> String {
    wrap(
        "Alice -> Bob : request\n\
         activate Bob\n\
         Bob -> Charlie : delegate\n\
         activate Charlie\n\
         Charlie --> Bob : done\n\
         deactivate Charlie\n\
         Bob --> Alice : result\n\
         deactivate Bob",
    )
}

/// Generates a sequence diagram using ++ / -- activation shorthand.
pub fn sequence_activation_shorthand() -> String {
    wrap(
        "Alice -> Bob ++ : call\n\
         Bob -> Charlie ++ : sub-call\n\
         return sub-done\n\
         return done",
    )
}

/// Generates a sequence diagram with destroy.
pub fn sequence_destroy() -> String {
    wrap(
        "Alice -> Bob : create\n\
         activate Bob\n\
         Bob --> Alice : ack\n\
         Alice -> Bob : kill\n\
         destroy Bob",
    )
}

/// Generates a sequence diagram with all note types.
pub fn sequence_note_types() -> String {
    wrap(
        "Alice -> Bob : msg\n\
         note left of Alice : Left note\n\
         note right of Bob : Right note\n\
         note over Alice : Over Alice\n\
         note over Alice, Bob : Spanning both\n\
         hnote over Alice : hexagonal\n\
         rnote over Bob : rectangular",
    )
}

/// Generates a sequence diagram with all grouping types.
pub fn sequence_groupings() -> String {
    wrap(
        "Alice -> Bob : start\n\
         alt success\n\
           Bob --> Alice : ok\n\
         else failure\n\
           Bob --> Alice : error\n\
         end\n\
         opt optional\n\
           Alice -> Bob : maybe\n\
         end\n\
         loop 3 times\n\
           Alice -> Bob : repeat\n\
         end\n\
         par\n\
           Alice -> Bob : parallel a\n\
         else\n\
           Alice -> Bob : parallel b\n\
         end\n\
         break on error\n\
           Alice -> Bob : abort\n\
         end\n\
         critical\n\
           Alice -> Bob : exclusive\n\
         end\n\
         group Custom Label\n\
           Alice -> Bob : grouped\n\
         end",
    )
}

/// Generates a sequence diagram with dividers, delays, and spacing.
pub fn sequence_dividers_delays() -> String {
    wrap(
        "Alice -> Bob : before divider\n\
         == Section A ==\n\
         Alice -> Bob : in section\n\
         ...5 minutes later...\n\
         Alice -> Bob : after delay\n\
         |||\n\
         Alice -> Bob : after space\n\
         ||45||\n\
         Alice -> Bob : after sized space",
    )
}

/// Generates a sequence diagram with ref over.
pub fn sequence_ref() -> String {
    wrap(
        "Alice -> Bob : start\n\
         ref over Alice, Bob : See interaction foo\n\
         Bob --> Alice : done",
    )
}

/// Generates a sequence diagram with boxes.
pub fn sequence_boxes() -> String {
    wrap(
        "box \"System A\"\n\
           participant Alice\n\
           participant Bob\n\
         end box\n\
         box \"System B\" #LightBlue\n\
           participant Charlie\n\
         end box\n\
         Alice -> Bob : internal\n\
         Bob -> Charlie : cross-system",
    )
}

/// Generates a sequence diagram with basic autonumber.
pub fn sequence_autonumber() -> String {
    wrap(
        "autonumber\n\
         Alice -> Bob : first\n\
         Bob --> Alice : reply\n\
         autonumber stop\n\
         Alice -> Bob : unnumbered\n\
         autonumber resume\n\
         Alice -> Bob : continued",
    )
}

/// Generates a sequence diagram with autonumber format.
pub fn sequence_autonumber_format() -> String {
    wrap(
        "autonumber 10 10 \"<b>[00]</b>\"\n\
         Alice -> Bob : msg 10\n\
         Alice -> Bob : msg 20\n\
         Alice -> Bob : msg 30",
    )
}

/// Generates a sequence diagram with self-messages.
pub fn sequence_self_messages() -> String {
    wrap(
        "Alice -> Alice : self message\n\
         Alice --> Alice : self reply\n\
         Bob -> Bob : Bob thinks",
    )
}

/// Generates a sequence diagram with title, header, and footer.
pub fn sequence_title_header_footer() -> String {
    wrap(
        "title My Sequence Diagram\n\
         header Page 1\n\
         footer Generated by PlantUML\n\
         Alice -> Bob : hello\n\
         Bob --> Alice : world",
    )
}

/// Generates a sequence diagram with skinparam.
pub fn sequence_skinparam() -> String {
    wrap(
        "skinparam sequenceArrowThickness 2\n\
         skinparam roundcorner 20\n\
         skinparam maxmessagesize 60\n\
         Alice -> Bob : styled message\n\
         Bob --> Alice : styled reply",
    )
}

/// Generates a sequence diagram with colored participants.
pub fn sequence_colored_participants() -> String {
    wrap(
        "participant Alice #lightblue\n\
         participant Bob #lightgreen\n\
         Alice -> Bob : hello\n\
         Bob --> Alice : world",
    )
}

// ---------------------------------------------------------------------------
// Class diagrams
// ---------------------------------------------------------------------------

/// Generates a class diagram with inheritance and fields/methods.
pub fn class_diagram() -> String {
    wrap(
        "class Animal {\n  +name : String\n  +makeSound() : void\n}\n\
         class Dog extends Animal {\n  +fetch() : void\n}\n\
         Animal <|-- Dog",
    )
}

/// Generates a class diagram with interfaces and composition.
pub fn class_diagram_with_relationships() -> String {
    wrap(
        "interface Drawable {\n  +draw() : void\n}\n\
         class Shape implements Drawable {\n  #color : Color\n}\n\
         class Circle extends Shape {\n  -radius : double\n}\n\
         class Canvas {\n  -shapes : List<Shape>\n}\n\
         Canvas *-- Shape\n\
         Drawable <|.. Shape",
    )
}

/// Generates a class diagram with all entity types.
pub fn class_all_entity_types() -> String {
    wrap(
        "class MyClass {\n  +field : int\n}\n\
         abstract class AbstractBase {\n  +{abstract} doIt() : void\n}\n\
         interface MyInterface {\n  +method() : void\n}\n\
         enum Color {\n  RED\n  GREEN\n  BLUE\n}\n\
         annotation MyAnnotation\n\
         entity DataEntity {\n  id : int\n  name : String\n}\n\
         MyClass --|> AbstractBase\n\
         MyClass ..|> MyInterface",
    )
}

/// Generates a class diagram with all visibility modifiers.
pub fn class_visibility_modifiers() -> String {
    wrap(
        "class Visibility {\n\
           +publicField : int\n\
           -privateField : String\n\
           #protectedField : boolean\n\
           ~packageField : double\n\
           +publicMethod() : void\n\
           -privateMethod() : void\n\
           #protectedMethod() : void\n\
           ~packageMethod() : void\n\
         }",
    )
}

/// Generates a class diagram with static and abstract members.
pub fn class_static_abstract() -> String {
    wrap(
        "class Example {\n\
           +{static} staticField : int\n\
           +{abstract} abstractMethod() : void\n\
           +{static} staticMethod() : String\n\
         }",
    )
}

/// Generates a class diagram with generics.
pub fn class_generics() -> String {
    wrap(
        "class Container<T> {\n\
           -items : List<T>\n\
           +add(item : T) : void\n\
           +get(index : int) : T\n\
         }\n\
         class Pair<K, V> {\n\
           +key : K\n\
           +value : V\n\
         }",
    )
}

/// Generates a class diagram with all relationship types.
pub fn class_all_relationships() -> String {
    wrap(
        "ClassA <|-- ClassB : inheritance\n\
         ClassC ..|> InterfaceD : realization\n\
         ClassE *-- ClassF : composition\n\
         ClassG o-- ClassH : aggregation\n\
         ClassI -- ClassJ : association\n\
         ClassK ..> ClassL : dependency\n\
         ClassM ()-- ClassN : required interface",
    )
}

/// Generates a class diagram with labels and multiplicity on relationships.
pub fn class_relationship_labels() -> String {
    wrap(
        "Customer \"1\" --> \"*\" Order : places\n\
         Order \"1\" *-- \"1..*\" OrderLine : contains\n\
         Product \"*\" <-- \"*\" OrderLine : references",
    )
}

/// Generates a class diagram with packages and namespaces.
pub fn class_packages() -> String {
    wrap(
        "package com.example {\n\
           class Service {\n\
             +execute() : void\n\
           }\n\
           class Repository {\n\
             +findById(id : int) : Entity\n\
           }\n\
         }\n\
         namespace com.example.model {\n\
           class Entity {\n\
             +id : int\n\
           }\n\
         }\n\
         com.example.Service --> com.example.Repository",
    )
}

/// Generates a class diagram with notes on classes.
pub fn class_notes() -> String {
    wrap(
        "class Foo {\n  +bar() : void\n}\n\
         note top of Foo : This is a top note\n\
         note bottom of Foo : Bottom note\n\
         note left of Foo : Left note\n\
         note right of Foo : Right note",
    )
}

/// Generates a class diagram with stereotypes.
pub fn class_stereotypes() -> String {
    wrap(
        "class AuthService <<Service>> {\n\
           +login(user: String, pass: String) : Token\n\
         }\n\
         class UserEntity <<Entity>> {\n\
           +id : int\n\
           +username : String\n\
         }\n\
         class UserRepository <<Repository>> {\n\
           +findByUsername(name: String) : UserEntity\n\
         }",
    )
}

/// Generates a class diagram with body separators.
pub fn class_body_separators() -> String {
    wrap(
        "class WithSeparators {\n\
           +fieldA : int\n\
           --\n\
           +methodA() : void\n\
           ..\n\
           #internalMethod() : void\n\
           ==\n\
           +publicApi() : void\n\
         }",
    )
}

/// Generates an enum diagram with values.
pub fn class_enum_with_values() -> String {
    wrap(
        "enum Status {\n\
           PENDING\n\
           ACTIVE\n\
           INACTIVE\n\
           DELETED\n\
         }\n\
         enum Priority {\n\
           LOW\n\
           MEDIUM\n\
           HIGH\n\
         }",
    )
}

/// Generates an object diagram.
pub fn class_object_diagram() -> String {
    wrap(
        "object alice {\n\
           name = \"Alice\"\n\
           age = 30\n\
         }\n\
         object bob {\n\
           name = \"Bob\"\n\
           age = 25\n\
         }\n\
         alice --> bob : knows",
    )
}

// ---------------------------------------------------------------------------
// State diagrams
// ---------------------------------------------------------------------------

/// Generates a simple state diagram with transitions.
pub fn state_diagram() -> String {
    wrap(
        "[*] --> Active\n\
         Active --> Inactive : disable\n\
         Inactive --> Active : enable\n\
         Active --> [*] : close",
    )
}

/// Generates a state diagram with nested states.
pub fn state_diagram_nested() -> String {
    wrap(
        "state Running {\n  \
           [*] --> Processing\n  \
           Processing --> Waiting : pause\n  \
           Waiting --> Processing : resume\n\
         }\n\
         [*] --> Running\n\
         Running --> [*] : shutdown",
    )
}

/// Generates a state diagram with concurrent regions.
pub fn state_diagram_concurrent() -> String {
    wrap(
        "state ConcurrentState {\n\
           state RegionA {\n\
             [*] --> A1\n\
             A1 --> A2\n\
           }\n\
           --\n\
           state RegionB {\n\
             [*] --> B1\n\
             B1 --> B2\n\
           }\n\
         }\n\
         [*] --> ConcurrentState",
    )
}

/// Generates a state diagram with history state.
pub fn state_diagram_history() -> String {
    wrap(
        "state Workflow {\n\
           [*] --> Step1\n\
           Step1 --> Step2\n\
           Step2 --> Step3\n\
         }\n\
         [*] --> Workflow\n\
         Workflow --> Paused : pause\n\
         Paused --> Workflow[H] : resume",
    )
}

/// Generates a state diagram with notes on states.
pub fn state_diagram_notes() -> String {
    wrap(
        "[*] --> Active\n\
         Active --> Inactive : disable\n\
         note right of Active : This is the active state\n\
         note left of Inactive : Waiting for enable",
    )
}

/// Generates a state diagram with colored states.
pub fn state_diagram_colored() -> String {
    wrap(
        "state Active #lightgreen\n\
         state Error #pink\n\
         state Pending #lightyellow\n\
         [*] --> Pending\n\
         Pending --> Active : approve\n\
         Pending --> Error : reject\n\
         Active --> [*]",
    )
}

// ---------------------------------------------------------------------------
// Activity diagrams
// ---------------------------------------------------------------------------

/// Generates a simple activity diagram with a condition.
pub fn activity_diagram() -> String {
    wrap(
        "start\n\
         :Step 1;\n\
         if (condition?) then (yes)\n  \
           :Step 2a;\n\
         else (no)\n  \
           :Step 2b;\n\
         endif\n\
         stop",
    )
}

/// Generates an activity diagram with a loop and fork/join.
pub fn activity_diagram_with_fork() -> String {
    wrap(
        "start\n\
         :Initialize;\n\
         fork\n  \
           :Task A;\n\
         fork again\n  \
           :Task B;\n\
         end fork\n\
         :Finalize;\n\
         stop",
    )
}

/// Generates an activity diagram with start, stop, and end.
pub fn activity_start_stop_end() -> String {
    wrap(
        "start\n\
         :Action;\n\
         if (done?) then (yes)\n\
           stop\n\
         else (no)\n\
           end\n\
         endif",
    )
}

/// Generates an activity diagram with elseif branches.
pub fn activity_elseif() -> String {
    wrap(
        "start\n\
         :Input;\n\
         if (value > 10?) then (yes)\n\
           :High;\n\
         elseif (value > 5?) then (yes)\n\
           :Medium;\n\
         else (no)\n\
           :Low;\n\
         endif\n\
         stop",
    )
}

/// Generates an activity diagram with switch/case.
pub fn activity_switch() -> String {
    wrap(
        "start\n\
         switch (color?)\n\
         case (red)\n\
           :Red action;\n\
         case (green)\n\
           :Green action;\n\
         case (blue)\n\
           :Blue action;\n\
         endswitch\n\
         stop",
    )
}

/// Generates an activity diagram with while loop.
pub fn activity_while() -> String {
    wrap(
        "start\n\
         while (items remain?) is (yes)\n\
           :Process item;\n\
         endwhile (no)\n\
         stop",
    )
}

/// Generates an activity diagram with repeat/repeatwhile.
pub fn activity_repeat() -> String {
    wrap(
        "start\n\
         repeat\n\
           :Try action;\n\
         repeat while (failed?) is (yes)\n\
         stop",
    )
}

/// Generates an activity diagram with split.
pub fn activity_split() -> String {
    wrap(
        "start\n\
         split\n\
           :Branch A;\n\
         split again\n\
           :Branch B;\n\
         split again\n\
           :Branch C;\n\
         end split\n\
         stop",
    )
}

/// Generates an activity diagram with swimlanes.
pub fn activity_swimlanes() -> String {
    wrap(
        "|User|\n\
         start\n\
         :Submit request;\n\
         |System|\n\
         :Validate request;\n\
         if (valid?) then (yes)\n\
           |Database|\n\
           :Save data;\n\
           |User|\n\
           :Show success;\n\
         else (no)\n\
           |User|\n\
           :Show error;\n\
         endif\n\
         stop",
    )
}

/// Generates an activity diagram with partition.
pub fn activity_partition() -> String {
    wrap(
        "start\n\
         partition \"Phase 1\" {\n\
           :Setup;\n\
           :Configure;\n\
         }\n\
         partition \"Phase 2\" {\n\
           :Execute;\n\
           :Verify;\n\
         }\n\
         stop",
    )
}

/// Generates an activity diagram with notes.
pub fn activity_notes() -> String {
    wrap(
        "start\n\
         :Main action;\n\
         note right\n\
           This is a note\n\
           on the right\n\
         end note\n\
         :Another action;\n\
         note left : Quick left note\n\
         stop",
    )
}

/// Generates an activity diagram with kill and detach.
pub fn activity_kill_detach() -> String {
    wrap(
        "start\n\
         :Process;\n\
         if (error?) then (yes)\n\
           kill\n\
         else (no)\n\
           :Continue;\n\
           detach\n\
         endif",
    )
}

/// Generates an activity diagram with connectors.
pub fn activity_connectors() -> String {
    wrap(
        "start\n\
         :Step A;\n\
         (connector1)\n\
         :Step B;\n\
         if (done?) then (yes)\n\
           stop\n\
         else (no)\n\
           (connector1)\n\
         endif",
    )
}

// ---------------------------------------------------------------------------
// Component diagrams
// ---------------------------------------------------------------------------

/// Generates a simple component diagram.
pub fn component_diagram() -> String {
    wrap(
        "component \"Web Server\" as WS\n\
         component \"Database\" as DB\n\
         component \"Cache\" as C\n\
         WS --> DB : query\n\
         WS --> C : read",
    )
}

/// Generates a component diagram with interfaces.
pub fn component_with_interfaces() -> String {
    wrap(
        "component Frontend\n\
         component Backend\n\
         component DataStore\n\
         Frontend - [Backend] : REST\n\
         [Backend] - [DataStore] : SQL\n\
         () \"HTTP\" as HTTP\n\
         Frontend ..> HTTP\n\
         HTTP ..> Backend",
    )
}

/// Generates a component diagram with packages.
pub fn component_packages() -> String {
    wrap(
        "package \"UI Layer\" {\n\
           component WebApp\n\
           component MobileApp\n\
         }\n\
         package \"Service Layer\" {\n\
           component AuthService\n\
           component DataService\n\
         }\n\
         package \"Data Layer\" {\n\
           database PostgreSQL\n\
           database Redis\n\
         }\n\
         WebApp --> AuthService\n\
         WebApp --> DataService\n\
         DataService --> PostgreSQL\n\
         AuthService --> Redis",
    )
}

// ---------------------------------------------------------------------------
// Use case diagrams
// ---------------------------------------------------------------------------

/// Generates a use case diagram with actors and use cases.
pub fn use_case_diagram() -> String {
    wrap(
        "actor User\n\
         actor Admin\n\
         usecase \"Login\" as UC1\n\
         usecase \"View Dashboard\" as UC2\n\
         usecase \"Manage Users\" as UC3\n\
         User --> UC1\n\
         User --> UC2\n\
         Admin --> UC1\n\
         Admin --> UC3",
    )
}

/// Generates a use case diagram with stereotypes and extends.
pub fn use_case_extended() -> String {
    wrap(
        "actor \"Customer\" as C\n\
         actor \"Admin\" <<admin>> as A\n\
         usecase \"Browse Catalog\" as UC1\n\
         usecase \"Place Order\" as UC2\n\
         usecase \"Apply Coupon\" as UC3\n\
         usecase \"Track Order\" as UC4\n\
         usecase \"Manage Products\" as UC5\n\
         C --> UC1\n\
         C --> UC2\n\
         C --> UC4\n\
         UC2 ..> UC3 : extends\n\
         A --> UC5",
    )
}

/// Generates a use case diagram with packages.
pub fn use_case_packages() -> String {
    wrap(
        "actor User\n\
         rectangle System {\n\
           usecase Login\n\
           usecase Logout\n\
           usecase ViewData\n\
         }\n\
         User --> Login\n\
         User --> Logout\n\
         User --> ViewData",
    )
}

// ---------------------------------------------------------------------------
// Deployment diagrams
// ---------------------------------------------------------------------------

/// Generates a deployment diagram with nodes, artifacts, and cloud/frame.
pub fn deployment_diagram() -> String {
    wrap(
        "node \"App Server\" as AppServer {\n\
           artifact \"webapp.war\" as webapp\n\
         }\n\
         node \"Database Server\" as DBServer {\n\
           database \"PostgreSQL\" as DB\n\
         }\n\
         cloud \"CDN\" as CDN\n\
         frame \"Client Browser\" as Browser\n\
         Browser --> CDN : HTTPS\n\
         Browser --> AppServer : HTTPS\n\
         AppServer --> DBServer : JDBC",
    )
}

/// Generates a deployment diagram with various node types.
pub fn deployment_node_types() -> String {
    wrap(
        "node WebServer\n\
         node AppServer\n\
         database PostgreSQL\n\
         cloud AWS\n\
         frame Browser\n\
         storage FileStorage\n\
         WebServer --> AppServer : HTTP\n\
         AppServer --> PostgreSQL : SQL\n\
         AppServer --> FileStorage : read/write\n\
         AWS --> WebServer : hosts",
    )
}

// ---------------------------------------------------------------------------
// Salt (wireframe) diagrams
// ---------------------------------------------------------------------------

/// Generates a simple Salt wireframe dialog.
pub fn salt_dialog() -> String {
    wrap_custom(
        "startsalt",
        "{+\n\
           Login Dialog\n\
           ===\n\
           Username | \"user@example.com\"\n\
           Password | \"***\"\n\
           ===\n\
           [Cancel] | [  OK  ]\n\
         }",
        "endsalt",
    )
}

/// Generates a Salt wireframe with a tree widget.
pub fn salt_tree() -> String {
    wrap_custom(
        "startsalt",
        "{\n\
           {T\n\
             + Root\n\
             ++ Child 1\n\
             +++ Grandchild\n\
             ++ Child 2\n\
           }\n\
         }",
        "endsalt",
    )
}

// ---------------------------------------------------------------------------
// JSON diagrams
// ---------------------------------------------------------------------------

/// Generates a JSON diagram.
pub fn json_diagram() -> String {
    wrap_custom(
        "startjson",
        "{\n\
           \"name\": \"Alice\",\n\
           \"age\": 30,\n\
           \"address\": {\n\
             \"street\": \"123 Main St\",\n\
             \"city\": \"Springfield\"\n\
           },\n\
           \"hobbies\": [\"reading\", \"coding\"]\n\
         }",
        "endjson",
    )
}

// ---------------------------------------------------------------------------
// YAML diagrams
// ---------------------------------------------------------------------------

/// Generates a YAML diagram.
pub fn yaml_diagram() -> String {
    wrap_custom(
        "startyaml",
        "name: Alice\n\
         age: 30\n\
         address:\n\
           street: 123 Main St\n\
           city: Springfield\n\
         hobbies:\n\
           - reading\n\
           - coding",
        "endyaml",
    )
}

// ---------------------------------------------------------------------------
// Mind map diagrams
// ---------------------------------------------------------------------------

/// Generates a mind map diagram.
pub fn mindmap_diagram() -> String {
    wrap_custom(
        "startmindmap",
        "* Root Topic\n\
         ** Branch A\n\
         *** Leaf A1\n\
         *** Leaf A2\n\
         ** Branch B\n\
         *** Leaf B1\n\
         left side\n\
         ** Branch C\n\
         *** Leaf C1",
        "endmindmap",
    )
}

// ---------------------------------------------------------------------------
// WBS (Work Breakdown Structure) diagrams
// ---------------------------------------------------------------------------

/// Generates a WBS diagram.
pub fn wbs_diagram() -> String {
    wrap_custom(
        "startwbs",
        "* Project\n\
         ** Planning\n\
         *** Requirements\n\
         *** Design\n\
         ** Development\n\
         *** Backend\n\
         *** Frontend\n\
         ** Testing\n\
         *** Unit Tests\n\
         *** Integration Tests",
        "endwbs",
    )
}

// ---------------------------------------------------------------------------
// Gantt charts
// ---------------------------------------------------------------------------

/// Generates a Gantt chart.
pub fn gantt_diagram() -> String {
    wrap_custom(
        "startgantt",
        "Project starts 2025-01-01\n\
         [Requirements] lasts 5 days\n\
         [Design] lasts 7 days\n\
         [Design] starts at [Requirements]'s end\n\
         [Development] lasts 14 days\n\
         [Development] starts at [Design]'s end\n\
         [Testing] lasts 7 days\n\
         [Testing] starts at [Development]'s end",
        "endgantt",
    )
}

// ---------------------------------------------------------------------------
// Timing diagrams
// ---------------------------------------------------------------------------

/// Generates a timing diagram with robust and concise signals.
pub fn timing_diagram() -> String {
    // Timing diagrams use @startuml/@enduml with robust/concise keywords.
    wrap(
        "robust \"Signal A\" as SA\n\
         concise \"Signal B\" as SB\n\
         @0\n\
         SA is idle\n\
         SB is idle\n\
         @100\n\
         SA is active\n\
         @200\n\
         SB is active\n\
         @300\n\
         SA is idle\n\
         SB is idle",
    )
}

// ---------------------------------------------------------------------------
// Combinatorial generation
// ---------------------------------------------------------------------------

/// A named test case with PlantUML source.
pub struct TestCase {
    pub name: String,
    pub source: String,
}

/// Generates all combinatorial test cases across diagram types.
///
/// Currently produces:
/// - 16 sequence diagram variants (4 boolean features = 2^4)
/// - N multi-message sequences (1..=message_counts)
/// - Expanded sequence, class, state, activity, component, use case,
///   deployment, salt, json, yaml, mindmap, wbs, gantt, and timing cases
pub fn all_cases(message_counts: usize) -> Vec<TestCase> {
    let mut cases = Vec::new();

    // Sequence diagram combinatorics: 4 boolean features = 16 combinations.
    for bits in 0u8..16 {
        let opts = SequenceOptions {
            reply_arrows: bits & 1 != 0,
            notes: bits & 2 != 0,
            groups: bits & 4 != 0,
            participant_declarations: bits & 8 != 0,
        };
        let name = format!(
            "seq_reply{}_notes{}_groups{}_decl{}",
            opts.reply_arrows as u8,
            opts.notes as u8,
            opts.groups as u8,
            opts.participant_declarations as u8,
        );
        cases.push(TestCase {
            name,
            source: sequence_with_features(&opts),
        });
    }

    // Multi-message sequences with varying counts.
    for n in 1..=message_counts {
        cases.push(TestCase {
            name: format!("seq_messages_{n}"),
            source: multi_message_sequence(n),
        });
    }

    // Sequence diagram feature cases.
    let seq_cases: &[(&str, fn() -> String)] = &[
        ("seq_arrow_types", sequence_arrow_types),
        ("seq_participant_types", sequence_participant_types),
        ("seq_participant_aliases", sequence_participant_aliases),
        ("seq_activation", sequence_activation),
        ("seq_activation_shorthand", sequence_activation_shorthand),
        ("seq_destroy", sequence_destroy),
        ("seq_note_types", sequence_note_types),
        ("seq_groupings", sequence_groupings),
        ("seq_dividers_delays", sequence_dividers_delays),
        ("seq_ref", sequence_ref),
        ("seq_boxes", sequence_boxes),
        ("seq_autonumber", sequence_autonumber),
        ("seq_autonumber_format", sequence_autonumber_format),
        ("seq_self_messages", sequence_self_messages),
        ("seq_title_header_footer", sequence_title_header_footer),
        ("seq_skinparam", sequence_skinparam),
        ("seq_colored_participants", sequence_colored_participants),
    ];
    for (name, f) in seq_cases {
        cases.push(TestCase {
            name: (*name).into(),
            source: f(),
        });
    }

    // Class diagram cases.
    let class_cases: &[(&str, fn() -> String)] = &[
        ("class_basic", class_diagram),
        ("class_relationships", class_diagram_with_relationships),
        ("class_all_entity_types", class_all_entity_types),
        ("class_visibility_modifiers", class_visibility_modifiers),
        ("class_static_abstract", class_static_abstract),
        ("class_generics", class_generics),
        ("class_all_relationships", class_all_relationships),
        ("class_relationship_labels", class_relationship_labels),
        ("class_packages", class_packages),
        ("class_notes", class_notes),
        ("class_stereotypes", class_stereotypes),
        ("class_body_separators", class_body_separators),
        ("class_enum_with_values", class_enum_with_values),
        ("class_object_diagram", class_object_diagram),
    ];
    for (name, f) in class_cases {
        cases.push(TestCase {
            name: (*name).into(),
            source: f(),
        });
    }

    // State diagram cases.
    let state_cases: &[(&str, fn() -> String)] = &[
        ("state_basic", state_diagram),
        ("state_nested", state_diagram_nested),
        ("state_concurrent", state_diagram_concurrent),
        ("state_history", state_diagram_history),
        ("state_notes", state_diagram_notes),
        ("state_colored", state_diagram_colored),
    ];
    for (name, f) in state_cases {
        cases.push(TestCase {
            name: (*name).into(),
            source: f(),
        });
    }

    // Activity diagram cases.
    let activity_cases: &[(&str, fn() -> String)] = &[
        ("activity_basic", activity_diagram),
        ("activity_fork", activity_diagram_with_fork),
        ("activity_start_stop_end", activity_start_stop_end),
        ("activity_elseif", activity_elseif),
        ("activity_switch", activity_switch),
        ("activity_while", activity_while),
        ("activity_repeat", activity_repeat),
        ("activity_split", activity_split),
        ("activity_swimlanes", activity_swimlanes),
        ("activity_partition", activity_partition),
        ("activity_notes", activity_notes),
        ("activity_kill_detach", activity_kill_detach),
        ("activity_connectors", activity_connectors),
    ];
    for (name, f) in activity_cases {
        cases.push(TestCase {
            name: (*name).into(),
            source: f(),
        });
    }

    // Component diagram cases.
    let component_cases: &[(&str, fn() -> String)] = &[
        ("component_basic", component_diagram),
        ("component_with_interfaces", component_with_interfaces),
        ("component_packages", component_packages),
    ];
    for (name, f) in component_cases {
        cases.push(TestCase {
            name: (*name).into(),
            source: f(),
        });
    }

    // Use case diagram cases.
    let use_case_cases: &[(&str, fn() -> String)] = &[
        ("use_case_basic", use_case_diagram),
        ("use_case_extended", use_case_extended),
        ("use_case_packages", use_case_packages),
    ];
    for (name, f) in use_case_cases {
        cases.push(TestCase {
            name: (*name).into(),
            source: f(),
        });
    }

    // Deployment diagram cases.
    let deployment_cases: &[(&str, fn() -> String)] = &[
        ("deployment_basic", deployment_diagram),
        ("deployment_node_types", deployment_node_types),
    ];
    for (name, f) in deployment_cases {
        cases.push(TestCase {
            name: (*name).into(),
            source: f(),
        });
    }

    // Salt wireframe cases.
    let salt_cases: &[(&str, fn() -> String)] = &[
        ("salt_dialog", salt_dialog),
        ("salt_tree", salt_tree),
    ];
    for (name, f) in salt_cases {
        cases.push(TestCase {
            name: (*name).into(),
            source: f(),
        });
    }

    // Non-UML diagram cases (JSON, YAML, mindmap, WBS, Gantt, timing).
    let other_cases: &[(&str, fn() -> String)] = &[
        ("json_diagram", json_diagram),
        ("yaml_diagram", yaml_diagram),
        ("mindmap_diagram", mindmap_diagram),
        ("wbs_diagram", wbs_diagram),
        ("gantt_diagram", gantt_diagram),
        ("timing_diagram", timing_diagram),
    ];
    for (name, f) in other_cases {
        cases.push(TestCase {
            name: (*name).into(),
            source: f(),
        });
    }

    cases
}
