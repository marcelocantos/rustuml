// Menlo Bold Italic fallback and modelled-name probe. Read-only.
// JDK 21.0.5, macOS. Do not link PlantUML. Measure the way FaceProbe did
// (FontRenderContext with antialiasing and fractional metrics) and the way
// FontStack.getFont builds the face: Font.decode, deriveFont(PLAIN, size),
// then UFontFace TextAttribute WEIGHT and POSTURE.
// Usage: reference --probe MenloFallbackProbe.java
import java.awt.Font;
import java.awt.font.FontRenderContext;
import java.awt.font.TextAttribute;
import java.awt.geom.AffineTransform;
import java.util.HashMap;
import java.util.Map;

public class MenloFallbackProbe {
    static final FontRenderContext FRC = new FontRenderContext(new AffineTransform(), true, true);

    static Font plantumlFace(String name, boolean bold, boolean italic) {
        Font decoded = Font.decode(name).deriveFont(Font.PLAIN, 14f);
        Map<TextAttribute, Object> attrs = new HashMap<TextAttribute, Object>();
        attrs.put(TextAttribute.WEIGHT, bold ? TextAttribute.WEIGHT_BOLD : TextAttribute.WEIGHT_REGULAR);
        if (italic) {
            attrs.put(TextAttribute.POSTURE, TextAttribute.POSTURE_OBLIQUE);
        }
        return decoded.deriveFont(attrs);
    }

    static void name(String label, String fontName, boolean bold, boolean italic, String text) {
        Font font = plantumlFace(fontName, bold, italic);
        double width = font.getStringBounds(text, FRC).getWidth();
        System.out.println("NAME\t" + label + "\t" + font.getFontName() + "\t" + font.getPSName()
                + "\t" + font.getFamily() + "\t" + width);
    }

    public static void main(String[] args) {
        System.out.println("# " + System.getProperty("java.vm.name") + " " + System.getProperty("java.runtime.version"));
        name("menlo-wharf", "Menlo", false, false, "Wharf qr");
        name("MENLO-wharf", "MENLO", false, false, "Wharf qr");
        name("menlo-regular-wharf", "Menlo-Regular", false, false, "Wharf qr");
        name("menlo-bold-token-wharf", "Menlo-Bold", false, false, "Wharf qr");
        name("menlo-pier", "Menlo", false, false, "Pier x");
        name("menlo-bold-pier", "Menlo", true, false, "Pier x");
        name("menlo-italic-pier", "Menlo", false, true, "Pier x");
        name("menlo-bi-pier", "Menlo", true, true, "Pier x");
        name("arial-boldmt-bang", "Arial-BoldMT", false, false, "!");
        name("arial-bold-token-bang", "Arial-Bold", false, false, "!");
        name("helvetica-oblique-bang", "Helvetica-Oblique", false, false, "!");
        name("times-wharf", "Times", false, false, "Wharf qr");
        name("serif-wharf", "serif", false, false, "Wharf qr");
        name("courier-wharf", "courier", false, false, "Wharf qr");

        // U+2580 is in both own cmaps (control: one Menlo cell). The 267
        // scalars are Menlo-Bold own minus Menlo-BoldItalic own.
        int[] control = {0x2580};
        int[] fallback = {0x01F0, 0x0220, 0x2060, 0x2601, 0x2602, 0x2603, 0x2604, 0x2605, 0x2606, 0x2607, 0x2608, 0x2609, 0x260A, 0x260B, 0x260C, 0x260D, 0x260E, 0x260F, 0x2610, 0x2611, 0x2612, 0x2613, 0x2614, 0x2615, 0x2616, 0x2617, 0x2618, 0x2619, 0x261A, 0x261B, 0x261C, 0x261D, 0x261E, 0x261F, 0x2620, 0x2621, 0x2622, 0x2623, 0x2624, 0x2625, 0x2626, 0x2627, 0x2628, 0x2629, 0x262A, 0x262B, 0x262C, 0x262D, 0x262E, 0x262F, 0x263D, 0x263E, 0x2648, 0x2649, 0x264A, 0x264B, 0x264C, 0x264D, 0x264E, 0x264F, 0x2650, 0x2651, 0x2652, 0x2653, 0x2654, 0x2655, 0x2656, 0x2657, 0x2658, 0x2659, 0x265A, 0x265B, 0x265C, 0x265D, 0x265E, 0x265F, 0x2668, 0x2670, 0x2671, 0x2672, 0x2673, 0x2674, 0x2675, 0x2676, 0x2677, 0x2678, 0x2679, 0x267A, 0x267B, 0x267C, 0x267D, 0x267E, 0x267F, 0x2680, 0x2681, 0x2682, 0x2683, 0x2684, 0x2685, 0x2686, 0x2687, 0x2688, 0x2689, 0x268A, 0x268B, 0x2690, 0x2691, 0x2692, 0x2693, 0x2694, 0x2695, 0x2696, 0x2697, 0x2698, 0x2699, 0x269A, 0x269B, 0x269C, 0x26A0, 0x26A1, 0x26B0, 0x26B1, 0x2701, 0x2702, 0x2703, 0x2704, 0x2706, 0x2707, 0x2708, 0x2709, 0x270C, 0x270D, 0x270E, 0x270F, 0x2710, 0x2711, 0x2712, 0x2713, 0x2714, 0x2715, 0x2716, 0x2717, 0x2718, 0x2719, 0x271A, 0x271B, 0x271C, 0x271D, 0x271E, 0x271F, 0x2720, 0x2721, 0x2722, 0x2723, 0x2724, 0x2725, 0x2726, 0x2727, 0x2729, 0x272A, 0x272B, 0x272C, 0x272D, 0x272E, 0x272F, 0x2730, 0x2731, 0x2732, 0x2733, 0x2734, 0x2735, 0x2736, 0x2737, 0x2738, 0x2739, 0x273A, 0x273B, 0x273C, 0x273D, 0x273E, 0x273F, 0x2740, 0x2741, 0x2742, 0x2743, 0x2744, 0x2745, 0x2746, 0x2747, 0x2748, 0x2749, 0x274A, 0x274B, 0x274D, 0x274F, 0x2750, 0x2751, 0x2752, 0x2756, 0x2758, 0x2759, 0x275A, 0x275B, 0x275C, 0x275D, 0x275E, 0x2761, 0x2762, 0x2763, 0x2764, 0x2765, 0x2766, 0x2767, 0x2768, 0x2769, 0x276A, 0x276B, 0x276C, 0x276D, 0x276E, 0x276F, 0x2770, 0x2771, 0x2772, 0x2773, 0x2774, 0x2775, 0x2794, 0x2798, 0x2799, 0x279A, 0x279B, 0x279C, 0x279D, 0x279E, 0x279F, 0x27A0, 0x27A1, 0x27A2, 0x27A3, 0x27A4, 0x27A5, 0x27A6, 0x27A7, 0x27A8, 0x27A9, 0x27AA, 0x27AB, 0x27AC, 0x27AD, 0x27AE, 0x27AF, 0x27B1, 0x27B2, 0x27B3, 0x27B4, 0x27B5, 0x27B6, 0x27B7, 0x27B8, 0x27B9, 0x27BA, 0x27BB, 0x27BC, 0x27BD, 0x27BE, 0xFEFF};
        Font bold = plantumlFace("Menlo", true, false);
        Font boldItalic = plantumlFace("Menlo", true, true);
        for (int cp : control) {
            String s = new String(Character.toChars(cp));
            System.out.println("CONTROL\t" + Integer.toHexString(cp) + "\t"
                    + bold.getStringBounds(s, FRC).getWidth() + "\t"
                    + boldItalic.getStringBounds(s, FRC).getWidth());
        }
        for (int cp : fallback) {
            String s = new String(Character.toChars(cp));
            System.out.println("FALLBACK\t" + Integer.toHexString(cp) + "\t"
                    + bold.getStringBounds(s, FRC).getWidth() + "\t"
                    + boldItalic.getStringBounds(s, FRC).getWidth());
        }
    }
}
