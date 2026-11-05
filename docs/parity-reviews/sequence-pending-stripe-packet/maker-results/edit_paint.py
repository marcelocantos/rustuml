from pathlib import Path
p=Path('crates/rustuml-parser/src/diagram/sequence.rs');s=p.read_text().replace('Result<Vec<Option<usize>>, (u32, String)>','Result<Vec<Option<usize>>, (usize, String)>');p.write_text(s)
p=Path('crates/rustuml-render/src/sequence.rs');s=p.read_text().replace('''    // Pending `create X` ids awaiting their first message — that message reserves
    // extra horizontal space for the inline head box centered on X's lifeline.
''','''    // Creation spacing uses the same attached raw events as head painting.
''')
# Queue call transports a component origin, not a guessed first baseline.
old='''            let text_y = pill_top
                + 5.0
                + ascent_with_family(
                    svg.participant_font_size as f64,
                    &svg.participant_font_family,
                );''';assert old in s;s=s.replace(old,'''            let text_top = pill_top + 5.0;''')
a=s.index('''            svg.queue_shape(''');b=s.index('''        ParticipantKind::Participant => {''',a);s=s[:a]+s[a:b].replace('                text_y,','                text_top,')+s[b:]
a=s.index('    fn queue_shape(');b=s.index('    /// Write one cut segment',a);q=s[a:b].replace('        text_y: f64,','        text_top: f64,')
q=q.replace('''        // Text. With a stereotype, draw it on the `text_y` line and push the
        // name down one line; both are centred in the pill's text region
        // [box_x + pad, box_x + box_w - pad].''','''        // USymbolQueue#asSmall translates mergeTB(stereotype,label) by its
        // margin. Each child contributes its own first-stripe ascent.
        let text_y = text_top + stereo_shift + label.lines[0].2.ascent;''')
old='''            ParticipantTextBlock::new(
                &st_display,
                self.participant_font_size as f64,
                &self.participant_font_family,
                false,
            )
            .draw('''
new='''            let stereotype = ParticipantTextBlock::new(
                &st_display,
                self.participant_font_size as f64,
                &self.participant_font_family,
                false,
            );
            stereotype.draw(''';assert old in q;q=q.replace(old,new).replace('''                    y: text_y,''','''                    y: text_top + stereotype.lines[0].2.ascent,''').replace('''self.participant_text(name_x, text_y + stereo_shift, text_content, text_len);''','''self.participant_text(name_x, text_y, text_content, text_len);''');s=s[:a]+q+s[b:]
# Advance stripe origins, then choose the current stripe baseline.
a=s.index('    fn emit_message_label(');b=s.index('    /// Write an actor stick figure',a);q=s[a:b];old='''        let mut y = text_y;
        for line in text_content.split("\\\\n") {''';new='''        let mut y = text_y;
        let mut previous_ascent = None;
        for line in text_content.split("\\\\n") {
            // SheetBlock1#initMap translates each Sea to the cumulative stripe
            // top. Its baseline is that origin plus this stripe's ascent.
            let metrics = rendered_line_metrics_with_family(
                line, self.message_font_size as f64, &self.message_font_family);
            if let Some(ascent) = previous_ascent {
                y += metrics.ascent - ascent;
            }
            previous_ascent = Some(metrics.ascent);''';assert old in q;q=q.replace(old,new);s=s[:a]+q+s[b:]
# Source polygon order is semantic, independent of x orientation.
old='''                            // Filled triangle: wing base, tip, wing tip.''';s=s.replace(old,'''                            // ComponentRoseSelfArrow#getPolygon: TOP visits
                            // wing-top, tip, wing-base; BOTTOM visits base first.''')
a=s.index('''                            let tip_x = special_return_tip.unwrap_or(active_anchor + 1.0) - 1.0;''');b=s.index('''                            write!(''',a);q=s[a:b];q=q.replace('''                                fmt_coord(loop_bottom),
                                fmt_coord(tip_x),''','''                                fmt_coord(if top { wing_y } else { loop_bottom }),
                                fmt_coord(tip_x),''',1).replace('''                                fmt_coord(wing_y),''','''                                fmt_coord(if top { loop_bottom } else { wing_y }),''');s=s[:a]+q+s[b:]
s=s.replace('''// ComponentRoseCollections surrounds its complete Display with the
        // same marginY7 as ComponentRoseParticipant; the second rect is offset.''','''// Rose#createComponentParticipant selects ComponentRoseParticipant
        // with collections=true: marginY7 and getDeltaCollection=4. This is
        // a later citation correction; the original account remains unchanged.''')
p.write_text(s)
