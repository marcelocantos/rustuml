"""Generate crates/rustuml-render/src/java_bidi_data.rs from JDK 21.0.5's ICU bidi data.

Usage: python3 -I tools/java_bidi_data.py <out.rs> <ubidi.icu> [<DerivedBidiClass.txt> <BidiBrackets.txt>]

<ubidi.icu> is src/java.base/share/classes/jdk/internal/icu/impl/data/icudt72b/ubidi.icu of
https://github.com/openjdk/jdk21u at tag jdk-21.0.5-ga (the reference JVM is Homebrew openjdk@21
21.0.5). jdk.internal.icu.impl.UBiDiProps reads it: a serialized 16-bit UTrie2
(jdk.internal.icu.impl.Trie2_16.get) whose value holds the bidi class in bits 0..4 (getClass), the
Bidi_Paired_Bracket_Type in bits 8..9 (getPairedBracketType) and a mirroring delta in bits 13..15,
escaped to the mirrors[] table when -4 (getMirror; getPairedBracket returns the mirror of a bracket).
BidiBase.getCustomizedClass maps a class >= CHAR_DIRECTION_COUNT (23) to ON.

When the Unicode 15.0.0 files (https://www.unicode.org/Public/15.0.0/ucd/, the data version the
file declares) are given, every code point's class, bracket type and paired bracket are checked
against them (DerivedBidiClass.txt with its @missing defaults; BidiBrackets.txt).

* BIDI_CLASSES: runs of code points whose class is not L (ICU UCharDirection values).
* BRACKETS: code points with a paired bracket type: (code point, paired bracket, opening).
"""
import hashlib, struct, sys

out_path, icu_path = sys.argv[1:3]
ucd = sys.argv[3:5] if len(sys.argv) > 4 else None

NAMES = "L R EN ES ET AN CS B S WS ON LRE LRO AL RLE RLO PDF NSM BN FSI LRI RLI PDI".split()
LONG = {"Left_To_Right": "L", "Right_To_Left": "R", "European_Number": "EN", "European_Separator": "ES",
        "European_Terminator": "ET", "Arabic_Number": "AN", "Common_Separator": "CS",
        "Paragraph_Separator": "B", "Segment_Separator": "S", "White_Space": "WS", "Other_Neutral": "ON",
        "Left_To_Right_Embedding": "LRE", "Left_To_Right_Override": "LRO", "Arabic_Letter": "AL",
        "Right_To_Left_Embedding": "RLE", "Right_To_Left_Override": "RLO", "Pop_Directional_Format": "PDF",
        "Nonspacing_Mark": "NSM", "Boundary_Neutral": "BN", "First_Strong_Isolate": "FSI",
        "Left_To_Right_Isolate": "LRI", "Right_To_Left_Isolate": "RLI", "Pop_Directional_Isolate": "PDI"}

raw = open(icu_path, "rb").read()
# ICU data header (ICUBinary.readHeader): headerSize, magic 0xda27, UDataInfo.
header_size = struct.unpack_from("<H", raw, 0)[0]
assert raw[2:4] == b"\xda\x27"
info = raw[4:header_size]
assert info[4] == 0 and info[8:12] == b"BiDi" and info[12] == 2, info
data_version = ".".join(str(v) for v in info[16:20])
body = raw[header_size:]
count = struct.unpack_from("<i", body, 0)[0]
indexes = struct.unpack_from(f"<{count}i", body, 0)
trie_at = 4 * count
signature, options, index_length, shifted_data_length, _, _, shifted_high_start = struct.unpack_from(
    "<IHHHHHH", body, trie_at)
assert signature == 0x54726932 and options & 0xF == 0
data_length = shifted_data_length << 2
trie = struct.unpack_from(f"<{index_length + data_length}H", body, trie_at + 16)
high_start = shifted_high_start << 11
high_value_index = data_length - 4 + index_length
mirrors = struct.unpack_from(f"<{indexes[3]}i", body, trie_at + indexes[2])


def props(c):
    """Trie2_16.get (UTRIE2_SHIFT_1 11, SHIFT_2 5, INDEX_SHIFT 2, LSCP_INDEX_2_OFFSET 2048,
    INDEX_1_OFFSET 2112, OMITTED_BMP_INDEX_1_LENGTH 32, INDEX_2_MASK 63)."""
    if c < 0xD800 or 0xDBFF < c <= 0xFFFF:
        return trie[(trie[c >> 5] << 2) + (c & 31)]
    if c <= 0xFFFF:
        return trie[(trie[2048 + ((c - 0xD800) >> 5)] << 2) + (c & 31)]
    if c < high_start:
        ix = trie[2080 + (c >> 11)]
        ix = trie[ix + ((c >> 5) & 63)]
        return trie[(ix << 2) + (c & 31)]
    return trie[high_value_index]


def mirror(c, p):
    delta = (p if p < 0x8000 else p - 0x10000) >> 13
    if delta != -4:
        return c + delta
    for m in mirrors:
        c2 = m & 0x1FFFFF
        if c == c2:
            return mirrors[m >> 21] & 0x1FFFFF
        if c < c2:
            break
    return c


classes, brackets = [], []
for c in range(0x110000):
    p = props(c)
    cls = p & 0x1F
    if cls >= 23:
        cls = NAMES.index("ON")
    classes.append(cls)
    bpt = (p & 0x300) >> 8
    if bpt:
        brackets.append((c, mirror(c, p), bpt == 1))

if ucd:
    derived_path, brackets_path = ucd
    expected = [0] * 0x110000
    for line in open(derived_path, encoding="utf-8"):
        if line.startswith("# @missing:"):
            cps, name = [x.strip() for x in line[len("# @missing:"):].split(";")]
            cls = NAMES.index(LONG[name])
        else:
            line = line.split("#")[0].strip()
            if not line:
                continue
            cps, short = [x.strip() for x in line.split(";")]
            cls = NAMES.index(short)
        first, last = (cps.split("..") + [cps])[:2]
        for c in range(int(first, 16), int(last, 16) + 1):
            expected[c] = cls
    diff = [c for c in range(0x110000) if expected[c] != classes[c]]
    assert not diff, f"ubidi.icu classes differ from DerivedBidiClass on {len(diff)} code points: {diff[:8]}"
    expected_brackets = []
    for line in open(brackets_path, encoding="utf-8"):
        line = line.split("#")[0].strip()
        if not line:
            continue
        cp, pair, kind = [x.strip() for x in line.split(";")]
        expected_brackets.append((int(cp, 16), int(pair, 16), kind == "o"))
    assert sorted(expected_brackets) == brackets, "ubidi.icu brackets differ from BidiBrackets.txt"

runs = []
for c, cls in enumerate(classes):
    if cls == 0:
        continue
    if runs and runs[-1][1] + 1 == c and runs[-1][2] == cls:
        runs[-1][1] = c
    else:
        runs.append([c, c, cls])


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


sha = hashlib.sha256(raw).hexdigest()
checked = ""
if ucd:
    checked = "".join(f"\n// Checked equal to Unicode 15.0.0 {p.rsplit('/', 1)[-1]} sha256 "
                      f"{hashlib.sha256(open(p, 'rb').read()).hexdigest()}" for p in ucd)
text = f"""// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

// Generated by tools/java_bidi_data.py; do not edit by hand.
// Source: OpenJDK jdk21u jdk-21.0.5-ga
//   src/java.base/share/classes/jdk/internal/icu/impl/data/icudt72b/ubidi.icu
//   sha256 {sha} (data version {data_version}){checked}

/// SHA256 of the `ubidi.icu` these tables are decoded from.
#[cfg(test)]
pub(crate) const UBIDI_ICU_SHA256: &str = "{sha}";

/// Code point runs whose ICU bidi class (`UCharDirection`) is not L (0).
pub(crate) const BIDI_CLASSES: &[(u32, u32, u8)] = &[
{rows(runs, lambda r: f"(0x{r[0]:X}, 0x{r[1]:X}, {r[2]})", 4)}
];

/// Bidi paired brackets: (code point, paired bracket, opening bracket).
pub(crate) const BRACKETS: &[(u32, u32, bool)] = &[
{rows(brackets, lambda r: f"(0x{r[0]:X}, 0x{r[1]:X}, {'true' if r[2] else 'false'})", 4)}
];
"""
open(out_path, "w", encoding="utf-8").write(text)
print(len(runs), "class runs,", len(brackets), "brackets, data version", data_version)
