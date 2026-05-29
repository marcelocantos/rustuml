// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
//
// Extract per-character widths and line metrics for Java AWT's "Monospaced"
// logical font, using the exact same Graphics2D configuration PlantUML uses
// in src/main/java/net/sourceforge/plantuml/FileFormat.java (lines 152-159).
//
// Output format mirrors crates/rustuml-render/src/plantuml_metrics.rs so the
// produced tables can be pasted into Rust constants verbatim.
//
// Usage:
//   javac tools/MonoMetricsExtract.java -d /tmp
//   java -cp /tmp MonoMetricsExtract
//
// Run this on the same machine and JVM that generated the golden SVGs.

import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.font.LineMetrics;
import java.awt.geom.Rectangle2D;
import java.awt.image.BufferedImage;

public class MonoMetricsExtract {

    static Graphics2D gg() {
        // Mirror FileFormat.gg in PlantUML's source:
        //   BufferedImage(100,100,TYPE_INT_RGB) with TEXT_ANTIALIASING=ON and
        //   FRACTIONALMETRICS=ON.
        BufferedImage img = new BufferedImage(100, 100, BufferedImage.TYPE_INT_RGB);
        Graphics2D g = img.createGraphics();
        g.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING,
                           RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
        g.setRenderingHint(RenderingHints.KEY_FRACTIONALMETRICS,
                           RenderingHints.VALUE_FRACTIONALMETRICS_ON);
        return g;
    }

    public static void main(String[] args) {
        Graphics2D g = gg();

        int[] sizes = {10, 11, 12, 13, 14};
        boolean[] bolds = {false, true};

        // Print per-size LineMetrics first so the Rust ascent/descent/height
        // tables can be filled in.
        for (int size : sizes) {
            for (boolean bold : bolds) {
                Font font = new Font("Monospaced", bold ? Font.BOLD : Font.PLAIN, size);
                LineMetrics lm = g.getFontMetrics(font).getLineMetrics("M", g);
                System.out.printf(
                    "// size=%d bold=%-5s ascent=%s descent=%s height=%s%n",
                    size, bold,
                    repr(lm.getAscent()),
                    repr(lm.getDescent()),
                    repr(lm.getHeight()));
            }
        }
        System.out.println();

        // Now per-character widths for printable ASCII (0x20..0x7E).
        for (int size : sizes) {
            for (boolean bold : bolds) {
                Font font = new Font("Monospaced", bold ? Font.BOLD : Font.PLAIN, size);
                FontMetrics fm = g.getFontMetrics(font);
                String name = bold
                    ? String.format("CHAR_WIDTHS_MONO_%d_BOLD", size)
                    : String.format("CHAR_WIDTHS_MONO_%d", size);
                System.out.printf("const %s: [f64; 95] = [%n", name);
                for (int c = 32; c <= 126; c++) {
                    String s = String.valueOf((char) c);
                    Rectangle2D r = fm.getStringBounds(s, g);
                    System.out.printf("    %s, // 0x%02X %s%n",
                        repr(r.getWidth()), c, displayChar(c));
                }
                System.out.println("];");
                System.out.println();
            }
        }

        // Sanity check: monospaced should give the same advance for any
        // printable char. Print the advance of a few characters so we can
        // confirm at a glance.
        System.out.println("// Sanity: per-size advance for 'M', 'i', 'W' at PLAIN");
        for (int size : sizes) {
            Font font = new Font("Monospaced", Font.PLAIN, size);
            FontMetrics fm = g.getFontMetrics(font);
            System.out.printf("//   size=%d  M=%s  i=%s  W=%s%n",
                size,
                repr(fm.getStringBounds("M", g).getWidth()),
                repr(fm.getStringBounds("i", g).getWidth()),
                repr(fm.getStringBounds("W", g).getWidth()));
        }
    }

    // Print a double in a form that round-trips exactly through Rust's f64
    // parser. Double.toString already uses the minimal-digit shortest-round-
    // trip representation per JLS, which Rust's f64::from_str parses back to
    // the same bits.
    static String repr(double d) {
        return Double.toString(d);
    }

    static String displayChar(int c) {
        if (c == '\\') return "'\\\\'";
        if (c == '\'') return "'\\''";
        return "'" + (char) c + "'";
    }
}
