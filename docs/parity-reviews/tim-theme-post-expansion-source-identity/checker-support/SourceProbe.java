// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import net.sourceforge.plantuml.SourceStringReader;

public class SourceProbe {
    public static void main(String[] args) throws Exception {
        String source = Files.readString(Path.of(args[0]), StandardCharsets.UTF_8);
        SourceStringReader reader = new SourceStringReader(source);
        System.out.print(reader.getBlocks().get(0).getDiagram().getSource().getPlainString("\n"));
    }
}
