// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Write;

/// Wraps diagram body in @startuml/@enduml.
fn wrap(body: &str) -> String {
    format!("@startuml\n{body}\n@enduml\n")
}

/// Wraps diagram body in custom start/end tags.
fn wrap_custom(start: &str, body: &str, end: &str) -> String {
    format!("{start}\n{body}\n{end}\n")
}

// ===========================================================================
// Sequence diagrams
// ===========================================================================

pub fn simple_sequence() -> String {
    wrap("Alice -> Bob : hello")
}

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

#[derive(Default)]
pub struct SequenceOptions {
    pub reply_arrows: bool,
    pub notes: bool,
    pub groups: bool,
    pub participant_declarations: bool,
}

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

pub fn seq_arrow_types() -> String {
    wrap(
        "A -> B : solid\n\
         A --> B : dotted\n\
         A ->> B : thin\n\
         A -->> B : thin dotted\n\
         A ->x B : lost\n\
         A ->o B : circle end\n\
         A o-> B : circle start\n\
         A <-> B : bidirectional",
    )
}

pub fn seq_participant_types() -> String {
    wrap(
        "participant P\n\
         actor A\n\
         boundary B\n\
         control C\n\
         entity E\n\
         database D\n\
         collections Co\n\
         queue Q\n\
         P -> A : msg",
    )
}

pub fn seq_activation() -> String {
    wrap(
        "A -> B ++ : activate\n\
         B -> C ++ : nested\n\
         C --> B -- : return\n\
         B --> A -- : return",
    )
}

pub fn seq_notes_all() -> String {
    wrap(
        "A -> B : msg\n\
         note left : left note\n\
         note right : right note\n\
         note over A : over A\n\
         note over A, B : over both",
    )
}

pub fn seq_alt_else() -> String {
    wrap(
        "A -> B : check\n\
         alt success\n\
           B --> A : ok\n\
         else failure\n\
           B --> A : error\n\
         end",
    )
}

pub fn seq_loop() -> String {
    wrap(
        "A -> B : start\n\
         loop 3 times\n\
           B --> A : iterate\n\
         end",
    )
}

pub fn seq_opt() -> String {
    wrap(
        "A -> B : check\n\
         opt condition met\n\
           B --> A : response\n\
         end",
    )
}

pub fn seq_critical() -> String {
    wrap(
        "A -> B : start\n\
         critical Critical section\n\
           A -> B : important\n\
         end",
    )
}

pub fn seq_par() -> String {
    wrap(
        "participant A\nparticipant B\nparticipant C\n\
         par Branch 1\n\
           A -> B : msg1\n\
         else Branch 2\n\
           A -> C : msg2\n\
         end",
    )
}

pub fn seq_break() -> String {
    wrap(
        "A -> B : start\n\
         loop forever\n\
           A -> B : try\n\
           break on error\n\
             B --> A : abort\n\
           end\n\
         end",
    )
}

pub fn seq_divider() -> String {
    wrap(
        "A -> B : before\n\
         == Phase 2 ==\n\
         A -> B : after",
    )
}

pub fn seq_delay() -> String {
    wrap(
        "A -> B : before\n\
         ...5 minutes later...\n\
         A -> B : after",
    )
}

pub fn seq_spacing() -> String {
    wrap(
        "A -> B : msg1\n\
         |||\n\
         A -> B : msg2\n\
         ||45||\n\
         A -> B : msg3",
    )
}

pub fn seq_box() -> String {
    wrap(
        "box \"Internal\"\n\
         participant A\n\
         participant B\n\
         end box\n\
         box \"External\" #LightBlue\n\
         participant C\n\
         end box\n\
         A -> C : call",
    )
}

pub fn seq_autonumber() -> String {
    wrap(
        "autonumber\n\
         A -> B : first\n\
         B -> C : second\n\
         C --> B : third",
    )
}

pub fn seq_autonumber_format() -> String {
    wrap(
        "autonumber 10 10 \"[000]\"\n\
         A -> B : first\n\
         B --> A : second\n\
         autonumber stop\n\
         A -> B : unnumbered\n\
         autonumber resume\n\
         A -> B : numbered again",
    )
}

pub fn seq_return() -> String {
    wrap(
        "A -> B ++ : request\n\
         return response",
    )
}

pub fn seq_self_message() -> String {
    wrap(
        "A -> A : self call\n\
         A --> A : self return",
    )
}

pub fn seq_create() -> String {
    wrap(
        "A -> B : normal\n\
         create C\n\
         A -> C : create",
    )
}

pub fn seq_title_header_footer() -> String {
    wrap(
        "title My Diagram\n\
         header Page Header\n\
         footer Page Footer\n\
         caption Figure 1\n\
         A -> B : msg",
    )
}

pub fn seq_legend() -> String {
    wrap(
        "A -> B : msg\n\
         legend\n\
           This is the legend\n\
         endlegend",
    )
}

pub fn seq_hide_unlinked() -> String {
    wrap(
        "participant A\n\
         participant B\n\
         participant C\n\
         hide unlinked\n\
         A -> B : only A and B",
    )
}

pub fn seq_newpage() -> String {
    wrap(
        "A -> B : page 1\n\
         newpage\n\
         A -> B : page 2",
    )
}

pub fn seq_skinparam() -> String {
    wrap(
        "skinparam sequenceArrowThickness 2\n\
         skinparam sequenceParticipantBorderColor Blue\n\
         A -> B : styled",
    )
}

pub fn seq_ref() -> String {
    wrap(
        "participant A\n\
         participant B\n\
         ref over A, B : See other diagram\n\
         A -> B : continue",
    )
}

pub fn seq_external() -> String {
    wrap(
        "participant A\n\
         [-> A : from outside\n\
         A ->] : to outside",
    )
}

pub fn seq_alias() -> String {
    wrap(
        "participant \"Alice Johnson\" as A\n\
         participant \"Bob Smith\" as B\n\
         A -> B : hello",
    )
}

pub fn seq_destroy() -> String {
    wrap(
        "A -> B : request\n\
         B --> A : response\n\
         destroy B",
    )
}

// ===========================================================================
// Class diagrams
// ===========================================================================

pub fn class_diagram() -> String {
    wrap(
        "class Animal {\n  +name : String\n  +makeSound() : void\n}\n\
         class Dog extends Animal {\n  +fetch() : void\n}\n\
         Animal <|-- Dog",
    )
}

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

pub fn class_entity_types() -> String {
    wrap(
        "class MyClass\n\
         abstract class MyAbstract\n\
         interface MyInterface\n\
         enum MyEnum\n\
         annotation MyAnnotation\n\
         entity MyEntity",
    )
}

pub fn class_visibility() -> String {
    wrap(
        "class Foo {\n\
           +publicField : String\n\
           -privateField : int\n\
           #protectedField : boolean\n\
           ~packageField : double\n\
           +publicMethod()\n\
           -privateMethod()\n\
         }",
    )
}

pub fn class_static_abstract() -> String {
    wrap(
        "class Foo {\n\
           {static} counter : int\n\
           {abstract} process()\n\
           {static} getInstance()\n\
         }",
    )
}

pub fn class_generics() -> String {
    wrap(
        "class Container<T>\n\
         class Map<K, V>\n\
         Container -> Map",
    )
}

pub fn class_all_relationships() -> String {
    wrap(
        "A <|-- B\n\
         C ..|> D\n\
         E *-- F\n\
         G o-- H\n\
         I -- J\n\
         K ..> L",
    )
}

pub fn class_labels_multiplicity() -> String {
    wrap(
        "Parent \"1\" -- \"0..*\" Child : has\n\
         Teacher \"1\" -- \"many\" Student : teaches",
    )
}

pub fn class_packages() -> String {
    wrap(
        "package com.example {\n\
           class Foo\n\
           class Bar\n\
         }\n\
         package org.test {\n\
           class Baz\n\
         }\n\
         Foo --> Baz",
    )
}

pub fn class_nested_packages() -> String {
    wrap(
        "package outer {\n\
           package inner {\n\
             class Foo\n\
           }\n\
           class Bar\n\
         }",
    )
}

pub fn class_notes() -> String {
    wrap(
        "class Foo\n\
         note right of Foo : This is Foo\n\
         class Bar\n\
         Foo -- Bar\n\
         note on link : relationship note",
    )
}

pub fn class_stereotypes() -> String {
    wrap(
        "class Foo <<singleton>>\n\
         class Bar <<deprecated>>\n\
         interface Baz <<callback>>",
    )
}

pub fn class_separators() -> String {
    wrap(
        "class Foo {\n\
           +field1\n\
           --\n\
           +method1()\n\
           ==\n\
           -internal\n\
           ..\n\
           #protectedStuff\n\
         }",
    )
}

pub fn class_enum_values() -> String {
    wrap(
        "enum Color {\n\
           RED\n\
           GREEN\n\
           BLUE\n\
         }",
    )
}

pub fn class_together() -> String {
    wrap(
        "together {\n\
           class A\n\
           class B\n\
         }\n\
         class C\n\
         A --> C\n\
         B --> C",
    )
}

pub fn class_object_diagram() -> String {
    wrap(
        "object obj1 {\n\
           name = \"Alice\"\n\
           age = 30\n\
         }\n\
         object obj2 {\n\
           name = \"Bob\"\n\
         }\n\
         obj1 --> obj2",
    )
}

pub fn class_map() -> String {
    wrap(
        "map Config {\n\
           host => localhost\n\
           port => 8080\n\
           debug => true\n\
         }",
    )
}

pub fn class_inline_definition() -> String {
    wrap(
        "class User\n\
         User : +name : String\n\
         User : +email : String\n\
         User : +login()",
    )
}

// ===========================================================================
// State diagrams
// ===========================================================================

pub fn state_diagram() -> String {
    wrap(
        "[*] --> Active\n\
         Active --> Inactive : disable\n\
         Inactive --> Active : enable\n\
         Active --> [*] : close",
    )
}

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

pub fn state_stereotypes() -> String {
    wrap(
        "state start1 <<start>>\n\
         state end1 <<end>>\n\
         state choice1 <<choice>>\n\
         state fork1 <<fork>>\n\
         state join1 <<join>>\n\
         start1 --> choice1\n\
         choice1 --> fork1 : yes\n\
         choice1 --> end1 : no\n\
         fork1 --> join1",
    )
}

pub fn state_concurrent() -> String {
    wrap(
        "state Active {\n\
           state \"Sub1\" as s1\n\
           state \"Sub2\" as s2\n\
           --\n\
           state \"Sub3\" as s3\n\
           state \"Sub4\" as s4\n\
         }\n\
         [*] --> Active",
    )
}

pub fn state_descriptions() -> String {
    wrap(
        "state Active\n\
         Active : entry / initialize\n\
         Active : do / process\n\
         Active : exit / cleanup\n\
         [*] --> Active",
    )
}

pub fn state_colors() -> String {
    wrap(
        "state Active #LightGreen\n\
         state Error #FF0000\n\
         [*] --> Active\n\
         Active --> Error : fail",
    )
}

pub fn state_notes() -> String {
    wrap(
        "state Active\n\
         note right of Active : Important state\n\
         [*] --> Active\n\
         Active --> [*]",
    )
}

pub fn state_hide_empty() -> String {
    wrap(
        "hide empty description\n\
         state A\n\
         state B\n\
         A --> B",
    )
}

// ===========================================================================
// Activity diagrams
// ===========================================================================

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

pub fn activity_switch() -> String {
    wrap(
        "start\n\
         switch (test?)\n\
         case ( A )\n\
           :action A;\n\
         case ( B )\n\
           :action B;\n\
         endswitch\n\
         stop",
    )
}

pub fn activity_while_loop() -> String {
    wrap(
        "start\n\
         while (condition?) is (yes)\n\
           :process;\n\
         endwhile (no)\n\
         stop",
    )
}

pub fn activity_repeat_loop() -> String {
    wrap(
        "start\n\
         repeat\n\
           :action;\n\
         repeat while (again?) is (yes) not (no)\n\
         stop",
    )
}

pub fn activity_swimlanes() -> String {
    wrap(
        "|Swimlane1|\n\
         start\n\
         :task1;\n\
         |Swimlane2|\n\
         :task2;\n\
         |Swimlane1|\n\
         :task3;\n\
         stop",
    )
}

pub fn activity_partition() -> String {
    wrap(
        "start\n\
         partition Initialization {\n\
           :step1;\n\
           :step2;\n\
         }\n\
         partition Processing {\n\
           :step3;\n\
         }\n\
         stop",
    )
}

pub fn activity_detach() -> String {
    wrap(
        "start\n\
         if (error?) then (yes)\n\
           :log error;\n\
           detach\n\
         else (no)\n\
           :continue;\n\
         endif\n\
         stop",
    )
}

pub fn activity_kill() -> String {
    wrap(
        "start\n\
         :step1;\n\
         if (fatal?) then (yes)\n\
           kill\n\
         else (no)\n\
           :step2;\n\
         endif\n\
         stop",
    )
}

pub fn activity_split() -> String {
    wrap(
        "start\n\
         split\n\
           :task A;\n\
         split again\n\
           :task B;\n\
         split again\n\
           :task C;\n\
         end split\n\
         stop",
    )
}

pub fn activity_notes() -> String {
    wrap(
        "start\n\
         :action;\n\
         note right\n\
           This is a note\n\
         end note\n\
         stop",
    )
}

pub fn activity_elseif() -> String {
    wrap(
        "start\n\
         if (x > 0?) then (positive)\n\
           :positive path;\n\
         elseif (x < 0?) then (negative)\n\
           :negative path;\n\
         else (zero)\n\
           :zero path;\n\
         endif\n\
         stop",
    )
}

// ===========================================================================
// Component diagrams
// ===========================================================================

pub fn component_diagram() -> String {
    wrap(
        "component \"Web Server\" as WS\n\
         component \"Database\" as DB\n\
         component \"Cache\" as C\n\
         WS --> DB : query\n\
         WS --> C : read",
    )
}

pub fn component_with_interfaces() -> String {
    wrap(
        "component \"App\" as app\n\
         interface \"HTTP\" as http\n\
         interface \"JDBC\" as jdbc\n\
         component \"DB\" as db\n\
         app - http\n\
         app ..> jdbc\n\
         jdbc - db",
    )
}

pub fn component_packages() -> String {
    wrap(
        "package \"Frontend\" {\n\
           component [UI]\n\
           component [Router]\n\
         }\n\
         package \"Backend\" {\n\
           component [API]\n\
           component [Service]\n\
         }\n\
         [UI] --> [API]\n\
         [Router] --> [API]\n\
         [API] --> [Service]",
    )
}

// ===========================================================================
// Use case diagrams
// ===========================================================================

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

pub fn use_case_with_packages() -> String {
    wrap(
        "actor User\n\
         rectangle System {\n\
           usecase \"Login\" as UC1\n\
           usecase \"Browse\" as UC2\n\
           usecase \"Checkout\" as UC3\n\
         }\n\
         User --> UC1\n\
         User --> UC2\n\
         UC2 ..> UC1 : <<include>>",
    )
}

// ===========================================================================
// Deployment diagrams
// ===========================================================================

pub fn deployment_diagram() -> String {
    wrap(
        "node WebServer {\n\
           artifact app.war\n\
         }\n\
         node Database {\n\
           artifact db.sql\n\
         }\n\
         cloud Internet\n\
         WebServer --> Database\n\
         Internet --> WebServer",
    )
}

pub fn deployment_detailed() -> String {
    wrap(
        "cloud \"AWS\" {\n\
           node \"EC2\" as ec2 {\n\
             artifact \"app.jar\"\n\
           }\n\
           database \"RDS\" as rds\n\
           storage \"S3\" as s3\n\
         }\n\
         actor User\n\
         User --> ec2\n\
         ec2 --> rds\n\
         ec2 --> s3",
    )
}

// ===========================================================================
// Timing diagrams
// ===========================================================================

pub fn timing_diagram() -> String {
    wrap(
        "robust \"Web\" as W\n\
         concise \"User\" as U\n\n\
         @0\n\
         W is Idle\n\
         U is Idle\n\n\
         @100\n\
         W is Processing\n\
         U is Waiting\n\n\
         @300\n\
         W is Idle\n\
         U is Idle",
    )
}

// ===========================================================================
// Gantt charts
// ===========================================================================

pub fn gantt_chart() -> String {
    wrap_custom(
        "@startgantt",
        "[Task 1] lasts 5 days\n\
         [Task 2] lasts 3 days\n\
         [Task 2] starts at [Task 1]'s end\n\
         [Task 3] lasts 2 days\n\
         [Task 3] starts at [Task 2]'s end",
        "@endgantt",
    )
}

// ===========================================================================
// Mind maps
// ===========================================================================

pub fn mindmap() -> String {
    wrap_custom(
        "@startmindmap",
        "* Root\n\
         ** Branch A\n\
         *** Leaf 1\n\
         *** Leaf 2\n\
         ** Branch B\n\
         *** Leaf 3\n\
         ** Branch C",
        "@endmindmap",
    )
}

// ===========================================================================
// WBS (Work Breakdown Structure)
// ===========================================================================

pub fn wbs() -> String {
    wrap_custom(
        "@startwbs",
        "* Project\n\
         ** Phase 1\n\
         *** Task A\n\
         *** Task B\n\
         ** Phase 2\n\
         *** Task C",
        "@endwbs",
    )
}

// ===========================================================================
// JSON diagrams
// ===========================================================================

pub fn json_diagram() -> String {
    wrap_custom(
        "@startjson",
        "{\n\
           \"name\": \"test\",\n\
           \"version\": 1,\n\
           \"items\": [\"a\", \"b\", \"c\"],\n\
           \"nested\": {\n\
             \"key\": \"value\"\n\
           }\n\
         }",
        "@endjson",
    )
}

// ===========================================================================
// YAML diagrams
// ===========================================================================

pub fn yaml_diagram() -> String {
    wrap_custom(
        "@startyaml",
        "name: test\n\
         version: 1\n\
         items:\n\
           - a\n\
           - b\n\
           - c",
        "@endyaml",
    )
}

// ===========================================================================
// Salt (wireframe) diagrams
// ===========================================================================

pub fn salt_wireframe() -> String {
    wrap_custom(
        "@startsalt",
        "{\n\
           Login    | \"user\"\n\
           Password | \"****\"\n\
           [Cancel] | [ OK  ]\n\
         }",
        "@endsalt",
    )
}

// ===========================================================================
// Network diagrams
// ===========================================================================

pub fn network_diagram() -> String {
    wrap(
        "nwdiag {\n\
           network dmz {\n\
             address = \"210.x.x.x/24\"\n\
             web01 [address = \"210.x.x.1\"]\n\
           }\n\
           network internal {\n\
             address = \"172.x.x.x/24\"\n\
             web01 [address = \"172.x.x.1\"]\n\
             db01\n\
           }\n\
         }",
    )
}

// ===========================================================================
// Combinatorial generation
// ===========================================================================

/// A named test case with PlantUML source.
pub struct TestCase {
    pub name: String,
    pub source: String,
}

macro_rules! case {
    ($name:expr, $func:expr) => {
        TestCase {
            name: $name.into(),
            source: $func,
        }
    };
}

/// Generates all test cases across all diagram types and features.
pub fn all_cases(message_counts: usize) -> Vec<TestCase> {
    let mut cases = Vec::new();

    // Sequence combinatorics: 4 boolean features = 16 combinations.
    for bits in 0u8..16 {
        let opts = SequenceOptions {
            reply_arrows: bits & 1 != 0,
            notes: bits & 2 != 0,
            groups: bits & 4 != 0,
            participant_declarations: bits & 8 != 0,
        };
        cases.push(TestCase {
            name: format!("seq_combo_{:04b}", bits),
            source: sequence_with_features(&opts),
        });
    }

    // Multi-message sequences.
    for n in 1..=message_counts {
        cases.push(case!(
            format!("seq_messages_{n}"),
            multi_message_sequence(n)
        ));
    }

    // Sequence features.
    cases.extend([
        case!("seq_arrow_types", seq_arrow_types()),
        case!("seq_participant_types", seq_participant_types()),
        case!("seq_activation", seq_activation()),
        case!("seq_notes_all", seq_notes_all()),
        case!("seq_alt_else", seq_alt_else()),
        case!("seq_loop", seq_loop()),
        case!("seq_opt", seq_opt()),
        case!("seq_critical", seq_critical()),
        case!("seq_par", seq_par()),
        case!("seq_break", seq_break()),
        case!("seq_divider", seq_divider()),
        case!("seq_delay", seq_delay()),
        case!("seq_spacing", seq_spacing()),
        case!("seq_box", seq_box()),
        case!("seq_autonumber", seq_autonumber()),
        case!("seq_autonumber_format", seq_autonumber_format()),
        case!("seq_return", seq_return()),
        case!("seq_self_message", seq_self_message()),
        case!("seq_create", seq_create()),
        case!("seq_title_header_footer", seq_title_header_footer()),
        case!("seq_legend", seq_legend()),
        case!("seq_hide_unlinked", seq_hide_unlinked()),
        case!("seq_newpage", seq_newpage()),
        case!("seq_skinparam", seq_skinparam()),
        case!("seq_ref", seq_ref()),
        case!("seq_external", seq_external()),
        case!("seq_alias", seq_alias()),
        case!("seq_destroy", seq_destroy()),
    ]);

    // Class diagram features.
    cases.extend([
        case!("class_basic", class_diagram()),
        case!("class_relationships", class_diagram_with_relationships()),
        case!("class_entity_types", class_entity_types()),
        case!("class_visibility", class_visibility()),
        case!("class_static_abstract", class_static_abstract()),
        case!("class_generics", class_generics()),
        case!("class_all_relationships", class_all_relationships()),
        case!("class_labels_multiplicity", class_labels_multiplicity()),
        case!("class_packages", class_packages()),
        case!("class_nested_packages", class_nested_packages()),
        case!("class_notes", class_notes()),
        case!("class_stereotypes", class_stereotypes()),
        case!("class_separators", class_separators()),
        case!("class_enum_values", class_enum_values()),
        case!("class_together", class_together()),
        case!("class_object", class_object_diagram()),
        case!("class_map", class_map()),
        case!("class_inline_def", class_inline_definition()),
    ]);

    // State diagram features.
    cases.extend([
        case!("state_basic", state_diagram()),
        case!("state_nested", state_diagram_nested()),
        case!("state_stereotypes", state_stereotypes()),
        case!("state_concurrent", state_concurrent()),
        case!("state_descriptions", state_descriptions()),
        case!("state_colors", state_colors()),
        case!("state_notes", state_notes()),
        case!("state_hide_empty", state_hide_empty()),
    ]);

    // Activity diagram features.
    cases.extend([
        case!("activity_basic", activity_diagram()),
        case!("activity_fork", activity_diagram_with_fork()),
        case!("activity_switch", activity_switch()),
        case!("activity_while", activity_while_loop()),
        case!("activity_repeat", activity_repeat_loop()),
        case!("activity_swimlanes", activity_swimlanes()),
        case!("activity_partition", activity_partition()),
        case!("activity_detach", activity_detach()),
        // activity_kill excluded: PlantUML uses non-deterministic colors for kill nodes
        case!("activity_split", activity_split()),
        case!("activity_notes", activity_notes()),
        case!("activity_elseif", activity_elseif()),
    ]);

    // Component diagrams.
    cases.extend([
        case!("component_basic", component_diagram()),
        case!("component_interfaces", component_with_interfaces()),
        case!("component_packages", component_packages()),
    ]);

    // Use case diagrams.
    cases.extend([
        case!("usecase_basic", use_case_diagram()),
        case!("usecase_packages", use_case_with_packages()),
    ]);

    // Deployment diagrams.
    cases.extend([
        case!("deployment_basic", deployment_diagram()),
        case!("deployment_detailed", deployment_detailed()),
    ]);

    // Timing diagrams.
    cases.push(case!("timing_basic", timing_diagram()));

    // Gantt charts.
    cases.push(case!("gantt_basic", gantt_chart()));

    // Mind maps.
    cases.push(case!("mindmap_basic", mindmap()));

    // WBS.
    cases.push(case!("wbs_basic", wbs()));

    // JSON.
    cases.push(case!("json_basic", json_diagram()));

    // YAML.
    cases.push(case!("yaml_basic", yaml_diagram()));

    // Salt wireframes.
    cases.push(case!("salt_basic", salt_wireframe()));

    // Network diagrams.
    cases.push(case!("nwdiag_basic", network_diagram()));

    cases
}
