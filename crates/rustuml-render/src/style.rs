// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Style system — themes and skinparams for diagram rendering.
//!
//! Provides a cascading style model inspired by CSS. Styles are
//! resolved from most specific to least specific:
//!
//! 1. Per-element inline style (e.g., `class Foo #FF0000`)
//! 2. Per-type style (e.g., `skinparam classBorderColor`)
//! 3. Theme defaults
//! 4. Built-in defaults

use serde::{Deserialize, Serialize};

/// A complete style theme for diagram rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub sequence: SequenceStyle,
    pub class: ClassStyle,
    pub state: StateStyle,
    pub activity: ActivityStyle,
    pub global: GlobalStyle,
    pub note: NoteStyle,
    pub actor: ActorStyle,
    pub package: PackageStyle,
    pub component: ComponentStyle,
    pub usecase: UseCaseStyle,
    pub deployment: DeploymentStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStyle {
    pub font_family: String,
    pub font_size: f64,
    pub background_color: String,
    pub default_font_color: String,
    pub arrow_color: String,
    pub border_color: String,
    pub default_font_size: f64,
    pub monochrome: bool,
    pub shadowing: bool,
    pub handwritten: bool,
    pub dpi: f64,
    pub arrow_thickness: f64,
    pub linetype: String,
    pub nodesep: f64,
    pub ranksep: f64,
    pub padding: f64,
    pub default_font_style: String,
    pub default_text_alignment: String,
    pub arrow_font_color: String,
    pub arrow_font_size: f64,
    pub border_thickness: f64,
    pub round_corner: f64,
    pub wrap_width: f64,
    pub max_message_size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceStyle {
    pub participant_background: String,
    pub participant_border: String,
    pub lifeline_color: String,
    pub arrow_color: String,
    pub note_background: String,
    pub note_border: String,
    pub group_border: String,
    pub divider_color: String,
    pub actor_background: String,
    pub actor_border: String,
    pub actor_font_color: String,
    pub actor_font_size: f64,
    pub actor_font_style: String,
    pub arrow_font_color: String,
    pub arrow_font_size: f64,
    pub arrow_font_style: String,
    pub arrow_thickness: f64,
    pub divider_background: String,
    pub divider_border: String,
    pub divider_font_color: String,
    pub divider_font_size: f64,
    pub group_background: String,
    pub group_font_color: String,
    pub group_font_size: f64,
    pub group_font_style: String,
    pub group_header_font_color: String,
    pub group_header_font_size: f64,
    pub group_header_font_style: String,
    pub lifeline_background: String,
    pub lifeline_border_thickness: f64,
    pub message_align: String,
    pub participant_border_thickness: f64,
    pub participant_font_color: String,
    pub participant_font_size: f64,
    pub participant_font_style: String,
    pub participant_padding: f64,
    pub reference_background: String,
    pub reference_border: String,
    pub reference_font_color: String,
    /// When true, response messages are rendered below the arrow line.
    pub response_message_below_arrow: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassStyle {
    pub class_background: String,
    pub abstract_background: String,
    pub interface_background: String,
    pub enum_background: String,
    pub border_color: String,
    pub stereotype_color: String,
    pub arrow_color: String,
    pub arrow_thickness: f64,
    pub font_size: f64,
    pub font_color: String,
    pub header_background: String,
    pub attribute_font_size: f64,
    pub arrow_font_color: String,
    pub arrow_font_size: f64,
    pub arrow_font_style: String,
    pub attribute_font_color: String,
    pub attribute_font_style: String,
    pub attribute_icon_size: f64,
    pub border_thickness: f64,
    pub font_name: String,
    pub font_style: String,
    pub round_corner: f64,
    pub stereotype_font_color: String,
    pub stereotype_font_size: f64,
    pub stereotype_font_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateStyle {
    pub state_background: String,
    pub initial_color: String,
    pub border_color: String,
    pub arrow_color: String,
    pub arrow_font_color: String,
    pub arrow_font_size: f64,
    pub arrow_font_style: String,
    pub attribute_font_color: String,
    pub attribute_font_size: f64,
    pub border_thickness: f64,
    pub end_color: String,
    pub font_color: String,
    pub font_name: String,
    pub font_size: f64,
    pub font_style: String,
    pub round_corner: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityStyle {
    pub action_background: String,
    pub action_border: String,
    pub decision_background: String,
    pub bar_color: String,
    pub start_color: String,
    pub stop_color: String,
    pub arrow_color: String,
    pub arrow_font_color: String,
    pub arrow_font_size: f64,
    pub border_thickness: f64,
    pub diamond_font_color: String,
    pub diamond_font_size: f64,
    pub end_color: String,
    pub font_color: String,
    pub font_name: String,
    pub font_size: f64,
    pub font_style: String,
    pub round_corner: f64,
    pub swimlane_background: String,
    pub swimlane_border: String,
    pub swimlane_font_color: String,
    pub swimlane_font_size: f64,
    pub swimlane_title_font_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NoteStyle {
    pub background: String,
    pub border: String,
    pub font_color: String,
    pub font_name: String,
    pub font_size: f64,
    pub font_style: String,
    pub text_alignment: String,
    pub round_corner: f64,
    pub border_thickness: f64,
    pub shadowing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActorStyle {
    pub background: String,
    pub border: String,
    pub font_color: String,
    pub font_size: f64,
    pub font_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageStyle {
    pub background: String,
    pub border: String,
    pub font_color: String,
    pub font_name: String,
    pub font_size: f64,
    pub font_style: String,
    pub style: String,
    pub border_thickness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComponentStyle {
    pub background: String,
    pub border: String,
    pub arrow_color: String,
    pub arrow_font_color: String,
    pub arrow_font_size: f64,
    pub border_thickness: f64,
    pub font_color: String,
    pub font_size: f64,
    pub font_style: String,
    pub round_corner: f64,
    /// Component rendering style: "rectangle" or "uml2" (default UML2 icon).
    pub style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UseCaseStyle {
    pub background: String,
    pub border: String,
    pub arrow_color: String,
    pub arrow_font_color: String,
    pub arrow_font_size: f64,
    pub border_thickness: f64,
    pub font_color: String,
    pub font_size: f64,
    pub font_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeploymentStyle {
    pub boundary_background: String,
    pub cloud_background: String,
    pub cloud_border: String,
    pub collections_background: String,
    pub control_background: String,
    pub database_background: String,
    pub database_border: String,
    pub entity_background: String,
    pub frame_background: String,
    pub frame_border: String,
    pub node_background: String,
    pub node_border: String,
    pub queue_background: String,
    pub rectangle_background: String,
    pub rectangle_border: String,
    pub storage_background: String,
    pub storage_border: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self::plantuml_default()
    }
}

impl Theme {
    /// The classic PlantUML color scheme.
    pub fn plantuml_default() -> Self {
        Self {
            name: "default".into(),
            sequence: SequenceStyle {
                participant_background: "#E2E2F0".into(),
                participant_border: "#181818".into(),
                lifeline_color: "#999999".into(),
                arrow_color: "#181818".into(),
                note_background: "#FEFFDD".into(),
                note_border: "#181818".into(),
                group_border: "#999999".into(),
                divider_color: "#999999".into(),
                actor_background: "#E2E2F0".into(),
                actor_border: "#181818".into(),
                actor_font_color: "".into(),
                actor_font_size: 0.0,
                actor_font_style: "".into(),
                arrow_font_color: "".into(),
                arrow_font_size: 0.0,
                arrow_font_style: "".into(),
                arrow_thickness: 1.0,
                divider_background: "#EEEEEE".into(),
                divider_border: "#999999".into(),
                divider_font_color: "".into(),
                divider_font_size: 0.0,
                group_background: "".into(),
                group_font_color: "".into(),
                group_font_size: 0.0,
                group_font_style: "".into(),
                group_header_font_color: "".into(),
                group_header_font_size: 0.0,
                group_header_font_style: "".into(),
                lifeline_background: "".into(),
                lifeline_border_thickness: 0.5,
                message_align: "".into(),
                participant_border_thickness: 0.5,
                participant_font_color: "".into(),
                participant_font_size: 0.0,
                participant_font_style: "".into(),
                participant_padding: 0.0,
                reference_background: "#FEFFDD".into(),
                reference_border: "#181818".into(),
                reference_font_color: "".into(),
                response_message_below_arrow: false,
            },
            class: ClassStyle {
                class_background: "#FDEBD0".into(),
                abstract_background: "#FDEBD0".into(),
                interface_background: "#D4E6F1".into(),
                enum_background: "#D5F5E3".into(),
                border_color: "#000000".into(),
                stereotype_color: "#666666".into(),
                arrow_color: "#181818".into(),
                arrow_thickness: 1.0,
                font_size: 13.0,
                font_color: "#000000".into(),
                header_background: "".into(),
                attribute_font_size: 11.0,
                arrow_font_color: "".into(),
                arrow_font_size: 0.0,
                arrow_font_style: "".into(),
                attribute_font_color: "".into(),
                attribute_font_style: "".into(),
                attribute_icon_size: 0.0,
                border_thickness: 0.5,
                font_name: "".into(),
                font_style: "".into(),
                round_corner: 0.0,
                stereotype_font_color: "".into(),
                stereotype_font_size: 0.0,
                stereotype_font_style: "".into(),
            },
            state: StateStyle {
                state_background: "#FEEBD0".into(),
                initial_color: "#000000".into(),
                border_color: "#000000".into(),
                arrow_color: "#000000".into(),
                arrow_font_color: "".into(),
                arrow_font_size: 0.0,
                arrow_font_style: "".into(),
                attribute_font_color: "".into(),
                attribute_font_size: 0.0,
                border_thickness: 0.5,
                end_color: "#000000".into(),
                font_color: "".into(),
                font_name: "".into(),
                font_size: 0.0,
                font_style: "".into(),
                round_corner: 0.0,
            },
            activity: ActivityStyle {
                action_background: "#E2E2F0".into(),
                action_border: "#000000".into(),
                decision_background: "#FFFACD".into(),
                bar_color: "#000000".into(),
                start_color: "#000000".into(),
                stop_color: "#000000".into(),
                arrow_color: "#181818".into(),
                arrow_font_color: "".into(),
                arrow_font_size: 0.0,
                border_thickness: 0.5,
                diamond_font_color: "".into(),
                diamond_font_size: 0.0,
                end_color: "#000000".into(),
                font_color: "".into(),
                font_name: "".into(),
                font_size: 0.0,
                font_style: "".into(),
                round_corner: 0.0,
                swimlane_background: "".into(),
                swimlane_border: "".into(),
                swimlane_font_color: "".into(),
                swimlane_font_size: 0.0,
                swimlane_title_font_style: "".into(),
            },
            global: GlobalStyle {
                font_family: "sans-serif".into(),
                font_size: 13.0,
                background_color: "#FFFFFF".into(),
                default_font_color: "#000000".into(),
                arrow_color: "#181818".into(),
                border_color: "#181818".into(),
                default_font_size: 13.0,
                monochrome: false,
                shadowing: true,
                handwritten: false,
                dpi: 96.0,
                arrow_thickness: 1.0,
                linetype: "".into(),
                nodesep: 25.0,
                ranksep: 40.0,
                padding: 8.0,
                default_font_style: "".into(),
                default_text_alignment: "".into(),
                arrow_font_color: "".into(),
                arrow_font_size: 0.0,
                border_thickness: 0.5,
                round_corner: 0.0,
                wrap_width: 0.0,
                max_message_size: 0.0,
            },
            note: NoteStyle::default(),
            actor: ActorStyle::default(),
            package: PackageStyle::default(),
            component: ComponentStyle::default(),
            usecase: UseCaseStyle::default(),
            deployment: DeploymentStyle::default(),
        }
    }

    /// A modern, cleaner theme.
    pub fn modern() -> Self {
        Self {
            name: "modern".into(),
            sequence: SequenceStyle {
                participant_background: "#F8F9FA".into(),
                participant_border: "#495057".into(),
                lifeline_color: "#CED4DA".into(),
                arrow_color: "#495057".into(),
                note_background: "#FFF3CD".into(),
                note_border: "#FFC107".into(),
                group_border: "#6C757D".into(),
                divider_color: "#ADB5BD".into(),
                actor_background: "#F8F9FA".into(),
                actor_border: "#495057".into(),
                actor_font_color: "".into(),
                actor_font_size: 0.0,
                actor_font_style: "".into(),
                arrow_font_color: "".into(),
                arrow_font_size: 0.0,
                arrow_font_style: "".into(),
                arrow_thickness: 1.0,
                divider_background: "#F1F3F5".into(),
                divider_border: "#ADB5BD".into(),
                divider_font_color: "".into(),
                divider_font_size: 0.0,
                group_background: "".into(),
                group_font_color: "".into(),
                group_font_size: 0.0,
                group_font_style: "".into(),
                group_header_font_color: "".into(),
                group_header_font_size: 0.0,
                group_header_font_style: "".into(),
                lifeline_background: "".into(),
                lifeline_border_thickness: 0.5,
                message_align: "".into(),
                participant_border_thickness: 0.5,
                participant_font_color: "".into(),
                participant_font_size: 0.0,
                participant_font_style: "".into(),
                participant_padding: 0.0,
                reference_background: "#FFF3CD".into(),
                reference_border: "#FFC107".into(),
                reference_font_color: "".into(),
                response_message_below_arrow: false,
            },
            class: ClassStyle {
                class_background: "#F8F9FA".into(),
                abstract_background: "#E9ECEF".into(),
                interface_background: "#D0E8FF".into(),
                enum_background: "#D1E7DD".into(),
                border_color: "#495057".into(),
                stereotype_color: "#6C757D".into(),
                arrow_color: "#495057".into(),
                arrow_thickness: 1.0,
                font_size: 14.0,
                font_color: "#212529".into(),
                header_background: "".into(),
                attribute_font_size: 11.0,
                arrow_font_color: "".into(),
                arrow_font_size: 0.0,
                arrow_font_style: "".into(),
                attribute_font_color: "".into(),
                attribute_font_style: "".into(),
                attribute_icon_size: 0.0,
                border_thickness: 0.5,
                font_name: "".into(),
                font_style: "".into(),
                round_corner: 0.0,
                stereotype_font_color: "".into(),
                stereotype_font_size: 0.0,
                stereotype_font_style: "".into(),
            },
            state: StateStyle {
                state_background: "#F8F9FA".into(),
                initial_color: "#212529".into(),
                border_color: "#495057".into(),
                arrow_color: "#495057".into(),
                arrow_font_color: "".into(),
                arrow_font_size: 0.0,
                arrow_font_style: "".into(),
                attribute_font_color: "".into(),
                attribute_font_size: 0.0,
                border_thickness: 0.5,
                end_color: "#212529".into(),
                font_color: "".into(),
                font_name: "".into(),
                font_size: 0.0,
                font_style: "".into(),
                round_corner: 0.0,
            },
            activity: ActivityStyle {
                action_background: "#F8F9FA".into(),
                action_border: "#495057".into(),
                decision_background: "#FFF3CD".into(),
                bar_color: "#212529".into(),
                start_color: "#212529".into(),
                stop_color: "#212529".into(),
                arrow_color: "#495057".into(),
                arrow_font_color: "".into(),
                arrow_font_size: 0.0,
                border_thickness: 0.5,
                diamond_font_color: "".into(),
                diamond_font_size: 0.0,
                end_color: "#212529".into(),
                font_color: "".into(),
                font_name: "".into(),
                font_size: 0.0,
                font_style: "".into(),
                round_corner: 0.0,
                swimlane_background: "".into(),
                swimlane_border: "".into(),
                swimlane_font_color: "".into(),
                swimlane_font_size: 0.0,
                swimlane_title_font_style: "".into(),
            },
            global: GlobalStyle {
                font_family: "system-ui, -apple-system, sans-serif".into(),
                font_size: 14.0,
                background_color: "#FFFFFF".into(),
                default_font_color: "#212529".into(),
                arrow_color: "#495057".into(),
                border_color: "#495057".into(),
                default_font_size: 14.0,
                monochrome: false,
                shadowing: false,
                handwritten: false,
                dpi: 96.0,
                arrow_thickness: 1.0,
                linetype: "".into(),
                nodesep: 25.0,
                ranksep: 40.0,
                padding: 8.0,
                default_font_style: "".into(),
                default_text_alignment: "".into(),
                arrow_font_color: "".into(),
                arrow_font_size: 0.0,
                border_thickness: 0.5,
                round_corner: 0.0,
                wrap_width: 0.0,
                max_message_size: 0.0,
            },
            note: NoteStyle::default(),
            actor: ActorStyle::default(),
            package: PackageStyle::default(),
            component: ComponentStyle::default(),
            usecase: UseCaseStyle::default(),
            deployment: DeploymentStyle::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_has_colors() {
        let t = Theme::default();
        assert!(!t.sequence.participant_background.is_empty());
        assert!(!t.class.class_background.is_empty());
    }

    #[test]
    fn modern_theme_differs_from_default() {
        let d = Theme::plantuml_default();
        let m = Theme::modern();
        assert_ne!(
            d.sequence.participant_background,
            m.sequence.participant_background
        );
    }

    #[test]
    fn theme_serializes_to_yaml() {
        let t = Theme::modern();
        let yaml = serde_yml::to_string(&t).unwrap();
        assert!(yaml.contains("modern"));
        // Round-trip.
        let reparsed: Theme = serde_yml::from_str(&yaml).unwrap();
        assert_eq!(reparsed.name, "modern");
    }
}
