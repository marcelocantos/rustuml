"""Independent check: java_bidi_data.rs BIDI_CLASSES/BRACKETS vs Unicode 15.0.0 DerivedBidiClass.txt (with @missing
defaults) and BidiBrackets.txt, for every code point. Also an independent decode of ubidi.icu (Trie2_16 per ICU
utrie2.h / UBiDiProps) written from the ICU format, not from the maker's code.
Usage: python3 -I check_ubidi_tables.py <java_bidi_data.rs> <DerivedBidiClass.txt> <BidiBrackets.txt> <ubidi.icu>"""
import re, sys, struct
rs, dbc, bb, icu = sys.argv[1:5]
NAMES = ["L","R","EN","ES","ET","AN","CS","B","S","WS","ON","LRE","LRO","AL","RLE","RLO","PDF","NSM","BN","FSI","LRI","RLI","PDI"]
src = open(rs).read()
cls_block = src[src.index("BIDI_CLASSES"):src.index("BRACKETS: &")]
runs = [(int(a,16), int(b,16), int(c)) for a,b,c in re.findall(r"\(0x([0-9A-F]+), 0x([0-9A-F]+), (\d+)\)", cls_block)]
rs_cls = [0]*0x110000
for a,b,c in runs:
    for x in range(a,b+1): rs_cls[x] = c
br_block = src[src.index("BRACKETS: &"):]
rs_br = [(int(a,16), int(b,16), t == "true") for a,b,t in re.findall(r"\(0x([0-9A-F]+), 0x([0-9A-F]+), (true|false)\)", br_block)]
# DerivedBidiClass with @missing defaults (later @missing lines override earlier within their range)
uc = [None]*0x110000
for line in open(dbc, encoding="utf-8"):
    m = re.match(r"#\s*@missing:\s*([0-9A-F]+)\.\.([0-9A-F]+);\s*(\S+)", line)
    if m:
        a,b,v = int(m.group(1),16), int(m.group(2),16), m.group(3)
        long = {"Left_To_Right":"L","Right_To_Left":"R","Arabic_Letter":"AL","European_Terminator":"ET","Boundary_Neutral":"BN","Arabic_Number":"AN","European_Number":"EN"}
        v = long.get(v, v)
        for x in range(a,b+1): uc[x] = NAMES.index(v)
        continue
    line = line.split("#")[0].strip()
    if not line: continue
    r, v = [s.strip() for s in line.split(";")]
    a, b = (r.split("..") + [r])[:2] if ".." in r else (r, r)
    for x in range(int(a,16), int(b,16)+1): uc[x] = NAMES.index(v)
mism = [x for x in range(0x110000) if uc[x] != rs_cls[x]]
print("classes: code points differing rs vs DerivedBidiClass-15.0:", len(mism), [hex(x) for x in mism[:10]])
ub = []
for line in open(bb, encoding="utf-8"):
    line = line.split("#")[0].strip()
    if not line: continue
    c, p, t = [s.strip() for s in line.split(";")]
    ub.append((int(c,16), int(p,16), t == "o"))
print("brackets: rs", len(rs_br), "ucd", len(ub), "equal" if rs_br == sorted(ub) else "DIFFER")
# independent ubidi.icu decode
raw = open(icu,"rb").read()
hdr = struct.unpack_from("<H", raw, 0)[0]
assert raw[8] == 0, "little-endian expected"  # isBigEndian byte in UDataInfo
fmt = raw[12:16]; dver = tuple(raw[20:24])
base = hdr
nidx = struct.unpack_from("<i", raw, base)[0]
idx = struct.unpack_from(f"<{nidx}i", raw, base)
trie_size = idx[2]  # IX_TRIE_SIZE
t = base + 4*nidx
sig, opts, ilen, dlen16, nullidx, nulldata, shifted_high = struct.unpack_from("<IHHHHHH", raw, t)
data_len = dlen16 << 2; high_start = shifted_high << 11
arr = struct.unpack_from(f"<{ilen+data_len}H", raw, t+16)
def get(c):
    if c < 0xD800 or 0xDC00 <= c <= 0xFFFF: i2 = arr[c >> 5]
    elif c <= 0xFFFF: i2 = arr[(0x10000 >> 5) + ((c - 0xD800) >> 5)]
    elif c < high_start: i1 = arr[(0x820 + 0x20 - (0x10000 >> 11)) + (c >> 11)]; i2 = arr[i1 + ((c >> 5) & 0x3F)]
    else: return arr[ilen + data_len - 4]
    return arr[(i2 << 2) + (c & 0x1F)]
mirrors_off = t + idx[3-1+0] if False else None
icu_cls = [(get(c) & 0x1F) for c in range(0x110000)]
icu_cls = [x if x < 23 else 10 for x in icu_cls]
m2 = [x for x in range(0x110000) if icu_cls[x] != rs_cls[x]]
print("ubidi.icu format", fmt, "data version", dver, "trie sig", hex(sig))
print("classes: code points differing rs vs independent ubidi.icu decode:", len(m2), [hex(x) for x in m2[:10]])
bpt = [(c, (get(c) >> 8) & 3) for c in range(0x110000) if (get(c) >> 8) & 3]
print("bracket-type code points in ubidi.icu:", len(bpt), "ucd", len(ub), "same set" if [c for c,_ in bpt] == [c for c,_,_ in sorted(ub)] and all((t==1)==o for (c,t),(_,_,o) in zip(bpt, sorted(ub))) else "DIFFER")
