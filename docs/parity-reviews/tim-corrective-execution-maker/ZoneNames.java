// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
// Derive locale data through JDK TimeZone.getDisplayName, not diagram output.
import java.util.Arrays;
import java.util.Locale;
import java.util.TimeZone;
public class ZoneNames {
    static String quote(String s) { return "\"" + s.replace("\\", "\\\\").replace("\"", "\\\"") + "\""; }
    public static void main(String[] args) {
        if (!Boolean.getBoolean("java.awt.headless")) throw new IllegalStateException("headless required");
        String[] ids = TimeZone.getAvailableIDs();
        Arrays.sort(ids);
        System.out.println("{");
        for (int i = 0; i < ids.length; i++) {
            TimeZone zone = TimeZone.getTimeZone(ids[i]);
            System.out.print(quote(ids[i]) + ":[");
            System.out.print(quote(zone.getDisplayName(false, TimeZone.SHORT, Locale.forLanguageTag("en-AU"))) + ",");
            System.out.print(quote(zone.getDisplayName(false, TimeZone.LONG, Locale.forLanguageTag("en-AU"))) + ",");
            System.out.print(quote(zone.getDisplayName(true, TimeZone.SHORT, Locale.forLanguageTag("en-AU"))) + ",");
            System.out.print(quote(zone.getDisplayName(true, TimeZone.LONG, Locale.forLanguageTag("en-AU"))) + "]");
            System.out.println(i + 1 == ids.length ? "" : ",");
        }
        System.out.println("}");
    }
}
