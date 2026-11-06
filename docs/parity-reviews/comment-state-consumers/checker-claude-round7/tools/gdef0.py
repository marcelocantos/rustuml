"""GDEF class of glyph 0, GPOS/GSUB script tags and kern table summary of a TrueType file (read-only)."""
import struct, sys, hashlib
def tables(b):
    n = struct.unpack(">H", b[4:6])[0]; t = {}
    for i in range(n):
        tag, _, off, ln = struct.unpack(">4sIII", b[12 + 16 * i: 28 + 16 * i]); t[tag.decode()] = (off, ln)
    return t
def classdef(b, o, g):
    fmt = struct.unpack(">H", b[o:o + 2])[0]
    if fmt == 1:
        start, cnt = struct.unpack(">HH", b[o + 2:o + 6])
        return struct.unpack(">H", b[o + 6 + 2 * (g - start): o + 8 + 2 * (g - start)])[0] if start <= g < start + cnt else 0
    cnt = struct.unpack(">H", b[o + 2:o + 4])[0]
    for i in range(cnt):
        s, e, c = struct.unpack(">HHH", b[o + 4 + 6 * i: o + 10 + 6 * i])
        if s <= g <= e: return c
    return 0
def scripts(b, t, tag):
    if tag not in t: return None
    o = t[tag][0]; sl = o + struct.unpack(">H", b[o + 4:o + 6])[0]; n = struct.unpack(">H", b[sl:sl + 2])[0]
    return [b[sl + 2 + 6 * i: sl + 6 + 6 * i].decode() for i in range(n)]
for path in sys.argv[1:]:
    b = open(path, "rb").read(); t = tables(b)
    g = t.get("GDEF"); cls = None; glyphs = []
    if g:
        o = g[0]; cd = struct.unpack(">H", b[o + 4:o + 6])[0]
        if cd:
            cls = classdef(b, o + cd, 0)
            glyphs = [(k, classdef(b, o + cd, k)) for k in range(0, 4)]
    print(hashlib.sha256(b).hexdigest()[:8], path.split("/")[-1], "GDEF" in t, "glyph0 class", cls, glyphs, "GSUB", scripts(b, t, "GSUB"), "GPOS", scripts(b, t, "GPOS"), "kern" in t, "morx" in t)
