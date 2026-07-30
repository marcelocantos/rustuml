// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
//
// Extract Java AWT line metrics and printable-ASCII advances for a physical
// font using PlantUML's Graphics2D rendering hints.
//
// Usage:
//   javac tools/PhysicalFontMetricsExtract.java -d /tmp
//   java -Djava.awt.headless=true -cp /tmp PhysicalFontMetricsExtract "Courier New"

import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.font.LineMetrics;
import java.awt.geom.Rectangle2D;
import java.awt.image.BufferedImage;

public class PhysicalFontMetricsExtract {

    static Graphics2D graphics() {
        BufferedImage image = new BufferedImage(100, 100, BufferedImage.TYPE_INT_RGB);
        Graphics2D graphics = image.createGraphics();
        graphics.setRenderingHint(
            RenderingHints.KEY_TEXT_ANTIALIASING,
            RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
        graphics.setRenderingHint(
            RenderingHints.KEY_FRACTIONALMETRICS,
            RenderingHints.VALUE_FRACTIONALMETRICS_ON);
        return graphics;
    }

    public static void main(String[] args) {
        if (args.length != 1) {
            throw new IllegalArgumentException("expected one physical font family");
        }
        Graphics2D graphics = graphics();
        int[] sizes = {10, 11, 12, 13, 14, 16, 21};
        int[] styles = {Font.PLAIN, Font.BOLD, Font.ITALIC, Font.BOLD | Font.ITALIC};

        for (int size : sizes) {
            for (int style : styles) {
                Font font = new Font(args[0], style, size);
                FontMetrics metrics = graphics.getFontMetrics(font);
                LineMetrics line = metrics.getLineMetrics("M", graphics);
                Rectangle2D advance = metrics.getStringBounds("M", graphics);
                System.out.printf(
                    "requested=%s resolved=%s size=%d style=%d advance=%s ascent=%s descent=%s height=%s%n",
                    args[0],
                    font.getFamily(),
                    size,
                    style,
                    repr(advance.getWidth()),
                    repr(line.getAscent()),
                    repr(line.getDescent()),
                    repr(line.getHeight()));
            }
        }

        int unitsPerEm = 2048;
        for (int style : styles) {
            Font font = new Font(args[0], style, unitsPerEm);
            FontMetrics metrics = graphics.getFontMetrics(font);
            StringBuilder units = new StringBuilder();
            for (char c = 32; c <= 126; c++) {
                if (units.length() > 0) {
                    units.append(',');
                }
                units.append(Math.round(metrics.getStringBounds(
                    Character.toString(c), graphics).getWidth()));
            }
            LineMetrics line = metrics.getLineMetrics("M", graphics);
            System.out.printf(
                "table requested=%s resolved=%s style=%d unitsPerEm=%d ascent=%d descent=%d height=%d%n%s%n",
                args[0],
                font.getFamily(),
                style,
                unitsPerEm,
                Math.round(line.getAscent()),
                Math.round(line.getDescent()),
                Math.round(line.getHeight()),
                units);
        }
        graphics.dispose();
    }

    static String repr(double value) {
        return Double.toString(value);
    }
}
