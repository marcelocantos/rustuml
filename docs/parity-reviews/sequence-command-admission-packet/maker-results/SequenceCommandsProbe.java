// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
import java.lang.reflect.*;
import java.util.*;
import net.sourceforge.plantuml.command.Command;
import net.sourceforge.plantuml.json.*;
import net.sourceforge.plantuml.regex.*;
import net.sourceforge.plantuml.sequencediagram.SequenceDiagramFactory;

class SequenceCommandsProbe {
    public static void main(String[] args) throws Exception {
        List<Command> commands = new ArrayList<>();
        Method init = SequenceDiagramFactory.class.getDeclaredMethod("initCommandsList", List.class);
        init.setAccessible(true);
        init.invoke(new SequenceDiagramFactory(), commands);
        JsonArray output = new JsonArray();
        for (int i = 0; i < commands.size(); i++) {
            Object command = commands.get(i);
            JsonObject row = new JsonObject().add("registration_index", i).add("class", command.getClass().getName());
            JsonArray fields = new JsonArray();
            for (Class<?> type = command.getClass(); type != null; type = type.getSuperclass()) {
                for (Field field : type.getDeclaredFields()) {
                    if (Modifier.isStatic(field.getModifiers())) continue;
                    field.setAccessible(true);
                    Object value = field.get(command);
                    if (value instanceof IRegex || value instanceof Pattern2 || field.getName().equals("doTrim")) {
                        JsonObject f = new JsonObject().add("owner", type.getName()).add("name", field.getName());
                        if (value instanceof IRegex) {
                            f.add("pattern", ((IRegex)value).getPatternAsString());
                            if (value instanceof RegexConcat) {
                                Method fox = RegexConcat.class.getDeclaredMethod("foxRegex");
                                fox.setAccessible(true);
                                f.add("fox_signature", ((Long)fox.invoke(value)).toString());
                            }
                        } else if (value instanceof Pattern2) {
                            f.add("pattern", ((Pattern2)value).pattern());
                        } else f.add("value", String.valueOf(value));
                        fields.add(f);
                    }
                }
            }
            try {
                Method end = command.getClass().getMethod("getEndPattern");
                end.setAccessible(true);
                Object pattern = end.invoke(command);
                if (pattern instanceof Pattern2) row.add("end_pattern", ((Pattern2)pattern).pattern());
            } catch (NoSuchMethodException absent) { }
            row.add("fields", fields);
            output.add(row);
        }
        System.out.println(new JsonObject().add("commands", output));
    }
}
