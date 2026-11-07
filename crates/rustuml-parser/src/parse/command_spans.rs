// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Which post-TIM lines Java's `PSystemCommandFactory#getCandidate` never
//! offers as a command start, per diagram factory (pinned 71806a23).
//!
//! At a command-start line `getCandidate` tries every registered command in
//! `initCommandsList` order. A single-line command that accepts the line
//! selects it; a multiline command whose start pattern accepts it
//! (`OK_PARTIAL`) is completed by `#isMultilineCommandOk`, which adds the
//! following lines one at a time until `Command#isValid` is `OK` (or gives up
//! with `NOT_OK`, or reaches the end, and the next command is tried). Every
//! line of a completed block belongs to that command, so its body is never
//! itself a command start, whichever group (the sprite commands included) is
//! registered before or after it.
//!
//! The Rust family parsers do not model all of these multiline commands. The
//! [`walk`] below replays `getCandidate` over a registry holding, in each
//! factory's registration order, every multiline command the factory
//! registers, the `CommonCommands` sprite group at its position, and the
//! single-line commands that can accept a start line of a later multiline
//! command (and so take it first). Other single-line commands cannot accept
//! any modelled multiline start line: their grammars begin with a different
//! keyword or require a token (`:`, an arrow, a closing `}` or `"`) the
//! multiline starts exclude. A line no registry entry accepts is a one-line
//! command (or a syntax error, which is the family parser's business).

use std::sync::LazyLock;

use regex::Regex;

use super::sequence::{registered_regex, sequence_command_trim};

/// The Java diagram factory whose `initCommandsList` a family follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Factory {
    /// `descdiagram.DescriptionDiagramFactory` (component, deployment, usecase).
    Description,
    /// `statediagram.StateDiagramFactory`.
    State,
    /// `activitydiagram.ActivityDiagramFactory` (legacy activity syntax).
    LegacyActivity,
    /// `classdiagram.ClassDiagramFactory` (class and object diagrams).
    Class,
}

/// Lines a non-sprite multiline command owns (start, body and END).
pub(super) struct CommandSpans {
    owned: Vec<bool>,
}

impl CommandSpans {
    /// True when Java's `getCandidate` assigns `index` to a multiline command
    /// other than a sprite command, so the sprite group never sees it.
    pub(super) fn owned_by_other(&self, index: usize) -> bool {
        self.owned.get(index).copied().unwrap_or(false)
    }
}

enum End {
    /// END pattern on the trimmed last line (`CommandMultilines`,
    /// `CommandMultilines2`, Trim.BOTH).
    Pattern(Regex),
    /// `CommandCreateJson`: END, then `#finalVerification` keeps the block
    /// `OK_PARTIAL` until its body parses as JSON.
    Json(Regex),
    /// `CommandMultilinesBracket` (`CommandSkinParamMultilines`).
    Bracket,
}

enum Entry {
    Single(Regex),
    Multi {
        start: Regex,
        end: End,
        /// `CommandMultilines2#syntaxWithFinalBracket`.
        final_bracket: bool,
    },
    /// `CommonCommands#addCommonCommands2` sprite commands.
    Sprites,
}

fn single(pattern: &str) -> Entry {
    Entry::Single(registered_regex(pattern))
}

fn multi(start: &str, end: &str) -> Entry {
    Entry::Multi {
        start: registered_regex(start),
        end: End::Pattern(registered_regex(end)),
        final_bracket: false,
    }
}

// ---------------------------------------------------------------------------
// Regex fragments, built the way the Java helpers build them.

/// `StereotypePattern#optional`.
const STEREO_OPTIONAL: &str = r"[%s]*(?:(\<\<.+?\>\>))?[%s]*";
/// `StereotypePattern#optionalArchimate`.
const STEREO_OPTIONAL_ARCHIMATE: &str = r"[%s]*(?:(\<\<[-\w]+?\>\>))?[%s]*";
/// `Stereotag#pattern() + "?"`.
const STEREOTAGS: &str = r"((\$[^%s{}%g<>$]+)([%s]+(\$[^%s{}%g<>$]+))*)?";
/// `UrlBuilder#getRegexp` (QUOTED | ONLY_TOOLTIP | ONLY_TOOLTIP_AND_LABEL |
/// LINK_TOOLTIP_NOLABEL | LINK_WITH_OPTIONAL_TOOLTIP_WITH_OPTIONAL_LABEL).
pub(super) const URL_REGEXP: &str = concat!(
    r"\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]",
    r"|\[\[[%s]*\{(.*)\}[%s]*\]\]",
    r"|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]",
    r"|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]",
    r"|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]",
);

fn url_optional() -> String {
    // UrlBuilder.OPTIONAL = RegexOptional(RegexLeaf("(" + getRegexp() + ")")).
    format!("(?:({URL_REGEXP}))?")
}

/// `ColorParser#simpleColor` / `#exp1`: `(COLORS_REGEXP)?`.
///
/// PART2 ends each style with `(?:;|(?![\w;:.]))`. The regex crate has no
/// lookahead; a non-final style must still end with `;` (the next style
/// starts with a word character), so PART2 is written with `;` after every
/// style but the last, whose `;` is optional. That accepts the same lines
/// wherever the colour leaf is followed by end of line, whitespace, `{`, `[`
/// or `#`, which is every use below except `CommandCreateElementMultilines`
/// TYPE0, where `as` may follow directly (a composite colour glued to `as`).
fn color() -> &'static str {
    concat!(
        r"((?:#(?:\w+[-\\|/]?\w+;)?",
        r"(?:(?:text|back|header|line|line\.dashed|line\.dotted|line\.bold|shadowing)(?::\w+[-\\|/]?\w+)?;)*",
        r"(?:text|back|header|line|line\.dashed|line\.dotted|line\.bold|shadowing)(?::\w+[-\\|/]?\w+)?;?)",
        r"|(?:#\w+[-\\|/]?\w+))?"
    )
}

/// `CommandCreateElementFull.ALL_TYPES`.
const ALL_TYPES: &str = "person|artifact|actor/|actor|folder|card|file|package|rectangle|hexagon|label|node|frame|cloud|action|process|database|queue|stack|storage|agent|usecase/|usecase|component|boundary|control|entity|interface|circle|collections|port|portin|portout";
/// `CommandCreateElementFull.CODE_CORE`.
const ELEMENT_CODE_CORE: &str =
    r"[%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]";
/// `CommandCreateElementFull.DISPLAY_CORE`.
const ELEMENT_DISPLAY_CORE: &str = r"[%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]";

/// `NameAndCodeParser#nameAndCode`.
const NAME_AND_CODE: &str = r"(?:[%g]([^%g]*)[%g][%s]+as[%s]+([^%s{}%g<>]+)|([^%s{}%g<>]+)[%s]+as[%s]+[%g]([^%g]*)[%g]|([^%s{}%g<>]+)|[%g]([^%g]+)[%g])";

/// `GenericRegexProducer.PATTERN` (`getGenericRegex(4)`).
fn generic_pattern() -> String {
    let mut level = r"(?:[^\<\>/]|\<\>)*".to_string();
    for _ in 0..4 {
        level = format!(r"(?:[^\<\>/]|\<{level}\>)*");
    }
    format!(r"[^\<\>/]{level}")
}

/// `NameAndCodeParser#nameAndCodeForClassWithGeneric`.
fn name_and_code_for_class_with_generic() -> String {
    let generic = generic_pattern();
    let display = format!(r"[%g](.+?)(?:\<({generic})\>)?[%g]");
    let code = r"[^%s{}%g<>]+";
    format!(
        r"(?:{display}[%s]+as[%s]+({code})|({code})[%s]+as[%s]+{display}|({code})|[%g]([^%g]+)[%g])"
    )
}

// ---------------------------------------------------------------------------
// Commands shared by several factories.

/// `CommonCommands#addTitleCommands`, in order. The patterns are the ones in
/// the sequence inventory (REGISTERED_SEQUENCE_COMMANDS 0-11);
/// `title_group_matches_sequence_registry` keeps them identical.
const TITLE_GROUP: [(&str, &str, Option<&str>); 12] = [
    (
        "CommandTitle",
        r"^title(?:[%s]*:[%s]*|[%s]+)(?:[%g](.*)[%g]|(.*[%pLN_.].*))$",
        None,
    ),
    (
        "CommandMainframe",
        r"^mainframe(?:[%s]*:[%s]*|[%s]+)(.*[%pLN_.].*)$",
        None,
    ),
    (
        "CommandCaption",
        r"^caption(?:[%s]*:[%s]*|[%s]+)(?:[%g](.*)[%g]|(.*[%pLN_.].*))$",
        None,
    ),
    (
        "CommandMultilinesCaption",
        r"^caption$",
        Some(r"^end[%s]?caption$"),
    ),
    (
        "CommandMultilinesTitle",
        r"^title$",
        Some(r"^end[%s]?title$"),
    ),
    (
        "CommandMultilinesLegend",
        r"^legend(?:[%s]+(top|bottom))?(?:[%s]+(left|right|center))?$",
        Some(r"^end[%s]?legend$"),
    ),
    (
        "CommandLegend",
        r"^legend(?:[%s]*:[%s]*|[%s]+)(?:[%g](.*)[%g]|(.*[%pLN_.].*))$",
        None,
    ),
    (
        "CommandFooter",
        r"^(?:(left|right|center))?[%s]*footer(?:[%s]*:[%s]*|[%s]+)(?:[%g](.*)[%g]|(.*[%pLN_.].*))$",
        None,
    ),
    (
        "CommandMultilinesFooter",
        r"^(?:(left|right|center)?[%s]*)footer$",
        Some(r"^end[%s]?footer$"),
    ),
    (
        "CommandHeader",
        r"^(?:(left|right|center))?[%s]*header(?:[%s]*:[%s]*|[%s]+)(?:[%g](.*)[%g]|(.*[%pLN_.].*))$",
        None,
    ),
    (
        "CommandMultilinesHeader",
        r"^(?:(left|right|center)?[%s]*)header$",
        Some(r"^end[%s]?header$"),
    ),
    (
        "CommandNamespaceSeparator",
        r"^set[%s]+(?:separator|namespaceseparator)[%s]+(\S+)$",
        None,
    ),
];

fn title_commands(registry: &mut Vec<Entry>) {
    for (_, start, end) in TITLE_GROUP {
        registry.push(match end {
            Some(end) => multi(start, end),
            None => single(start),
        });
    }
}

/// The multiline commands of `CommonCommands#addCommonCommands2`, with the
/// sprite group at its position. Its single-line commands (CommandNope,
/// CommandPragma, CommandSkinParam, ...) are disjoint from every modelled
/// multiline start (CommandSkinParam's value excludes `{`).
fn common_commands2(registry: &mut Vec<Entry>) {
    registry.push(Entry::Multi {
        start: registered_regex(r"^skinparam[%s]*(?:[%s]+([\w.]*(?:\<\<.*\>\>)?[\w.]*))?[%s]*\{$"),
        end: End::Bracket,
        final_bracket: false,
    });
    registry.push(Entry::Sprites);
    // CommandStyleMultilinesCSS.
    registry.push(multi(r"^\<style\>$", r"^[%s]*\</?style\>[%s]*$"));
}

/// `CommandFactoryNote#createMultiLine` (`note as CODE`).
fn note_multi() -> Entry {
    multi(
        &format!(
            r"^note[%s]+as[%s]+([%pLN_.]+)[%s]*{STEREOTAGS}{STEREO_OPTIONAL}{}$",
            color()
        ),
        r"^[%s]*end[%s]?note$",
    )
}

/// `CommandFactoryNoteOnLink#createMultiLine`.
fn note_on_link_multi() -> Entry {
    multi(
        &format!(
            r"^note[%s]+(right|left|top|bottom)?[%s]*(on|of)[%s]+link[%s]*{}$",
            color()
        ),
        r"^end[%s]?note$",
    )
}

/// `CommandFactoryNoteOnEntity#createMultiLine(withBracket)` for the factory's
/// CODE partial pattern.
fn note_on_entity_multi(code: &str, with_bracket: bool) -> Entry {
    let prefix = format!(
        r"^note[%s]+(right|left|top|bottom)(?:[%s]+of[%s]+{code}|)[%s]*{STEREOTAGS}{STEREO_OPTIONAL}{STEREOTAGS}[%s]*{}[%s]*{}",
        color(),
        url_optional()
    );
    if with_bracket {
        multi(&format!(r"{prefix}[%s]*\{{$"), r"^(\})$")
    } else {
        multi(&format!("{prefix}$"), r"^[%s]*(end[%s]?note)$")
    }
}

/// `CommandCreateElementMultilines` TYPE0 (`as "...`) and TYPE1 (`[ ...`).
fn create_element_multilines(registry: &mut Vec<Entry>) {
    let head = format!(
        r"^({ALL_TYPES})[%s]+([%pLN_.]+){STEREO_OPTIONAL}{}[%s]*{}[%s]*",
        url_optional(),
        color()
    );
    registry.push(multi(
        &format!(r"{head}as[%s]*[%g]([^%g]*)$"),
        r"^(.*)[%g]$",
    ));
    registry.push(multi(&format!(r"{head}\[(.*)$"), r"^([^\[\]]*)\]$"));
}

/// `CommandCreateMap`.
fn create_map() -> Entry {
    multi(
        &format!(
            r"^map[%s]+(?:[%g]([^%g]+)[%g][%s]+as[%s]+)?([%pLN_.]+){STEREO_OPTIONAL}{}[%s]*{}[%s]*(?:##(?:\[(dotted|dashed|bold)\])?(\w+)?)?[%s]*\{{$",
            url_optional(),
            color()
        ),
        r"^[%s]*\}[%s]*$",
    )
}

/// `CommandCreateJson`.
fn create_json() -> Entry {
    Entry::Multi {
        start: registered_regex(&format!(
            r"^json[%s]+{NAME_AND_CODE}{STEREO_OPTIONAL}{}[%s]*{}[%s]*\{{$",
            url_optional(),
            color()
        )),
        end: End::Json(registered_regex(r"^[%s]*\}[%s]*$")),
        final_bracket: false,
    }
}

// ---------------------------------------------------------------------------
// Factory registries, in initCommandsList order.

/// `DescriptionDiagramFactory#initCommandsList`.
fn description_registry() -> Vec<Entry> {
    // The CODE partial of the description CommandFactoryNoteOnEntity. Its
    // `\((?!\*\))[^\)]+\)` alternative excludes exactly `(*)`, written here
    // without the lookahead.
    const CODE: &str = r"(?:[%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|\[[^\]*]+[^\]]*\]|\((?:[^\)*][^\)]*|\*[^\)]+)\)|:[^:]+:|[%g][^%g]+[%g])";
    let mut registry = Vec::new();
    // CommandFootboxIgnored, CommandRankDir, CommandNewpage: single-line.
    title_commands(&mut registry);
    common_commands2(&mut registry);
    // addCommonScaleCommands / addCommonHides, CommandLinkElement,
    // CommandHideShow2, CommandRemoveRestore, CommandPackageWithUSymbol
    // (ends with `{`), CommandTogether, CommandEndPackage: single-line.
    registry.push(note_multi());
    registry.push(note_on_link_multi());
    // The NoteOnLink and NoteOnEntity single lines require ':' after the
    // position/link/code, which the multiline forms exclude.
    // CommandCreateElementFull accepts some lines TYPE1 also starts (an
    // element with a [[url]]), and is registered first.
    registry.push(single(&format!(
        r"^(?:({ALL_TYPES}|\(\))[%s]+)?{color}[%s]*(?:({ELEMENT_CODE_CORE}|[%g].+?[%g])|(?:({ELEMENT_DISPLAY_CORE}){STEREO_OPTIONAL}as[%s]+({ELEMENT_CODE_CORE}))|(?:({ELEMENT_CODE_CORE}){STEREO_OPTIONAL}as[%s]*({ELEMENT_DISPLAY_CORE}))|(?:({ELEMENT_DISPLAY_CORE}|[%pLN_.]+){STEREO_OPTIONAL}as[%s]+({ELEMENT_CODE_CORE})))[%s]*{STEREOTAGS}{STEREO_OPTIONAL}{STEREOTAGS}[%s]*{url}[%s]*{color}$",
        color = color(),
        url = url_optional()
    )));
    create_element_multilines(&mut registry);
    registry.push(note_on_entity_multi(CODE, true));
    registry.push(note_on_entity_multi(CODE, false));
    registry.push(note_multi());
    registry.push(create_map());
    registry.push(create_json());
    // CommandCreateJsonSingleLine, CommandArchimate (ends with a stereotype,
    // no '['): single-line.
    // CommandArchimateMultilines.
    registry.push(multi(
        &format!(
            r"^archimate[%s]+{color}[%s]+([%pLN_.]+){STEREO_OPTIONAL_ARCHIMATE}{url}[%s]*{color}[%s]*\[(.*)$",
            color = color(),
            url = url_optional()
        ),
        r"^(.*)\]$",
    ));
    registry
}

/// `StateDiagramFactory#initCommandsList`.
fn state_registry() -> Vec<Entry> {
    const CODE: &str = r"(?:[%pLN_.]+|[%g][^%g]+[%g])";
    // CommandCreateState ... CommandConcurrentState: single-line commands
    // whose grammars start with `state`, a code followed by an arrow, ':' or
    // '{', '}' or a separator run; none accepts a modelled multiline start.
    let mut registry = vec![
        note_on_entity_multi(CODE, true),
        note_on_entity_multi(CODE, false),
        note_on_link_multi(),
        note_multi(),
        create_map(),
        create_json(),
    ];
    title_commands(&mut registry);
    common_commands2(&mut registry);
    registry
}

/// `ActivityDiagramFactory#initCommandsList` (legacy activity).
fn legacy_activity_registry() -> Vec<Entry> {
    const LINE_STYLE: &str = r"(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*";
    let mut registry = Vec::new();
    title_commands(&mut registry);
    common_commands2(&mut registry);
    // UBrexCommandRankDir, UBrexCommandPartition, UBrexCommandEndPartition:
    // single-line.
    // CommandLinkLongActivity.
    registry.push(multi(
        &format!(
            r"^(?:(?:(\(\*(top)?\))|([%pLN][%pLN_.]*)|(?:==+)[%s]*([%pLN_.]+)[%s]*(?:==+)|[%g]([^%g]+)[%g](?:[%s]+as[%s]+([%pLN_.]+))?))?{STEREO_OPTIONAL}(#\w+)?[%s]*{}([-.]+)(?:\[({LINE_STYLE})\])?(\*|left|right|up|down|le?|ri?|up?|do?)?(?:\[({LINE_STYLE})\])?([-.]*)\>[%s]*(?:\[([^\]*]+[^\]]*)\])?[%s]*[%g]([^%g]*?)[%s]*$",
            url_optional()
        ),
        r"^[%s]*([^%g]*)[%g](?:[%s]+as[%s]+([%pLN][%pLN_.]*))?[%s]*(\<\<.*\>\>)?[%s]*(?:in[%s]+([%g][^%g]+[%g]|\S+))?[%s]*(#\w+)?$",
    ));
    // CommandFactoryNoteActivity: the single line requires ':'.
    registry.push(multi(
        &format!(r"^note[%s]+(right|left|top|bottom)[%s]*{}[%s]*$", color()),
        r"^[%s]*end[%s]?note$",
    ));
    registry.push(note_on_link_multi());
    registry
}

/// `ClassDiagramFactory#initCommandsList` (class and object diagrams).
fn class_registry() -> Vec<Entry> {
    const CLASS_CODE: &str = r"(?:[^%s{}%g<>]+|[%g][^%g]+[%g])";
    // CommandLinkClass.getSeparator.
    const SEP: &str = r"(?:\.|::|\\|\\\\)";
    let generic = generic_pattern();
    let code = format!(r"{SEP}?[%pLN_$]+(?:{SEP}[%pLN_$]+)*");
    let codes = format!(r"{code}(?:\s*,\s*{code})*");
    let mut registry = Vec::new();
    // CommandFootboxIgnored, CommandRankDir, CommandNewpage, scale commands,
    // CommandAddMethod, hides, CommandHideShow2, CommandRemoveRestore:
    // single-line.
    // CommandCreateClassMultilines (syntaxWithFinalBracket).
    registry.push(Entry::Multi {
        start: registered_regex(&format!(
            r"^([-#+~])?[%s]*(interface|enum|annotation|abstract[%s]+class|static[%s]+class|abstract|class|entity|protocol|struct|exception|metaclass|stereotype|dataclass|record)[%s]+{}(?:[%s]*\<({generic})\>)?[%s]*{STEREOTAGS}{STEREO_OPTIONAL}{STEREOTAGS}[%s]*{url}[%s]*{color}[%s]*(?:##(?:\[(dotted|dashed|bold)\])?(\w+)?)?(?:[%s]+(extends)[%s]+({codes}|[%g]([^%g]+)[%g])(?:[%s]*\<({generic})\>)?)?(?:[%s]+(implements)[%s]+({codes}|[%g]([^%g]+)[%g])(?:[%s]*\<({generic})\>)?)?[%s]*\{{[%s]*$",
            name_and_code_for_class_with_generic(),
            url = url_optional(),
            color = color()
        )),
        end: End::Pattern(registered_regex(r"^[%s]*\}[%s]*$")),
        final_bracket: true,
    });
    // CommandCreateEntityObjectMultilines.
    registry.push(multi(
        &format!(
            r"^object[%s]+{NAME_AND_CODE}{STEREO_OPTIONAL}{}[%s]*{}[%s]*\{{$",
            url_optional(),
            color()
        ),
        r"^[%s]*\}[%s]*$",
    ));
    registry.push(create_map());
    registry.push(create_json());
    // CommandCreateJsonSingleLine, CommandCreateClass, CommandCreateEntityObject,
    // CommandAllowMixing, CommandCreateElementParenthesis, CommandLayoutNewLine,
    // the package/namespace/together commands: single-line.
    // CommandCreateElementFull2 (NORMAL_KEYWORD, WITH_MIX_PREFIX) accept some
    // lines CommandCreateElementMultilines TYPE1 also starts.
    for prefix in ["", "mix_"] {
        registry.push(single(&format!(
            r"^{prefix}(state|{ALL_TYPES})[%s]+(?:({ELEMENT_CODE_CORE}|[%g].+?[%g])|(?:({ELEMENT_DISPLAY_CORE}){STEREO_OPTIONAL}as[%s]+({ELEMENT_CODE_CORE})))[%s]*{STEREOTAGS}{STEREO_OPTIONAL}{STEREOTAGS}[%s]*{url}[%s]*{color}$",
            url = url_optional(),
            color = color()
        )));
    }
    // CommandFactoryNote single, CommandNamespace*, CommandStereotype,
    // CommandLinkClass, CommandLinkLollipop: single-line.
    // CommandFactoryTipOnEntity (NameAndCodeParser#codeWithMemberForClass).
    let tip = format!(
        r"^note[%s]+(right|left)[%s]+of[%s]+([^%s{{}}%g<>:]+|[%g][^%g]+[%g])::([%g][^%g]+[%g]|[^%s]+)[%s]*{STEREOTAGS}{STEREO_OPTIONAL}{STEREOTAGS}[%s]*{}[%s]*{}",
        color(),
        url_optional()
    );
    registry.push(multi(&format!(r"{tip}[%s]*\{{$"), r"^(\})$"));
    registry.push(multi(&format!("{tip}$"), r"^[%s]*(end[%s]?note)$"));
    // The CommandFactoryNoteOnEntity single line (registered before) requires ':'.
    registry.push(note_on_entity_multi(CLASS_CODE, true));
    registry.push(note_on_entity_multi(CLASS_CODE, false));
    registry.push(note_multi());
    registry.push(note_on_link_multi());
    // CommandConstraintOnLinks, CommandDiamondAssociation: single-line.
    create_element_multilines(&mut registry);
    title_commands(&mut registry);
    common_commands2(&mut registry);
    registry
}

static DESCRIPTION: LazyLock<Vec<Entry>> = LazyLock::new(description_registry);
static STATE: LazyLock<Vec<Entry>> = LazyLock::new(state_registry);
static LEGACY_ACTIVITY: LazyLock<Vec<Entry>> = LazyLock::new(legacy_activity_registry);
static CLASS: LazyLock<Vec<Entry>> = LazyLock::new(class_registry);

// ---------------------------------------------------------------------------
// getCandidate replay.

/// StringLocated#getTrimmed (StringUtils#trin).
fn trimmed(line: &str) -> &str {
    sequence_command_trim(super::source_text(line))
}

/// Pattern2's ASCII case folding of the trimmed line.
fn folded(line: &str) -> String {
    trimmed(line).to_ascii_lowercase()
}

/// `EmbeddedDiagram#getEmbeddedType` is not null.
fn is_embedded_start(text: &str) -> bool {
    matches!(
        text,
        "{{" | "{{ditaa"
            | "{{salt"
            | "{{uml"
            | "{{wbs"
            | "{{mindmap"
            | "{{gantt"
            | "{{json"
            | "{{yaml"
            | "{{wire"
            | "{{creole"
            | "{{board"
            | "{{ebnf"
            | "{{regex"
            | "{{files"
            | "{{chronology"
            | "{{chen"
            | "{{chart"
            | "{{nwdiag"
            | "{{packetdiag"
    )
}

/// `PSystemCommandFactory#addOneSingleLineManageEmbedded2`: the index of the
/// last line one step adds, starting at `index`.
fn step_end(lines: &[String], index: usize) -> Option<usize> {
    if !is_embedded_start(trimmed(&lines[index])) {
        return Some(index);
    }
    let mut nested = 1;
    for (k, line) in lines.iter().enumerate().skip(index + 1) {
        let text = trimmed(line);
        if is_embedded_start(text) {
            nested += 1;
        } else if text == "}}" {
            nested -= 1;
            if nested == 0 {
                return Some(k);
            }
        }
    }
    None
}

/// CommandCreateJson#getJsonValue on the block `start..=end`.
fn json_parses(lines: &[String], start: usize, end: usize) -> bool {
    let body: String = lines[start + 1..end]
        .iter()
        .map(|line| super::source_text(line))
        .collect();
    serde_json::from_str::<serde_json::Value>(&format!("{{{body}}}")).is_ok()
        || serde_json::from_str::<serde_json::Value>(&body).is_ok()
}

/// `CommandMultilinesBracket#isValid` with `CommandSkinParamMultilines`'s
/// `#isLineConsistent`.
fn bracket_end(lines: &[String], start: usize) -> Option<usize> {
    static CONSISTENT: LazyLock<Regex> = LazyLock::new(|| {
        registered_regex(r"^([\w.]*(?:\<\<.*\>\>)?[\w.]*)[%s]+(?:(\{)|(.*))$|^\}?$")
    });
    static COMMENT: LazyLock<Regex> =
        LazyLock::new(|| registered_regex(r"^[%s]*([%q].*||/[%q].*[%q]/[%s]*)$"));
    let mut level = 1i64;
    for (k, line) in lines.iter().enumerate().skip(start + 1) {
        let text = folded(line);
        if !COMMENT.is_match(&text) && !CONSISTENT.is_match(&text) {
            return None;
        }
        if text.ends_with('{') {
            level += 1;
        }
        if text.ends_with('}') {
            level -= 1;
        }
        if level < 0 {
            return None;
        }
        if level == 0 {
            return Some(k);
        }
    }
    None
}

/// The last line of the block a multiline command starting at `index`
/// completes, or `None` when `#isMultilineCommandOk` returns null.
fn multiline_end(
    lines: &[String],
    index: usize,
    start: &Regex,
    end: &End,
    final_bracket: bool,
) -> Option<usize> {
    let first = folded(&lines[index]);
    let body_from = if start.is_match(&first) {
        index + 1
    } else if final_bracket
        && !first.ends_with('{')
        && start.is_match(&format!("{first} {{"))
        && lines
            .get(index + 1)
            .is_some_and(|line| trimmed(line) == "{")
    {
        // BlocLines#eventuallyMoveBracket joins a lone '{' second line.
        index + 2
    } else {
        return None;
    };
    if let End::Bracket = end {
        return bracket_end(lines, index);
    }
    let mut k = body_from;
    while k < lines.len() {
        let last = step_end(lines, k)?;
        let text = folded(&lines[last]);
        match end {
            End::Pattern(pattern) if pattern.is_match(&text) => return Some(last),
            End::Json(pattern) if pattern.is_match(&text) && json_parses(lines, index, last) => {
                return Some(last);
            }
            _ => {}
        }
        k = last + 1;
    }
    None
}

/// Replay `getCandidate` over `lines` with `factory`'s registry.
pub(super) fn walk(lines: &[String], factory: Factory) -> CommandSpans {
    let registry: &[Entry] = match factory {
        Factory::Description => &DESCRIPTION,
        Factory::State => &STATE,
        Factory::LegacyActivity => &LEGACY_ACTIVITY,
        Factory::Class => &CLASS,
    };
    let mut owned = vec![false; lines.len()];
    let mut index = 0;
    while index < lines.len() {
        let text = folded(&lines[index]);
        let mut next = index + 1;
        if !text.is_empty() {
            for entry in registry {
                match entry {
                    Entry::Single(pattern) => {
                        if pattern.is_match(&text) {
                            break;
                        }
                    }
                    Entry::Multi {
                        start,
                        end,
                        final_bracket,
                    } => {
                        if let Some(last) = multiline_end(lines, index, start, end, *final_bracket)
                        {
                            owned[index..=last].fill(true);
                            next = last + 1;
                            break;
                        }
                    }
                    Entry::Sprites => {
                        if let Some(last) = super::sprite::complete_span(lines, index) {
                            next = last + 1;
                            break;
                        }
                    }
                }
            }
        }
        index = next;
    }
    CommandSpans { owned }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    fn owned(text: &str, factory: Factory) -> Vec<bool> {
        let input = lines(text);
        let spans = walk(&input, factory);
        (0..input.len()).map(|i| spans.owned_by_other(i)).collect()
    }

    #[test]
    fn title_group_matches_sequence_registry() {
        for (class, start, end) in TITLE_GROUP {
            let (registered_start, _, registered_end) =
                super::super::sequence::registered_grammar(class).expect(class);
            assert_eq!(
                registered_regex(start).as_str(),
                registered_start,
                "{class}"
            );
            assert_eq!(
                end.map(|end| registered_regex(end).as_str().to_string()),
                registered_end,
                "{class}"
            );
        }
    }

    #[test]
    fn title_group_bodies_are_owned_in_every_factory() {
        for factory in [
            Factory::Description,
            Factory::State,
            Factory::LegacyActivity,
            Factory::Class,
        ] {
            for (open, close) in [
                ("header", "endheader"),
                ("right footer", "end footer"),
                ("title", "end title"),
                ("legend top left", "endlegend"),
                ("caption", "endcaption"),
            ] {
                let text = format!("{open}\nSprite sheets are loaded\n{close}\nsprite $x foo");
                assert_eq!(
                    owned(&text, factory),
                    [true, true, true, false],
                    "{factory:?} {open}"
                );
            }
            // An unclosed multiline legend falls back to nothing here and its
            // next line is a command start.
            assert_eq!(owned("legend\nsprite $x foo", factory), [false, false]);
        }
    }

    #[test]
    fn description_element_bodies_are_owned() {
        let text = "component Engine [\nfirst\nSprite sheets are loaded\n]\nnode N as \"two\nSprite x\nlines\"\nsprite $x foo";
        assert_eq!(
            owned(text, Factory::Description),
            [true, true, true, true, true, true, true, false]
        );
        // CommandCreateElementFull is registered before TYPE1 and takes an
        // element whose only bracket is its [[url]].
        assert_eq!(
            owned(
                "component Engine [[http://x]]\nSprite x\n]",
                Factory::Description
            ),
            [false, false, false]
        );
    }

    #[test]
    fn sprite_bodies_are_not_reowned() {
        // A pixel sprite body containing a header lookalike stays a sprite.
        let text = "sprite $x {\nheader\n}\nheader\nbody\nendheader";
        assert_eq!(
            owned(text, Factory::Description),
            [false, false, false, true, true, true]
        );
    }

    #[test]
    fn legacy_activity_note_and_link_bodies_are_owned() {
        let text =
            "(*) --> \"load\"\nnote right\nSprite frame {\nend note\n\"a\" --> \"multi\nSprite b\"";
        assert_eq!(
            owned(text, Factory::LegacyActivity),
            [false, true, true, true, true, true]
        );
    }

    #[test]
    fn class_bodies_including_a_separate_bracket_line() {
        let text = "class Player\n{\nSprite x\n}\nobject O {\nsprite y\n}\nnote left of Player::name\nsprite z\nend note";
        assert_eq!(
            owned(text, Factory::Class),
            [true, true, true, true, true, true, true, true, true, true]
        );
        assert_eq!(
            owned("class Player\nsprite $x foo", Factory::Class),
            [false, false]
        );
    }

    #[test]
    fn json_bodies_extend_until_they_parse() {
        let text = "json J {\n\"a\": {\n\"b\": 1\n}\n}\nsprite $x foo";
        assert_eq!(
            owned(text, Factory::State),
            [true, true, true, true, true, false]
        );
    }

    #[test]
    fn embedded_diagrams_hide_end_lookalikes() {
        let text = "note as N\n{{\nend note\n}}\nend note\nsprite $x foo";
        assert_eq!(
            owned(text, Factory::Description),
            [true, true, true, true, true, false]
        );
    }

    #[test]
    fn skinparam_brace_blocks_are_owned() {
        let text = "skinparam component {\nBackgroundColor red\n}\nsprite $x foo";
        assert_eq!(owned(text, Factory::Description), [true, true, true, false]);
    }
}
