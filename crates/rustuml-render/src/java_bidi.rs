// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! The bidi levels JDK 21's `TextLayout` splits complex text by.
//!
//! `TextLine.fastCreateTextLine` (897-980) builds one `TextLineComponent`
//! per run of equal bidi levels (`createComponentsOnRun` 994-1039,
//! `firstVisualChunk` 1296-1305) when `Bidi.requiresBidi` holds and the
//! resulting `java.text.Bidi` is not left-to-right; `GlyphLayout.layout`
//! then shapes each component on its own (script runs and font-slot runs
//! inside it). The levels are those of `jdk.internal.icu.text.BidiBase`
//! (OpenJDK jdk21u jdk-21.0.5-ga), ported here for what `TextLine` uses:
//! `setPara` with `DIRECTION_DEFAULT_LEFT_TO_RIGHT`, no embedding array,
//! `REORDER_DEFAULT` and no options. Bidi classes and paired brackets are
//! JDK's own ICU data (`icudt72b/ubidi.icu`, `java_bidi_data`).

use crate::java_bidi_data as data;

// ICU `UCharDirection` (BidiBase.java 956-980).
const L: u8 = 0;
const R: u8 = 1;
const EN: u8 = 2;
const ES: u8 = 3;
const ET: u8 = 4;
const AN: u8 = 5;
const CS: u8 = 6;
const B: u8 = 7;
const S: u8 = 8;
const WS: u8 = 9;
const ON: u8 = 10;
const LRE: u8 = 11;
const LRO: u8 = 12;
const AL: u8 = 13;
const RLE: u8 = 14;
const RLO: u8 = 15;
const PDF: u8 = 16;
const NSM: u8 = 17;
const BN: u8 = 18;
const FSI: u8 = 19;
const LRI: u8 = 20;
const RLI: u8 = 21;
const PDI: u8 = 22;
/// EN after W7.
const ENL: u8 = 23;
/// EN not subject to W7.
const ENR: u8 = 24;

/// `BidiBase.MAX_EXPLICIT_LEVEL`.
const MAX_EXPLICIT_LEVEL: u8 = 125;
/// `BidiBase.LEVEL_DEFAULT_LTR`.
pub(crate) const LEVEL_DEFAULT_LTR: u8 = 0x7e;
/// `BidiBase.LEVEL_OVERRIDE`.
const LEVEL_OVERRIDE: u8 = 0x80;

const CR: u16 = 0x0D;
/// U+232A, paired with U+2329; canonically equivalent to U+3009.
const RIGHT_POINTING_ANGLE_BRACKET: u32 = 0x232A;
/// U+3009, paired with U+3008.
const RIGHT_ANGLE_BRACKET: u32 = 0x3009;
const LF: u16 = 0x0A;

const fn flag(dir: u8) -> u32 {
    1 << dir
}
const DIR_PROP_FLAG_MULTI_RUNS: u32 = flag(31);
const MASK_LTR: u32 =
    flag(L) | flag(EN) | flag(ENL) | flag(ENR) | flag(AN) | flag(LRE) | flag(LRO) | flag(LRI);
const MASK_RTL: u32 = flag(R) | flag(AL) | flag(RLE) | flag(RLO) | flag(RLI);
const MASK_EXPLICIT: u32 = flag(LRE) | flag(LRO) | flag(RLE) | flag(RLO) | flag(PDF);
const MASK_BN_EXPLICIT: u32 = flag(BN) | MASK_EXPLICIT;
const MASK_ISO: u32 = flag(LRI) | flag(RLI) | flag(FSI) | flag(PDI);
const MASK_B_S: u32 = flag(B) | flag(S);
const MASK_WS: u32 = MASK_B_S | flag(WS) | MASK_BN_EXPLICIT | MASK_ISO;
const MASK_POSSIBLE_N: u32 = flag(ON) | flag(CS) | flag(ES) | flag(ET) | MASK_WS;
const MASK_EMBEDDING: u32 = flag(NSM) | MASK_POSSIBLE_N;

fn flag_lr(level: u8) -> u32 {
    flag(level & 1)
}
fn flag_e(level: u8) -> u32 {
    if level & 1 == 0 { flag(LRE) } else { flag(RLE) }
}
fn flag_o(level: u8) -> u32 {
    if level & 1 == 0 { flag(LRO) } else { flag(RLO) }
}
fn no_override(level: u8) -> u8 {
    level & !LEVEL_OVERRIDE
}
fn dir_from_strong(strong: u8) -> u8 {
    if strong == L { L } else { R }
}
fn is_default_level(level: u8) -> bool {
    level & LEVEL_DEFAULT_LTR == LEVEL_DEFAULT_LTR
}

/// `UBiDiProps.getClass` with `BidiBase.getCustomizedClass` (3971-3978).
pub(crate) fn bidi_class(code: u32) -> u8 {
    let index = data::BIDI_CLASSES.partition_point(|(start, _, _)| *start <= code);
    match index.checked_sub(1).map(|i| data::BIDI_CLASSES[i]) {
        Some((_, end, class)) if code <= end => {
            if class >= 23 {
                ON
            } else {
                class
            }
        }
        _ => L,
    }
}

/// `UBiDiProps.getPairedBracket` and `getPairedBracketType`: the paired
/// bracket of `code` and whether `code` opens.
fn paired_bracket(code: u32) -> Option<(u32, bool)> {
    data::BRACKETS
        .binary_search_by_key(&code, |(c, _, _)| *c)
        .ok()
        .map(|index| (data::BRACKETS[index].1, data::BRACKETS[index].2))
}

/// Overall direction (`BidiBase.LTR`, `RTL`, `MIXED`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    Ltr,
    Rtl,
    Mixed,
}

/// The result of `BidiBase.setPara`.
pub(crate) struct Paragraph {
    pub(crate) direction: Direction,
    /// The (first) paragraph level, resolved when contextual.
    pub(crate) para_level: u8,
    /// The level of each UTF-16 unit as `BidiUtils.getLevels` returns it.
    pub(crate) levels: Vec<u8>,
}

impl Paragraph {
    /// `BidiBase.isLeftToRight` (4354-4358).
    pub(crate) fn is_left_to_right(&self) -> bool {
        self.direction == Direction::Ltr && self.para_level & 1 == 0
    }
}

/// `BidiBase.requiresBidi` (4534-4561): some code point is R, AL, RLE, RLO
/// or AN.
pub(crate) fn requires_bidi(units: &[u16]) -> bool {
    const RTL_MASK: u32 = flag(R) | flag(AL) | flag(RLE) | flag(RLO) | flag(AN);
    let mut i = 0;
    while i < units.len() {
        let code = if is_high(units[i]) && i + 1 < units.len() && is_low(units[i + 1]) {
            combine(units[i], units[i + 1])
        } else {
            u32::from(units[i])
        };
        if flag(bidi_class(code)) & RTL_MASK != 0 {
            return true;
        }
        i += 1;
    }
    false
}

fn is_high(unit: u16) -> bool {
    (0xD800..=0xDBFF).contains(&unit)
}
fn is_low(unit: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&unit)
}
fn combine(high: u16, low: u16) -> u32 {
    0x10000 + ((u32::from(high) - 0xD800) << 10) + (u32::from(low) - 0xDC00)
}

/// The levels `TextLine.fastCreateTextLine` (933-948) splits components
/// by, one per UTF-16 unit: `None` when the text does not require bidi or
/// its `Bidi` is left-to-right (one component).
pub(crate) fn text_line_levels(units: &[u16]) -> Option<Vec<u8>> {
    if !requires_bidi(units) {
        return None;
    }
    let paragraph = set_para(units, LEVEL_DEFAULT_LTR);
    (!paragraph.is_left_to_right()).then_some(paragraph.levels)
}

/// `BidiBase.Opening`.
#[derive(Clone, Copy, Default)]
struct Opening {
    position: usize,
    /// Matching char, or -position of the closing bracket, or 0.
    matching: i64,
    context_pos: usize,
    flags: u32,
    context_dir: u8,
}

/// `BidiBase.IsoRun`.
#[derive(Clone, Copy, Default)]
struct IsoRun {
    context_pos: usize,
    start: usize,
    limit: usize,
    level: u8,
    last_strong: u8,
    last_base: u8,
    context_dir: u8,
}

/// `BidiBase.BracketData`.
struct BracketData {
    openings: Vec<Opening>,
    iso_run_last: usize,
    iso_runs: Vec<IsoRun>,
}

/// `BidiBase.Isolate`.
#[derive(Clone, Copy, Default)]
struct Isolate {
    start_on: i64,
    start1: usize,
    state_imp: u8,
    state: u8,
}

/// `BidiBase.LevState` (REORDER_DEFAULT: the insert-point fields are unused).
struct LevState {
    imp_tab: &'static [[u8; 8]],
    start_on: i64,
    run_start: usize,
    state: u8,
    run_level: u8,
}

const FOUND_L: u32 = flag(L);
const FOUND_R: u32 = flag(R);

// impTabProps (BidiBase.java 2470-2497): column groups, then the table.
const GROUP_PROP: [u8; 25] = [
    0, 1, 2, 7, 8, 3, 9, 6, 5, 4, 4, 10, 10, 12, 10, 10, 10, 11, 10, 4, 4, 4, 4, 13, 14,
];
const P_L: u8 = 0;
const P_R: u8 = 1;
const P_EN: u8 = 2;
const P_AN: u8 = 3;
const P_ON: u8 = 4;
const P_S: u8 = 5;
const P_B: u8 = 6;
const IMP_TAB_PROPS: [[u8; 16]; 24] = [
    [1, 2, 4, 5, 7, 15, 17, 7, 9, 7, 0, 7, 3, 18, 21, P_ON],
    [1, 34, 36, 37, 39, 47, 49, 39, 41, 39, 1, 1, 35, 50, 53, P_L],
    [33, 2, 36, 37, 39, 47, 49, 39, 41, 39, 2, 2, 35, 50, 53, P_R],
    [33, 34, 38, 38, 40, 48, 49, 40, 40, 40, 3, 3, 3, 50, 53, P_R],
    [
        33, 34, 4, 37, 39, 47, 49, 74, 11, 74, 4, 4, 35, 18, 21, P_EN,
    ],
    [
        33, 34, 36, 5, 39, 47, 49, 39, 41, 76, 5, 5, 35, 50, 53, P_AN,
    ],
    [33, 34, 6, 6, 40, 48, 49, 40, 40, 77, 6, 6, 35, 18, 21, P_AN],
    [33, 34, 36, 37, 7, 47, 49, 7, 78, 7, 7, 7, 35, 50, 53, P_ON],
    [33, 34, 38, 38, 8, 48, 49, 8, 8, 8, 8, 8, 35, 50, 53, P_ON],
    [33, 34, 4, 37, 7, 47, 49, 7, 9, 7, 9, 9, 35, 18, 21, P_ON],
    [
        97, 98, 4, 101, 135, 111, 113, 135, 142, 135, 10, 135, 99, 18, 21, P_EN,
    ],
    [
        33, 34, 4, 37, 39, 47, 49, 39, 11, 39, 11, 11, 35, 18, 21, P_EN,
    ],
    [
        97, 98, 100, 5, 135, 111, 113, 135, 142, 135, 12, 135, 99, 114, 117, P_AN,
    ],
    [
        97, 98, 6, 6, 136, 112, 113, 136, 136, 136, 13, 136, 99, 18, 21, P_AN,
    ],
    [
        33, 34, 132, 37, 7, 47, 49, 7, 14, 7, 14, 14, 35, 146, 149, P_ON,
    ],
    [
        33, 34, 36, 37, 39, 15, 49, 39, 41, 39, 15, 39, 35, 50, 53, P_S,
    ],
    [
        33, 34, 38, 38, 40, 16, 49, 40, 40, 40, 16, 40, 35, 50, 53, P_S,
    ],
    [
        33, 34, 36, 37, 39, 47, 17, 39, 41, 39, 17, 39, 35, 50, 53, P_B,
    ],
    [
        33, 34, 18, 37, 39, 47, 49, 83, 20, 83, 18, 18, 35, 18, 21, P_L,
    ],
    [
        97, 98, 18, 101, 135, 111, 113, 135, 142, 135, 19, 135, 99, 18, 21, P_L,
    ],
    [
        33, 34, 18, 37, 39, 47, 49, 39, 20, 39, 20, 20, 35, 18, 21, P_L,
    ],
    [
        33, 34, 21, 37, 39, 47, 49, 86, 23, 86, 21, 21, 35, 18, 21, P_AN,
    ],
    [
        97, 98, 21, 101, 135, 111, 113, 135, 142, 135, 22, 135, 99, 18, 21, P_AN,
    ],
    [
        33, 34, 21, 37, 39, 47, 49, 39, 23, 39, 23, 23, 35, 18, 21, P_AN,
    ],
];
const IMPTABPROPS_RES: usize = 15;

// impTabL_DEFAULT / impTabR_DEFAULT (2565-2604); cells: state | action << 4.
const IMP_TAB_L_DEFAULT: [[u8; 8]; 6] = [
    [0, 1, 0, 2, 0, 0, 0, 0],
    [0, 1, 3, 3, 0x14, 0x14, 0, 1],
    [0, 1, 0, 2, 0x15, 0x15, 0, 2],
    [0, 1, 3, 3, 0x14, 0x14, 0, 2],
    [0, 0x21, 0x33, 0x33, 4, 4, 0, 0],
    [0, 0x21, 0, 0x32, 5, 5, 0, 0],
];
const IMP_TAB_R_DEFAULT: [[u8; 8]; 6] = [
    [1, 0, 2, 2, 0, 0, 0, 0],
    [1, 0, 1, 3, 0x14, 0x14, 0, 1],
    [1, 0, 2, 2, 0, 0, 0, 1],
    [1, 0, 1, 3, 5, 5, 0, 1],
    [0x21, 0, 0x21, 3, 4, 4, 0, 0],
    [1, 0, 1, 3, 5, 5, 0, 0],
];
const IMPTABLEVELS_RES: usize = 7;

struct Bidi<'a> {
    text: &'a [u16],
    length: usize,
    para_level: u8,
    default_para_level: u8,
    flags: u32,
    dir_props: Vec<u8>,
    levels: Vec<u8>,
    paras_limit: Vec<usize>,
    paras_level: Vec<u8>,
    isolates: Vec<Isolate>,
    isolate_count: i64,
}

/// `BidiBase.setPara(char[], paraLevel, null)` (3526-3710) for
/// REORDER_DEFAULT without options. `para_level` is 0, 1 or
/// [`LEVEL_DEFAULT_LTR`].
pub(crate) fn set_para(text: &[u16], para_level: u8) -> Paragraph {
    let length = text.len();
    if length == 0 {
        let level = if is_default_level(para_level) {
            para_level & 1
        } else {
            para_level
        };
        return Paragraph {
            direction: if level & 1 == 0 {
                Direction::Ltr
            } else {
                Direction::Rtl
            },
            para_level: level,
            levels: Vec::new(),
        };
    }
    let mut bidi = Bidi {
        text,
        length,
        para_level,
        default_para_level: if is_default_level(para_level) {
            para_level
        } else {
            0
        },
        flags: 0,
        dir_props: vec![0; length],
        levels: vec![0; length],
        paras_limit: vec![0],
        paras_level: vec![0],
        isolates: Vec::new(),
        isolate_count: 0,
    };
    bidi.get_dir_props();
    let direction = bidi.resolve_explicit_levels();
    bidi.isolates = vec![Isolate::default(); bidi.isolate_count.max(0) as usize + 3];
    bidi.isolate_count = -1;
    match direction {
        Direction::Ltr | Direction::Rtl => {
            // All levels are implicitly at paraLevel (trailingWSStart 0).
            let level = bidi.para_level;
            return Paragraph {
                direction,
                para_level: level,
                levels: (0..length).map(|i| bidi.para_level_at(i)).collect(),
            };
        }
        Direction::Mixed => {}
    }
    if bidi.paras_limit.len() <= 1 && bidi.flags & DIR_PROP_FLAG_MULTI_RUNS == 0 {
        let sor = bidi.para_level_at(0) & 1;
        let eor = bidi.para_level_at(length - 1) & 1;
        bidi.resolve_implicit_levels(0, length, sor, eor);
    } else {
        let mut limit = 0;
        let mut level = bidi.para_level_at(0);
        let mut next_level = bidi.levels[0];
        let mut eor = if level < next_level {
            next_level & 1
        } else {
            level & 1
        };
        loop {
            let mut start = limit;
            level = next_level;
            let sor = if start > 0 && bidi.dir_props[start - 1] == B {
                bidi.para_level_at(start) & 1
            } else {
                eor
            };
            limit += 1;
            while limit < length
                && (bidi.levels[limit] == level
                    || flag(bidi.dir_props[limit]) & MASK_BN_EXPLICIT != 0)
            {
                limit += 1;
            }
            next_level = if limit < length {
                bidi.levels[limit]
            } else {
                bidi.para_level_at(length - 1)
            };
            eor = if no_override(level) < no_override(next_level) {
                next_level & 1
            } else {
                level & 1
            };
            if level & LEVEL_OVERRIDE == 0 {
                bidi.resolve_implicit_levels(start, limit, sor, eor);
            } else {
                while start < limit {
                    bidi.levels[start] &= !LEVEL_OVERRIDE;
                    start += 1;
                }
            }
            if limit >= length {
                break;
            }
        }
    }
    bidi.adjust_ws_levels();
    Paragraph {
        direction,
        para_level: bidi.para_level,
        levels: bidi.levels,
    }
}

impl Bidi<'_> {
    /// `GetParaLevelAt` (1679-1690).
    fn para_level_at(&self, index: usize) -> u8 {
        if self.default_para_level == 0 || index < self.paras_limit[0] {
            return self.para_level;
        }
        let mut i = 1;
        while i < self.paras_limit.len() {
            if index < self.paras_limit[i] {
                break;
            }
            i += 1;
        }
        if i >= self.paras_limit.len() {
            i = self.paras_limit.len() - 1;
        }
        self.paras_level[i]
    }

    /// `getDirProps` (1476-1677).
    fn get_dir_props(&mut self) {
        const NOT_SEEKING_STRONG: u8 = 0;
        const SEEKING_STRONG_FOR_PARA: u8 = 1;
        const SEEKING_STRONG_FOR_FSI: u8 = 2;
        const LOOKING_FOR_PDI: u8 = 3;
        let original_length = self.length;
        self.flags = 0;
        let is_default = is_default_level(self.para_level);
        let default_level = self.para_level & 1;
        let mut state;
        let mut isolate_start_stack = [0usize; MAX_EXPLICIT_LEVEL as usize + 1];
        let mut previous_state_stack = [0u8; MAX_EXPLICIT_LEVEL as usize + 1];
        let mut stack_last: i64 = -1;
        if is_default {
            self.paras_level[0] = default_level;
            state = SEEKING_STRONG_FOR_PARA;
        } else {
            self.paras_level[0] = self.para_level;
            state = NOT_SEEKING_STRONG;
        }
        let mut i = 0;
        while i < original_length {
            let i0 = i;
            let unit = self.text[i];
            let uchar = if is_high(unit) && i + 1 < original_length && is_low(self.text[i + 1]) {
                i += 2;
                combine(unit, self.text[i0 + 1])
            } else {
                i += 1;
                u32::from(unit)
            };
            let mut i1 = i - 1;
            let dir_prop = bidi_class(uchar);
            self.flags |= flag(dir_prop);
            self.dir_props[i1] = dir_prop;
            if i1 > i0 {
                self.flags |= flag(BN);
                while i1 > i0 {
                    i1 -= 1;
                    self.dir_props[i1] = BN;
                }
            }
            let last = self.paras_level.len() - 1;
            if dir_prop == L {
                if state == SEEKING_STRONG_FOR_PARA {
                    self.paras_level[last] = 0;
                    state = NOT_SEEKING_STRONG;
                } else if state == SEEKING_STRONG_FOR_FSI {
                    if stack_last <= i64::from(MAX_EXPLICIT_LEVEL) {
                        self.flags |= flag(LRI);
                    }
                    state = LOOKING_FOR_PDI;
                }
                continue;
            }
            if dir_prop == R || dir_prop == AL {
                if state == SEEKING_STRONG_FOR_PARA {
                    self.paras_level[last] = 1;
                    state = NOT_SEEKING_STRONG;
                } else if state == SEEKING_STRONG_FOR_FSI {
                    if stack_last <= i64::from(MAX_EXPLICIT_LEVEL) {
                        self.dir_props[isolate_start_stack[stack_last as usize]] = RLI;
                        self.flags |= flag(RLI);
                    }
                    state = LOOKING_FOR_PDI;
                }
                continue;
            }
            if (FSI..=RLI).contains(&dir_prop) {
                stack_last += 1;
                if stack_last <= i64::from(MAX_EXPLICIT_LEVEL) {
                    isolate_start_stack[stack_last as usize] = i - 1;
                    previous_state_stack[stack_last as usize] = state;
                }
                if dir_prop == FSI {
                    self.dir_props[i - 1] = LRI;
                    state = SEEKING_STRONG_FOR_FSI;
                } else {
                    state = LOOKING_FOR_PDI;
                }
                continue;
            }
            if dir_prop == PDI {
                if state == SEEKING_STRONG_FOR_FSI && stack_last <= i64::from(MAX_EXPLICIT_LEVEL) {
                    self.flags |= flag(LRI);
                }
                if stack_last >= 0 {
                    if stack_last <= i64::from(MAX_EXPLICIT_LEVEL) {
                        state = previous_state_stack[stack_last as usize];
                    }
                    stack_last -= 1;
                }
                continue;
            }
            if dir_prop == B {
                if i < original_length && uchar == u32::from(CR) && self.text[i] == LF {
                    continue;
                }
                self.paras_limit[last] = i;
                if i < original_length {
                    if is_default {
                        self.paras_level.push(default_level);
                        state = SEEKING_STRONG_FOR_PARA;
                    } else {
                        self.paras_level.push(self.para_level);
                        state = NOT_SEEKING_STRONG;
                    }
                    self.paras_limit.push(0);
                    stack_last = -1;
                }
                continue;
            }
        }
        if stack_last > i64::from(MAX_EXPLICIT_LEVEL) {
            stack_last = i64::from(MAX_EXPLICIT_LEVEL);
            state = SEEKING_STRONG_FOR_FSI;
        }
        while stack_last >= 0 {
            if state == SEEKING_STRONG_FOR_FSI {
                self.flags |= flag(LRI);
                break;
            }
            state = previous_state_stack[stack_last as usize];
            stack_last -= 1;
        }
        let last = self.paras_limit.len() - 1;
        self.paras_limit[last] = original_length;
        if is_default {
            self.para_level = self.paras_level[0];
        }
        for &level in &self.paras_level {
            self.flags |= flag_lr(level);
        }
    }

    /// `directionFromFlags` (2025-2040).
    fn direction_from_flags(&self) -> Direction {
        if !(self.flags & MASK_RTL != 0
            || (self.flags & flag(AN) != 0 && self.flags & MASK_POSSIBLE_N != 0))
        {
            Direction::Ltr
        } else if self.flags & MASK_LTR == 0 {
            Direction::Rtl
        } else {
            Direction::Mixed
        }
    }

    fn bracket_init(&self) -> BracketData {
        let level = self.para_level_at(0);
        let mut iso_runs = vec![IsoRun::default(); MAX_EXPLICIT_LEVEL as usize + 2];
        iso_runs[0] = IsoRun {
            context_pos: 0,
            start: 0,
            limit: 0,
            level,
            last_strong: level & 1,
            last_base: level & 1,
            context_dir: level & 1,
        };
        BracketData {
            openings: Vec::new(),
            iso_run_last: 0,
            iso_runs,
        }
    }

    /// `bracketProcessB` (1722-1729).
    fn bracket_process_b(bd: &mut BracketData, level: u8) {
        bd.iso_run_last = 0;
        let run = &mut bd.iso_runs[0];
        run.limit = 0;
        run.level = level;
        run.last_strong = level & 1;
        run.last_base = level & 1;
        run.context_dir = level & 1;
        run.context_pos = 0;
    }

    /// `bracketProcessBoundary` (1731-1743).
    fn bracket_process_boundary(
        &self,
        bd: &mut BracketData,
        last_cc_pos: usize,
        mut context_level: u8,
        embedding_level: u8,
    ) {
        if flag(self.dir_props[last_cc_pos]) & MASK_ISO != 0 {
            return;
        }
        if no_override(embedding_level) > no_override(context_level) {
            context_level = embedding_level;
        }
        let run = &mut bd.iso_runs[bd.iso_run_last];
        run.limit = run.start;
        run.level = embedding_level;
        run.last_strong = context_level & 1;
        run.last_base = context_level & 1;
        run.context_dir = context_level & 1;
        run.context_pos = last_cc_pos;
    }

    /// `bracketProcessLRI_RLI` (1745-1759).
    fn bracket_process_lri_rli(bd: &mut BracketData, level: u8) {
        bd.iso_runs[bd.iso_run_last].last_base = ON;
        let last_limit = bd.iso_runs[bd.iso_run_last].limit;
        bd.iso_run_last += 1;
        bd.iso_runs[bd.iso_run_last] = IsoRun {
            context_pos: 0,
            start: last_limit,
            limit: last_limit,
            level,
            last_strong: level & 1,
            last_base: level & 1,
            context_dir: level & 1,
        };
    }

    /// `bracketProcessPDI` (1761-1767).
    fn bracket_process_pdi(bd: &mut BracketData) {
        bd.iso_run_last -= 1;
        bd.iso_runs[bd.iso_run_last].last_base = ON;
    }

    /// `bracketAddOpening` (1769-1793).
    fn bracket_add_opening(bd: &mut BracketData, matching: u32, position: usize) {
        let run = bd.iso_runs[bd.iso_run_last];
        let opening = Opening {
            position,
            matching: i64::from(matching),
            context_dir: run.context_dir,
            context_pos: run.context_pos,
            flags: 0,
        };
        if run.limit < bd.openings.len() {
            bd.openings[run.limit] = opening;
        } else {
            bd.openings.push(opening);
        }
        bd.iso_runs[bd.iso_run_last].limit += 1;
    }

    /// `fixN0c` (1795-1819).
    fn fix_n0c(
        &mut self,
        bd: &mut BracketData,
        opening_index: usize,
        new_prop_position: usize,
        new_prop: u8,
    ) {
        let limit = bd.iso_runs[bd.iso_run_last].limit;
        let mut k = opening_index + 1;
        while k < limit {
            let q = bd.openings[k];
            k += 1;
            if q.matching >= 0 {
                continue;
            }
            if new_prop_position < q.context_pos {
                break;
            }
            if new_prop_position >= q.position {
                continue;
            }
            if new_prop == q.context_dir {
                break;
            }
            let opening_position = q.position;
            self.dir_props[opening_position] = new_prop;
            let closing_position = (-q.matching) as usize;
            self.dir_props[closing_position] = new_prop;
            bd.openings[k - 1].matching = 0;
            self.fix_n0c(bd, k - 1, opening_position, new_prop);
            self.fix_n0c(bd, k - 1, closing_position, new_prop);
        }
    }

    /// `bracketProcessClosing` (1821-1896): L or R for N0b/N0c, ON for N0d.
    fn bracket_process_closing(
        &mut self,
        bd: &mut BracketData,
        open_idx: usize,
        position: usize,
    ) -> u8 {
        let run = bd.iso_runs[bd.iso_run_last];
        let opening = bd.openings[open_idx];
        let direction = run.level & 1;
        let mut stable = true;
        let new_prop;
        if (direction == 0 && opening.flags & FOUND_L != 0)
            || (direction == 1 && opening.flags & FOUND_R != 0)
        {
            new_prop = direction;
        } else if opening.flags & (FOUND_L | FOUND_R) != 0 {
            stable = open_idx == run.start;
            new_prop = if direction != opening.context_dir {
                opening.context_dir
            } else {
                direction
            };
        } else {
            bd.iso_runs[bd.iso_run_last].limit = open_idx;
            return ON;
        }
        self.dir_props[opening.position] = new_prop;
        self.dir_props[position] = new_prop;
        self.fix_n0c(bd, open_idx, opening.position, new_prop);
        if stable {
            let run = &mut bd.iso_runs[bd.iso_run_last];
            run.limit = open_idx;
            while run.limit > run.start && bd.openings[run.limit - 1].position == opening.position {
                run.limit -= 1;
            }
        } else {
            bd.openings[open_idx].matching = -(position as i64);
            let run = bd.iso_runs[bd.iso_run_last];
            let mut k = open_idx as i64 - 1;
            while k >= run.start as i64 && bd.openings[k as usize].position == opening.position {
                bd.openings[k as usize].matching = 0;
                k -= 1;
            }
            for k in open_idx + 1..run.limit {
                if bd.openings[k].position >= position {
                    break;
                }
                if bd.openings[k].matching > 0 {
                    bd.openings[k].matching = 0;
                }
            }
        }
        new_prop
    }

    /// `bracketProcessChar` (1898-2020).
    fn bracket_process_char(&mut self, bd: &mut BracketData, position: usize) {
        let dir_prop = self.dir_props[position];
        let mut new_prop;
        if dir_prop == ON {
            let mut c = u32::from(self.text[position]);
            let run = bd.iso_runs[bd.iso_run_last];
            let mut idx = run.limit;
            while idx > run.start {
                idx -= 1;
                if bd.openings[idx].matching != i64::from(c) {
                    continue;
                }
                new_prop = self.bracket_process_closing(bd, idx, position);
                if new_prop == ON {
                    c = 0;
                    break;
                }
                let run = &mut bd.iso_runs[bd.iso_run_last];
                run.last_base = ON;
                run.context_dir = new_prop;
                run.context_pos = position;
                let level = self.levels[position];
                if level & LEVEL_OVERRIDE != 0 {
                    let new_prop = level & 1;
                    run.last_strong = new_prop;
                    let start = run.start;
                    for i in start..idx {
                        bd.openings[i].flags |= flag(new_prop);
                    }
                    self.levels[position] &= !LEVEL_OVERRIDE;
                }
                let opened = bd.openings[idx].position;
                self.levels[opened] &= !LEVEL_OVERRIDE;
                return;
            }
            // Synonyms: an opening entry for each canonical equivalent.
            if c != 0
                && let Some((matching, true)) = paired_bracket(c)
                && matching != c
            {
                if matching == RIGHT_POINTING_ANGLE_BRACKET {
                    Self::bracket_add_opening(bd, RIGHT_ANGLE_BRACKET, position);
                } else if matching == RIGHT_ANGLE_BRACKET {
                    Self::bracket_add_opening(bd, RIGHT_POINTING_ANGLE_BRACKET, position);
                }
                Self::bracket_add_opening(bd, matching, position);
            }
        }
        let level = self.levels[position];
        let run = &mut bd.iso_runs[bd.iso_run_last];
        if level & LEVEL_OVERRIDE != 0 {
            new_prop = level & 1;
            if dir_prop != S && dir_prop != WS && dir_prop != ON {
                self.dir_props[position] = new_prop;
            }
            run.last_base = new_prop;
            run.last_strong = new_prop;
            run.context_dir = new_prop;
            run.context_pos = position;
        } else if dir_prop <= R || dir_prop == AL {
            new_prop = dir_from_strong(dir_prop);
            run.last_base = dir_prop;
            run.last_strong = dir_prop;
            run.context_dir = new_prop;
            run.context_pos = position;
        } else if dir_prop == EN {
            run.last_base = EN;
            if run.last_strong == L {
                new_prop = L;
                self.dir_props[position] = ENL;
                run.context_dir = L;
                run.context_pos = position;
            } else {
                new_prop = R;
                self.dir_props[position] = if run.last_strong == AL { AN } else { ENR };
                run.context_dir = R;
                run.context_pos = position;
            }
        } else if dir_prop == AN {
            new_prop = R;
            run.last_base = AN;
            run.context_dir = R;
            run.context_pos = position;
        } else if dir_prop == NSM {
            new_prop = run.last_base;
            if new_prop == ON {
                self.dir_props[position] = new_prop;
            }
        } else {
            new_prop = dir_prop;
            run.last_base = dir_prop;
        }
        if new_prop <= R || new_prop == AL {
            let found = flag(dir_from_strong(new_prop));
            let run = bd.iso_runs[bd.iso_run_last];
            for i in run.start..run.limit {
                if position > bd.openings[i].position {
                    bd.openings[i].flags |= found;
                }
            }
        }
    }

    /// `resolveExplicitLevels` (2090-2340).
    fn resolve_explicit_levels(&mut self) -> Direction {
        let mut level = self.para_level_at(0);
        self.isolate_count = 0;
        let direction = self.direction_from_flags();
        if direction != Direction::Mixed {
            return direction;
        }
        if self.flags & (MASK_EXPLICIT | MASK_ISO) == 0 {
            let mut bd = self.bracket_init();
            for para in 0..self.paras_limit.len() {
                let start = if para == 0 {
                    0
                } else {
                    self.paras_limit[para - 1]
                };
                let limit = self.paras_limit[para];
                level = self.paras_level[para];
                for i in start..limit {
                    self.levels[i] = level;
                    let dir_prop = self.dir_props[i];
                    if dir_prop == BN {
                        continue;
                    }
                    if dir_prop == B {
                        if i + 1 < self.length {
                            if self.text[i] == CR && self.text[i + 1] == LF {
                                continue;
                            }
                            Self::bracket_process_b(&mut bd, level);
                        }
                        continue;
                    }
                    self.bracket_process_char(&mut bd, i);
                }
            }
            return direction;
        }
        let mut embedding_level = level;
        let mut previous_level = level;
        let mut last_cc_pos = 0;
        // (level, isolate) entries.
        let mut stack = [(0u8, false); MAX_EXPLICIT_LEVEL as usize + 2];
        let mut stack_last = 0usize;
        let mut overflow_isolate_count = 0u32;
        let mut overflow_embedding_count = 0u32;
        let mut valid_isolate_count = 0i64;
        let mut bd = self.bracket_init();
        stack[0] = (level, false);
        self.flags = 0;
        for i in 0..self.length {
            let dir_prop = self.dir_props[i];
            match dir_prop {
                LRE | RLE | LRO | RLO => {
                    self.flags |= flag(BN);
                    self.levels[i] = previous_level;
                    let new_level = if dir_prop == LRE || dir_prop == LRO {
                        (embedding_level.wrapping_add(2)) & !(LEVEL_OVERRIDE | 1)
                    } else {
                        (no_override(embedding_level) + 1) | 1
                    };
                    if new_level <= MAX_EXPLICIT_LEVEL
                        && overflow_isolate_count == 0
                        && overflow_embedding_count == 0
                    {
                        last_cc_pos = i;
                        embedding_level = new_level;
                        if dir_prop == LRO || dir_prop == RLO {
                            embedding_level |= LEVEL_OVERRIDE;
                        }
                        stack_last += 1;
                        stack[stack_last] = (embedding_level, false);
                    } else if overflow_isolate_count == 0 {
                        overflow_embedding_count += 1;
                    }
                }
                PDF => {
                    self.flags |= flag(BN);
                    self.levels[i] = previous_level;
                    if overflow_isolate_count > 0 {
                        continue;
                    }
                    if overflow_embedding_count > 0 {
                        overflow_embedding_count -= 1;
                        continue;
                    }
                    if stack_last > 0 && !stack[stack_last].1 {
                        last_cc_pos = i;
                        stack_last -= 1;
                        embedding_level = stack[stack_last].0;
                    }
                }
                LRI | RLI => {
                    self.flags |= flag(ON) | flag_lr(embedding_level);
                    self.levels[i] = no_override(embedding_level);
                    if no_override(embedding_level) != no_override(previous_level) {
                        self.bracket_process_boundary(
                            &mut bd,
                            last_cc_pos,
                            previous_level,
                            embedding_level,
                        );
                        self.flags |= DIR_PROP_FLAG_MULTI_RUNS;
                    }
                    previous_level = embedding_level;
                    let new_level = if dir_prop == LRI {
                        (embedding_level.wrapping_add(2)) & !(LEVEL_OVERRIDE | 1)
                    } else {
                        (no_override(embedding_level) + 1) | 1
                    };
                    if new_level <= MAX_EXPLICIT_LEVEL
                        && overflow_isolate_count == 0
                        && overflow_embedding_count == 0
                    {
                        self.flags |= flag(dir_prop);
                        last_cc_pos = i;
                        valid_isolate_count += 1;
                        if valid_isolate_count > self.isolate_count {
                            self.isolate_count = valid_isolate_count;
                        }
                        embedding_level = new_level;
                        stack_last += 1;
                        stack[stack_last] = (embedding_level, true);
                        Self::bracket_process_lri_rli(&mut bd, embedding_level);
                    } else {
                        self.dir_props[i] = WS;
                        overflow_isolate_count += 1;
                    }
                }
                PDI => {
                    if no_override(embedding_level) != no_override(previous_level) {
                        self.bracket_process_boundary(
                            &mut bd,
                            last_cc_pos,
                            previous_level,
                            embedding_level,
                        );
                        self.flags |= DIR_PROP_FLAG_MULTI_RUNS;
                    }
                    if overflow_isolate_count > 0 {
                        overflow_isolate_count -= 1;
                        self.dir_props[i] = WS;
                    } else if valid_isolate_count > 0 {
                        self.flags |= flag(PDI);
                        last_cc_pos = i;
                        overflow_embedding_count = 0;
                        while !stack[stack_last].1 {
                            stack_last -= 1;
                        }
                        stack_last -= 1;
                        valid_isolate_count -= 1;
                        Self::bracket_process_pdi(&mut bd);
                    } else {
                        self.dir_props[i] = WS;
                    }
                    embedding_level = stack[stack_last].0;
                    self.flags |= flag(ON) | flag_lr(embedding_level);
                    previous_level = embedding_level;
                    self.levels[i] = no_override(embedding_level);
                }
                B => {
                    self.flags |= flag(B);
                    self.levels[i] = self.para_level_at(i);
                    if i + 1 < self.length {
                        if self.text[i] == CR && self.text[i + 1] == LF {
                            continue;
                        }
                        overflow_embedding_count = 0;
                        overflow_isolate_count = 0;
                        valid_isolate_count = 0;
                        stack_last = 0;
                        embedding_level = self.para_level_at(i + 1);
                        previous_level = embedding_level;
                        stack[0] = (embedding_level, false);
                        Self::bracket_process_b(&mut bd, embedding_level);
                    }
                }
                BN => {
                    self.levels[i] = previous_level;
                    self.flags |= flag(BN);
                }
                _ => {
                    if no_override(embedding_level) != no_override(previous_level) {
                        self.bracket_process_boundary(
                            &mut bd,
                            last_cc_pos,
                            previous_level,
                            embedding_level,
                        );
                        self.flags |= DIR_PROP_FLAG_MULTI_RUNS;
                        if embedding_level & LEVEL_OVERRIDE != 0 {
                            self.flags |= flag_o(embedding_level);
                        } else {
                            self.flags |= flag_e(embedding_level);
                        }
                    }
                    previous_level = embedding_level;
                    self.levels[i] = embedding_level;
                    self.bracket_process_char(&mut bd, i);
                    self.flags |= flag(self.dir_props[i]);
                }
            }
        }
        if self.flags & MASK_EMBEDDING != 0 {
            self.flags |= flag_lr(self.para_level);
        }
        self.direction_from_flags()
    }

    /// `setLevelsOutsideIsolates` (2788-2802).
    fn set_levels_outside_isolates(&mut self, start: usize, limit: usize, level: u8) {
        let mut isolate_count = 0i64;
        for k in start..limit {
            let dir_prop = self.dir_props[k];
            if dir_prop == PDI {
                isolate_count -= 1;
            }
            if isolate_count == 0 {
                self.levels[k] = level;
            }
            if dir_prop == LRI || dir_prop == RLI {
                isolate_count += 1;
            }
        }
    }

    /// `processPropertySeq` (2819-3012), impTab_DEFAULT (actions 1-3).
    fn process_property_seq(
        &mut self,
        lev: &mut LevState,
        prop: u8,
        mut start: usize,
        limit: usize,
    ) {
        let start0 = start;
        let old_state = lev.state;
        let cell = lev.imp_tab[old_state as usize][prop as usize];
        lev.state = cell & 0x0f;
        let action = cell >> 4;
        let add_level = lev.imp_tab[lev.state as usize][IMPTABLEVELS_RES];
        match action {
            0 => {}
            1 => lev.start_on = start0 as i64,
            2 => start = lev.start_on as usize,
            3 => {
                let level = lev.run_level + 1;
                self.set_levels_outside_isolates(lev.start_on as usize, start0, level);
            }
            _ => unreachable!("impTab_DEFAULT has actions 1..3 only"),
        }
        if add_level != 0 || start < start0 {
            let level = lev.run_level + add_level;
            if start >= lev.run_start {
                for k in start..limit {
                    self.levels[k] = level;
                }
            } else {
                self.set_levels_outside_isolates(start, limit, level);
            }
        }
    }

    /// `resolveImplicitLevels` (3014-3165) for REORDER_DEFAULT.
    fn resolve_implicit_levels(&mut self, start: usize, limit: usize, sor: u8, eor: u8) {
        let run_level = self.levels[start];
        let mut lev = LevState {
            imp_tab: if run_level & 1 == 0 {
                &IMP_TAB_L_DEFAULT
            } else {
                &IMP_TAB_R_DEFAULT
            },
            start_on: -1,
            run_start: start,
            state: 0,
            run_level,
        };
        let mut start1;
        let mut state_imp: u8;
        if self.dir_props[start] == PDI {
            let isolate = self.isolates[self.isolate_count as usize];
            lev.start_on = isolate.start_on;
            start1 = isolate.start1;
            state_imp = isolate.state_imp;
            lev.state = isolate.state;
            self.isolate_count -= 1;
        } else {
            lev.start_on = -1;
            start1 = start;
            state_imp = if self.dir_props[start] == NSM {
                1 + sor
            } else {
                0
            };
            lev.state = 0;
            self.process_property_seq(&mut lev, sor, start, start);
        }
        let mut start2 = start;
        let mut i = start;
        while i <= limit {
            let gprop = if i >= limit {
                let mut k = limit - 1;
                while k > start && flag(self.dir_props[k]) & MASK_BN_EXPLICIT != 0 {
                    k -= 1;
                }
                let dir_prop = self.dir_props[k];
                if dir_prop == LRI || dir_prop == RLI {
                    break;
                }
                eor
            } else {
                let prop = self.dir_props[i];
                if prop == B {
                    self.isolate_count = -1;
                }
                GROUP_PROP[prop as usize]
            };
            let old_state_imp = state_imp;
            let cell = IMP_TAB_PROPS[old_state_imp as usize][gprop as usize];
            state_imp = cell & 0x1f;
            let mut action = cell >> 5;
            if i == limit && action == 0 {
                action = 1;
            }
            if action != 0 {
                let res_prop = IMP_TAB_PROPS[old_state_imp as usize][IMPTABPROPS_RES];
                match action {
                    1 => {
                        self.process_property_seq(&mut lev, res_prop, start1, i);
                        start1 = i;
                    }
                    2 => start2 = i,
                    3 => {
                        self.process_property_seq(&mut lev, res_prop, start1, start2);
                        self.process_property_seq(&mut lev, P_ON, start2, i);
                        start1 = i;
                    }
                    4 => {
                        self.process_property_seq(&mut lev, res_prop, start1, start2);
                        start1 = start2;
                        start2 = i;
                    }
                    _ => unreachable!("impTabProps actions are 1..4"),
                }
            }
            i += 1;
        }
        let mut i = limit - 1;
        while i > start && flag(self.dir_props[i]) & MASK_BN_EXPLICIT != 0 {
            i -= 1;
        }
        let dir_prop = self.dir_props[i];
        if (dir_prop == LRI || dir_prop == RLI) && limit < self.length {
            self.isolate_count += 1;
            let index = self.isolate_count as usize;
            if index >= self.isolates.len() {
                self.isolates.resize(index + 1, Isolate::default());
            }
            self.isolates[index] = Isolate {
                state_imp,
                state: lev.state,
                start1,
                start_on: lev.start_on,
            };
        } else {
            self.process_property_seq(&mut lev, eor, limit, limit);
        }
    }

    /// `adjustWSLevels` (3167-3199): (L1) and the levels of X9-removed
    /// characters.
    fn adjust_ws_levels(&mut self) {
        if self.flags & MASK_WS == 0 {
            return;
        }
        let mut i = self.length;
        while i > 0 {
            while i > 0 {
                let f = flag(self.dir_props[i - 1]);
                if f & MASK_WS == 0 {
                    break;
                }
                i -= 1;
                self.levels[i] = self.para_level_at(i);
            }
            while i > 0 {
                i -= 1;
                let f = flag(self.dir_props[i]);
                if f & MASK_BN_EXPLICIT != 0 {
                    self.levels[i] = self.levels[i + 1];
                } else if f & MASK_B_S != 0 {
                    self.levels[i] = self.para_level_at(i);
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels_of(text: &[u32], para: u8) -> (u8, Vec<u8>, Vec<usize>) {
        let mut units = Vec::new();
        let mut first = Vec::new();
        for &code in text {
            first.push(units.len());
            if code >= 0x10000 {
                let c = code - 0x10000;
                units.push(0xD800 + (c >> 10) as u16);
                units.push(0xDC00 + (c & 0x3FF) as u16);
            } else {
                units.push(code as u16);
            }
        }
        let paragraph = set_para(&units, para);
        (paragraph.para_level, paragraph.levels, first)
    }

    fn conformance_line(line: &str) -> bool {
        let fields: Vec<&str> = line.split(';').collect();
        let codes: Vec<u32> = fields[0]
            .split_whitespace()
            .map(|h| u32::from_str_radix(h, 16).unwrap())
            .collect();
        let para = match fields[1] {
            "0" => 0,
            "1" => 1,
            _ => LEVEL_DEFAULT_LTR,
        };
        let expected_para: u8 = fields[2].parse().unwrap();
        let (para_level, levels, first) = levels_of(&codes, para);
        para_level == expected_para
            && fields[3]
                .split_whitespace()
                .enumerate()
                .all(|(i, e)| e == "x" || e.parse::<u8>().unwrap() == levels[first[i]])
    }

    /// A stored sample of Unicode 15.0.0 BidiCharacterTest.txt. (The full
    /// file differs from this port, as from JDK's BidiBase, on 31 of 91707
    /// lines: overridden brackets still pair, an NSM after a bracket keeps
    /// ON (no Unicode 8.0 N0 amendment), more than 63 openings still pair,
    /// LRE/LRO in otherwise left-to-right text are not resolved, and
    /// isolate initiators under an override are not overridden.)
    #[test]
    fn conformance_sample() {
        let text = String::from_utf8(crate::font_chain::pinned_shaping_source(
            "unicode-15.0.0/BidiCharacterTest-sample.txt",
        ))
        .unwrap();
        let lines: Vec<&str> = text.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(lines.len(), 946);
        for line in lines {
            assert!(conformance_line(line), "{line}");
        }
    }

    /// Drift gate: `BIDI_CLASSES` and `BRACKETS` are JDK 21.0.5's
    /// `ubidi.icu` (Trie2_16.get, UBiDiProps.getClass, getPairedBracket).
    #[test]
    fn bidi_tables_are_jdk_ubidi_icu() {
        let raw = crate::font_chain::pinned_shaping_source("jdk-21.0.5-ga/ubidi.icu");
        let u16_at = |o: usize| u16::from_le_bytes([raw[o], raw[o + 1]]);
        let i32_at = |o: usize| i32::from_le_bytes([raw[o], raw[o + 1], raw[o + 2], raw[o + 3]]);
        let header = usize::from(u16_at(0));
        assert_eq!(&raw[12..16], b"BiDi");
        let count = i32_at(header) as usize;
        let indexes: Vec<i32> = (0..count).map(|i| i32_at(header + 4 * i)).collect();
        let trie = header + 4 * count;
        assert_eq!(i32_at(trie), 0x5472_6932);
        let index_length = usize::from(u16_at(trie + 6));
        let data_length = usize::from(u16_at(trie + 8)) << 2;
        let high_start = u32::from(u16_at(trie + 14)) << 11;
        let array: Vec<u16> = (0..index_length + data_length)
            .map(|i| u16_at(trie + 16 + 2 * i))
            .collect();
        let at = |i: u16, c: u32| usize::from(i) * 4 + (c & 31) as usize;
        let props = |c: u32| -> u16 {
            if c < 0xD800 || (0xDC00..=0xFFFF).contains(&c) {
                array[at(array[(c >> 5) as usize], c)]
            } else if c <= 0xFFFF {
                array[at(array[2048 + ((c - 0xD800) >> 5) as usize], c)]
            } else if c < high_start {
                let i1 = array[2080 + (c >> 11) as usize];
                array[at(array[usize::from(i1) + ((c >> 5) & 63) as usize], c)]
            } else {
                array[data_length - 4 + index_length]
            }
        };
        let mirrors: Vec<u32> = (0..indexes[3] as usize)
            .map(|i| i32_at(trie + indexes[2] as usize + 4 * i) as u32)
            .collect();
        let mirror = |c: u32, p: u16| -> u32 {
            let delta = (p as i16) >> 13;
            if delta != -4 {
                return (c as i64 + i64::from(delta)) as u32;
            }
            for &m in &mirrors {
                if m & 0x1F_FFFF == c {
                    return mirrors[(m >> 21) as usize] & 0x1F_FFFF;
                }
            }
            c
        };
        let mut brackets = Vec::new();
        for c in 0..0x11_0000u32 {
            let p = props(c);
            let class = (p & 0x1F) as u8;
            let class = if class >= 23 { ON } else { class };
            assert_eq!(bidi_class(c), class, "U+{c:04X}");
            let bpt = (p & 0x300) >> 8;
            if bpt != 0 {
                brackets.push((c, mirror(c, p), bpt == 1));
            }
        }
        assert_eq!(brackets, data::BRACKETS);
        use sha2::{Digest, Sha256};
        let hash: String = Sha256::digest(&raw)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(hash, data::UBIDI_ICU_SHA256);
    }

    /// Full Unicode 15.0.0 conformance files, when `JAVA_BIDI_CONFORMANCE_DIR`
    /// names a directory holding BidiCharacterTest.txt (diagnostic only).
    #[test]
    #[ignore]
    fn conformance_bidi_character_test() {
        let dir = std::env::var("JAVA_BIDI_CONFORMANCE_DIR").unwrap();
        let text = std::fs::read_to_string(format!("{dir}/BidiCharacterTest.txt")).unwrap();
        let (mut total, mut failed) = (0, 0);
        for line in text.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split(';').collect();
            let codes: Vec<u32> = fields[0]
                .split_whitespace()
                .map(|h| u32::from_str_radix(h, 16).unwrap())
                .collect();
            let para = match fields[1] {
                "0" => 0,
                "1" => 1,
                _ => LEVEL_DEFAULT_LTR,
            };
            let expected_para: u8 = fields[2].parse().unwrap();
            let expected: Vec<&str> = fields[3].split_whitespace().collect();
            let (para_level, levels, first) = levels_of(&codes, para);
            total += 1;
            let ok = para_level == expected_para
                && expected
                    .iter()
                    .enumerate()
                    .all(|(i, e)| *e == "x" || e.parse::<u8>().unwrap() == levels[first[i]]);
            if !ok {
                failed += 1;
                if failed < 8 {
                    eprintln!("FAIL {line}\n  got para {para_level} levels {levels:?}");
                }
            }
        }
        eprintln!("{failed}/{total} failed");
        // The 31 lines where JDK's BidiBase departs from UAX #9 (see
        // conformance_sample).
        assert_eq!(failed, 31);
    }
}
