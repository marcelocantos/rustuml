// Menlo Bold Italic JRS fallback face probe. Read-only of fonts.
// JDK 21.0.5 macOS. Do not link PlantUML.
//
// The previous probe measured getStringBounds and did not name the face.
// JDK 21.0.5 CoreTextSupport.m CTS_GetGlyphsAsIntsForCharacters (99-135)
// stores a negative Unicode scalar when JRSFontCreateFallbackFontForCharacters
// returns a font whose glyph is non-zero. CTS_CopyCTFallbackFontAndGlyphForJavaGlyphCode
// then measures that CTFont. Pure Java only sees the negative scalar, so this
// probe compiles a small C helper in java.io.tmpdir, calls the same JRS entry
// FontChainExtract.swift calls, and prints the fallback PostScript name and
// its advance. It deletes the temporary source and binary before exit.
//
// Usage:
//   reference --probe WORKER_PATH/MenloJrsFaceProbe.java
import java.awt.Font;
import java.awt.font.FontRenderContext;
import java.awt.font.TextAttribute;
import java.awt.geom.AffineTransform;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.Map;

public class MenloJrsFaceProbe {
    static final FontRenderContext FRC = new FontRenderContext(new AffineTransform(), true, true);
    static final String C_SOURCE = """
#include <CoreFoundation/CoreFoundation.h>
#include <CoreText/CoreText.h>
#include <dlfcn.h>
#include <stdio.h>
#include <stdint.h>

typedef CTFontRef (*JrsFn)(CTFontRef, const UniChar *, CFIndex);

static const uint32_t kControl[] = {0x2580};
static const uint32_t kFallback[] = {0x01F0, 0x0220, 0x2060, 0x2601, 0x2602, 0x2603, 0x2604, 0x2605, 0x2606, 0x2607, 0x2608, 0x2609, 0x260A, 0x260B, 0x260C, 0x260D, 0x260E, 0x260F, 0x2610, 0x2611, 0x2612, 0x2613, 0x2614, 0x2615, 0x2616, 0x2617, 0x2618, 0x2619, 0x261A, 0x261B, 0x261C, 0x261D, 0x261E, 0x261F, 0x2620, 0x2621, 0x2622, 0x2623, 0x2624, 0x2625, 0x2626, 0x2627, 0x2628, 0x2629, 0x262A, 0x262B, 0x262C, 0x262D, 0x262E, 0x262F, 0x263D, 0x263E, 0x2648, 0x2649, 0x264A, 0x264B, 0x264C, 0x264D, 0x264E, 0x264F, 0x2650, 0x2651, 0x2652, 0x2653, 0x2654, 0x2655, 0x2656, 0x2657, 0x2658, 0x2659, 0x265A, 0x265B, 0x265C, 0x265D, 0x265E, 0x265F, 0x2668, 0x2670, 0x2671, 0x2672, 0x2673, 0x2674, 0x2675, 0x2676, 0x2677, 0x2678, 0x2679, 0x267A, 0x267B, 0x267C, 0x267D, 0x267E, 0x267F, 0x2680, 0x2681, 0x2682, 0x2683, 0x2684, 0x2685, 0x2686, 0x2687, 0x2688, 0x2689, 0x268A, 0x268B, 0x2690, 0x2691, 0x2692, 0x2693, 0x2694, 0x2695, 0x2696, 0x2697, 0x2698, 0x2699, 0x269A, 0x269B, 0x269C, 0x26A0, 0x26A1, 0x26B0, 0x26B1, 0x2701, 0x2702, 0x2703, 0x2704, 0x2706, 0x2707, 0x2708, 0x2709, 0x270C, 0x270D, 0x270E, 0x270F, 0x2710, 0x2711, 0x2712, 0x2713, 0x2714, 0x2715, 0x2716, 0x2717, 0x2718, 0x2719, 0x271A, 0x271B, 0x271C, 0x271D, 0x271E, 0x271F, 0x2720, 0x2721, 0x2722, 0x2723, 0x2724, 0x2725, 0x2726, 0x2727, 0x2729, 0x272A, 0x272B, 0x272C, 0x272D, 0x272E, 0x272F, 0x2730, 0x2731, 0x2732, 0x2733, 0x2734, 0x2735, 0x2736, 0x2737, 0x2738, 0x2739, 0x273A, 0x273B, 0x273C, 0x273D, 0x273E, 0x273F, 0x2740, 0x2741, 0x2742, 0x2743, 0x2744, 0x2745, 0x2746, 0x2747, 0x2748, 0x2749, 0x274A, 0x274B, 0x274D, 0x274F, 0x2750, 0x2751, 0x2752, 0x2756, 0x2758, 0x2759, 0x275A, 0x275B, 0x275C, 0x275D, 0x275E, 0x2761, 0x2762, 0x2763, 0x2764, 0x2765, 0x2766, 0x2767, 0x2768, 0x2769, 0x276A, 0x276B, 0x276C, 0x276D, 0x276E, 0x276F, 0x2770, 0x2771, 0x2772, 0x2773, 0x2774, 0x2775, 0x2794, 0x2798, 0x2799, 0x279A, 0x279B, 0x279C, 0x279D, 0x279E, 0x279F, 0x27A0, 0x27A1, 0x27A2, 0x27A3, 0x27A4, 0x27A5, 0x27A6, 0x27A7, 0x27A8, 0x27A9, 0x27AA, 0x27AB, 0x27AC, 0x27AD, 0x27AE, 0x27AF, 0x27B1, 0x27B2, 0x27B3, 0x27B4, 0x27B5, 0x27B6, 0x27B7, 0x27B8, 0x27B9, 0x27BA, 0x27BB, 0x27BC, 0x27BD, 0x27BE, 0xFEFF};

static void cstr(CFStringRef s, char *buf, size_t n) {
    if (s == NULL || !CFStringGetCString(s, buf, n, kCFStringEncodingUTF8)) {
        if (n > 0) buf[0] = 0;
    }
}

static void emit(const char *role, const char *kind, uint32_t cp, const char *disposition,
                 CTFontRef font, CGGlyph glyph, double advance) {
    char ps[512], family[512], path[1024];
    CFStringRef psr = CTFontCopyPostScriptName(font);
    CFStringRef fam = CTFontCopyFamilyName(font);
    cstr(psr, ps, sizeof ps);
    cstr(fam, family, sizeof family);
    if (psr) CFRelease(psr);
    if (fam) CFRelease(fam);
    CFURLRef url = CTFontCopyAttribute(font, kCTFontURLAttribute);
    path[0] = 0;
    if (url) {
        CFStringRef p = CFURLCopyFileSystemPath(url, kCFURLPOSIXPathStyle);
        cstr(p, path, sizeof path);
        if (p) CFRelease(p);
        CFRelease(url);
    }
    unsigned upem = CTFontGetUnitsPerEm(font);
    double units = advance / 14.0 * (double)upem;
    printf("JRS\t%s\t%s\t%04X\t%s\t%s\t%s\t%u\t%.17g\t%u\t%.17g\t%s\n",
           role, kind, cp, disposition, ps, family, (unsigned)glyph, advance, upem, units, path);
}

static void one(JrsFn jrs, const char *role, CTFontRef font, const char *kind, uint32_t cp) {
    UniChar u = (UniChar)cp;
    CGGlyph g = 0;
    Boolean ok = CTFontGetGlyphsForCharacters(font, &u, &g, 1);
    if (ok && g != 0) {
        CGSize adv = {0, 0};
        CTFontGetAdvancesForGlyphs(font, kCTFontOrientationHorizontal, &g, &adv, 1);
        emit(role, kind, cp, "OWN", font, g, adv.width);
        return;
    }
    CTFontRef fb = jrs(font, &u, 1);
    if (fb == NULL) {
        emit(role, kind, cp, "MISSING", font, 0, 0);
        return;
    }
    CGGlyph fg = 0;
    Boolean fok = CTFontGetGlyphsForCharacters(fb, &u, &fg, 1);
    if (!fok || fg == 0) {
        emit(role, kind, cp, "MISSING", fb, 0, 0);
        CFRelease(fb);
        return;
    }
    CGSize adv = {0, 0};
    CTFontGetAdvancesForGlyphs(fb, kCTFontOrientationHorizontal, &fg, &adv, 1);
    emit(role, kind, cp, "FALLBACK", fb, fg, adv.width);
    CFRelease(fb);
}

static void face(JrsFn jrs, const char *role, CFStringRef name) {
    CTFontRef font = CTFontCreateWithName(name, 14.0, NULL);
    char ps[512];
    CFStringRef psr = CTFontCopyPostScriptName(font);
    cstr(psr, ps, sizeof ps);
    if (psr) CFRelease(psr);
    printf("# primary\t%s\t%s\n", role, ps);
    for (size_t i = 0; i < sizeof kControl / sizeof kControl[0]; i++) {
        one(jrs, role, font, "CONTROL", kControl[i]);
    }
    for (size_t i = 0; i < sizeof kFallback / sizeof kFallback[0]; i++) {
        one(jrs, role, font, "FALLBACK", kFallback[i]);
    }
    CFRelease(font);
}

int main(void) {
    void *h = dlopen("/System/Library/Frameworks/JavaRuntimeSupport.framework/JavaRuntimeSupport", RTLD_NOW);
    if (h == NULL) {
        const char *err = dlerror();
        printf("JRS_LOAD_FAIL\t%s\n", err != NULL ? err : "dlopen failed");
        return 2;
    }
    JrsFn jrs = (JrsFn)dlsym(h, "JRSFontCreateFallbackFontForCharacters");
    if (jrs == NULL) {
        const char *err = dlerror();
        printf("JRS_SYMBOL_FAIL\t%s\n", err != NULL ? err : "dlsym failed");
        return 3;
    }
    face(jrs, "BOLD", CFSTR("Menlo-Bold"));
    face(jrs, "BOLDITALIC", CFSTR("Menlo-BoldItalic"));
    return 0;
}
""";

    static Font plantumlFace(String name, boolean bold, boolean italic) {
        Font decoded = Font.decode(name).deriveFont(Font.PLAIN, 14f);
        Map<TextAttribute, Object> attrs = new HashMap<TextAttribute, Object>();
        attrs.put(TextAttribute.WEIGHT, bold ? TextAttribute.WEIGHT_BOLD : TextAttribute.WEIGHT_REGULAR);
        if (italic) {
            attrs.put(TextAttribute.POSTURE, TextAttribute.POSTURE_OBLIQUE);
        }
        return decoded.deriveFont(attrs);
    }

    static final int[] CONTROL = {0x2580};
    static final int[] FALLBACK = {0x01F0, 0x0220, 0x2060, 0x2601, 0x2602, 0x2603, 0x2604, 0x2605, 0x2606, 0x2607, 0x2608, 0x2609, 0x260A, 0x260B, 0x260C, 0x260D, 0x260E, 0x260F, 0x2610, 0x2611, 0x2612, 0x2613, 0x2614, 0x2615, 0x2616, 0x2617, 0x2618, 0x2619, 0x261A, 0x261B, 0x261C, 0x261D, 0x261E, 0x261F, 0x2620, 0x2621, 0x2622, 0x2623, 0x2624, 0x2625, 0x2626, 0x2627, 0x2628, 0x2629, 0x262A, 0x262B, 0x262C, 0x262D, 0x262E, 0x262F, 0x263D, 0x263E, 0x2648, 0x2649, 0x264A, 0x264B, 0x264C, 0x264D, 0x264E, 0x264F, 0x2650, 0x2651, 0x2652, 0x2653, 0x2654, 0x2655, 0x2656, 0x2657, 0x2658, 0x2659, 0x265A, 0x265B, 0x265C, 0x265D, 0x265E, 0x265F, 0x2668, 0x2670, 0x2671, 0x2672, 0x2673, 0x2674, 0x2675, 0x2676, 0x2677, 0x2678, 0x2679, 0x267A, 0x267B, 0x267C, 0x267D, 0x267E, 0x267F, 0x2680, 0x2681, 0x2682, 0x2683, 0x2684, 0x2685, 0x2686, 0x2687, 0x2688, 0x2689, 0x268A, 0x268B, 0x2690, 0x2691, 0x2692, 0x2693, 0x2694, 0x2695, 0x2696, 0x2697, 0x2698, 0x2699, 0x269A, 0x269B, 0x269C, 0x26A0, 0x26A1, 0x26B0, 0x26B1, 0x2701, 0x2702, 0x2703, 0x2704, 0x2706, 0x2707, 0x2708, 0x2709, 0x270C, 0x270D, 0x270E, 0x270F, 0x2710, 0x2711, 0x2712, 0x2713, 0x2714, 0x2715, 0x2716, 0x2717, 0x2718, 0x2719, 0x271A, 0x271B, 0x271C, 0x271D, 0x271E, 0x271F, 0x2720, 0x2721, 0x2722, 0x2723, 0x2724, 0x2725, 0x2726, 0x2727, 0x2729, 0x272A, 0x272B, 0x272C, 0x272D, 0x272E, 0x272F, 0x2730, 0x2731, 0x2732, 0x2733, 0x2734, 0x2735, 0x2736, 0x2737, 0x2738, 0x2739, 0x273A, 0x273B, 0x273C, 0x273D, 0x273E, 0x273F, 0x2740, 0x2741, 0x2742, 0x2743, 0x2744, 0x2745, 0x2746, 0x2747, 0x2748, 0x2749, 0x274A, 0x274B, 0x274D, 0x274F, 0x2750, 0x2751, 0x2752, 0x2756, 0x2758, 0x2759, 0x275A, 0x275B, 0x275C, 0x275D, 0x275E, 0x2761, 0x2762, 0x2763, 0x2764, 0x2765, 0x2766, 0x2767, 0x2768, 0x2769, 0x276A, 0x276B, 0x276C, 0x276D, 0x276E, 0x276F, 0x2770, 0x2771, 0x2772, 0x2773, 0x2774, 0x2775, 0x2794, 0x2798, 0x2799, 0x279A, 0x279B, 0x279C, 0x279D, 0x279E, 0x279F, 0x27A0, 0x27A1, 0x27A2, 0x27A3, 0x27A4, 0x27A5, 0x27A6, 0x27A7, 0x27A8, 0x27A9, 0x27AA, 0x27AB, 0x27AC, 0x27AD, 0x27AE, 0x27AF, 0x27B1, 0x27B2, 0x27B3, 0x27B4, 0x27B5, 0x27B6, 0x27B7, 0x27B8, 0x27B9, 0x27BA, 0x27BB, 0x27BC, 0x27BD, 0x27BE, 0xFEFF};

    static void widths(String role, Font font) {
        for (int cp : CONTROL) {
            String s = new String(Character.toChars(cp));
            System.out.println("WIDTH\t" + role + "\tCONTROL\t" + Integer.toHexString(cp)
                    + "\t" + font.getStringBounds(s, FRC).getWidth()
                    + "\t" + font.getPSName() + "\t" + font.getFamily());
        }
        for (int cp : FALLBACK) {
            String s = new String(Character.toChars(cp));
            System.out.println("WIDTH\t" + role + "\tFALLBACK\t" + Integer.toHexString(cp)
                    + "\t" + font.getStringBounds(s, FRC).getWidth()
                    + "\t" + font.getPSName() + "\t" + font.getFamily());
        }
    }

    static int run(Path bin) throws Exception {
        Process p = new ProcessBuilder(bin.toString()).redirectErrorStream(true).start();
        byte[] out = p.getInputStream().readAllBytes();
        int code = p.waitFor();
        System.out.write(out);
        System.out.flush();
        return code;
    }

    public static void main(String[] args) throws Exception {
        System.out.println("# " + System.getProperty("java.vm.name") + " " + System.getProperty("java.runtime.version"));
        System.out.println("# os " + System.getProperty("os.name") + " " + System.getProperty("os.version") + " " + System.getProperty("os.arch"));
        widths("BOLD", plantumlFace("Menlo", true, false));
        widths("BOLDITALIC", plantumlFace("Menlo", true, true));

        Path dir = Files.createTempDirectory("menlo-jrs-face");
        Path src = dir.resolve("menlo_jrs_face.c");
        Path bin = dir.resolve("menlo_jrs_face");
        Files.writeString(src, C_SOURCE);
        String cc = findCc();
        if (cc == null) {
            System.out.println("JRS_COMPILE_FAIL\tno cc, clang, or xcrun clang");
            delete(dir);
            return;
        }
        Process c = new ProcessBuilder(cc, "-x", "c", "-O2", "-framework", "CoreText", "-framework", "CoreFoundation", "-o", bin.toString(), src.toString())
                .redirectErrorStream(true).start();
        byte[] log = c.getInputStream().readAllBytes();
        int ccCode = c.waitFor();
        if (ccCode != 0) {
            System.out.println("JRS_COMPILE_FAIL\t" + ccCode);
            System.out.write(log);
            System.out.println();
            delete(dir);
            return;
        }
        int nativeCode = run(bin);
        System.out.println("JRS_EXIT\t" + nativeCode);
        delete(dir);
    }

    static String findCc() throws Exception {
        for (String cand : new String[] {"cc", "clang"}) {
            Process p = new ProcessBuilder("sh", "-c", "command -v " + cand).redirectErrorStream(true).start();
            String path = new String(p.getInputStream().readAllBytes(), StandardCharsets.UTF_8).trim();
            if (p.waitFor() == 0 && !path.isEmpty()) {
                return path;
            }
        }
        Process x = new ProcessBuilder("xcrun", "--find", "clang").redirectErrorStream(true).start();
        String path = new String(x.getInputStream().readAllBytes(), StandardCharsets.UTF_8).trim();
        if (x.waitFor() == 0 && !path.isEmpty()) {
            return path;
        }
        return null;
    }

    static void delete(Path dir) {
        try {
            Files.walk(dir).sorted(java.util.Comparator.reverseOrder()).forEach(p -> {
                try { Files.deleteIfExists(p); } catch (Exception ignored) {}
            });
        } catch (Exception ignored) {}
    }
}
