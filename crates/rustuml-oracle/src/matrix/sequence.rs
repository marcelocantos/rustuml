// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagram feature matrix.

use super::features::{StaticFeatureSet, variant};
use super::{FeatureSet, FeatureVariant, MatrixCase};

/// Arrow styles — the different ways to draw message arrows.
pub fn arrow_styles() -> StaticFeatureSet {
    StaticFeatureSet {
        dimension: "arrow_style",
        variants: vec![
            variant("solid", "->", &["arrow", "arrow:solid"], &[]),
            variant("dotted", "-->", &["arrow", "arrow:dotted"], &[]),
            variant("open", "->>", &["arrow", "arrow:open"], &[]),
            variant("open_dotted", "-->>", &["arrow", "arrow:open_dotted"], &[]),
            variant("lost", "->x", &["arrow", "arrow:lost"], &[]),
            variant("circle_end", "->o", &["arrow", "arrow:circle"], &[]),
            variant("circle_start", "o->", &["arrow", "arrow:circle"], &[]),
            variant(
                "bidirectional",
                "<->",
                &["arrow", "arrow:bidirectional"],
                &[],
            ),
        ],
    }
}

/// Participant types — the visual shape of lifelines.
pub fn participant_types() -> StaticFeatureSet {
    StaticFeatureSet {
        dimension: "participant_type",
        variants: vec![
            variant(
                "participant",
                "participant",
                &["participant", "participant:default"],
                &[],
            ),
            variant("actor", "actor", &["participant", "participant:actor"], &[]),
            variant(
                "boundary",
                "boundary",
                &["participant", "participant:boundary"],
                &[],
            ),
            variant(
                "control",
                "control",
                &["participant", "participant:control"],
                &[],
            ),
            variant(
                "entity",
                "entity",
                &["participant", "participant:entity"],
                &[],
            ),
            variant(
                "database",
                "database",
                &["participant", "participant:database"],
                &[],
            ),
            variant(
                "collections",
                "collections",
                &["participant", "participant:collections"],
                &[],
            ),
            variant("queue", "queue", &["participant", "participant:queue"], &[]),
        ],
    }
}

/// Message labels — different content in message text.
pub fn message_labels() -> StaticFeatureSet {
    StaticFeatureSet {
        dimension: "message_label",
        variants: vec![
            variant("simple", "hello", &["label", "label:simple"], &["hello"]),
            variant(
                "spaces",
                "hello world",
                &["label", "label:spaces"],
                &["hello world"],
            ),
            variant("empty", "", &["label", "label:empty"], &[]),
            variant("numeric", "42", &["label", "label:numeric"], &["42"]),
            variant("special_chars", "a/b/c", &["label", "label:special"], &[]),
        ],
    }
}

/// Decorations — optional features added alongside a basic message.
pub fn decorations() -> StaticFeatureSet {
    StaticFeatureSet {
        dimension: "decoration",
        variants: vec![
            variant("none", "", &["decoration:none"], &[]),
            variant(
                "note_right",
                "note right : A note",
                &["decoration", "note", "note:right"],
                &["A note"],
            ),
            variant(
                "note_left",
                "note left : Left note",
                &["decoration", "note", "note:left"],
                &["Left note"],
            ),
            variant(
                "note_over",
                "note over {FROM} : Over note",
                &["decoration", "note", "note:over"],
                &["Over note"],
            ),
            variant(
                "divider",
                "== Section ==",
                &["decoration", "divider"],
                &["Section"],
            ),
            variant(
                "delay",
                "...waiting...",
                &["decoration", "delay"],
                &["waiting"],
            ),
            variant("space", "|||", &["decoration", "space"], &[]),
            variant("space_px", "||30||", &["decoration", "space:sized"], &[]),
        ],
    }
}

/// Grouping constructs wrapping a message.
pub fn groupings() -> StaticFeatureSet {
    StaticFeatureSet {
        dimension: "grouping",
        variants: vec![
            variant("none", "", &["grouping:none"], &[]),
            variant(
                "alt",
                "alt Success\n{MSG}\nelse Failure\n{FROM} --> {TO} : error\nend",
                &["grouping", "grouping:alt"],
                &[],
            ),
            variant(
                "opt",
                "opt Condition\n{MSG}\nend",
                &["grouping", "grouping:opt"],
                &[],
            ),
            variant(
                "loop",
                "loop 3 times\n{MSG}\nend",
                &["grouping", "grouping:loop"],
                &[],
            ),
            variant(
                "par",
                "par Branch1\n{MSG}\nelse Branch2\n{FROM} --> {TO} : alt\nend",
                &["grouping", "grouping:par"],
                &[],
            ),
            variant(
                "critical",
                "critical Section\n{MSG}\nend",
                &["grouping", "grouping:critical"],
                &[],
            ),
            variant(
                "group",
                "group Custom\n{MSG}\nend",
                &["grouping", "grouping:group"],
                &[],
            ),
        ],
    }
}

/// Activation patterns — lifeline activation/deactivation.
pub fn activations() -> StaticFeatureSet {
    StaticFeatureSet {
        dimension: "activation",
        variants: vec![
            variant("none", "", &["activation:none"], &[]),
            variant("activate", "++", &["activation", "activation:on"], &[]),
            variant("deactivate", "--", &["activation", "activation:off"], &[]),
            variant("destroy", "!!", &["activation", "activation:destroy"], &[]),
        ],
    }
}

/// Participant counts — how many lifelines in the diagram.
pub fn participant_counts() -> StaticFeatureSet {
    StaticFeatureSet {
        dimension: "participant_count",
        variants: vec![
            variant("two", "2", &["count:2"], &[]),
            variant("three", "3", &["count:3"], &[]),
            variant("five", "5", &["count:5"], &[]),
        ],
    }
}

/// Assemble a sequence diagram from selected feature variants.
///
/// The variants array maps to these dimensions (in order):
/// [arrow_style, from_type, to_type, label, decoration, grouping, activation, count]
///
/// Use with `generate_matrix` for partial combinations, or call
/// directly for specific assemblies.
pub fn assemble(variants: &[&FeatureVariant]) -> Option<MatrixCase> {
    if variants.len() < 6 {
        return None;
    }

    let arrow = &variants[0]; // arrow_style
    let from_type = &variants[1]; // participant_type (from)
    let label = &variants[2]; // message_label
    let decoration = &variants[3]; // decoration
    let grouping = &variants[4]; // grouping
    let activation = &variants[5]; // activation

    let from_name = "Alice";
    let to_name = "Bob";
    let from_decl = format!("{} {from_name}", from_type.syntax);
    let to_decl = format!("participant {to_name}");

    // Build the core message line.
    let activation_suffix = if activation.syntax.is_empty() {
        String::new()
    } else {
        format!(" {}", activation.syntax)
    };
    let label_suffix = if label.syntax.is_empty() {
        String::new()
    } else {
        format!(" : {}", label.syntax)
    };
    let msg_line = format!(
        "{from_name} {} {to_name}{activation_suffix}{label_suffix}",
        arrow.syntax,
    );

    // Build the body.
    let mut body = String::new();
    body.push_str(&format!("{from_decl}\n{to_decl}\n"));

    // Apply grouping if present.
    if grouping.syntax.is_empty() {
        body.push_str(&msg_line);
        body.push('\n');
    } else {
        let wrapped = grouping
            .syntax
            .replace("{MSG}", &msg_line)
            .replace("{FROM}", from_name)
            .replace("{TO}", to_name);
        body.push_str(&wrapped);
        body.push('\n');
    }

    // Apply decoration if present.
    if !decoration.syntax.is_empty() {
        let dec = decoration
            .syntax
            .replace("{FROM}", from_name)
            .replace("{TO}", to_name);
        body.push_str(&dec);
        body.push('\n');
    }

    let source = format!("@startuml\n{body}@enduml\n");

    // Collect tags and expected texts.
    let mut tags: Vec<&'static str> = Vec::new();
    let mut expected_texts: Vec<String> = vec![from_name.to_string(), to_name.to_string()];

    for v in variants {
        tags.extend_from_slice(&v.tags);
        expected_texts.extend(v.expected_texts.iter().cloned());
    }

    let name = format!(
        "{}_{}_{}_{}_{}_{}_{}",
        arrow.name,
        from_type.name,
        label.name,
        decoration.name,
        grouping.name,
        activation.name,
        "2p",
    );

    Some(MatrixCase {
        name,
        source,
        tags,
        expected_texts,
    })
}

/// Generate a focused subset: arrow × participant × label × decoration.
/// This is the "breadth" matrix — every feature exercised at least once.
pub fn breadth_cases() -> Vec<MatrixCase> {
    let arrows = arrow_styles();
    let ptypes = participant_types();
    let labels = message_labels();
    let decos = decorations();
    let groups = groupings();
    let acts = activations();

    super::generate_matrix(
        "seq",
        assemble,
        &[
            &arrows as &dyn super::FeatureSet,
            &ptypes,
            &labels,
            &decos,
            &groups,
            &acts,
        ],
    )
}

/// Generate a focused subset for quick CI: arrow × label only.
/// Keeps the most important dimensions, drops combinatorial explosion.
pub fn quick_cases() -> Vec<MatrixCase> {
    let arrows = arrow_styles();
    let labels = message_labels();

    // Use default for other dimensions.
    let default_ptype = variant("participant", "participant", &["participant:default"], &[]);
    let default_deco = variant("none", "", &["decoration:none"], &[]);
    let default_group = variant("none", "", &["grouping:none"], &[]);
    let default_act = variant("none", "", &["activation:none"], &[]);

    let mut cases = Vec::new();
    for a in arrows.variants() {
        for l in labels.variants() {
            if let Some(case) = assemble(&[
                &a,
                &default_ptype,
                &l,
                &default_deco,
                &default_group,
                &default_act,
            ]) {
                cases.push(case);
            }
        }
    }
    cases
}

/// Medium-sized matrix: arrow × decoration + arrow × grouping + participant × activation.
/// Covers the important cross-cutting feature interactions without full Cartesian explosion.
/// Produces ~200 cases.
pub fn medium_cases() -> Vec<MatrixCase> {
    let arrows = arrow_styles();
    let decos = decorations();
    let groups = groupings();
    let ptypes = participant_types();
    let acts = activations();
    let default_ptype = variant("participant", "participant", &["participant:default"], &[]);
    let default_label = variant("simple", "hello", &["label:simple"], &["hello"]);
    let default_deco = variant("none", "", &["decoration:none"], &[]);
    let default_group = variant("none", "", &["grouping:none"], &[]);
    let default_act = variant("none", "", &["activation:none"], &[]);

    let mut cases = Vec::new();

    // Arrow × decoration (8 × 8 = 64).
    for a in arrows.variants() {
        for d in decos.variants() {
            if let Some(case) = assemble(&[
                &a,
                &default_ptype,
                &default_label,
                &d,
                &default_group,
                &default_act,
            ]) {
                cases.push(case);
            }
        }
    }

    // Arrow × grouping (8 × 7 = 56).
    for a in arrows.variants() {
        for g in groups.variants() {
            if let Some(case) = assemble(&[
                &a,
                &default_ptype,
                &default_label,
                &default_deco,
                &g,
                &default_act,
            ]) {
                cases.push(case);
            }
        }
    }

    // Participant type × activation (8 × 4 = 32).
    for p in ptypes.variants() {
        for act in acts.variants() {
            if let Some(case) = assemble(&[
                &variant("solid", "->", &["arrow:solid"], &[]),
                &p,
                &default_label,
                &default_deco,
                &default_group,
                &act,
            ]) {
                cases.push(case);
            }
        }
    }

    // Decoration × grouping (8 × 7 = 56).
    for d in decos.variants() {
        for g in groups.variants() {
            if let Some(case) = assemble(&[
                &variant("solid", "->", &["arrow:solid"], &[]),
                &default_ptype,
                &default_label,
                &d,
                &g,
                &default_act,
            ]) {
                cases.push(case);
            }
        }
    }

    cases
}

/// Targeted edge cases that don't fit the matrix pattern.
pub fn edge_cases() -> Vec<MatrixCase> {
    vec![
        MatrixCase {
            name: "seq/edge/empty_diagram".into(),
            source: "@startuml\n@enduml\n".into(),
            tags: vec!["edge", "edge:empty"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "seq/edge/single_participant".into(),
            source: "@startuml\nparticipant Alice\n@enduml\n".into(),
            tags: vec!["edge", "edge:single_participant"],
            expected_texts: vec!["Alice".into()],
        },
        MatrixCase {
            name: "seq/edge/self_message".into(),
            source: "@startuml\nAlice -> Alice : self\n@enduml\n".into(),
            tags: vec!["edge", "edge:self_message"],
            expected_texts: vec!["Alice".into(), "self".into()],
        },
        MatrixCase {
            name: "seq/edge/many_participants".into(),
            source: "@startuml\nparticipant A\nparticipant B\nparticipant C\nparticipant D\nparticipant E\nparticipant F\nparticipant G\nparticipant H\nA -> H : across many\n@enduml\n".into(),
            tags: vec!["edge", "edge:many_participants"],
            expected_texts: vec!["across many".into()],
        },
        MatrixCase {
            name: "seq/edge/long_label".into(),
            source: "@startuml\nAlice -> Bob : This is a very long message label that should still render correctly in the diagram\n@enduml\n".into(),
            tags: vec!["edge", "edge:long_label"],
            expected_texts: vec!["This is a very long message".into()],
        },
        MatrixCase {
            name: "seq/edge/unicode_label".into(),
            source: "@startuml\nAlice -> Bob : héllo wörld 你好\n@enduml\n".into(),
            tags: vec!["edge", "edge:unicode"],
            expected_texts: vec![],  // Unicode rendering varies; just check it doesn't crash.
        },
        MatrixCase {
            name: "seq/edge/nested_groups".into(),
            source: "@startuml\nAlice -> Bob : start\nalt outer\n  alt inner\n    Bob --> Alice : deep\n  end\nend\n@enduml\n".into(),
            tags: vec!["edge", "edge:nested_groups", "grouping"],
            expected_texts: vec!["deep".into()],
        },
        MatrixCase {
            name: "seq/edge/many_messages".into(),
            source: {
                let mut s = String::from("@startuml\n");
                for i in 1..=20 {
                    s.push_str(&format!("Alice -> Bob : msg{i}\n"));
                }
                s.push_str("@enduml\n");
                s
            },
            tags: vec!["edge", "edge:many_messages"],
            expected_texts: vec!["msg1".into(), "msg10".into(), "msg20".into()],
        },
        MatrixCase {
            name: "seq/edge/autonumber_stop_resume".into(),
            source: "@startuml\nautonumber\nA -> B : first\nautonumber stop\nA -> B : unnumbered\nautonumber resume\nA -> B : numbered\n@enduml\n".into(),
            tags: vec!["edge", "autonumber"],
            expected_texts: vec!["first".into()],
        },
        MatrixCase {
            name: "seq/edge/mixed_arrows".into(),
            source: "@startuml\nA -> B : solid\nB --> A : dotted\nA ->> B : open\nB -->> A : open_dotted\nA ->x B : lost\nA ->o B : circle\n@enduml\n".into(),
            tags: vec!["edge", "edge:mixed_arrows"],
            expected_texts: vec!["solid".into(), "dotted".into()],
        },
        MatrixCase {
            name: "seq/edge/all_group_types".into(),
            source: "@startuml\nA -> B : s\nalt a\n  A -> B : 1\nend\nopt o\n  A -> B : 2\nend\nloop l\n  A -> B : 3\nend\ncritical c\n  A -> B : 4\nend\ngroup g\n  A -> B : 5\nend\n@enduml\n".into(),
            tags: vec!["edge", "edge:all_groups", "grouping"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "seq/edge/box_with_participants".into(),
            source: "@startuml\nbox \"System\" #LightBlue\nparticipant A\nparticipant B\nend box\nparticipant C\nA -> C : cross-box\n@enduml\n".into(),
            tags: vec!["edge", "box"],
            expected_texts: vec!["cross-box".into()],
        },
        MatrixCase {
            name: "seq/edge/ref_over".into(),
            source: "@startuml\nparticipant A\nparticipant B\nref over A, B : See other diagram\nA -> B : continue\n@enduml\n".into(),
            tags: vec!["edge", "ref"],
            expected_texts: vec!["See other diagram".into()],
        },
        MatrixCase {
            name: "seq/edge/create_and_destroy".into(),
            source: "@startuml\nAlice -> Bob : normal\ncreate Charlie\nAlice -> Charlie : create\nCharlie --> Alice : ack\ndestroy Charlie\n@enduml\n".into(),
            tags: vec!["edge", "create", "destroy"],
            expected_texts: vec!["normal".into(), "create".into()],
        },
        MatrixCase {
            name: "seq/edge/external_messages".into(),
            source: "@startuml\nparticipant A\n[-> A : from outside\nA ->] : to outside\n@enduml\n".into(),
            tags: vec!["edge", "external"],
            expected_texts: vec!["from outside".into(), "to outside".into()],
        },
        MatrixCase {
            name: "seq/edge/return_message".into(),
            source: "@startuml\nAlice -> Bob ++ : request\nreturn response\n@enduml\n".into(),
            tags: vec!["edge", "return", "activation"],
            expected_texts: vec!["request".into(), "response".into()],
        },
    ]
}
