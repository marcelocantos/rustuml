// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Class diagram model.

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// Position of a note relative to its target entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotePosition {
    Top,
    Bottom,
    Left,
    Right,
}

/// A note attached to an entity or floating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// Note content lines (may contain Creole/HTML markup).
    pub lines: Vec<String>,
    /// Entity this note is attached to, if any.
    pub target: Option<String>,
    /// Position relative to target entity.
    pub position: Option<NotePosition>,
    /// Named note alias (for `note "..." as N`).
    pub alias: Option<String>,
    /// Optional note background color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// 1-based source line (first content line for multiline notes).
    #[serde(default)]
    pub source_line: usize,
}

/// A complete class diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDiagram {
    pub meta: DiagramMeta,
    #[serde(default)]
    pub direction: ClassLayoutDirection,
    pub entities: Vec<ClassEntity>,
    pub relationships: Vec<Relationship>,
    /// Association classes declared via `(A, B) .. C` syntax.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub association_classes: Vec<AssociationClass>,
    pub packages: Vec<Package>,
    pub notes: Vec<Note>,
    /// Visibility-control directives accumulated from `hide ...` / `show ...`
    /// statements. Stored as the raw argument list after the keyword so the
    /// renderer can interpret per-entity selectors as well as global ones.
    #[serde(default)]
    pub hide_show: Vec<HideShow>,
    /// 1-based source line of the `header`/`footer`/`title`/`caption`/`legend`
    /// declaration, when present. Used to populate `data-source-line` on the
    /// `<g class="header">`-style decoration wrappers PlantUML emits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer_line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_line: Option<usize>,
}

/// Dot/SVEK rank direction selected by the diagram direction command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ClassLayoutDirection {
    #[default]
    TopToBottom,
    LeftToRight,
}

/// One `hide`/`show` directive (verbatim arguments).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HideShow {
    pub show: bool,
    /// `true` when the directive came from `remove` rather than `hide`. For
    /// whole-entity suppression `remove` and `hide` behave identically in the
    /// renderer; the flag is kept for fidelity.
    #[serde(default)]
    pub remove: bool,
    /// Lower-cased space-collapsed argument text (e.g. `"circle"`, `"empty members"`,
    /// `"<<myStereo>> circle"`, `"myClass attributes"`).
    pub arg: String,
}

/// A class, interface, enum, or other entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassEntity {
    pub id: String,
    pub label: String,
    pub kind: EntityKind,
    pub members: Vec<Member>,
    pub stereotypes: Vec<String>,
    /// Generic type parameter(s) declared with `<...>` after the entity name,
    /// e.g. `class Foo<T>` → `Some("T")`, `class Foo<K, V>` → `Some("K, V")`.
    /// Rendered as a dashed box at the entity's top-right corner; never part of
    /// the entity id, label, or qualified name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Optional tooltip from `[[url{tooltip}]]` syntax. When present it
    /// becomes the link anchor's `title`/`xlink:title`; otherwise the URL is
    /// used. A plain ` label` after the URL does not populate this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_tooltip: Option<String>,
    /// Optional hex spot color from `<< (X,#HEX) Name >>` notation, applied to
    /// the stereotype circle fill. Only hex colors override the fill; named
    /// colors are ignored for the circle (PlantUML behavior).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spot_color: Option<String>,
    /// Circled character from a valid `<< (X,#RRGGBB) Name >>` stereotype.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spot_character: Option<char>,
    /// Optional background color (e.g., "#lightblue", "#FF0000").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Optional text colour from `text:colour` shorthand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
    /// Optional border colour from `line:colour` shorthand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_color: Option<String>,
    /// Optional border stroke from `line.bold`, `line.dashed`, or `line.dotted`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_style: Option<EntityLineStyle>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityLineStyle {
    Bold,
    Dashed,
    Dotted,
}

/// The kind of entity in a class diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EntityKind {
    #[default]
    Class,
    AbstractClass,
    Interface,
    Enum,
    Annotation,
    Entity,
    Object,
    State,
    Circle,
    Diamond,
}

/// A field or method in a class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub name: String,
    pub return_type: Option<String>,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_abstract: bool,
    pub kind: MemberKind,
    /// Verbatim display text (after stripping visibility prefix and modifiers).
    /// Preserves original colon spacing, e.g. "field: String" or "field : String".
    pub display_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Visibility {
    #[default]
    Default,
    Public,
    Private,
    Protected,
    Package,
    /// IE (entity-relationship) mandatory column, denoted by the `*` prefix
    /// in PlantUML's entity-syntax diagrams.
    IeMandatory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberKind {
    Field,
    Method,
    /// A labeled separator line within a class body (e.g. `-- Section --`, `== Title ==`).
    Separator,
}

/// An association class: `(A, B) .. C` (or `(A, B) -- C`). PlantUML synthesizes
/// a tiny anchor point (`apoint`) on the A–B association line and draws a dashed
/// (`..`) or solid (`--`) connector from it to the association class `C`. The
/// apoint's id, position, and the three connector geometries are produced by
/// Java's layout engine and surfaced via the oracle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociationClass {
    /// First endpoint of the association line.
    pub a: String,
    /// Second endpoint of the association line.
    pub b: String,
    /// The association class hanging off the apoint.
    pub c: String,
    /// Whether the apoint→C connector is dashed (`..`) rather than solid (`--`).
    #[serde(default = "default_true")]
    pub dashed: bool,
    /// 1-based source line within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

fn default_true() -> bool {
    true
}

/// A relationship (association, inheritance, etc.) between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub kind: RelationshipKind,
    pub label: Option<String>,
    /// Directional marker embedded in the relationship label (`< label` or
    /// `label >`), matching PlantUML's `StringWithArrow` / `LinkArrow` model.
    #[serde(default)]
    pub label_arrow: LinkArrow,
    pub from_multiplicity: Option<String>,
    pub to_multiplicity: Option<String>,
    /// Crow's-foot / IE endpoint decorations, as parsed from PlantUML's
    /// `LinkDecor` tokens by `CommandLinkElement.getLinkType`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_decor: Option<EndpointDecor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_decor: Option<EndpointDecor>,
    /// Endpoint that carries PlantUML's built-in relationship decoration
    /// (arrowhead, inheritance triangle, composition diamond, etc.).
    #[serde(default)]
    pub decorated_end: RelationshipEnd,
    /// Whether the line is dashed (e.g. `..>` vs `-->`).
    #[serde(default)]
    pub dashed: bool,
    /// Number of line characters in the arrow body. PlantUML
    /// `CommandLinkClass.getQueueLength` stores this on `LinkArg`; length one
    /// is later emitted by `SvekEdge.rankSame` as a horizontal link.
    #[serde(default = "default_relationship_length")]
    pub length: usize,
    /// Visual modifiers attached to the arrow body (`-[#blue,dashed]`,
    /// `-[thickness=3]`, or `-[hidden]`).
    ///
    /// PlantUML parses these in `CommandLinkClass.executeArg`, then delegates
    /// to `WithLinkType.applyStyle` before `SvekEdge.drawU` resolves the
    /// effective stroke and color.
    #[serde(default, skip_serializing_if = "RelationshipStyle::is_default")]
    pub style: RelationshipStyle,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

fn default_relationship_length() -> usize {
    2
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_style: Option<EntityLineStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thickness: Option<u32>,
    #[serde(default)]
    pub hidden: bool,
}

impl RelationshipStyle {
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipKind {
    Inheritance,
    Implementation,
    Composition,
    Aggregation,
    Association,
    Dependency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RelationshipEnd {
    #[default]
    None,
    From,
    To,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LinkArrow {
    #[default]
    None,
    Direct,
    Backward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointDecor {
    CrowFoot,
    CircleCrowFoot,
    CircleLine,
    DoubleLine,
    LineCrowFoot,
}

/// Container type for grouping entities in a class diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PackageKind {
    #[default]
    Package,
    Namespace,
    Cloud,
    Database,
    Folder,
    Frame,
    Rectangle,
    Node,
}

/// A package/namespace/container grouping entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub kind: PackageKind,
    /// Optional background color (CSS name or hex without leading `#`).
    pub color: Option<String>,
    pub entities: Vec<String>,
    /// Parent package index, when this package was declared inside another
    /// package/namespace block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<usize>,
    /// 1-based source line within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
    /// Stereotypes applied to this package (e.g. `<<Application>>`).
    #[serde(default)]
    pub stereotypes: Vec<String>,
    /// Display label override (used for auto-created namespace packages where `name`
    /// is the full qualified path but we only want to show the short last segment).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}
