// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Apply skinparam overrides to a theme.

use crate::style::Theme;
use rustuml_parser::diagram::SkinParam;

/// Apply a list of skinparams to a theme, returning a modified copy.
pub fn apply_skinparams(theme: &Theme, params: &[SkinParam]) -> Theme {
    let mut t = theme.clone();

    for param in params {
        match param.key.as_str() {
            // Theme switch (from !theme directive).
            "__theme" => match param.value.as_str() {
                "modern" => t = Theme::modern(),
                "default" => t = Theme::plantuml_default(),
                _ => {} // Unknown theme — keep current.
            },

            // Global
            "backgroundColor" => t.global.background_color = param.value.clone(),
            "defaultFontName" => t.global.font_family = param.value.clone(),
            "defaultFontColor" => t.global.default_font_color = param.value.clone(),
            "defaultFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.default_font_size = v;
                }
            }
            "monochrome" => {
                t.global.monochrome = param.value.to_lowercase() == "true";
            }
            "shadowing" => {
                t.global.shadowing = param.value.to_lowercase() != "false";
            }
            "handwritten" => {
                t.global.handwritten = param.value.to_lowercase() == "true";
            }
            "dpi" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.dpi = v;
                }
            }
            "arrowThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.arrow_thickness = v;
                }
            }
            "linetype" => t.global.linetype = param.value.clone(),
            "nodesep" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.nodesep = v;
                }
            }
            "ranksep" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.ranksep = v;
                }
            }
            "padding" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.padding = v;
                }
            }

            // Sequence
            "sequenceArrowColor" | "ArrowColor" => {
                t.sequence.arrow_color = param.value.clone();
            }
            "sequenceParticipantBackgroundColor" | "participantBackgroundColor" => {
                t.sequence.participant_background = param.value.clone();
            }
            "sequenceParticipantBorderColor" | "participantBorderColor" => {
                t.sequence.participant_border = param.value.clone();
            }
            "sequenceLifeLineBorderColor" => {
                t.sequence.lifeline_color = param.value.clone();
            }
            "noteBackgroundColor" => {
                t.sequence.note_background = param.value.clone();
            }

            // Class
            "classBackgroundColor" => t.class.class_background = param.value.clone(),
            "classBorderColor" => t.class.border_color = param.value.clone(),
            "interfaceBackgroundColor" => {
                t.class.interface_background = param.value.clone();
            }
            "classArrowColor" => t.class.arrow_color = param.value.clone(),
            "classArrowThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.arrow_thickness = v;
                }
            }
            "ClassFontSize" | "classFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.font_size = v;
                }
            }
            "classFontColor" | "ClassFontColor" => {
                t.class.font_color = param.value.clone();
            }
            "classHeaderBackgroundColor" | "ClassHeaderBackgroundColor" => {
                t.class.header_background = param.value.clone();
            }
            "classAttributeFontSize" | "ClassAttributeFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.attribute_font_size = v;
                }
            }

            // State
            "stateBackgroundColor" => t.state.state_background = param.value.clone(),
            "stateBorderColor" => t.state.border_color = param.value.clone(),

            // Activity
            "activityBackgroundColor" => {
                t.activity.action_background = param.value.clone();
            }
            "activityBorderColor" => t.activity.action_border = param.value.clone(),
            "activityArrowColor" => t.activity.arrow_color = param.value.clone(),

            _ => {} // Unknown skinparams silently ignored.
        }
    }

    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_background_color() {
        let theme = Theme::default();
        let params = vec![SkinParam {
            key: "backgroundColor".into(),
            value: "#FF0000".into(),
        }];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.global.background_color, "#FF0000");
    }

    #[test]
    fn apply_multiple_params() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "sequenceArrowColor".into(),
                value: "Blue".into(),
            },
            SkinParam {
                key: "classBackgroundColor".into(),
                value: "#AABBCC".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.sequence.arrow_color, "Blue");
        assert_eq!(t.class.class_background, "#AABBCC");
    }

    #[test]
    fn unknown_params_ignored() {
        let theme = Theme::default();
        let original_bg = theme.global.background_color.clone();
        let params = vec![SkinParam {
            key: "nonexistentParam".into(),
            value: "whatever".into(),
        }];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.global.background_color, original_bg);
    }
}
