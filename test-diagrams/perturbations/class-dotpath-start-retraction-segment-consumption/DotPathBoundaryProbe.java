import java.util.ArrayList;
import java.util.List;

import net.sourceforge.plantuml.klimt.geom.XCubicCurve2D;
import net.sourceforge.plantuml.klimt.shape.DotPath;

public final class DotPathBoundaryProbe {
    private static void probe(
            String name, List<XCubicCurve2D> curves, double dx, double dy) {
        List<XCubicCurve2D> copies = new ArrayList<>();
        for (XCubicCurve2D curve : curves) {
            copies.add(
                    new XCubicCurve2D(
                            curve.x1,
                            curve.y1,
                            curve.ctrlx1,
                            curve.ctrly1,
                            curve.ctrlx2,
                            curve.ctrly2,
                            curve.x2,
                            curve.y2));
        }
        DotPath path = DotPath.fromBeziers(copies);
        path.moveStartPoint(dx, dy);
        System.out.println(name + " count=" + path.getBeziers().size() + " path=" + path);
    }

    public static void main(String[] args) {
        XCubicCurve2D horizontal4 =
                new XCubicCurve2D(0, 0, 0, 0, 4, 0, 4, 0);
        XCubicCurve2D horizontalNext =
                new XCubicCurve2D(4, 0, 8, 0, 12, 0, 16, 0);
        XCubicCurve2D horizontalThird =
                new XCubicCurve2D(16, 0, 20, 0, 24, 0, 28, 0);

        probe("equal-two", List.of(horizontal4, horizontalNext), 4, 0);
        probe("below-two", List.of(horizontal4, horizontalNext), 3.999, 0);
        probe("single-large", List.of(horizontal4), 8, 0);
        probe(
                "three-large-consumes-one",
                List.of(horizontal4, horizontalNext, horizontalThird),
                20,
                0);

        XCubicCurve2D obliqueChord =
                new XCubicCurve2D(0, 0, 0, 3, 3, 4, 3, 4);
        XCubicCurve2D obliqueNext =
                new XCubicCurve2D(3, 4, 5, 7, 7, 9, 9, 12);
        probe("oblique-residual", List.of(obliqueChord, obliqueNext), 0, 8);

        XCubicCurve2D controlAtEndpoint =
                new XCubicCurve2D(0, 0, 4, 0, 4, 0, 4, 0);
        probe("control-at-endpoint", List.of(controlAtEndpoint, horizontalNext), 6, 0);

        XCubicCurve2D degenerate =
                new XCubicCurve2D(0, 0, 0, 0, 0, 0, 0, 0);
        XCubicCurve2D afterDegenerate =
                new XCubicCurve2D(0, 0, 3, 0, 6, 0, 9, 0);
        probe(
                "fully-degenerate-first",
                List.of(degenerate, afterDegenerate),
                6,
                0);
    }
}
