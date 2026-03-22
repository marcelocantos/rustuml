// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Class diagram feature matrix.

use super::features::{StaticFeatureSet, variant};
use super::{FeatureSet, FeatureVariant, MatrixCase};

/// Entity types — different kinds of class-like declarations.
pub fn entity_types() -> StaticFeatureSet {
    StaticFeatureSet {
        dimension: "entity_type",
        variants: vec![
            variant("class", "class", &["entity", "entity:class"], &[]),
            variant(
                "abstract_class",
                "abstract class",
                &["entity", "entity:abstract"],
                &[],
            ),
            variant(
                "interface",
                "interface",
                &["entity", "entity:interface"],
                &[],
            ),
            variant("enum", "enum", &["entity", "entity:enum"], &[]),
            variant(
                "annotation",
                "annotation",
                &["entity", "entity:annotation"],
                &[],
            ),
        ],
    }
}

/// Relationship types between entities.
pub fn relationship_types() -> StaticFeatureSet {
    StaticFeatureSet {
        dimension: "relationship",
        variants: vec![
            variant(
                "inheritance",
                "<|--",
                &["relationship", "relationship:inheritance"],
                &[],
            ),
            variant(
                "implementation",
                "..|>",
                &["relationship", "relationship:implementation"],
                &[],
            ),
            variant(
                "composition",
                "*--",
                &["relationship", "relationship:composition"],
                &[],
            ),
            variant(
                "aggregation",
                "o--",
                &["relationship", "relationship:aggregation"],
                &[],
            ),
            variant(
                "association",
                "--",
                &["relationship", "relationship:association"],
                &[],
            ),
            variant(
                "dependency",
                "..>",
                &["relationship", "relationship:dependency"],
                &[],
            ),
        ],
    }
}

/// Member visibility variants.
pub fn member_visibility() -> StaticFeatureSet {
    StaticFeatureSet {
        dimension: "visibility",
        variants: vec![
            variant("public", "+", &["visibility", "visibility:public"], &[]),
            variant("private", "-", &["visibility", "visibility:private"], &[]),
            variant(
                "protected",
                "#",
                &["visibility", "visibility:protected"],
                &[],
            ),
            variant("package", "~", &["visibility", "visibility:package"], &[]),
            variant("default", "", &["visibility", "visibility:default"], &[]),
        ],
    }
}

/// Member types — fields vs methods with modifiers.
pub fn member_types() -> StaticFeatureSet {
    StaticFeatureSet {
        dimension: "member_type",
        variants: vec![
            variant("field", "name : String", &["member", "member:field"], &[]),
            variant("method", "doSomething()", &["member", "member:method"], &[]),
            variant(
                "typed_method",
                "getName() : String",
                &["member", "member:typed_method"],
                &[],
            ),
            variant(
                "static_field",
                "{static} count : int",
                &["member", "member:static"],
                &[],
            ),
            variant(
                "abstract_method",
                "{abstract} process()",
                &["member", "member:abstract"],
                &[],
            ),
        ],
    }
}

/// Assemble a class diagram from entity_type × relationship × visibility × member_type.
pub fn assemble(variants: &[&FeatureVariant]) -> Option<MatrixCase> {
    if variants.len() < 4 {
        return None;
    }

    let entity = &variants[0];
    let rel = &variants[1];
    let vis = &variants[2];
    let member = &variants[3];

    let member_line = format!("  {}{}", vis.syntax, member.syntax);
    let source = format!(
        "@startuml\n\
         {} Foo {{\n\
         {member_line}\n\
         }}\n\
         class Bar\n\
         Foo {} Bar\n\
         @enduml\n",
        entity.syntax, rel.syntax,
    );

    let mut tags = Vec::new();
    let mut expected = vec!["Foo".to_string(), "Bar".to_string()];
    for v in variants {
        tags.extend_from_slice(&v.tags);
        expected.extend(v.expected_texts.iter().cloned());
    }

    let name = format!(
        "{}_{}_{}_{}_2e",
        entity.name, rel.name, vis.name, member.name,
    );

    Some(MatrixCase {
        name,
        source,
        tags,
        expected_texts: expected,
    })
}

/// Quick class matrix: entity_type × relationship (30 cases).
pub fn quick_cases() -> Vec<MatrixCase> {
    let entities = entity_types();
    let rels = relationship_types();

    let default_vis = variant("public", "+", &["visibility:public"], &[]);
    let default_member = variant("field", "name : String", &["member:field"], &[]);

    let mut cases = Vec::new();
    for e in entities.variants() {
        for r in rels.variants() {
            if let Some(case) = assemble(&[&e, &r, &default_vis, &default_member]) {
                cases.push(case);
            }
        }
    }
    cases
}

/// Medium class matrix: entity × visibility × member + relationship × member.
/// ~80 cases covering cross-cutting dimensions.
pub fn medium_cases() -> Vec<MatrixCase> {
    let entities = entity_types();
    let vis = member_visibility();
    let members = member_types();
    let rels = relationship_types();
    let default_rel = variant("association", "--", &["relationship:association"], &[]);
    let default_vis = variant("public", "+", &["visibility:public"], &[]);
    let default_member = variant("field", "name : String", &["member:field"], &[]);

    let mut cases = Vec::new();

    // Entity × visibility (5 × 5 = 25).
    for e in entities.variants() {
        for v in vis.variants() {
            if let Some(case) = assemble(&[&e, &default_rel, &v, &default_member]) {
                cases.push(case);
            }
        }
    }

    // Entity × member_type (5 × 5 = 25).
    for e in entities.variants() {
        for m in members.variants() {
            if let Some(case) = assemble(&[&e, &default_rel, &default_vis, &m]) {
                cases.push(case);
            }
        }
    }

    // Relationship × member_type (6 × 5 = 30).
    for r in rels.variants() {
        for m in members.variants() {
            if let Some(case) = assemble(&[
                &variant("class", "class", &["entity:class"], &[]),
                &r,
                &default_vis,
                &m,
            ]) {
                cases.push(case);
            }
        }
    }

    cases
}

/// Class diagram edge cases.
pub fn edge_cases() -> Vec<MatrixCase> {
    vec![
        MatrixCase {
            name: "class/edge/empty_class".into(),
            source: "@startuml\nclass Empty\n@enduml\n".into(),
            tags: vec!["edge", "edge:empty_class"],
            expected_texts: vec!["Empty".into()],
        },
        MatrixCase {
            name: "class/edge/many_members".into(),
            source: "@startuml\nclass Big {\n  +a : int\n  +b : int\n  +c : int\n  +d : int\n  +e : int\n  +f : int\n  +g : int\n  +h : int\n}\n@enduml\n".into(),
            tags: vec!["edge", "edge:many_members"],
            expected_texts: vec!["Big".into()],
        },
        MatrixCase {
            name: "class/edge/self_relationship".into(),
            source: "@startuml\nclass Node\nNode --> Node : parent\n@enduml\n".into(),
            tags: vec!["edge", "edge:self_rel"],
            expected_texts: vec!["Node".into()],
        },
        MatrixCase {
            name: "class/edge/diamond_inheritance".into(),
            source: "@startuml\ninterface A\nclass B\nclass C\nclass D\nA <|.. B\nA <|.. C\nB <|-- D\nC <|-- D\n@enduml\n".into(),
            tags: vec!["edge", "edge:diamond"],
            expected_texts: vec!["A".into(), "D".into()],
        },
        MatrixCase {
            name: "class/edge/generics".into(),
            source: "@startuml\nclass Container<T>\nclass Map<K, V>\nContainer -> Map\n@enduml\n".into(),
            tags: vec!["edge", "edge:generics"],
            expected_texts: vec!["Container".into(), "Map".into()],
        },
        MatrixCase {
            name: "class/edge/stereotypes".into(),
            source: "@startuml\nclass Foo <<singleton>>\nclass Bar <<service>>\nFoo --> Bar\n@enduml\n".into(),
            tags: vec!["edge", "edge:stereotypes"],
            expected_texts: vec!["Foo".into(), "Bar".into()],
        },
        MatrixCase {
            name: "class/edge/enum_values".into(),
            source: "@startuml\nenum Color {\n  RED\n  GREEN\n  BLUE\n  ALPHA\n}\n@enduml\n".into(),
            tags: vec!["edge", "edge:enum"],
            expected_texts: vec!["Color".into(), "RED".into()],
        },
        MatrixCase {
            name: "class/edge/package_nesting".into(),
            source: "@startuml\npackage com.example {\n  package inner {\n    class Foo\n  }\n  class Bar\n}\nFoo --> Bar\n@enduml\n".into(),
            tags: vec!["edge", "edge:packages"],
            expected_texts: vec!["Foo".into(), "Bar".into()],
        },
        MatrixCase {
            name: "class/edge/multiplicity".into(),
            source: "@startuml\nclass Parent\nclass Child\nParent \"1\" -- \"0..*\" Child : has\n@enduml\n".into(),
            tags: vec!["edge", "edge:multiplicity"],
            expected_texts: vec!["Parent".into(), "Child".into()],
        },
        MatrixCase {
            name: "class/edge/object_diagram".into(),
            source: "@startuml\nobject alice {\n  name = \"Alice\"\n  age = 30\n}\nobject bob {\n  name = \"Bob\"\n}\nalice --> bob\n@enduml\n".into(),
            tags: vec!["edge", "edge:object"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "class/edge/many_classes".into(),
            source: {
                let mut s = String::from("@startuml\n");
                for i in 0..12 {
                    s.push_str(&format!("class C{i}\n"));
                }
                for i in 0..11 {
                    s.push_str(&format!("C{i} --> C{}\n", i + 1));
                }
                s.push_str("@enduml\n");
                s
            },
            tags: vec!["edge", "edge:many_classes"],
            expected_texts: vec!["C0".into(), "C11".into()],
        },
    ]
}
