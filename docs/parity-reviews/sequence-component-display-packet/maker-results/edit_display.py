from pathlib import Path
p=Path('crates/rustuml-parser/src/parse/sequence.rs');s=p.read_text().replace('inline_note_display','inline_sequence_display').replace('// Display#getWithNewlines expands inline note escapes before the component','// Display#getWithNewlines expands inline note/participant escapes before the component');s=s.replace('label: id.clone(),','label: inline_sequence_display(&id),');s=s.replace('''            let stereotype = caps
                .get(10)''','''            // CommandParticipant passes its label through Display#getWithNewlines.
            let label = inline_sequence_display(&label);
            let stereotype = caps
                .get(10)''');p.write_text(s)
p=Path('crates/rustuml-render/src/sequence.rs');s=p.read_text();a=s.index('/// BodyEnhanced2 segments Display');s=s[:a]+'''/// Evaluated participant Display, shared by component measurement and painting.
/// AbstractTextualComponent builds a centered SheetBlock from Display lines;
/// each stripe contributes its own metrics, not a fixed line-count surcharge.
struct ParticipantTextBlock<'a> {
    lines: Vec<(&'a str, f64, RenderedLineMetrics)>,
    width: f64,
    height: f64,
}
impl<'a> ParticipantTextBlock<'a> {
    fn new(text: &'a str, size: f64, family: &str, bold: bool) -> Self {
        let lines: Vec<_> = text.split('\\n').map(|line| {
            // StripeSimple#getAtoms supplies a space for a present empty line.
            let line = if line.is_empty() { " " } else { line };
            let width = text_render::measure_with_family(line, size, bold, family);
            (line, width, rendered_line_metrics_with_family(line, size, family))
        }).collect();
        let width = lines.iter().map(|line| line.1).fold(0.0, f64::max);
        let height = lines.iter().map(|line| line.2.height).sum();
        Self { lines, width, height }
    }
    fn draw(&self, buf: &mut String, base: &TextBase<'_>) {
        let mut top = base.y - self.lines[0].2.ascent;
        for (text, width, metrics) in &self.lines {
            text_render::emit_text(buf, text, &TextBase {
                x: base.x + (self.width - width) / 2.0,
                y: top + metrics.ascent,
                ..base.clone()
            });
            top += metrics.height;
        }
    }
}

'''+s[a:]
# Both rectangle and symbol labels use the same Display text block.
a=s.index('        let _ = text_len;',s.index('fn participant_box'));b=s.index('        self.participant_group_close();',a);s=s[:a]+'''        self.participant_text(label_x, text_y, text_content, text_len);

'''+s[b:]
a=s.index('        let _ = text_len;',s.index('fn participant_text'));b=s.index('\n    /// Draw a figure',a);s=s[:a]+'''        let block = ParticipantTextBlock::new(text_content, self.participant_font_size as f64,
            &self.participant_font_family, self.participant_font_bold);
        let _ = text_len;
        block.draw(&mut self.buf, &TextBase {
            x: text_x, y: text_y, font_size: self.participant_font_size,
            font_family: &self.participant_font_family, fill: &self.participant_font_color,
            bold: self.participant_font_bold, italic: self.participant_font_italic,
            underline: false, skip_underline: false,
        });
    }
''' +s[b:]
# A queue's row band is its complete component minus the common Rose +1 band;
# bottom alignment itself supplies the old mixed-kind displacement.
a=s.index('    let all_participants_are_queues');b=s.index('    let mut participants:',a);s=s[:a]+'''    // ComponentRoseQueue has no separate internal head translation. Its
    // preferred dimension participates in the same bottom alignment as heads.
    let queue_head_offset = 0.0;

'''+s[b:]
a=s.index('            let tw = if participant_font_bold');b=s.index('            // Box width must accommodate',a);s=s[:a]+'''            let label_block = ParticipantTextBlock::new(&label, participant_font_size_f,
                &participant_font_family, participant_font_bold);
            let tw = label_block.width;
            let label_height = label_block.height;
            let stereotype_height = st_display.as_ref().map(|text|
                ParticipantTextBlock::new(text, participant_font_size_f, &participant_font_family, false).height
            ).unwrap_or(0.0);
''' +s[b:]
s=s.replace('let h = HEAD_BOX_H + ACTOR_EXTRA_H;','''// ActorStickMan preferred geometry60 plus Display, minus
                    // the common one-pixel Rose band tracked by preferred_height.
                    let h = label_height + 14.0 + ACTOR_EXTRA_H;''').replace('let h = HEAD_BOX_H + CIRCLE_SHAPE_EXTRA_H;','let h = label_height + 14.0 + CIRCLE_SHAPE_EXTRA_H;').replace('let h = HEAD_BOX_H + DB_EXTRA_H;','let h = label_height + 14.0 + DB_EXTRA_H;').replace('let h = HEAD_BOX_H + COLLECTIONS_EXTRA_H;','let h = label_height + 14.0 + COLLECTIONS_EXTRA_H;')
s=s.replace('let h = queue_layout_h;','''// USymbolQueue Margin(5,15,5,5); preferred_height adds
                    // back the common band pixel, not an extra text line.
                    let h = label_height + 5.0 + 5.0 - 1.0;''')
s=s.replace('participant_box_h + participant_line_h','label_height + 14.0 + 2.0 * global_padding + stereotype_height').replace('''                    } else {
                        participant_box_h
                    };''','''                    } else {
                        label_height + 14.0 + 2.0 * global_padding
                    };''')
s=s.replace('bh + participant_line_h\n            } else {','bh + stereotype_height\n            } else {')
a=s.index('                preferred_height: if p.kind == ParticipantKind::Queue');b=s.index('                } else {\n                    bh + if matches!',a);s=s[:a]+'''                preferred_height: if p.kind == ParticipantKind::Queue {
                    // USymbolQueue#getMargin surrounds the complete Display.
                    bh + 1.0
'''+s[b:]
# Queue primitive dimensions and label origin consume the same Display model.
a=s.index('    fn queue_shape');b=s.index('    /// Write one cut segment',a);q=s[a:b];q=q.replace('let stereo_shift = if stereo.is_some() { line_h } else { 0.0 };','''let stereo_shift = stereo.map(|(text, _)|
            ParticipantTextBlock::new(&format!("«{text}»"), self.participant_font_size as f64,
                &self.participant_font_family, false).height).unwrap_or(0.0);
        let label = ParticipantTextBlock::new(text_content, self.participant_font_size as f64,
            &self.participant_font_family, self.participant_font_bold);
        let _ = line_h;''');q=q.replace('let pill_h = HEAD_BOX_H - 4.0 + stereo_shift;','''// ComponentRoseQueue -> USymbolQueue uses Display height + margins5/5.
        let pill_h = label.height + stereo_shift + 5.0 + 5.0;''');s=s[:a]+q+s[b:]
s=s.replace('''            let text_y = pill_top + (HEAD_BOX_H - 4.0) / 2.0 + 5.29102;''','''            // USymbolQueue draws its text block at marginY1=5.
            let text_y = pill_top + 5.0 + ascent_with_family(
                svg.participant_font_size as f64, &svg.participant_font_family);''')
# The participant head band now derives from actual preferred size, including
# queue's lack of shadow reservation. Common band pixel is used by existing row.
a=s.index('    let participant_head_band =');b=s.index('    let max_head_band',a);s=s[:a]+'''    let participant_head_band = |p: &ParticipantLayout| p.preferred_height - 1.0;
'''+s[b:]
# Remove only variables rendered unused by the component model.
s=s.replace('    let participant_box_h = participant_line_h + 14.0 + 2.0 * global_padding;\n','')
p.write_text(s)
