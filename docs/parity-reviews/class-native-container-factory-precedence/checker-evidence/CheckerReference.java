// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
// Invoke with Java source-file mode and the pinned PlantUML JAR on the classpath.
// Pass scratch copies of inputs: each output uses the input's same-stem .svg.
import java.nio.file.Files;
import java.nio.file.Path;
import net.sourceforge.plantuml.SourceStringReader;
import net.sourceforge.plantuml.FileFormat;
import net.sourceforge.plantuml.FileFormatOption;

class CheckerReference {
    public static void main(String[] paths) throws Exception {
        for (String name : paths) {
            if (!name.endsWith(".puml")) throw new IllegalArgumentException(name);
            var reader = new SourceStringReader(Files.readString(Path.of(name)));
            if (reader.getBlocks().size() != 1) throw new IllegalArgumentException(name);
            var diagram = reader.getBlocks().get(0).getDiagram();
            System.out.println(Path.of(name).getFileName() + "\t" + diagram.getClass().getSimpleName());
            Path output = Path.of(name.substring(0, name.length() - 5) + ".svg");
            try (var stream = Files.newOutputStream(output)) {
                reader.outputImage(stream, new FileFormatOption(FileFormat.SVG));
            }
        }
    }
}
