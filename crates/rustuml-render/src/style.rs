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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateStyle {
    pub state_background: String,
    pub initial_color: String,
    pub border_color: String,
    pub arrow_color: String,
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
            },
            state: StateStyle {
                state_background: "#FEEBD0".into(),
                initial_color: "#000000".into(),
                border_color: "#000000".into(),
                arrow_color: "#000000".into(),
            },
            activity: ActivityStyle {
                action_background: "#E2E2F0".into(),
                action_border: "#000000".into(),
                decision_background: "#FFFACD".into(),
                bar_color: "#000000".into(),
                start_color: "#000000".into(),
                stop_color: "#000000".into(),
                arrow_color: "#181818".into(),
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
            },
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
            },
            state: StateStyle {
                state_background: "#F8F9FA".into(),
                initial_color: "#212529".into(),
                border_color: "#495057".into(),
                arrow_color: "#495057".into(),
            },
            activity: ActivityStyle {
                action_background: "#F8F9FA".into(),
                action_border: "#495057".into(),
                decision_background: "#FFF3CD".into(),
                bar_color: "#212529".into(),
                start_color: "#212529".into(),
                stop_color: "#212529".into(),
                arrow_color: "#495057".into(),
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
            },
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
        let yaml = serde_yaml::to_string(&t).unwrap();
        assert!(yaml.contains("modern"));
        // Round-trip.
        let reparsed: Theme = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(reparsed.name, "modern");
    }
}
