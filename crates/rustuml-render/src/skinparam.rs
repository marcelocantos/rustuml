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

            // State
            "stateBackgroundColor" => t.state.state_background = param.value.clone(),
            "stateBorderColor" => t.state.border_color = param.value.clone(),

            // Activity
            "activityBackgroundColor" => {
                t.activity.action_background = param.value.clone();
            }
            "activityBorderColor" => t.activity.action_border = param.value.clone(),

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
