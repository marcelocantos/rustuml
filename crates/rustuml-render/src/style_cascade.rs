// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Ordered sparse resolution for PlantUML style declarations.

use std::collections::BTreeMap;

use rustuml_parser::diagram::style::{StyleDeclaration, StyleOrigin, StyleProgram, StyleScheme};

/// The style signature requested by a renderer.
///
/// Selector tokens are set-like, matching PlantUML's `SName` signatures.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StyleSignature {
    selectors: Vec<String>,
    stereotypes: Vec<String>,
    depth: Option<u32>,
    star: bool,
}

impl StyleSignature {
    pub fn from_selectors<I, S>(selectors: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut signature = Self::default();
        for selector in selectors {
            let selector = normalize_selector(selector.as_ref());
            if !selector.is_empty() && !signature.selectors.contains(&selector) {
                signature.selectors.push(selector);
            }
        }
        signature
    }

    pub fn with_stereotype(mut self, stereotype: impl AsRef<str>) -> Self {
        let stereotype = normalize_stereotype(stereotype.as_ref());
        if !stereotype.is_empty() && !self.stereotypes.contains(&stereotype) {
            self.stereotypes.push(stereotype);
        }
        self
    }

    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = Some(depth);
        self
    }

    pub fn starred(mut self) -> Self {
        self.star = true;
        self
    }
}

/// A resolved sparse style. Missing properties remain observable as `None`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ResolvedStyle<'a> {
    declarations: BTreeMap<String, &'a StyleDeclaration>,
}

impl<'a> ResolvedStyle<'a> {
    pub fn property(&self, property: &str) -> Option<&'a str> {
        self.declaration(property)
            .map(|declaration| declaration.value.as_str())
    }

    pub fn declaration(&self, property: &str) -> Option<&'a StyleDeclaration> {
        self.declarations
            .get(&normalize_property(property))
            .copied()
    }

    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &'a str)> + '_ {
        self.declarations
            .iter()
            .map(|(property, declaration)| (property.as_str(), declaration.value.as_str()))
    }
}

/// Resolves one parser-owned style program without introducing renderer
/// defaults. The caller remains responsible for applying built-in defaults and
/// inline entity or link overrides.
#[derive(Debug, Clone, Copy)]
pub struct StyleCascade<'a> {
    program: &'a StyleProgram,
    legacy_skinparam_used: bool,
}

impl<'a> StyleCascade<'a> {
    pub fn new(program: &'a StyleProgram) -> Self {
        let legacy_skinparam_used = program.declarations.iter().any(|declaration| {
            matches!(
                &declaration.origin,
                StyleOrigin::UserSkinParam | StyleOrigin::ThemeSkinParam { .. }
            )
        });
        Self {
            program,
            legacy_skinparam_used,
        }
    }

    /// Resolve against the complete style program.
    pub fn resolve(&self, signature: &StyleSignature, scheme: StyleScheme) -> ResolvedStyle<'a> {
        self.resolve_bounded(signature, scheme, ResolutionBound::Final)
    }

    /// Resolve declarations whose source epoch is at or before `epoch_ceiling`.
    ///
    /// Java provenance: `StyleStorage#computeMergedStyle`,
    /// `StyleSignatureBasic#matchAll`, and `DarkString#mergeWith`.
    pub fn resolve_at(
        &self,
        signature: &StyleSignature,
        scheme: StyleScheme,
        epoch_ceiling: u64,
    ) -> ResolvedStyle<'a> {
        self.resolve_bounded(signature, scheme, ResolutionBound::Epoch(epoch_ceiling))
    }

    /// Resolve declarations visible at an element or link creation source line.
    ///
    /// Theme declarations carry the source line of their `!theme` directive,
    /// so the source-line bound preserves theme replay placement.
    pub fn resolve_at_source_line(
        &self,
        signature: &StyleSignature,
        scheme: StyleScheme,
        creation_source_line: usize,
    ) -> ResolvedStyle<'a> {
        self.resolve_bounded(
            signature,
            scheme,
            ResolutionBound::SourceLine(creation_source_line),
        )
    }

    /// Resolve an entity's creation snapshot, including Java's legacy
    /// skinparam compatibility refresh.
    ///
    /// Java provenance: `Entity#getCurrentStyleBuilder` returns the diagram's
    /// final builder after any legacy skinparam command; pure CSS entities
    /// retain the builder captured when they were created.
    pub fn resolve_entity_at_source_line(
        &self,
        signature: &StyleSignature,
        scheme: StyleScheme,
        creation_source_line: usize,
    ) -> ResolvedStyle<'a> {
        if self.legacy_skinparam_used {
            self.resolve(signature, scheme)
        } else {
            self.resolve_at_source_line(signature, scheme, creation_source_line)
        }
    }

    /// Resolve a link against the builder captured when the link was created.
    ///
    /// Java provenance: `Link#getStyleBuilder` always returns the captured
    /// builder, including when legacy skinparams refresh entity builders.
    pub fn resolve_link_at_source_line(
        &self,
        signature: &StyleSignature,
        scheme: StyleScheme,
        creation_source_line: usize,
    ) -> ResolvedStyle<'a> {
        self.resolve_at_source_line(signature, scheme, creation_source_line)
    }

    /// Whether a matching property has a declaration from `<style>` syntax,
    /// even if a later compatibility skinparam wins the resolved value.
    pub fn has_style_declaration(
        &self,
        signature: &StyleSignature,
        scheme: StyleScheme,
        property: &str,
    ) -> bool {
        let property = normalize_property(property);
        self.program.declarations.iter().any(|declaration| {
            declaration.scheme == scheme
                && normalize_property(&declaration.property) == property
                && matches!(
                    declaration.origin,
                    StyleOrigin::UserStyle | StyleOrigin::ThemeStyle { .. }
                )
                && declaration_matches(declaration, signature)
        })
    }

    fn resolve_bounded(
        &self,
        signature: &StyleSignature,
        scheme: StyleScheme,
        bound: ResolutionBound,
    ) -> ResolvedStyle<'a> {
        let mut regular = BTreeMap::<String, Winner<'a>>::new();
        let mut dark = BTreeMap::<String, Winner<'a>>::new();

        for (order, declaration) in self.program.declarations.iter().enumerate() {
            if !bound.includes(declaration) || !declaration_matches(declaration, signature) {
                continue;
            }

            let property = normalize_property(&declaration.property);
            if property.is_empty() {
                continue;
            }
            let candidate = Winner { declaration, order };
            let channel = match declaration.scheme {
                StyleScheme::Regular => &mut regular,
                StyleScheme::Dark => &mut dark,
            };

            // Java `ValueImpl#mergeWith` delegates to `DarkString#mergeWith`:
            // greater priority wins and an equal-priority later value replaces
            // the earlier value.
            let replaces = channel
                .get(&property)
                .is_none_or(|current| candidate.outranks(*current));
            if replaces {
                channel.insert(property, candidate);
            }
        }

        let mut declarations = regular
            .into_iter()
            .map(|(property, winner)| (property, winner.declaration))
            .collect::<BTreeMap<_, _>>();

        // Java `DarkString` retains regular and dark values independently. A
        // dark value is selected when available; otherwise dark rendering
        // inherits the regular value.
        if scheme == StyleScheme::Dark {
            for (property, winner) in dark {
                declarations.insert(property, winner.declaration);
            }
        }

        ResolvedStyle { declarations }
    }
}

#[derive(Debug, Clone, Copy)]
enum ResolutionBound {
    Final,
    Epoch(u64),
    SourceLine(usize),
}

impl ResolutionBound {
    fn includes(self, declaration: &StyleDeclaration) -> bool {
        match self {
            Self::Final => true,
            Self::Epoch(epoch_ceiling) => declaration.epoch <= epoch_ceiling,
            Self::SourceLine(source_line_ceiling) => declaration.source_line <= source_line_ceiling,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Winner<'a> {
    declaration: &'a StyleDeclaration,
    order: usize,
}

impl Winner<'_> {
    fn outranks(self, current: Self) -> bool {
        (
            self.declaration.priority,
            self.declaration.epoch,
            self.order,
        ) >= (
            current.declaration.priority,
            current.declaration.epoch,
            current.order,
        )
    }
}

fn declaration_matches(declaration: &StyleDeclaration, signature: &StyleSignature) -> bool {
    // Java provenance: `StyleSignatureBasic#matchAll`.
    if let Some(declaration_depth) = declaration.depth {
        let Some(element_depth) = signature.depth else {
            return false;
        };
        if declaration.star {
            if element_depth < declaration_depth {
                return false;
            }
        } else if element_depth != declaration_depth {
            return false;
        }
    }

    if signature.star && !declaration.star {
        return false;
    }

    if !declaration.selector.iter().all(|declaration_selector| {
        let declaration_selector = normalize_selector(declaration_selector);
        signature
            .selectors
            .iter()
            .any(|element_selector| element_selector == &declaration_selector)
    }) {
        return false;
    }

    declaration
        .stereotypes
        .iter()
        .all(|declaration_stereotype| {
            let declaration_stereotype = normalize_stereotype(declaration_stereotype);
            signature
                .stereotypes
                .iter()
                .any(|element_stereotype| element_stereotype == &declaration_stereotype)
        })
}

fn normalize_selector(selector: &str) -> String {
    selector
        .trim()
        .chars()
        .filter(|character| *character != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_stereotype(stereotype: &str) -> String {
    // Java provenance: `StyleSignatureBasic#clean`; guillemets and a leading
    // CSS dot are accepted here because renderer callers hold source-level
    // stereotype labels rather than Java's already-unwrapped labels.
    let stereotype = stereotype.trim();
    let stereotype = stereotype
        .strip_prefix("<<")
        .and_then(|value| value.strip_suffix(">>"))
        .unwrap_or(stereotype)
        .trim_start_matches('.');
    stereotype
        .chars()
        .filter(|character| !matches!(character, '_' | '.'))
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_property(property: &str) -> String {
    property
        .trim()
        .chars()
        .filter(|character| !matches!(character, '_' | '.'))
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use rustuml_parser::diagram::style::{
        StyleDeclaration, StyleOrigin, StyleProgram, StyleScheme,
    };

    use super::{StyleCascade, StyleSignature};

    fn declaration(
        selector: &[&str],
        stereotypes: &[&str],
        property: &str,
        value: &str,
        scheme: StyleScheme,
        epoch: u64,
        priority: i64,
    ) -> StyleDeclaration {
        StyleDeclaration {
            selector: selector.iter().map(|value| (*value).to_string()).collect(),
            stereotypes: stereotypes
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            depth: None,
            star: false,
            property: property.to_string(),
            value: value.to_string(),
            scheme,
            source_line: epoch as usize,
            epoch,
            priority,
            origin: StyleOrigin::UserStyle,
        }
    }

    fn program(declarations: Vec<StyleDeclaration>) -> StyleProgram {
        StyleProgram { declarations }
    }

    fn with_source_line(mut declaration: StyleDeclaration, source_line: usize) -> StyleDeclaration {
        declaration.source_line = source_line;
        declaration
    }

    fn with_origin(mut declaration: StyleDeclaration, origin: StyleOrigin) -> StyleDeclaration {
        declaration.origin = origin;
        declaration
    }

    #[test]
    fn root_properties_are_inherited_by_family_signatures() {
        let program = program(vec![declaration(
            &["root"],
            &[],
            "fontcolor",
            "black",
            StyleScheme::Regular,
            0,
            0,
        )]);
        let signature =
            StyleSignature::from_selectors(["root", "element", "classDiagram", "class"]);

        let resolved = StyleCascade::new(&program).resolve(&signature, StyleScheme::Regular);

        assert_eq!(resolved.property("fontColor"), Some("black"));
        assert_eq!(resolved.property("backgroundcolor"), None);
    }

    #[test]
    fn style_origin_remains_observable_after_skinparam_override() {
        let program = program(vec![
            declaration(&["root"], &[], "shadowing", "0", StyleScheme::Regular, 0, 0),
            with_origin(
                declaration(
                    &["root"],
                    &[],
                    "shadowing",
                    "false",
                    StyleScheme::Regular,
                    1,
                    1,
                ),
                StyleOrigin::ThemeSkinParam {
                    theme: "origin-probe".to_string(),
                },
            ),
        ]);
        let signature =
            StyleSignature::from_selectors(["root", "element", "componentDiagram", "component"]);
        let cascade = StyleCascade::new(&program);

        assert_eq!(
            cascade
                .resolve(&signature, StyleScheme::Regular)
                .property("shadowing"),
            Some("false")
        );
        assert!(cascade.has_style_declaration(&signature, StyleScheme::Regular, "shadowing"));
    }

    #[test]
    fn family_and_header_declarations_both_match_header_signatures() {
        let program = program(vec![
            declaration(
                &["root", "element", "classdiagram", "class"],
                &[],
                "backgroundcolor",
                "white",
                StyleScheme::Regular,
                0,
                0,
            ),
            declaration(
                &["root", "element", "classdiagram", "class", "header"],
                &[],
                "backgroundcolor",
                "silver",
                StyleScheme::Regular,
                1,
                1,
            ),
        ]);
        let family = StyleSignature::from_selectors(["root", "element", "classDiagram", "class"]);
        let header =
            StyleSignature::from_selectors(["root", "element", "classDiagram", "class", "header"]);
        let cascade = StyleCascade::new(&program);

        assert_eq!(
            cascade
                .resolve(&family, StyleScheme::Regular)
                .property("backgroundcolor"),
            Some("white")
        );
        assert_eq!(
            cascade
                .resolve(&header, StyleScheme::Regular)
                .property("backgroundcolor"),
            Some("silver")
        );
    }

    #[test]
    fn later_declaration_wins_when_priority_is_equal() {
        let program = program(vec![
            declaration(
                &["root"],
                &[],
                "linecolor",
                "red",
                StyleScheme::Regular,
                2,
                10,
            ),
            declaration(
                &["root"],
                &[],
                "linecolor",
                "blue",
                StyleScheme::Regular,
                3,
                10,
            ),
        ]);
        let signature = StyleSignature::from_selectors(["root"]);

        let resolved = StyleCascade::new(&program).resolve(&signature, StyleScheme::Regular);

        assert_eq!(resolved.property("linecolor"), Some("blue"));
    }

    #[test]
    fn stereotype_priority_and_normalization_override_plain_style() {
        let program = program(vec![
            declaration(
                &["root", "element", "classdiagram", "class"],
                &["critical_alert"],
                "linecolor",
                "red",
                StyleScheme::Regular,
                0,
                1_000,
            ),
            declaration(
                &["root", "element", "classdiagram", "class"],
                &[],
                "linecolor",
                "black",
                StyleScheme::Regular,
                1,
                1,
            ),
        ]);
        let plain = StyleSignature::from_selectors(["root", "element", "classdiagram", "class"]);
        let stereotyped = plain.clone().with_stereotype("<<Critical.Alert>>");
        let cascade = StyleCascade::new(&program);

        assert_eq!(
            cascade
                .resolve(&plain, StyleScheme::Regular)
                .property("linecolor"),
            Some("black")
        );
        assert_eq!(
            cascade
                .resolve(&stereotyped, StyleScheme::Regular)
                .property("linecolor"),
            Some("red")
        );
    }

    #[test]
    fn dark_scheme_selects_dark_values_and_falls_back_to_regular() {
        let program = program(vec![
            declaration(
                &["root"],
                &[],
                "backgroundcolor",
                "white",
                StyleScheme::Regular,
                0,
                0,
            ),
            declaration(
                &["root"],
                &[],
                "backgroundcolor",
                "black",
                StyleScheme::Dark,
                1,
                1,
            ),
            declaration(
                &["root"],
                &[],
                "fontcolor",
                "navy",
                StyleScheme::Regular,
                2,
                2,
            ),
        ]);
        let signature = StyleSignature::from_selectors(["root"]);
        let cascade = StyleCascade::new(&program);

        let regular = cascade.resolve(&signature, StyleScheme::Regular);
        assert_eq!(regular.property("backgroundcolor"), Some("white"));
        assert_eq!(regular.property("fontcolor"), Some("navy"));

        let dark = cascade.resolve(&signature, StyleScheme::Dark);
        assert_eq!(dark.property("backgroundcolor"), Some("black"));
        assert_eq!(dark.property("fontcolor"), Some("navy"));
    }

    #[test]
    fn epoch_ceiling_excludes_later_declarations() {
        let program = program(vec![
            declaration(
                &["root"],
                &[],
                "linecolor",
                "red",
                StyleScheme::Regular,
                2,
                2,
            ),
            declaration(
                &["root"],
                &[],
                "linecolor",
                "blue",
                StyleScheme::Regular,
                8,
                8,
            ),
        ]);
        let signature = StyleSignature::from_selectors(["root"]);
        let cascade = StyleCascade::new(&program);

        assert_eq!(
            cascade
                .resolve_at(&signature, StyleScheme::Regular, 1)
                .property("linecolor"),
            None
        );
        assert_eq!(
            cascade
                .resolve_at(&signature, StyleScheme::Regular, 5)
                .property("linecolor"),
            Some("red")
        );
        assert_eq!(
            cascade
                .resolve_at(&signature, StyleScheme::Regular, 8)
                .property("linecolor"),
            Some("blue")
        );
    }

    #[test]
    fn pure_css_entities_resolve_at_their_creation_source_lines() {
        let program = program(vec![
            with_source_line(
                declaration(
                    &["root", "element", "widgetDiagram", "widget"],
                    &[],
                    "signalColor",
                    "cedar-271",
                    StyleScheme::Regular,
                    40,
                    7,
                ),
                3,
            ),
            with_source_line(
                declaration(
                    &["root", "element", "widgetDiagram", "widget"],
                    &[],
                    "signalColor",
                    "amber-914",
                    StyleScheme::Regular,
                    41,
                    7,
                ),
                12,
            ),
        ]);
        let signature =
            StyleSignature::from_selectors(["root", "element", "widgetDiagram", "widget"]);
        let cascade = StyleCascade::new(&program);

        assert_eq!(
            cascade
                .resolve_entity_at_source_line(&signature, StyleScheme::Regular, 7)
                .property("signalColor"),
            Some("cedar-271")
        );
        assert_eq!(
            cascade
                .resolve_entity_at_source_line(&signature, StyleScheme::Regular, 16)
                .property("signalColor"),
            Some("amber-914")
        );
    }

    #[test]
    fn links_always_resolve_at_their_creation_source_lines() {
        let program = program(vec![
            with_origin(
                with_source_line(
                    declaration(
                        &["root", "element", "widgetDiagram", "arrow"],
                        &[],
                        "routeInk",
                        "plum-308",
                        StyleScheme::Regular,
                        70,
                        9,
                    ),
                    5,
                ),
                StyleOrigin::ThemeStyle {
                    theme: "renamed-cascade".to_string(),
                },
            ),
            with_source_line(
                declaration(
                    &["root", "element", "widgetDiagram", "arrow"],
                    &[],
                    "routeInk",
                    "teal-662",
                    StyleScheme::Regular,
                    71,
                    9,
                ),
                14,
            ),
        ]);
        let signature =
            StyleSignature::from_selectors(["root", "element", "widgetDiagram", "arrow"]);
        let cascade = StyleCascade::new(&program);

        assert_eq!(
            cascade
                .resolve_link_at_source_line(&signature, StyleScheme::Regular, 9)
                .property("routeInk"),
            Some("plum-308")
        );
        assert_eq!(
            cascade
                .resolve_link_at_source_line(&signature, StyleScheme::Regular, 18)
                .property("routeInk"),
            Some("teal-662")
        );
    }

    #[test]
    fn legacy_skinparams_refresh_entities_but_not_link_snapshots() {
        let legacy_origins = [
            StyleOrigin::UserSkinParam,
            StyleOrigin::ThemeSkinParam {
                theme: "renamed-legacy".to_string(),
            },
        ];

        for legacy_origin in legacy_origins {
            let program = program(vec![
                with_source_line(
                    declaration(
                        &["root", "element", "widgetDiagram", "widget"],
                        &[],
                        "signalColor",
                        "indigo-144",
                        StyleScheme::Regular,
                        90,
                        11,
                    ),
                    2,
                ),
                with_origin(
                    with_source_line(
                        declaration(
                            &["root"],
                            &[],
                            "compatibilityMarker",
                            "enabled-503",
                            StyleScheme::Regular,
                            91,
                            11,
                        ),
                        10,
                    ),
                    legacy_origin,
                ),
                with_source_line(
                    declaration(
                        &["root", "element", "widgetDiagram", "widget"],
                        &[],
                        "signalColor",
                        "copper-827",
                        StyleScheme::Regular,
                        92,
                        11,
                    ),
                    15,
                ),
            ]);
            let signature =
                StyleSignature::from_selectors(["root", "element", "widgetDiagram", "widget"]);
            let cascade = StyleCascade::new(&program);

            assert_eq!(
                cascade
                    .resolve_entity_at_source_line(&signature, StyleScheme::Regular, 6)
                    .property("signalColor"),
                Some("copper-827")
            );
            assert_eq!(
                cascade
                    .resolve_link_at_source_line(&signature, StyleScheme::Regular, 6)
                    .property("signalColor"),
                Some("indigo-144")
            );
            assert_eq!(
                cascade
                    .resolve_link_at_source_line(&signature, StyleScheme::Regular, 19)
                    .property("signalColor"),
                Some("copper-827")
            );
        }
    }
}
