// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import net.sourceforge.plantuml.SourceStringReader;
import net.sourceforge.plantuml.code.TranscoderUtil;

public class ReaderProbe {
    public static void main(String[] args) throws Exception {
        String source = Files.readString(Path.of(args[1]), StandardCharsets.UTF_8);
        if (args[0].equals("decode")) {
            System.out.print(TranscoderUtil.getDefaultTranscoderProtected().decode(source));
        } else {
            var diagram = new SourceStringReader(source).getBlocks().get(0).getDiagram();
            System.out.print(diagram.getSource().getPlainString("\n"));
        }
    }
}
