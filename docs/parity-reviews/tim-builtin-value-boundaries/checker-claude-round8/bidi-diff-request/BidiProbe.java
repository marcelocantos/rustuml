// Checker probe (round-8, claude): java.text.Bidi on the reference JVM, read-only.
// Usage: java BidiProbe.java <in.txt> <out.txt>
// Input lines: <dir>;<hex UTF-16 units>, dir 0 = DIRECTION_LEFT_TO_RIGHT, 1 = DIRECTION_RIGHT_TO_LEFT,
// D = DIRECTION_DEFAULT_LEFT_TO_RIGHT (TextLine.fastCreateTextLine's flag).
// Output line per input: requiresBidi;isLeftToRight;baseLevel;getLevelAt(i)...;run levels (BidiUtils.getLevels)
// The first output line is a header with the runtime identity.
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.text.Bidi;

public class BidiProbe {
    public static void main(String[] args) throws Exception {
        try (BufferedReader in = Files.newBufferedReader(Path.of(args[0]), StandardCharsets.US_ASCII);
             PrintWriter out = new PrintWriter(new BufferedWriter(new OutputStreamWriter(
                     new FileOutputStream(args[1]), StandardCharsets.US_ASCII)))) {
            out.println("# " + System.getProperty("java.vm.name") + " " + System.getProperty("java.runtime.version")
                    + " " + System.getProperty("java.home"));
            String line;
            while ((line = in.readLine()) != null) {
                int semi = line.indexOf(';');
                String dir = line.substring(0, semi);
                String[] hex = line.substring(semi + 1).trim().split(" ");
                char[] chars = new char[hex.length];
                for (int i = 0; i < hex.length; i++) chars[i] = (char) Integer.parseInt(hex[i], 16);
                int flag = dir.equals("0") ? Bidi.DIRECTION_LEFT_TO_RIGHT
                        : dir.equals("1") ? Bidi.DIRECTION_RIGHT_TO_LEFT : Bidi.DIRECTION_DEFAULT_LEFT_TO_RIGHT;
                StringBuilder sb = new StringBuilder();
                try {
                    boolean rb = Bidi.requiresBidi(chars, 0, chars.length);
                    Bidi b = new Bidi(chars, 0, null, 0, chars.length, flag);
                    sb.append(rb ? 1 : 0).append(';').append(b.isLeftToRight() ? 1 : 0).append(';')
                            .append(b.getBaseLevel()).append(';');
                    for (int i = 0; i < chars.length; i++) {
                        if (i > 0) sb.append(' ');
                        sb.append(b.getLevelAt(i));
                    }
                    sb.append(';');
                    int pos = 0;
                    for (int k = 0; k < b.getRunCount(); k++) {
                        int limit = b.getRunLimit(k);
                        int level = b.getRunLevel(k);
                        while (pos < limit) {
                            if (pos > 0) sb.append(' ');
                            sb.append(level);
                            pos++;
                        }
                    }
                } catch (Throwable t) {
                    sb.setLength(0);
                    sb.append("EXC ").append(t.getClass().getName()).append(' ')
                            .append(String.valueOf(t.getMessage()).replace('\n', ' '));
                }
                out.println(sb);
            }
        }
    }
}
