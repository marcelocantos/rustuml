// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Activity diagram parser (v3 / new syntax).

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::activity::*;

pub fn parse_activity(lines: &[String]) -> Result<ActivityDiagram, ParseError> {
    // Detect legacy v1 syntax by looking for `(*)` or `===NAME===` markers.
    let is_legacy = lines.iter().any(|l| {
        let t = l.trim();
        t == "(*)"
            || t.starts_with("(*) ")
            || t.ends_with(" (*)")
            || (t.starts_with("===") && t.ends_with("==="))
    });

    if is_legacy {
        return parse_legacy_activity(lines);
    }

    let mut parser = ActivityParser::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if parser.pending_note.is_some() {
                parser.accumulate_note_line("");
            }
            continue;
        }
        parser.parse_line(i + 1, trimmed)?;
    }
    Ok(parser.finish())
}

/// Parse legacy (v1) activity syntax.
///
/// Legacy syntax uses:
/// - `(*) --> "Node"` for the start arrow
/// - `"Node" --> (*)`  for the end arrow
/// - `"Node" --> "Other"` for transitions
/// - `"Node" -direction-> "Other"` for directed arrows
/// - `if "condition" then` / `else` / `endif` for decisions
/// - `-->[label] "Node"` for labelled arrows
/// - `===NAME===` for fork/join bars
/// - `note left/right: text` for notes
fn parse_legacy_activity(lines: &[String]) -> Result<ActivityDiagram, ParseError> {
    static RE_ARROW: LazyLock<Regex> = LazyLock::new(|| {
        // "Source" -[direction]-> "Target"  or  (*) --> "Target"  or  "Source" --> (*)
        Regex::new(r#"^(?:"([^"]+)"|\(\*\))\s+-[^>]*->\s+(?:"([^"]+)"|\(\*\))$"#).unwrap()
    });
    static RE_LABELLED_ARROW: LazyLock<Regex> = LazyLock::new(|| {
        // -->[label] "Target" or -->[label] (*)
        Regex::new(r#"^-+>\[([^\]]*)\]\s+(?:"([^"]+)"|\(\*\))$"#).unwrap()
    });
    static RE_BARE_ARROW: LazyLock<Regex> = LazyLock::new(|| {
        // --> "Target" or --> (*)  (no label, used inside if/else blocks)
        Regex::new(r#"^-+>\s+(?:"([^"]+)"|\(\*\))$"#).unwrap()
    });
    // "Source" --> ===FORKBAR=== or ===FORKBAR=== --> "Target"
    static RE_TO_FORK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^(?:"([^"]+)"|\(\*\))\s+-[^>]*->\s+===([^=]+)===$"#).unwrap()
    });
    static RE_FROM_FORK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^===([^=]+)===\s+-[^>]*->\s+(?:"([^"]+)"|\(\*\))$"#).unwrap()
    });
    static RE_IF: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^if\s+"([^"]+)"\s+then$"#).unwrap());
    static RE_NOTE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^note\s+(left|right)\s*:\s*(.+)$").unwrap());
    static RE_FORK_BAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^===([^=]+)===$").unwrap());
    static RE_PARTITION: LazyLock<Regex> = LazyLock::new(|| {
        // partition "Name" {  or  partition Name {
        Regex::new(r#"^partition\s+(?:"([^"]+)"|(\S+))\s*\{?\s*$"#).unwrap()
    });

    let mut meta = DiagramMeta::default();
    let mut steps: Vec<ActivityStep> = Vec::new();
    let mut seen_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Track fork bars by name: false = not yet seen (will be Fork), true = already seen.
    let mut fork_bars: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    // Nesting depth of partition/group blocks (for legacy `}` handling).
    let mut partition_depth: usize = 0;

    for line in lines {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }

        // Title.
        if let Some(rest) = t.strip_prefix("title ") {
            meta.title = Some(rest.trim().to_string());
            continue;
        }

        // Partition block: `partition "Name" {` or `partition Name {`
        if let Some(caps) = RE_PARTITION.captures(t) {
            let name = caps
                .get(1)
                .or(caps.get(2))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            steps.push(ActivityStep::Partition(PartitionBlock {
                name,
                color: None,
            }));
            partition_depth += 1;
            continue;
        }

        // Closing brace: ends the innermost partition block.
        if t == "}" {
            if partition_depth > 0 {
                steps.push(ActivityStep::EndPartition);
                partition_depth -= 1;
            }
            continue;
        }

        // Note.
        if let Some(caps) = RE_NOTE.captures(t) {
            let position = if &caps[1] == "left" {
                NotePosition::Left
            } else {
                NotePosition::Right
            };
            steps.push(ActivityStep::Note(NoteBlock {
                text: caps[2].trim().to_string(),
                color: None,
                position,
            }));
            continue;
        }

        // if "condition" then
        if let Some(caps) = RE_IF.captures(t) {
            steps.push(ActivityStep::If(IfBlock {
                condition: caps[1].to_string(),
                then_label: None,
            }));
            continue;
        }

        if t == "else" {
            steps.push(ActivityStep::Else(None));
            continue;
        }

        if t == "endif" {
            steps.push(ActivityStep::EndIf);
            continue;
        }

        // Fork bar ===NAME===
        if let Some(caps) = RE_FORK_BAR.captures(t) {
            let name = caps[1].trim().to_string();
            let already_seen = fork_bars.contains_key(&name);
            fork_bars.insert(name, true);
            if already_seen {
                steps.push(ActivityStep::EndFork);
            } else {
                steps.push(ActivityStep::Fork);
            }
            continue;
        }

        // "Node" --> ===FORKBAR===  (arrow into a fork bar)
        if let Some(caps) = RE_TO_FORK.captures(t) {
            // Source is a named node or (*).
            if let Some(src) = caps.get(1) {
                let src = src.as_str().to_string();
                if !seen_nodes.contains(&src) {
                    seen_nodes.insert(src.clone());
                    steps.push(ActivityStep::Action(src));
                }
            } else if !steps.iter().any(|s| matches!(s, ActivityStep::Start)) {
                steps.push(ActivityStep::Start);
            }
            // Emit Fork/EndFork for the bar.
            let bar_name = caps[2].trim().to_string();
            let already_seen = fork_bars.contains_key(&bar_name);
            fork_bars.insert(bar_name, true);
            if already_seen {
                steps.push(ActivityStep::EndFork);
            } else {
                steps.push(ActivityStep::Fork);
            }
            continue;
        }

        // ===FORKBAR=== --> "Node"  (arrow out of a fork bar)
        if let Some(caps) = RE_FROM_FORK.captures(t) {
            let bar_name = caps[1].trim().to_string();
            let already_seen = fork_bars.contains_key(&bar_name);
            fork_bars.insert(bar_name, true);
            if already_seen {
                steps.push(ActivityStep::EndFork);
            } else {
                steps.push(ActivityStep::Fork);
            }
            // Emit target node.
            if let Some(tgt) = caps.get(2) {
                let tgt = tgt.as_str().to_string();
                if !seen_nodes.contains(&tgt) {
                    seen_nodes.insert(tgt.clone());
                    steps.push(ActivityStep::Action(tgt));
                }
            } else {
                if !steps
                    .iter()
                    .any(|s| matches!(s, ActivityStep::End | ActivityStep::Stop))
                {
                    steps.push(ActivityStep::End);
                }
            }
            continue;
        }

        // (*) --> (*) (self-contained start-end)
        if t == "(*) --> (*)" {
            steps.push(ActivityStep::Start);
            steps.push(ActivityStep::End);
            continue;
        }

        // Labelled arrow: -->[label] "Target"
        if let Some(caps) = RE_LABELLED_ARROW.captures(t) {
            let label = caps[1].trim();
            if !label.is_empty() {
                steps.push(ActivityStep::Arrow(ArrowStep {
                    dashed: false,
                    color: None,
                    label: Some(label.to_string()),
                }));
            }
            // Emit the target node if it's a named node (not (*)).
            if let Some(target) = caps.get(2) {
                let node = target.as_str().to_string();
                if !seen_nodes.contains(&node) {
                    seen_nodes.insert(node.clone());
                    steps.push(ActivityStep::Action(node));
                }
            } else {
                // --> (*): end node.
                steps.push(ActivityStep::End);
            }
            continue;
        }

        // Bare arrow inside if/else: --> "Target"
        if let Some(caps) = RE_BARE_ARROW.captures(t) {
            if let Some(target) = caps.get(1) {
                let node = target.as_str().to_string();
                if !seen_nodes.contains(&node) {
                    seen_nodes.insert(node.clone());
                    steps.push(ActivityStep::Action(node));
                }
            } else {
                steps.push(ActivityStep::End);
            }
            continue;
        }

        // "Source" --> "Target" / (*) --> "Target" / "Source" --> (*)
        if let Some(caps) = RE_ARROW.captures(t) {
            let source = caps.get(1).map(|m| m.as_str());
            let target = caps.get(2).map(|m| m.as_str());

            // Source = (*): start node.
            if source.is_none() && !steps.iter().any(|s| matches!(s, ActivityStep::Start)) {
                steps.push(ActivityStep::Start);
            }
            // Emit source node action if not yet seen.
            if let Some(src) = source {
                let src = src.to_string();
                if !seen_nodes.contains(&src) {
                    seen_nodes.insert(src.clone());
                    steps.push(ActivityStep::Action(src));
                }
            }
            // Target = (*): end node.
            if target.is_none() {
                // Only add End if not already present.
                if !steps
                    .iter()
                    .any(|s| matches!(s, ActivityStep::End | ActivityStep::Stop))
                {
                    steps.push(ActivityStep::End);
                }
            } else if let Some(tgt) = target {
                let tgt = tgt.to_string();
                if !seen_nodes.contains(&tgt) {
                    seen_nodes.insert(tgt.clone());
                    steps.push(ActivityStep::Action(tgt));
                }
            }
            continue;
        }

        // Fork again (parallel branch): node --> === and === --> node both handled above.
        // Additional arrow from fork bar to node.
        // (already handled by RE_ARROW)
    }

    Ok(ActivityDiagram { meta, steps })
}

struct PendingNote {
    position: NotePosition,
    color: Option<String>,
    lines: Vec<String>,
}

struct ActivityParser {
    meta: DiagramMeta,
    steps: Vec<ActivityStep>,
    pending_note: Option<PendingNote>,
    pending_meta: Option<&'static str>, // "header", "footer", "legend", "caption"
    pending_meta_lines: Vec<String>,
    /// When an action ends with `\` (output connector), this holds the partial
    /// text so the next line can be appended to it.
    continuation_text: Option<String>,
    /// When an action ends with a non-`;`/non-`\` connector (`|`, `]`, `/`,
    /// `>`, `<`), the immediately following action should keep its `:` prefix.
    next_action_keep_colon: bool,
    /// True when we are inside a `skinparam <type> {` block.
    in_skinparam_block: bool,
}

impl ActivityParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            steps: Vec::new(),
            pending_note: None,
            pending_meta: None,
            pending_meta_lines: Vec::new(),
            continuation_text: None,
            next_action_keep_colon: false,
            in_skinparam_block: false,
        }
    }

    fn finish(self) -> ActivityDiagram {
        ActivityDiagram {
            meta: self.meta,
            steps: self.steps,
        }
    }

    fn accumulate_note_line(&mut self, line: &str) {
        if let Some(ref mut pn) = self.pending_note {
            pn.lines.push(line.to_string());
        }
    }

    fn flush_pending_meta(&mut self) {
        if let Some(kind) = self.pending_meta.take() {
            let text = self.pending_meta_lines.join("\n").trim().to_string();
            self.pending_meta_lines.clear();
            if !text.is_empty() {
                match kind {
                    "header" => self.meta.header = Some(text),
                    "footer" => self.meta.footer = Some(text),
                    "legend" => self.meta.legend = Some(text),
                    "caption" => self.meta.caption = Some(text),
                    _ => {}
                }
            }
        }
    }

    fn flush_pending_note(&mut self) {
        if let Some(pn) = self.pending_note.take() {
            let text = pn.lines.join("\n");
            let text = text.trim().to_string();
            if !text.is_empty() {
                self.steps.push(ActivityStep::Note(NoteBlock {
                    text,
                    color: pn.color,
                    position: pn.position,
                }));
            }
        }
    }

    fn parse_line(&mut self, _line_num: usize, line: &str) -> Result<(), ParseError> {
        // Inside a skinparam block: collect nested `Key Value` entries until `}`.
        if self.in_skinparam_block {
            if line == "}" {
                self.in_skinparam_block = false;
            } else {
                // `Key Value` inside block — prefix is stored in pending_meta_lines[0].
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    let prefix = self.pending_meta_lines.first().cloned().unwrap_or_default();
                    let key = if prefix.is_empty() {
                        parts[0].to_string()
                    } else {
                        format!("{}{}", prefix, parts[0])
                    };
                    self.meta.skinparams.push(crate::diagram::SkinParam {
                        key,
                        value: parts[1].trim().to_string(),
                    });
                }
            }
            return Ok(());
        }

        // Handle multi-line meta blocks (header/footer/legend/caption).
        if self.pending_meta.is_some() {
            let end_kw = format!("end {}", self.pending_meta.unwrap());
            if line == end_kw {
                self.flush_pending_meta();
            } else {
                self.pending_meta_lines.push(line.to_string());
            }
            return Ok(());
        }

        if self.pending_note.is_some() {
            if line == "end note" {
                self.flush_pending_note();
            } else {
                self.accumulate_note_line(line);
            }
            return Ok(());
        }

        // Handle output-connector continuation: the previous action ended with
        // `\`, so this entire line (stripped of its trailing terminator) is
        // appended to the pending text and emitted as one action.
        if let Some(partial) = self.continuation_text.take() {
            // Strip trailing action terminator only (keep leading `:` intact
            // so `:next action;` becomes `:next action` when appended).
            let appended = line.trim_end_matches(|c: char| ";|]/><\\".contains(c));
            let combined = format!("{}{}", partial, appended);
            self.steps.push(ActivityStep::Action(combined));
            // After a continuation, the next action preserves its `:` prefix.
            self.next_action_keep_colon = true;
            return Ok(());
        }

        match line {
            "start" => self.steps.push(ActivityStep::Start),
            "stop" => self.steps.push(ActivityStep::Stop),
            "end" => self.steps.push(ActivityStep::End),
            "endif" => self.steps.push(ActivityStep::EndIf),
            "endswitch" => self.steps.push(ActivityStep::EndSwitch),
            "fork" => self.steps.push(ActivityStep::Fork),
            "fork again" => self.steps.push(ActivityStep::ForkAgain),
            "end fork" => self.steps.push(ActivityStep::EndFork),
            "split" => self.steps.push(ActivityStep::Split),
            "split again" => self.steps.push(ActivityStep::SplitAgain),
            "end split" => self.steps.push(ActivityStep::EndSplit),
            "repeat" => self.steps.push(ActivityStep::Repeat),
            _ if line.starts_with("repeat :") => {
                // `repeat :label;` — push Repeat then parse the rest as an action.
                self.steps.push(ActivityStep::Repeat);
                let rest = line.strip_prefix("repeat ").unwrap_or("").trim();
                if !rest.is_empty() {
                    self.try_action(rest);
                }
            }
            "break" => self.steps.push(ActivityStep::Break),
            "detach" => self.steps.push(ActivityStep::Detach),
            "kill" => self.steps.push(ActivityStep::Kill),
            _ => {
                if !self.try_meta(line)
                    && !self.try_action(line)
                    && !self.try_deprecated_color_action(line)
                    && !self.try_arrow(line)
                    && !self.try_backward(line)
                    && !self.try_if(line)
                    && !self.try_elseif(line)
                    && !self.try_else(line)
                    && !self.try_while(line)
                    && !self.try_endwhile(line)
                    && !self.try_repeat_while(line)
                    && !self.try_switch(line)
                    && !self.try_case(line)
                    && !self.try_swimlane(line)
                    && !self.try_partition(line)
                    && !self.try_group(line)
                    && !self.try_note(line)
                {
                    // Silently ignore unknown lines.
                }
            }
        }
        Ok(())
    }

    fn try_meta(&mut self, line: &str) -> bool {
        static RE_TITLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^title\s+(.+)$").unwrap());
        static RE_HEADER: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^header\s+(.+)$").unwrap());
        static RE_FOOTER: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^footer\s+(.+)$").unwrap());
        static RE_CAPTION: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^caption\s+(.+)$").unwrap());
        // `skinparam key value`  or  `skinparam key {` (block start, ignored)
        static RE_SKINPARAM: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^skinparam\s+(\S+)\s+(.+)$").unwrap());

        if let Some(caps) = RE_TITLE.captures(line) {
            self.meta.title = Some(super::strip_title_quotes(&caps[1]).to_string());
            return true;
        }
        if let Some(caps) = RE_HEADER.captures(line) {
            self.meta.header = Some(caps[1].trim().to_string());
            return true;
        }
        if let Some(caps) = RE_FOOTER.captures(line) {
            self.meta.footer = Some(caps[1].trim().to_string());
            return true;
        }
        if let Some(caps) = RE_CAPTION.captures(line) {
            self.meta.caption = Some(caps[1].trim().to_string());
            return true;
        }
        if let Some(caps) = RE_SKINPARAM.captures(line) {
            let key = caps[1].trim().to_string();
            let value_raw = caps[2].trim();
            if value_raw.ends_with('{') {
                // Block form: `skinparam activity {` — enter skinparam block mode.
                // Store the prefix (key) so nested entries can be keyed correctly.
                self.in_skinparam_block = true;
                self.pending_meta_lines.clear();
                self.pending_meta_lines.push(key);
            } else {
                self.meta.skinparams.push(crate::diagram::SkinParam {
                    key,
                    value: value_raw.to_string(),
                });
            }
            return true;
        }
        // Block-start keywords (header/footer/legend/caption on their own line).
        match line {
            "header" => {
                self.pending_meta = Some("header");
                self.pending_meta_lines.clear();
                return true;
            }
            "footer" => {
                self.pending_meta = Some("footer");
                self.pending_meta_lines.clear();
                return true;
            }
            "legend" | "legend right" | "legend left" => {
                self.pending_meta = Some("legend");
                self.pending_meta_lines.clear();
                return true;
            }
            _ => {}
        }
        false
    }

    fn try_action(&mut self, line: &str) -> bool {
        // Match actions with various endings: ; | ] / > < \ (all PlantUML action terminators)
        // The ending char (except ;) is included in the display text as per PlantUML behavior.
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^:(.+?)([;|\]/>\\<])$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let text = caps[1].trim().to_string();
            let ending = &caps[2];

            if ending == "\\" {
                // Output connector: start a continuation. The partial text
                // (without `:` prefix) is held until the next line is seen.
                self.next_action_keep_colon = false;
                self.continuation_text = Some(text);
                return true;
            }

            let display = if ending == ";" {
                if self.next_action_keep_colon {
                    // Preserve the `:` prefix on this action.
                    self.next_action_keep_colon = false;
                    format!(":{}", text)
                } else {
                    text
                }
            } else {
                // Non-`;`/non-`\` ending: include the ending char, and signal
                // that the next action should keep its `:` prefix.
                self.next_action_keep_colon = true;
                format!("{}{}", text, ending)
            };
            self.steps.push(ActivityStep::Action(display));
            true
        } else {
            false
        }
    }

    fn try_deprecated_color_action(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(#(?:[0-9A-Fa-f]{6}|[A-Za-z]+)):(.+);$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let raw_color = caps[1].trim().to_string();
            let color = if raw_color.len() == 7
                && raw_color.starts_with('#')
                && raw_color[1..].chars().all(|c| c.is_ascii_hexdigit())
            {
                format!("#{}", raw_color[1..].to_uppercase())
            } else {
                raw_color
            };
            let text = caps[2].trim().to_string();
            self.steps
                .push(ActivityStep::DeprecatedColorAction(DeprecatedColorAction {
                    color,
                    text,
                }));
            true
        } else {
            false
        }
    }

    fn try_arrow(&mut self, line: &str) -> bool {
        // Matches: ->, -->, -[#color]->, -[#color]-->, optionally followed by label;
        // Group 1: full arrow, Group 2: color (optional), Group 3: extra dash (-> vs -->),
        // Group 4: label (optional)
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(-(?:\[([^\]]+)\])?(-?)>)\s*(.+)?$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let dashed = &caps[3] == "-";
            let color = caps.get(2).map(|m| {
                let s = m.as_str().trim();
                if s.starts_with('#')
                    && s.len() == 7
                    && s[1..].chars().all(|c| c.is_ascii_hexdigit())
                {
                    format!("#{}", s[1..].to_uppercase())
                } else {
                    s.to_string()
                }
            });
            let label = caps
                .get(4)
                .map(|m| {
                    let s = m.as_str().trim();
                    s.trim_end_matches(';').trim().to_string()
                })
                .filter(|s| !s.is_empty());
            self.steps.push(ActivityStep::Arrow(ArrowStep {
                dashed,
                color,
                label,
            }));
            true
        } else {
            false
        }
    }

    fn try_backward(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^backward\s*:(.+);$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps
                .push(ActivityStep::Backward(caps[1].trim().to_string()));
            true
        } else {
            false
        }
    }

    fn try_if(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^if\s*\((.+?)\)\s*then\s*(?:\((.+?)\))?$").unwrap());
        static RE_BARE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^if\s*\((.+?)\)\s*$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps.push(ActivityStep::If(IfBlock {
                condition: caps[1].trim().to_string(),
                then_label: caps.get(2).map(|m| m.as_str().trim().to_string()),
            }));
            true
        } else if let Some(caps) = RE_BARE.captures(line) {
            self.steps.push(ActivityStep::If(IfBlock {
                condition: caps[1].trim().to_string(),
                then_label: None,
            }));
            true
        } else {
            false
        }
    }

    fn try_elseif(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^elseif\s*\((.+?)\)\s*then\s*(?:\((.+?)\))?$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps.push(ActivityStep::ElseIf(ElseIfBranch {
                condition: caps[1].trim().to_string(),
                then_label: caps.get(2).map(|m| m.as_str().trim().to_string()),
            }));
            true
        } else {
            false
        }
    }

    fn try_else(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^else\s*(?:\((.+?)\))?$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let label = caps.get(1).map(|m| m.as_str().trim().to_string());
            self.steps.push(ActivityStep::Else(label));
            true
        } else {
            false
        }
    }

    fn try_while(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^while\s*\((.+?)\)\s*(?:is\s*\((.+?)\))?$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps.push(ActivityStep::While(WhileBlock {
                condition: caps[1].trim().to_string(),
                is_label: caps.get(2).map(|m| m.as_str().trim().to_string()),
            }));
            true
        } else {
            false
        }
    }

    fn try_endwhile(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^endwhile\s*(?:\((.+?)\))?$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let label = caps.get(1).map(|m| m.as_str().trim().to_string());
            self.steps.push(ActivityStep::EndWhile(label));
            true
        } else {
            false
        }
    }

    fn try_repeat_while(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^repeat\s*while\s*\((.+?)\)\s*(?:is\s*\((.+?)\))?\s*(?:not\s*\((.+?)\))?")
                .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            self.steps.push(ActivityStep::RepeatWhile(RepeatWhileBlock {
                condition: caps[1].trim().to_string(),
                is_label: caps.get(2).map(|m| m.as_str().trim().to_string()),
                not_label: caps.get(3).map(|m| m.as_str().trim().to_string()),
            }));
            true
        } else {
            false
        }
    }

    fn try_switch(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^switch\s*\((.+?)\)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps
                .push(ActivityStep::Switch(caps[1].trim().to_string()));
            true
        } else {
            false
        }
    }

    fn try_case(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^case\s*\(\s*(.+?)\s*\)$").unwrap());

        if line == "default" {
            self.steps.push(ActivityStep::Case("default".to_string()));
            return true;
        }
        if let Some(caps) = RE.captures(line) {
            self.steps
                .push(ActivityStep::Case(caps[1].trim().to_string()));
            true
        } else {
            false
        }
    }

    fn try_swimlane(&mut self, line: &str) -> bool {
        // Match |Name| or |#color|Name| (colored swimlane)
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\|(?:#[A-Za-z0-9]+\|)?([^|]+)\|$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps.push(ActivityStep::Swimlane(caps[1].to_string()));
            true
        } else {
            false
        }
    }

    fn try_partition(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^partition\s+(?:(#[A-Za-z0-9]+)\s+)?(?:"([^"]+)"|([A-Za-z_]\w*))\s*\{?"#)
                .unwrap()
        });

        if line == "}" {
            self.steps.push(ActivityStep::EndPartition);
            return true;
        }
        if let Some(caps) = RE.captures(line) {
            let color = caps.get(1).map(|m| m.as_str().to_string());
            let name = caps
                .get(2)
                .or(caps.get(3))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            self.steps
                .push(ActivityStep::Partition(PartitionBlock { name, color }));
            true
        } else {
            false
        }
    }

    fn try_group(&mut self, line: &str) -> bool {
        // `group [#color] "Name" {` — treated as a partition.
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^group\s+(?:(#[A-Za-z0-9]+)\s+)?(?:"([^"]+)"|([A-Za-z_]\w*))\s*\{?"#)
                .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let color = caps.get(1).map(|m| m.as_str().to_string());
            let name = caps
                .get(2)
                .or(caps.get(3))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            self.steps
                .push(ActivityStep::Partition(PartitionBlock { name, color }));
            true
        } else {
            false
        }
    }

    fn try_note(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(?:floating\s+)?note\s+(left|right)(?:\s+(#[0-9A-Fa-f]{6}|#\w+))?(?:\s*:\s*(.+))?$")
                .unwrap()
        });

        if !line.starts_with("note ") && !line.starts_with("floating note ") {
            return false;
        }

        if let Some(caps) = RE.captures(line) {
            let position = if &caps[1] == "left" {
                NotePosition::Left
            } else {
                NotePosition::Right
            };
            let color = caps.get(2).map(|m| {
                let s = m.as_str();
                if s.starts_with('#')
                    && s.len() == 7
                    && s[1..].chars().all(|c| c.is_ascii_hexdigit())
                {
                    format!("#{}", s[1..].to_uppercase())
                } else {
                    s.to_string()
                }
            });
            if let Some(inline_text) = caps.get(3) {
                let text = inline_text.as_str().trim().to_string();
                if !text.is_empty() {
                    self.steps.push(ActivityStep::Note(NoteBlock {
                        text,
                        color,
                        position,
                    }));
                }
            } else {
                self.pending_note = Some(PendingNote {
                    position,
                    color,
                    lines: Vec::new(),
                });
            }
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> ActivityDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_activity(&lines).unwrap()
    }

    #[test]
    fn basic_activity() {
        let d = parse("start\n:Step 1;\n:Step 2;\nstop");
        assert_eq!(d.steps.len(), 4);
        assert!(matches!(d.steps[0], ActivityStep::Start));
        assert!(matches!(d.steps[1], ActivityStep::Action(ref s) if s == "Step 1"));
        assert!(matches!(d.steps[3], ActivityStep::Stop));
    }

    #[test]
    fn if_else() {
        let d = parse(
            "start\nif (x > 0?) then (yes)\n  :positive;\nelse (no)\n  :negative;\nendif\nstop",
        );
        assert!(matches!(d.steps[1], ActivityStep::If(_)));
        if let ActivityStep::If(ref b) = d.steps[1] {
            assert_eq!(b.condition, "x > 0?");
            assert_eq!(b.then_label.as_deref(), Some("yes"));
        }
        assert!(matches!(d.steps[3], ActivityStep::Else(Some(ref s)) if s == "no"));
        assert!(matches!(d.steps[5], ActivityStep::EndIf));
    }

    #[test]
    fn elseif() {
        let d = parse(
            "start\nif (a?) then (yes)\n  :a;\nelseif (b?) then (maybe)\n  :b;\nelse (no)\n  :c;\nendif\nstop",
        );
        assert!(matches!(d.steps[3], ActivityStep::ElseIf(_)));
    }

    #[test]
    fn while_loop() {
        let d = parse("start\nwhile (cond?) is (yes)\n  :process;\nendwhile (no)\nstop");
        assert!(matches!(d.steps[1], ActivityStep::While(_)));
        if let ActivityStep::While(ref w) = d.steps[1] {
            assert_eq!(w.condition, "cond?");
            assert_eq!(w.is_label.as_deref(), Some("yes"));
        }
        assert!(matches!(d.steps[3], ActivityStep::EndWhile(Some(ref s)) if s == "no"));
    }

    #[test]
    fn repeat_loop() {
        let d = parse("start\nrepeat\n  :action;\nrepeat while (again?)\nstop");
        assert!(matches!(d.steps[1], ActivityStep::Repeat));
        assert!(matches!(d.steps[3], ActivityStep::RepeatWhile(ref b) if b.condition == "again?"));
    }

    #[test]
    fn fork() {
        let d = parse("start\nfork\n  :A;\nfork again\n  :B;\nend fork\nstop");
        assert!(matches!(d.steps[1], ActivityStep::Fork));
        assert!(matches!(d.steps[3], ActivityStep::ForkAgain));
        assert!(matches!(d.steps[5], ActivityStep::EndFork));
    }

    #[test]
    fn split() {
        let d = parse("start\nsplit\n  :A;\nsplit again\n  :B;\nend split\nstop");
        assert!(matches!(d.steps[1], ActivityStep::Split));
        assert!(matches!(d.steps[3], ActivityStep::SplitAgain));
        assert!(matches!(d.steps[5], ActivityStep::EndSplit));
    }

    #[test]
    fn switch_case() {
        let d = parse(
            "start\nswitch (test?)\ncase ( A )\n  :action A;\ncase ( B )\n  :action B;\nendswitch\nstop",
        );
        assert!(matches!(d.steps[1], ActivityStep::Switch(ref s) if s == "test?"));
        assert!(matches!(d.steps[2], ActivityStep::Case(ref s) if s == "A"));
        assert!(matches!(d.steps[4], ActivityStep::Case(ref s) if s == "B"));
        assert!(matches!(d.steps[6], ActivityStep::EndSwitch));
    }

    #[test]
    fn swimlanes() {
        let d = parse("|Lane1|\nstart\n:task1;\n|Lane2|\n:task2;\nstop");
        assert!(matches!(d.steps[0], ActivityStep::Swimlane(ref s) if s == "Lane1"));
        assert!(matches!(d.steps[3], ActivityStep::Swimlane(ref s) if s == "Lane2"));
    }

    #[test]
    fn swimlane_with_spaces() {
        let d = parse("|New Employee|\nstart\n:task;\nstop");
        assert!(matches!(d.steps[0], ActivityStep::Swimlane(ref s) if s == "New Employee"));
    }

    #[test]
    fn partition() {
        let d = parse("start\npartition Init {\n  :step1;\n}\nstop");
        assert!(matches!(d.steps[1], ActivityStep::Partition(ref s) if s.name == "Init"));
        assert!(matches!(d.steps[3], ActivityStep::EndPartition));
    }

    #[test]
    fn detach_and_kill() {
        let d = parse("start\ndetach");
        assert!(matches!(d.steps[1], ActivityStep::Detach));

        let d2 = parse("start\nkill");
        assert!(matches!(d2.steps[1], ActivityStep::Kill));
    }

    #[test]
    fn title_parsed() {
        let d = parse("title My Diagram\nstart\nstop");
        assert_eq!(d.meta.title.as_deref(), Some("My Diagram"));
    }

    #[test]
    fn deprecated_color_action() {
        let d = parse("start\n#blue:Do something;\nstop");
        assert!(
            matches!(d.steps[1], ActivityStep::DeprecatedColorAction(ref a) if a.text == "Do something" && a.color == "#blue")
        );
    }

    #[test]
    fn arrow_steps() {
        let d = parse("start\n:A;\n->\n:B;\n-->\n:C;\nstop");
        assert!(matches!(d.steps[2], ActivityStep::Arrow(ref a) if !a.dashed && a.label.is_none()));
        assert!(matches!(d.steps[4], ActivityStep::Arrow(ref a) if a.dashed && a.label.is_none()));
    }

    #[test]
    fn arrow_with_label() {
        let d = parse("start\n:A;\n--> label;\n:B;\nstop");
        if let ActivityStep::Arrow(ref a) = d.steps[2] {
            assert!(a.dashed);
            assert_eq!(a.label.as_deref(), Some("label"));
        } else {
            panic!("expected Arrow");
        }
    }

    #[test]
    fn backward_step() {
        let d = parse("start\nrepeat\n:action;\nbackward :retry;\nrepeat while (again?)\nstop");
        assert!(matches!(d.steps[3], ActivityStep::Backward(ref s) if s == "retry"));
    }
}
