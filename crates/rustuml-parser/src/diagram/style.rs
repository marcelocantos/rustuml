// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Renderer-neutral representation of PlantUML's ordered style mutations.

use serde::{Deserialize, Serialize};

/// Sparse color channels produced by PlantUML's shared `ColorParser` and
/// `Colors` model.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlantUmlColors {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrow: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_style: Option<PlantUmlLineStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadowing: Option<bool>,
}

impl PlantUmlColors {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// The channel receiving an unkeyed color in `Colors(data, set, mainType)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlantUmlColorType {
    Text,
    Line,
    Back,
    Header,
    Arrow,
}

/// A line-stroke directive retained independently of line color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlantUmlLineStyle {
    Dashed,
    Dotted,
    Bold,
}

/// The ordered sparse declarations that make up a diagram's style cascade.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleProgram {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub declarations: Vec<StyleDeclaration>,
}

impl StyleProgram {
    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }
}

/// One property mutation against a canonical PlantUML style signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleDeclaration {
    /// Canonical `SName` tokens in the declaration signature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selector: Vec<String>,
    /// Canonical stereotype identities attached to the signature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stereotypes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub star: bool,
    /// Canonical `PName` identity.
    pub property: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "StyleScheme::is_regular")]
    pub scheme: StyleScheme,
    /// User-source line at which this declaration became visible.
    ///
    /// Theme declarations use the `!theme` directive line rather than the
    /// relocated expansion body, so this can be compared directly with the
    /// existing `source_line` on parsed entities and links.
    pub source_line: usize,
    /// Monotonic declaration order in the logically executed source stream.
    pub epoch: u64,
    /// Java-compatible property priority, including stereotype specificity.
    pub priority: i64,
    pub origin: StyleOrigin,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StyleScheme {
    #[default]
    Regular,
    Dark,
}

impl StyleScheme {
    fn is_regular(&self) -> bool {
        *self == Self::Regular
    }
}

/// The syntax and source that introduced a style declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StyleOrigin {
    UserStyle,
    ThemeStyle { theme: String },
    UserSkinParam,
    ThemeSkinParam { theme: String },
}

fn is_false(value: &bool) -> bool {
    !value
}
