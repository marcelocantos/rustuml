// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
//
// Extract per-character widths for Java AWT's "SansSerif" BOLD logical font
// at sizes 10/11/12/13. The plantuml_metrics module already has size 14 bold
// (CHAR_WIDTHS_14_BOLD); without the smaller sizes, every bold sequence
// label, autonumber, divider, and group kind falls back to plain widths,
// producing wrong textLength values across the sequence diagram corpus.
//
// Mirrors PlantUML's Graphics2D configuration (FileFormat.java:152-159).
//
// Usage:
//   javac tools/SansBoldMetricsExtract.java -d /tmp
//   java -cp /tmp SansBoldMetricsExtract

import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.geom.Rectangle2D;
import java.awt.image.BufferedImage;

public class SansBoldMetricsExtract {

    static Graphics2D gg() {
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

        int[] sizes = {10, 11, 12, 13};
        for (int size : sizes) {
            Font font = new Font("SansSerif", Font.BOLD, size);
            FontMetrics fm = g.getFontMetrics(font);
            System.out.printf("const CHAR_WIDTHS_%d_BOLD: [f64; 95] = [%n", size);
            for (int c = 32; c <= 126; c++) {
                Rectangle2D r = fm.getStringBounds(String.valueOf((char) c), g);
                System.out.printf("    %s,%n", Double.toString(r.getWidth()));
            }
            System.out.println("];");
            System.out.println();
        }
    }
}
