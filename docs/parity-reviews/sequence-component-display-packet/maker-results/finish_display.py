from pathlib import Path
p=Path('crates/rustuml-render/src/sequence.rs');s=p.read_text()
# All stereotype painting consumes the same Display block as reservation.
start=s.index('    fn participant_box(');end=s.index('    /// Write a sequence message/arrow label.',start)
a=s[start:end]
a=a.replace('''        let line_h = atom_height_with_family(
            self.participant_font_size as f64,
            &self.participant_font_family,
        );''','''        let line_h = stereotype.map(|(text, _)| {
            ParticipantTextBlock::new(&format!("«{text}»"), self.participant_font_size as f64,
                &self.participant_font_family, false).height
        }).unwrap_or(0.0);''',1)
a=a.replace('''text_render::emit_text(
                &mut self.buf,
                &st_display,''','''ParticipantTextBlock::new(&st_display, self.participant_font_size as f64,
                &self.participant_font_family, false).draw(
                &mut self.buf,''')
a=a.replace('''text_render::emit_text(
            &mut self.buf,
            &st_display,''','''ParticipantTextBlock::new(&st_display, self.participant_font_size as f64,
            &self.participant_font_family, false).draw(
            &mut self.buf,''')
s=s[:start]+a+s[end:]
start=s.index('    fn collections_shape(');end=s.index('    /// Write one cut segment',start);a=s[start:end]
a=a.replace('''        let rect_h = HEAD_BOX_H + stereo_shift;''','''        // ComponentRoseCollections surrounds its complete Display with the
        // same marginY7 as ComponentRoseParticipant; the second rect is offset.
        let rect_h = ParticipantTextBlock::new(text_content,
            self.participant_font_size as f64, &self.participant_font_family,
            self.participant_font_bold).height + 2.0 * 7.0 + stereo_shift;''')
a=a.replace('''text_render::emit_text(
                &mut self.buf,
                &st_display,''','''ParticipantTextBlock::new(&st_display, self.participant_font_size as f64,
                &self.participant_font_family, false).draw(
                &mut self.buf,''')
a=a.replace('''        // A boxed stereotype adds one line: the pill grows by `line_h` and the
        // name shifts down, leaving the stereotype on the `text_y` line above.''','''        // USymbolQueue composes the stereotype and name Display vertically.''')
a=a.replace('''        // The queue pill is 4px shorter than a normal head box and is
        // vertically offset: pushed down 5px in the head region, flush with
        // the top in the tail region (matching PlantUML).
''','')
s=s[:start]+a+s[end:]
start=s.index('fn render_participant_shape(');end=s.index('/// Render a sequence diagram',start);a=s[start:end]
a=a.replace('''    // Stereotype line for figure participants (drawn italic above the name).
    // One participant text line; the head height already accounts for it.
    let figure_line_h = atom_height_with_family(
        svg.participant_font_size as f64,
        &svg.participant_font_family,
    );''','''    // Participant#getDisplay prepends the stereotype component to the label.
    // Use its complete height for both reservation and paint translation.
    let figure_line_h = p.stereotype.as_ref().map(|text| {
        ParticipantTextBlock::new(&format!("«{text}»"), p.font_size as f64,
            &svg.participant_font_family, false).height
    }).unwrap_or(0.0);''')
a=a.replace('''            let text_y = base_y + COLLECTIONS_OFFSET + BOX_TEXT_Y_OFFSET;''','''            let text_y = base_y + COLLECTIONS_OFFSET + p.text_y_offset;''')
a=a.replace('''                    atom_height_with_family(
                        svg.participant_font_size as f64,
                        &svg.participant_font_family,
                    )''','''                    figure_line_h''')
s=s[:start]+a+s[end:]
# Remove obsolete constants only when no longer referenced.
if s.count('BOX_TEXT_Y_OFFSET')==1:
 s=s.replace('const BOX_TEXT_Y_OFFSET: f64 = 20.535156250; // exact Java double: baseline from box top\n','')
s=s.replace('''            // A stereotype adds one text line to the head's text block (drawn
            // italic, above the participant name). For the figure participant
            // kinds this grows the head by `participant_line_h`; the figure
            // stays at the head top and the name keeps its baseline (the shape
            // renderers shift the name + stereotype down accordingly). The
            // `Participant` kind already folds this into its branch above.''','''            // Participant#getDisplay composes the stereotype above the label.
            // Its measured height participates in every rendered head kind;
            // Participant already includes it in its rectangle branch.''')
p.write_text(s)
