// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Apply skinparam overrides to a theme.

use crate::style::Theme;
use rustuml_parser::diagram::SkinParam;

/// Normalize a skinparam key to canonical camelCase form.
///
/// PlantUML treats skinparam keys case-insensitively for the first character
/// (e.g., `ClassBackgroundColor` == `classBackgroundColor`). We normalize by
/// lowercasing the first character, which covers the vast majority of
/// case-variant keys found in real-world diagrams.
fn normalize_key(key: &str) -> String {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) => {
            let mut s = c.to_lowercase().to_string();
            s.extend(chars);
            s
        }
        None => String::new(),
    }
}

/// Apply a list of skinparams to a theme, returning a modified copy.
#[allow(clippy::too_many_lines)]
pub fn apply_skinparams(theme: &Theme, params: &[SkinParam]) -> Theme {
    let mut t = theme.clone();

    for param in params {
        let key = normalize_key(&param.key);
        match key.as_str() {
            // Theme switch (from !theme directive).
            "__theme" => match param.value.as_str() {
                "modern" => t = Theme::modern(),
                "default" => t = Theme::plantuml_default(),
                _ => {} // Unknown theme — keep current.
            },

            // ── Global ────────────────────────────────────────────────────────
            "backgroundColor" => t.global.background_color = param.value.clone(),
            "defaultFontName" | "fontName" => t.global.font_family = param.value.clone(),
            "defaultFontColor" => t.global.default_font_color = param.value.clone(),
            "defaultFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.default_font_size = v;
                }
            }
            "defaultFontStyle" => t.global.default_font_style = param.value.clone(),
            "defaultTextAlignment" => t.global.default_text_alignment = param.value.clone(),
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
            "arrowColor" => t.global.arrow_color = param.value.clone(),
            "arrowThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.arrow_thickness = v;
                }
            }
            "arrowFontColor" => t.global.arrow_font_color = param.value.clone(),
            "arrowFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.arrow_font_size = v;
                }
            }
            "arrowFontStyle" => t.global.arrow_font_style = param.value.clone(),
            "borderColor" => t.global.border_color = param.value.clone(),
            "borderThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.border_thickness = v;
                }
            }
            "roundCorner" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.round_corner = v;
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
            "wrapWidth" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.wrap_width = v;
                }
            }
            "maxMessageSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.max_message_size = v;
                }
            }
            "titleFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.title_font_size = v;
                }
            }
            "titleFontStyle" => t.global.title_font_style = param.value.clone(),
            "svgLinkTarget" => t.global.svg_link_target = param.value.clone(),
            "style" => {}      // Diagram style variant — ignored at theme level.
            "guillemet" => {}  // Stereotype angle bracket style — ignored.
            "autonumber" => {} // Sequence autonumber — handled by parser.

            // ── Sequence ──────────────────────────────────────────────────────
            "sequenceArrowColor" => t.sequence.arrow_color = param.value.clone(),
            "sequenceArrowFontColor" => t.sequence.arrow_font_color = param.value.clone(),
            "sequenceArrowFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.sequence.arrow_font_size = v;
                }
            }
            "sequenceArrowFontStyle" => t.sequence.arrow_font_style = param.value.clone(),
            "sequenceArrowThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.sequence.arrow_thickness = v;
                }
            }
            "sequenceParticipantBackgroundColor" | "participantBackgroundColor" => {
                t.sequence.participant_background = param.value.clone();
            }
            "sequenceParticipantBorderColor" | "participantBorderColor" => {
                t.sequence.participant_border = param.value.clone();
            }
            "sequenceParticipantBorderThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.sequence.participant_border_thickness = v;
                }
            }
            "sequenceParticipantFontColor" => {
                t.sequence.participant_font_color = param.value.clone();
            }
            "sequenceParticipantFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.sequence.participant_font_size = v;
                }
            }
            "sequenceParticipantFontStyle" => {
                t.sequence.participant_font_style = param.value.clone();
            }
            "sequenceParticipantPadding" | "participantPadding" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.sequence.participant_padding = v;
                }
            }
            "sequenceLifeLineBorderColor" => t.sequence.lifeline_color = param.value.clone(),
            "sequenceLifeLineBackgroundColor" => {
                t.sequence.lifeline_background = param.value.clone();
            }
            "sequenceLifeLineBorderThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.sequence.lifeline_border_thickness = v;
                }
            }
            "sequenceLifeLineStrategy" => {
                t.sequence.lifeline_strategy = param.value.clone();
            }
            "noteBackgroundColor" => {
                t.sequence.note_background = param.value.clone();
                t.note.background = param.value.clone();
            }
            "noteBorderColor" => {
                t.sequence.note_border = param.value.clone();
                t.note.border = param.value.clone();
            }
            "noteBorderThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.note.border_thickness = v;
                }
            }
            "noteFontColor" => {
                t.note.font_color = param.value.clone();
            }
            "noteFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.note.font_size = v;
                }
            }
            "noteFontName" => t.note.font_name = param.value.clone(),
            "noteFontStyle" => t.note.font_style = param.value.clone(),
            "noteRoundCorner" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.note.round_corner = v;
                }
            }
            "noteShadowing" => {
                t.note.shadowing = param.value.to_lowercase() != "false";
            }
            "noteTextAlignment" => t.note.text_alignment = param.value.clone(),
            "sequenceGroupBackgroundColor" => t.sequence.group_background = param.value.clone(),
            "sequenceGroupBorderColor" => t.sequence.group_border = param.value.clone(),
            "sequenceGroupBodyBackgroundColor" => {
                t.sequence.group_body_background = param.value.clone();
            }
            "sequenceGroupFontColor" => t.sequence.group_font_color = param.value.clone(),
            "sequenceGroupFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.sequence.group_font_size = v;
                }
            }
            "sequenceGroupFontStyle" => t.sequence.group_font_style = param.value.clone(),
            "sequenceGroupHeaderFontColor" => {
                t.sequence.group_header_font_color = param.value.clone();
            }
            "sequenceGroupHeaderFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.sequence.group_header_font_size = v;
                }
            }
            "sequenceGroupHeaderFontStyle" => {
                t.sequence.group_header_font_style = param.value.clone();
            }
            "sequenceDividerBackgroundColor" => {
                t.sequence.divider_background = param.value.clone();
            }
            "sequenceDividerBorderColor" => t.sequence.divider_border = param.value.clone(),
            "sequenceDividerBorderThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.sequence.divider_border_thickness = v;
                }
            }
            "sequenceDividerFontColor" => t.sequence.divider_font_color = param.value.clone(),
            "sequenceDividerFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.sequence.divider_font_size = v;
                }
            }
            "sequenceReferenceBackgroundColor" => {
                t.sequence.reference_background = param.value.clone();
            }
            "sequenceReferenceBorderColor" => {
                t.sequence.reference_border = param.value.clone();
            }
            "sequenceReferenceFontColor" => {
                t.sequence.reference_font_color = param.value.clone();
            }
            "sequenceActorBackgroundColor" => t.sequence.actor_background = param.value.clone(),
            "sequenceActorBorderColor" => t.sequence.actor_border = param.value.clone(),
            "sequenceActorFontColor" => t.sequence.actor_font_color = param.value.clone(),
            "sequenceActorFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.sequence.actor_font_size = v;
                }
            }
            "sequenceActorFontStyle" => t.sequence.actor_font_style = param.value.clone(),
            "sequenceMessageAlign" => t.sequence.message_align = param.value.clone(),
            "boxPadding" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.sequence.box_padding = v;
                }
            }

            // ── Class ─────────────────────────────────────────────────────────
            "classBackgroundColor" => t.class.class_background = param.value.clone(),
            "classBorderColor" => t.class.border_color = param.value.clone(),
            "classBorderThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.border_thickness = v;
                }
            }
            "interfaceBackgroundColor" => {
                t.class.interface_background = param.value.clone();
            }
            "interfaceBorderColor" => t.class.border_color = param.value.clone(),
            "interfaceFontColor" => t.class.font_color = param.value.clone(),
            "classArrowColor" => t.class.arrow_color = param.value.clone(),
            "classArrowThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.arrow_thickness = v;
                }
            }
            "classArrowFontColor" => t.class.arrow_font_color = param.value.clone(),
            "classArrowFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.arrow_font_size = v;
                }
            }
            "classArrowFontStyle" => t.class.arrow_font_style = param.value.clone(),
            "classFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.font_size = v;
                }
            }
            "classFontColor" => t.class.font_color = param.value.clone(),
            "classFontName" => t.class.font_name = param.value.clone(),
            "classFontStyle" => t.class.font_style = param.value.clone(),
            "classHeaderBackgroundColor" => {
                t.class.header_background = param.value.clone();
            }
            "classAttributeFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.attribute_font_size = v;
                }
            }
            "classAttributeFontColor" => {
                t.class.attribute_font_color = param.value.clone();
            }
            "classAttributeFontStyle" => {
                t.class.attribute_font_style = param.value.clone();
            }
            "classAttributeIconSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.attribute_icon_size = v;
                }
            }
            "classRoundCorner" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.round_corner = v;
                }
            }
            "stereotypeFontColor" | "classStereotypeFontColor" => {
                t.class.stereotype_font_color = param.value.clone();
            }
            "stereotypeFontSize" | "classStereotypeFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.stereotype_font_size = v;
                }
            }
            "stereotypeFontStyle" | "classStereotypeFontStyle" => {
                t.class.stereotype_font_style = param.value.clone();
            }
            "stereotypeCBackgroundColor" => {
                t.class.stereotype_c_background = param.value.clone();
            }

            // ── State ─────────────────────────────────────────────────────────
            "stateBackgroundColor" => t.state.state_background = param.value.clone(),
            "stateBorderColor" => t.state.border_color = param.value.clone(),
            "stateBorderThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.state.border_thickness = v;
                }
            }
            "stateArrowColor" => t.state.arrow_color = param.value.clone(),
            "stateArrowFontColor" => t.state.arrow_font_color = param.value.clone(),
            "stateArrowFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.state.arrow_font_size = v;
                }
            }
            "stateArrowFontStyle" => t.state.arrow_font_style = param.value.clone(),
            "stateFontColor" => t.state.font_color = param.value.clone(),
            "stateFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.state.font_size = v;
                }
            }
            "stateFontName" => t.state.font_name = param.value.clone(),
            "stateFontStyle" => t.state.font_style = param.value.clone(),
            "stateAttributeFontColor" => t.state.attribute_font_color = param.value.clone(),
            "stateAttributeFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.state.attribute_font_size = v;
                }
            }
            "stateRoundCorner" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.state.round_corner = v;
                }
            }
            "stateStartColor" => t.state.initial_color = param.value.clone(),
            "stateEndColor" => t.state.end_color = param.value.clone(),

            // ── Activity ──────────────────────────────────────────────────────
            "activityBackgroundColor" => {
                t.activity.action_background = param.value.clone();
            }
            "activityBorderColor" => t.activity.action_border = param.value.clone(),
            "activityBorderThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.activity.border_thickness = v;
                }
            }
            "activityArrowColor" => t.activity.arrow_color = param.value.clone(),
            "activityArrowFontColor" => t.activity.arrow_font_color = param.value.clone(),
            "activityArrowFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.activity.arrow_font_size = v;
                }
            }
            "activityDiamondBackgroundColor" => {
                t.activity.decision_background = param.value.clone();
            }
            "activityDiamondBorderColor" => {
                t.activity.diamond_border_color = param.value.clone();
            }
            "activityDiamondFontColor" => t.activity.diamond_font_color = param.value.clone(),
            "activityDiamondFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.activity.diamond_font_size = v;
                }
            }
            "activityBarColor" => t.activity.bar_color = param.value.clone(),
            "activityStartColor" => t.activity.start_color = param.value.clone(),
            "activityEndColor" => t.activity.end_color = param.value.clone(),
            "activityFontColor" => t.activity.font_color = param.value.clone(),
            "activityFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.activity.font_size = v;
                }
            }
            "activityFontName" => t.activity.font_name = param.value.clone(),
            "activityFontStyle" => t.activity.font_style = param.value.clone(),
            "activityRoundCorner" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.activity.round_corner = v;
                }
            }
            "swimlaneBackgroundColor" => t.activity.swimlane_background = param.value.clone(),
            "swimlaneBorderColor" => t.activity.swimlane_border = param.value.clone(),
            "swimlaneFontColor" => t.activity.swimlane_font_color = param.value.clone(),
            "swimlaneFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.activity.swimlane_font_size = v;
                }
            }
            "swimlaneTitleFontStyle" => {
                t.activity.swimlane_title_font_style = param.value.clone();
            }

            // ── Component ─────────────────────────────────────────────────────
            "componentBackgroundColor" => t.component.background = param.value.clone(),
            "componentBorderColor" => t.component.border = param.value.clone(),
            "componentBorderThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.component.border_thickness = v;
                }
            }
            "componentFontColor" => t.component.font_color = param.value.clone(),
            "componentFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.component.font_size = v;
                }
            }
            "componentFontStyle" => t.component.font_style = param.value.clone(),
            "componentArrowColor" => t.component.arrow_color = param.value.clone(),
            "componentArrowFontColor" => t.component.arrow_font_color = param.value.clone(),
            "componentArrowFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.component.arrow_font_size = v;
                }
            }
            "componentRoundCorner" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.component.round_corner = v;
                }
            }
            "componentStyle" => t.component.style = param.value.clone(),

            // ── Use case ──────────────────────────────────────────────────────
            "usecaseBackgroundColor" => t.usecase.background = param.value.clone(),
            "usecaseBorderColor" => t.usecase.border = param.value.clone(),
            "usecaseBorderThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.usecase.border_thickness = v;
                }
            }
            "usecaseFontColor" => t.usecase.font_color = param.value.clone(),
            "usecaseFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.usecase.font_size = v;
                }
            }
            "usecaseFontStyle" => t.usecase.font_style = param.value.clone(),
            "usecaseArrowColor" => t.usecase.arrow_color = param.value.clone(),
            "usecaseArrowFontColor" => t.usecase.arrow_font_color = param.value.clone(),
            "usecaseArrowFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.usecase.arrow_font_size = v;
                }
            }

            // ── Actor ─────────────────────────────────────────────────────────
            "actorBackgroundColor" => {
                t.actor.background = param.value.clone();
                t.sequence.actor_background = param.value.clone();
            }
            "actorBorderColor" => {
                t.actor.border = param.value.clone();
                t.sequence.actor_border = param.value.clone();
            }
            "actorFontColor" => {
                t.actor.font_color = param.value.clone();
                t.sequence.actor_font_color = param.value.clone();
            }
            "actorFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.actor.font_size = v;
                    t.sequence.actor_font_size = v;
                }
            }
            "actorFontStyle" => {
                t.actor.font_style = param.value.clone();
                t.sequence.actor_font_style = param.value.clone();
            }

            // ── Package ───────────────────────────────────────────────────────
            "packageBackgroundColor" => t.package.background = param.value.clone(),
            "packageBorderColor" => t.package.border = param.value.clone(),
            "packageBorderThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.package.border_thickness = v;
                }
            }
            "packageFontColor" => t.package.font_color = param.value.clone(),
            "packageFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.package.font_size = v;
                }
            }
            "packageFontName" => t.package.font_name = param.value.clone(),
            "packageFontStyle" => t.package.font_style = param.value.clone(),
            "packageStyle" => t.package.style = param.value.clone(),

            // ── Deployment node types ─────────────────────────────────────────
            "nodeBackgroundColor" => t.deployment.node_background = param.value.clone(),
            "nodeBorderColor" => t.deployment.node_border = param.value.clone(),
            "nodeFontColor" => t.deployment.node_font_color = param.value.clone(),
            "nodeFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.deployment.node_font_size = v;
                }
            }
            "nodeFontStyle" => t.deployment.node_font_style = param.value.clone(),
            "databaseBackgroundColor" => t.deployment.database_background = param.value.clone(),
            "databaseBorderColor" => t.deployment.database_border = param.value.clone(),
            "databaseFontColor" => t.deployment.database_font_color = param.value.clone(),
            "databaseFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.deployment.database_font_size = v;
                }
            }
            "databaseFontStyle" => t.deployment.database_font_style = param.value.clone(),
            "cloudBackgroundColor" => t.deployment.cloud_background = param.value.clone(),
            "cloudBorderColor" => t.deployment.cloud_border = param.value.clone(),
            "cloudFontColor" => t.deployment.cloud_font_color = param.value.clone(),
            "cloudFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.deployment.cloud_font_size = v;
                }
            }
            "cloudFontStyle" => t.deployment.cloud_font_style = param.value.clone(),
            "storageBackgroundColor" => t.deployment.storage_background = param.value.clone(),
            "storageBorderColor" => t.deployment.storage_border = param.value.clone(),
            "queueBackgroundColor" => t.deployment.queue_background = param.value.clone(),
            "queueBorderColor" => t.deployment.queue_border = param.value.clone(),
            "boundaryBackgroundColor" => t.deployment.boundary_background = param.value.clone(),
            "boundaryBorderColor" => t.deployment.boundary_border = param.value.clone(),
            "controlBackgroundColor" => t.deployment.control_background = param.value.clone(),
            "controlBorderColor" => t.deployment.control_border = param.value.clone(),
            "entityBackgroundColor" => t.deployment.entity_background = param.value.clone(),
            "entityBorderColor" => t.deployment.entity_border = param.value.clone(),
            "collectionsBackgroundColor" => {
                t.deployment.collections_background = param.value.clone();
            }
            "collectionsBorderColor" => t.deployment.collections_border = param.value.clone(),
            "frameBackgroundColor" => t.deployment.frame_background = param.value.clone(),
            "frameBorderColor" => t.deployment.frame_border = param.value.clone(),
            "frameFontColor" => t.deployment.frame_font_color = param.value.clone(),
            "frameFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.deployment.frame_font_size = v;
                }
            }
            "frameFontStyle" => t.deployment.frame_font_style = param.value.clone(),
            "rectangleBackgroundColor" => t.deployment.rectangle_background = param.value.clone(),
            "rectangleBorderColor" => t.deployment.rectangle_border = param.value.clone(),
            "rectangleFontColor" => t.deployment.rectangle_font_color = param.value.clone(),
            "rectangleFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.deployment.rectangle_font_size = v;
                }
            }
            "rectangleFontStyle" => t.deployment.rectangle_font_style = param.value.clone(),
            "folderBackgroundColor" => t.deployment.folder_background = param.value.clone(),
            "folderBorderColor" => t.deployment.folder_border = param.value.clone(),
            "folderFontColor" => t.deployment.folder_font_color = param.value.clone(),
            "folderFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.deployment.folder_font_size = v;
                }
            }
            "folderFontStyle" => t.deployment.folder_font_style = param.value.clone(),

            // ── Namespace (maps to package-like styling) ─────────────────────
            "namespaceBackgroundColor" => t.namespace.background = param.value.clone(),
            "namespaceBorderColor" => t.namespace.border = param.value.clone(),
            "namespaceFontColor" => t.namespace.font_color = param.value.clone(),
            "namespaceFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.namespace.font_size = v;
                }
            }
            "namespaceFontStyle" => t.namespace.font_style = param.value.clone(),

            // ── Object (maps to class style) ─────────────────────────────────
            "objectBackgroundColor" => t.class.class_background = param.value.clone(),
            "objectBorderColor" => t.class.border_color = param.value.clone(),
            "objectFontColor" => t.class.font_color = param.value.clone(),
            "objectFontSize" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.font_size = v;
                }
            }
            "objectFontStyle" => t.class.font_style = param.value.clone(),
            "objectFontName" => t.class.font_name = param.value.clone(),
            "objectBorderThickness" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.class.border_thickness = v;
                }
            }
            "objectArrowColor" => t.class.arrow_color = param.value.clone(),

            // ── WBS ──────────────────────────────────────────────────────────
            "wbsBackgroundColor" => t.wbs.background = param.value.clone(),
            "wbsBorderColor" => t.wbs.border = param.value.clone(),

            // ── Mindmap ──────────────────────────────────────────────────────
            "mindmapBackgroundColor" => t.mindmap.background = param.value.clone(),
            "mindmapBorderColor" => t.mindmap.border = param.value.clone(),

            // ── Sequence aliases (PascalCase handled by normalize_key) ───────
            "sequenceResponseMessageBelowArrow" => {
                t.sequence.response_message_below_arrow = param.value.to_lowercase() == "true";
            }

            // ── Global alias: `roundcorner` (all lowercase) ──────────────────
            "roundcorner" => {
                if let Ok(v) = param.value.parse::<f64>() {
                    t.global.round_corner = v;
                }
            }

            // ── Block-level skinparam prefixes (parsed as key when followed
            //    by `{`). These are no-ops at the match level; the parser
            //    handles block expansion. ──────────────────────────────────────
            "class" | "state" | "usecase" | "component" | "activity" | "sequence" | "node"
            | "database" | "cloud" | "rectangle" | "queue" | "note" | "stereotype"
            | "interface" | "participant" | "actor" | "package" | "object" | "swimlane"
            | "storage" | "boundary" | "control" | "entity" | "collections" | "frame"
            | "folder" | "namespace" | "wbs" | "mindmap" => {}

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

    #[test]
    fn activity_skinparams() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "activityBarColor".into(),
                value: "gray".into(),
            },
            SkinParam {
                key: "activityRoundCorner".into(),
                value: "15".into(),
            },
            SkinParam {
                key: "activityDiamondBackgroundColor".into(),
                value: "#FFEECC".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.activity.bar_color, "gray");
        assert_eq!(t.activity.round_corner, 15.0);
        assert_eq!(t.activity.decision_background, "#FFEECC");
    }

    #[test]
    fn class_skinparams() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "classRoundCorner".into(),
                value: "10".into(),
            },
            SkinParam {
                key: "classBorderThickness".into(),
                value: "2".into(),
            },
            SkinParam {
                key: "classAttributeFontColor".into(),
                value: "#333333".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.class.round_corner, 10.0);
        assert_eq!(t.class.border_thickness, 2.0);
        assert_eq!(t.class.attribute_font_color, "#333333");
    }

    #[test]
    fn state_skinparams() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "stateRoundCorner".into(),
                value: "8".into(),
            },
            SkinParam {
                key: "stateFontColor".into(),
                value: "navy".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.state.round_corner, 8.0);
        assert_eq!(t.state.font_color, "navy");
    }

    #[test]
    fn note_skinparams() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "noteBackgroundColor".into(),
                value: "#FFEECC".into(),
            },
            SkinParam {
                key: "noteRoundCorner".into(),
                value: "5".into(),
            },
            SkinParam {
                key: "noteFontColor".into(),
                value: "blue".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.note.background, "#FFEECC");
        assert_eq!(t.sequence.note_background, "#FFEECC");
        assert_eq!(t.note.round_corner, 5.0);
        assert_eq!(t.note.font_color, "blue");
    }

    #[test]
    fn deployment_skinparams() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "databaseBackgroundColor".into(),
                value: "#AACCFF".into(),
            },
            SkinParam {
                key: "queueBackgroundColor".into(),
                value: "#FFCCAA".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.deployment.database_background, "#AACCFF");
        assert_eq!(t.deployment.queue_background, "#FFCCAA");
    }

    #[test]
    fn global_arrow_color() {
        let theme = Theme::default();
        let params = vec![SkinParam {
            key: "arrowColor".into(),
            value: "red".into(),
        }];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.global.arrow_color, "red");
    }

    #[test]
    fn case_insensitive_first_char() {
        let theme = Theme::default();
        // PascalCase variant: ClassBackgroundColor -> classBackgroundColor
        let params = vec![SkinParam {
            key: "ClassBackgroundColor".into(),
            value: "#AABBCC".into(),
        }];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.class.class_background, "#AABBCC");
    }

    #[test]
    fn pascal_case_arrow_color() {
        let theme = Theme::default();
        let params = vec![SkinParam {
            key: "ArrowColor".into(),
            value: "green".into(),
        }];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.global.arrow_color, "green");
    }

    #[test]
    fn component_style_param() {
        let theme = Theme::default();
        let params = vec![SkinParam {
            key: "componentStyle".into(),
            value: "rectangle".into(),
        }];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.component.style, "rectangle");
    }

    #[test]
    fn roundcorner_all_lowercase() {
        let theme = Theme::default();
        let params = vec![SkinParam {
            key: "roundcorner".into(),
            value: "20".into(),
        }];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.global.round_corner, 20.0);
    }

    #[test]
    fn sequence_response_message_below_arrow() {
        let theme = Theme::default();
        let params = vec![SkinParam {
            key: "SequenceResponseMessageBelowArrow".into(),
            value: "true".into(),
        }];
        let t = apply_skinparams(&theme, &params);
        assert!(t.sequence.response_message_below_arrow);
    }

    #[test]
    fn object_skinparams_map_to_class() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "objectBackgroundColor".into(),
                value: "#CCDDEE".into(),
            },
            SkinParam {
                key: "objectBorderColor".into(),
                value: "#112233".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.class.class_background, "#CCDDEE");
        assert_eq!(t.class.border_color, "#112233");
    }

    #[test]
    fn deployment_font_and_border_params() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "nodeFontColor".into(),
                value: "#111111".into(),
            },
            SkinParam {
                key: "databaseFontSize".into(),
                value: "16".into(),
            },
            SkinParam {
                key: "cloudFontStyle".into(),
                value: "bold".into(),
            },
            SkinParam {
                key: "frameFontColor".into(),
                value: "#222222".into(),
            },
            SkinParam {
                key: "folderBackgroundColor".into(),
                value: "#AABBCC".into(),
            },
            SkinParam {
                key: "rectangleFontSize".into(),
                value: "12".into(),
            },
            SkinParam {
                key: "boundaryBorderColor".into(),
                value: "#333333".into(),
            },
            SkinParam {
                key: "controlBorderColor".into(),
                value: "#444444".into(),
            },
            SkinParam {
                key: "entityBorderColor".into(),
                value: "#555555".into(),
            },
            SkinParam {
                key: "collectionsBorderColor".into(),
                value: "#666666".into(),
            },
            SkinParam {
                key: "queueBorderColor".into(),
                value: "#777777".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.deployment.node_font_color, "#111111");
        assert_eq!(t.deployment.database_font_size, 16.0);
        assert_eq!(t.deployment.cloud_font_style, "bold");
        assert_eq!(t.deployment.frame_font_color, "#222222");
        assert_eq!(t.deployment.folder_background, "#AABBCC");
        assert_eq!(t.deployment.rectangle_font_size, 12.0);
        assert_eq!(t.deployment.boundary_border, "#333333");
        assert_eq!(t.deployment.control_border, "#444444");
        assert_eq!(t.deployment.entity_border, "#555555");
        assert_eq!(t.deployment.collections_border, "#666666");
        assert_eq!(t.deployment.queue_border, "#777777");
    }

    #[test]
    fn namespace_skinparams() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "NamespaceBackgroundColor".into(),
                value: "#DDEEFF".into(),
            },
            SkinParam {
                key: "namespaceFontSize".into(),
                value: "14".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.namespace.background, "#DDEEFF");
        assert_eq!(t.namespace.font_size, 14.0);
    }

    #[test]
    fn wbs_and_mindmap_skinparams() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "wbsBackgroundColor".into(),
                value: "#AABB11".into(),
            },
            SkinParam {
                key: "wbsBorderColor".into(),
                value: "#CC2233".into(),
            },
            SkinParam {
                key: "mindmapBackgroundColor".into(),
                value: "#44EEFF".into(),
            },
            SkinParam {
                key: "mindmapBorderColor".into(),
                value: "#556677".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.wbs.background, "#AABB11");
        assert_eq!(t.wbs.border, "#CC2233");
        assert_eq!(t.mindmap.background, "#44EEFF");
        assert_eq!(t.mindmap.border, "#556677");
    }

    #[test]
    fn title_and_arrow_font_style_params() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "titleFontSize".into(),
                value: "20".into(),
            },
            SkinParam {
                key: "titleFontStyle".into(),
                value: "bold".into(),
            },
            SkinParam {
                key: "arrowFontStyle".into(),
                value: "italic".into(),
            },
            SkinParam {
                key: "svgLinkTarget".into(),
                value: "_blank".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.global.title_font_size, 20.0);
        assert_eq!(t.global.title_font_style, "bold");
        assert_eq!(t.global.arrow_font_style, "italic");
        assert_eq!(t.global.svg_link_target, "_blank");
    }

    #[test]
    fn sequence_extra_params() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "BoxPadding".into(),
                value: "10".into(),
            },
            SkinParam {
                key: "SequenceGroupBodyBackgroundColor".into(),
                value: "#EEEEFF".into(),
            },
            SkinParam {
                key: "SequenceLifeLineStrategy".into(),
                value: "solid".into(),
            },
            SkinParam {
                key: "SequenceDividerBorderThickness".into(),
                value: "2".into(),
            },
            SkinParam {
                key: "ParticipantPadding".into(),
                value: "15".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.sequence.box_padding, 10.0);
        assert_eq!(t.sequence.group_body_background, "#EEEEFF");
        assert_eq!(t.sequence.lifeline_strategy, "solid");
        assert_eq!(t.sequence.divider_border_thickness, 2.0);
        assert_eq!(t.sequence.participant_padding, 15.0);
    }

    #[test]
    fn activity_diamond_border_color() {
        let theme = Theme::default();
        let params = vec![SkinParam {
            key: "activityDiamondBorderColor".into(),
            value: "#AABBCC".into(),
        }];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.activity.diamond_border_color, "#AABBCC");
    }

    #[test]
    fn stereotype_c_background_color() {
        let theme = Theme::default();
        let params = vec![SkinParam {
            key: "stereotypeCBackgroundColor".into(),
            value: "#DD8800".into(),
        }];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.class.stereotype_c_background, "#DD8800");
    }

    #[test]
    fn class_stereotype_pascal_case_aliases() {
        let theme = Theme::default();
        let params = vec![
            SkinParam {
                key: "ClassStereotypeFontColor".into(),
                value: "#AA1122".into(),
            },
            SkinParam {
                key: "ClassStereotypeFontSize".into(),
                value: "18".into(),
            },
            SkinParam {
                key: "ClassStereotypeFontStyle".into(),
                value: "italic".into(),
            },
        ];
        let t = apply_skinparams(&theme, &params);
        assert_eq!(t.class.stereotype_font_color, "#AA1122");
        assert_eq!(t.class.stereotype_font_size, 18.0);
        assert_eq!(t.class.stereotype_font_style, "italic");
    }
}
