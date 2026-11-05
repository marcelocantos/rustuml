"""Generate crates/rustuml-render/src/font_chain_unicode.rs from Unicode Character Database files.

Usage: python3 -I tools/font_chain_unicode.py <out.rs> <Scripts-3.2.0.txt> <UnicodeData-18.0.0.txt[.gz]>
         <hb-ot-shaper.hh> <hb-script-list.h> <hb-ot-layout.cc> [ScriptRunData.java]

The HarfBuzz files are those of tag 15.0.0, stored under tools/font-chain-reference/shaping-sources/
(with UnicodeData-18.0.0.txt.gz); the drift gate (font_chain tests) re-derives every table below
from the stored files.

* NONSPACING_MARKS: general category Mn in Unicode 18.0.0, the UCD of HarfBuzz 15.0.0 (hb-ucd-table.hh;
  the library the reference JVM loads, /opt/homebrew/Cellar/harfbuzz/15.0.0_1, T14.6.8), which
  hb_synthesize_glyph_classes and zero_mark_advances read; UNICODE_MARKS: Mn, Mc and Me
  (_hb_glyph_info_is_unicode_mark, the cluster rule of _hb_ot_shape_fallback_mark_position).
* SCRIPTS: the strong scripts of Unicode 3.2.0 Scripts.txt as ICU UScriptCode values 2..45; Common,
  Inherited and unassigned code points are absent (weak). JDK 21's sun.font.ScriptRunData equals this
  file for every code point; pass ScriptRunData.java to re-check that equality while generating.
* HB_SCRIPTS: the HarfBuzz script (ISO 15924 tag, hb-script-list.h) of each ICU UScriptCode 0..45, as
  JDK's scriptMapping.c ICU_to_Harfbuzz_ScriptCode maps them (jdk21u jdk-21.0.5-ga, sha256 0a329fea...).
* SCRIPT_SHAPERS: the ICU codes whose HarfBuzz script is not in hb_ot_shaper_categorize's default case
  (hb-ot-shaper.hh 176-421, 15.0.0), with the case group it is in (font_chain::ShaperGroup); the
  group's conditions on the GSUB chosen script are ported in font_chain::ShaperGroup::categorize.
* GDEF_BLOCKLIST: the (GDEF, GSUB, GPOS) table lengths OT::GDEF::is_blocklisted lists
  (hb-ot-layout.cc 152-266, 15.0.0).
"""
import bisect, gzip, hashlib, re, sys

out_path, scripts_path, unicode_data_path, shaper_path, script_list_path, layout_path = sys.argv[1:7]
jdk_path = sys.argv[7] if len(sys.argv) > 7 else None

# ICU UScriptCode 0..45 (the codes JDK's scriptMapping.c ICU_to_Harfbuzz_ScriptCode maps).
ICU = ("COMMON INHERITED ARABIC ARMENIAN BENGALI BOPOMOFO CHEROKEE COPTIC CYRILLIC DESERET DEVANAGARI "
       "ETHIOPIC GEORGIAN GOTHIC GREEK GUJARATI GURMUKHI HAN HANGUL HEBREW HIRAGANA KANNADA KATAKANA KHMER "
       "LAO LATIN MALAYALAM MONGOLIAN MYANMAR OGHAM OLD_ITALIC ORIYA RUNIC SINHALA SYRIAC TAMIL TELUGU THAANA "
       "THAI TIBETAN CANADIAN_ABORIGINAL YI TAGALOG HANUNOO BUHID TAGBANWA").split()
# JDK scriptMapping.c: ICU code -> HB_SCRIPT_* name (the ICU name except CANADIAN_ABORIGINAL).
HB_NAMES = [("CANADIAN_SYLLABICS" if n == "CANADIAN_ABORIGINAL" else n) for n in ICU]


def sha(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def read_text(path):
    data = open(path, "rb").read()
    return (gzip.decompress(data) if path.endswith(".gz") else data).decode("utf-8")


def sha_text(path):
    return hashlib.sha256(read_text(path).encode("utf-8")).hexdigest()


tags = dict((m.group(1), "".join(m.group(2, 3, 4, 5))) for m in re.finditer(
    r"HB_SCRIPT_(\w+)\s*=\s*HB_TAG\s*\('(.)','(.)','(.)','(.)'\)", open(script_list_path).read()))
hb_scripts = [tags[n] for n in HB_NAMES]

# hb_ot_shaper_categorize: case labels collect until the returns that follow them.
GROUPS = {("arabic", "default"): "Arabic", ("thai",): "Thai", ("hangul",): "Hangul", ("hebrew",): "Hebrew",
          ("default", "use", "indic"): "Indic", ("khmer",): "Khmer", ("default", "myanmar"): "Myanmar",
          ("myanmar_zawgyi",): None, ("default", "use"): "Use", ("default",): "Default"}
body = open(shaper_path).read()
body = body[body.index("hb_ot_shaper_categorize (hb_script_t script"):]
body = body[:body.index("\n}\n")]
group_of, labels, returns = {}, [], []
for line in body.splitlines():
    case = re.match(r"\s*case HB_SCRIPT_(\w+):", line)
    ret = re.search(r"return &_hb_ot_shaper_(\w+);", line)
    if (case or re.match(r"\s*default:", line)) and returns:
        group = GROUPS[tuple(returns)]
        for label in labels:
            group_of[label] = group
        labels, returns = [], []
    if case:
        labels.append(case.group(1))
    if ret:
        returns.append(ret.group(1))
if returns:
    for label in labels:
        group_of[label] = GROUPS[tuple(returns)]
shapers = [(code, group_of[name]) for code, name in enumerate(HB_NAMES)
           if group_of.get(name, "Default") != "Default"]

layout = open(layout_path).read()
layout = layout[layout.index("OT::GDEF::is_blocklisted"):]
layout = layout[:layout.index("\n}\n")]
blocklist = [tuple(int(x) for x in m.groups()) for m in re.finditer(
    r"case HB_CODEPOINT_ENCODE3 \((\d+), (\d+), (\d+)\):", layout)]


script = {}
for line in open(scripts_path, encoding="utf-8"):
    line = line.split("#")[0].strip()
    if not line:
        continue
    cps, name = [x.strip() for x in line.split(";")]
    first, last = (cps.split("..") + [cps])[:2]
    code = ICU.index(name.upper())
    for cp in range(int(first, 16), int(last, 16) + 1):
        script[cp] = code

if jdk_path:
    body = re.sub(r"//[^\n]*", "", re.search(r"private static final int\[\] data\s*=\s*\{(.*?)\};",
                                             open(jdk_path).read(), re.S).group(1))
    nums = [int(x, 16) if x.lower().startswith("0x") else int(x) for x in re.findall(r"0x[0-9a-fA-F]+|\b\d+\b", body)]
    starts, codes = nums[0::2], nums[1::2]
    diff = sum(1 for cp in range(0x110000) if codes[bisect.bisect_right(starts, cp) - 1] != script.get(cp, 0))
    assert diff == 0, f"ScriptRunData differs from Scripts-3.2.0 on {diff} code points"

strong = []
for cp in sorted(script):
    code = script[cp]
    if code <= 1:
        continue
    if strong and strong[-1][1] + 1 == cp and strong[-1][2] == code:
        strong[-1][1] = cp
    else:
        strong.append([cp, cp, code])

def category_runs(categories):
    runs = []
    for line in read_text(unicode_data_path).splitlines():
        fields = line.split(";")
        if fields[2] not in categories:
            continue
        assert not fields[1].endswith(", First>"), fields
        cp = int(fields[0], 16)
        if runs and runs[-1][1] + 1 == cp:
            runs[-1][1] = cp
        else:
            runs.append([cp, cp])
    return runs


marks = category_runs({"Mn"})
unicode_marks = category_runs({"Mn", "Mc", "Me"})


def rows(items, fmt, per_line):
    out, line = [], []
    for item in items:
        line.append(fmt(item))
        if len(line) == per_line:
            out.append("    " + ", ".join(line) + ",")
            line = []
    if line:
        out.append("    " + ", ".join(line) + ",")
    return "\n".join(out)


text = f"""// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

// Generated by tools/font_chain_unicode.py; do not edit by hand.
// Sources (https://www.unicode.org/Public/):
//   3.2-Update/Scripts-3.2.0.txt sha256 {sha(scripts_path)}
//   18.0.0/ucd/UnicodeData.txt sha256 {sha_text(unicode_data_path)}
// HarfBuzz 15.0.0 (https://github.com/harfbuzz/harfbuzz, 35f656a1a2bab548ea7f54a6dd3fba2b09680d14):
//   src/hb-ot-shaper.hh sha256 {sha(shaper_path)}
//   src/hb-script-list.h sha256 {sha(script_list_path)}
//   src/hb-ot-layout.cc sha256 {sha(layout_path)}

use crate::font_chain::ShaperGroup;

/// General category Mn in Unicode 18.0.0 (HarfBuzz 15.0.0's UCD).
pub(crate) const NONSPACING_MARKS: &[(u32, u32)] = &[
{rows(marks, lambda r: f"(0x{r[0]:X}, 0x{r[1]:X})", 6)}
];

/// General categories Mn, Mc and Me in Unicode 18.0.0.
pub(crate) const UNICODE_MARKS: &[(u32, u32)] = &[
{rows(unicode_marks, lambda r: f"(0x{r[0]:X}, 0x{r[1]:X})", 6)}
];

/// Strong scripts of Unicode 3.2.0 (equal to JDK 21's `ScriptRunData`) as ICU
/// `UScriptCode` values; absent code points are Common, Inherited or
/// unassigned.
pub(crate) const SCRIPTS: &[(u32, u32, u8)] = &[
{rows(strong, lambda r: f"(0x{r[0]:X}, 0x{r[1]:X}, {r[2]})", 4)}
];

/// The HarfBuzz script tag of each ICU `UScriptCode` JDK maps (scriptMapping.c).
pub(crate) const HB_SCRIPTS: [[u8; 4]; {len(hb_scripts)}] = [
{rows(hb_scripts, lambda t: f"*b\"{t}\"", 6)}
];

/// ICU codes whose HarfBuzz script is in a non-default case group of
/// `hb_ot_shaper_categorize` (15.0.0), sorted.
pub(crate) const SCRIPT_SHAPERS: &[(u8, ShaperGroup)] = &[
{rows(shapers, lambda r: f"({r[0]}, ShaperGroup::{r[1]})", 4)}
];

/// `OT::GDEF::is_blocklisted` (hb-ot-layout.cc, 15.0.0): GDEF, GSUB and GPOS
/// table lengths whose GDEF HarfBuzz discards.
pub(crate) const GDEF_BLOCKLIST: &[(u32, u32, u32)] = &[
{rows(blocklist, lambda r: f"({r[0]}, {r[1]}, {r[2]})", 4)}
];
"""
open(out_path, "w", encoding="utf-8").write(text)
print(len(marks), "Mn runs,", len(unicode_marks), "mark runs,", len(strong), "script runs,", len(shapers),
      "shaper scripts,", len(blocklist), "blocklisted GDEFs")
