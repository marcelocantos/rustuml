// Round-9 maker (claude) probe, read-only: which physical face JDK 21 resolves for each modelled family and
// style (CFontManager.loadFonts / registerItalicDerived, Font2D.setStyle, FontFamily.getFont), and its
// advance for a test string (an unmapped mark included), measured as PlantUML measures it.
// Usage: java FaceProbe.java  (no arguments; stdout only)
import java.awt.Font;
import java.awt.font.FontRenderContext;
import java.awt.geom.AffineTransform;

public class FaceProbe {
    public static void main(String[] args) {
        System.out.println("# " + System.getProperty("java.vm.name") + " " + System.getProperty("java.runtime.version"));
        String[] families = {"SansSerif", "Lucida Grande", "Arial", "Helvetica", "Verdana", "Monospaced", "Menlo", "Courier New"};
        String[] styles = {"plain", "bold", "italic", "bolditalic"};
        FontRenderContext frc = new FontRenderContext(new AffineTransform(), true, true);
        String probe = "Pier x𝅧";
        for (String family : families) {
            for (int style = 0; style < 4; style++) {
                Font font = new Font(family, style, 14);
                double withMark = font.getStringBounds(probe, frc).getWidth();
                double without = font.getStringBounds("Pier x", frc).getWidth();
                System.out.println(family + "\t" + styles[style] + "\t" + font.getFontName() + "\t" + font.getPSName()
                        + "\t" + font.getFamily() + "\t" + withMark + "\t" + without);
            }
        }
    }
}
