"""The Java macOS font-chain model over fontchain.swift output (shared by the validator and the Rust table generator).
predict(cp) -> 'missing' | 'ignorable' | 'mapped' for a scalar measured alone with Font.getStringBounds."""
import bisect
def load(path):
    hdr = {}; runs = {'S': [], 'C': []}
    for l in open(path):
        f = l.rstrip('\n').split('\t')
        if f[0].startswith('#'): hdr.setdefault(f[0], []).append(f[1:]); continue
        runs[f[0]].append((int(f[1], 16), int(f[2], 16)))
    for k in runs: runs[k].sort()
    return hdr, runs
def inruns(runs, cp):
    i = bisect.bisect_right(runs, (cp, 0x7FFFFFFF)) - 1
    return i >= 0 and runs[i][0] <= cp <= runs[i][1]
def is_complex_char_code(c):  # sun.font.FontUtilities.isComplexCharCode (JDK 21)
    if c < 0x0300 or c > 0x206F: return False
    if c <= 0x036f: return True
    if c < 0x0590: return False
    if c <= 0x06ff: return True
    if c < 0x0900: return False
    if c <= 0x0e7f: return True
    if c < 0x0f00: return False
    if c <= 0x0fff: return True
    if c < 0x10A0: return True
    if c < 0x1100: return False
    if c < 0x11ff: return True
    if c < 0x1780: return False
    if c <= 0x17ff: return True
    if c < 0x200c: return False
    if c <= 0x200d: return True
    if 0x202a <= c <= 0x202e: return True
    if 0x206a <= c <= 0x206f: return True
    return False
def hb_default_ignorable_supplementary(c):  # HarfBuzz _hb_unicode_is_default_ignorable, planes 1-16
    return 0x1D173 <= c <= 0x1D17A or 0xE0000 <= c <= 0xE0FFF
def predict(runs, cp):
    if cp >= 0x10000:
        if inruns(runs['C'], cp): return 'mapped'
        return 'ignorable' if hb_default_ignorable_supplementary(cp) else 'missing'
    if is_complex_char_code(cp):
        return 'mapped' if inruns(runs['C'], cp) else 'missing'
    return 'missing' if inruns(runs['S'], cp) else 'mapped'
