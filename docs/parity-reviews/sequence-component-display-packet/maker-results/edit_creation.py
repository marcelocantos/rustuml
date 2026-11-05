from pathlib import Path
p=Path('crates/rustuml-render/src/sequence.rs');s=p.read_text()
s=s.replace('create_msg_idx: &HashMap<String, usize>','create_msg_idx: &HashMap<String, Vec<usize>>')
s=s.replace('create_msg_idx.get(participant) == Some(&index)','create_msg_idx.get(participant).is_some_and(|indices| indices.contains(&index))')
s=s.replace('fn sequence_create_message_indices(events: &[Event]) -> HashMap<String, usize> {\n    let mut result = HashMap::new();','''fn sequence_create_message_indices(events: &[Event]) -> HashMap<String, Vec<usize>> {
    // SequenceDiagram#addMessage attaches each pending CREATE life event to
    // its compatible message. Participant creation is not a one-shot index.
    let mut result: HashMap<String, Vec<usize>> = HashMap::new();''')
s=s.replace('result.entry(participant).or_insert(event_index);','result.entry(participant).or_default().push(event_index);')
s=s.replace('creates.get(id).copied()','''// Each raw CREATE owns its next attached message, not the
                    // participant's first or final creation callback.
                    creates.get(id).and_then(|indices| indices.iter().find(|&&owner| owner > index)).copied()''')
s=s.replace('create_msg_idx.get(&msg.to) == Some(&idx)','create_msg_idx.get(&msg.to).is_some_and(|indices| indices.contains(&idx))')
s=s.replace('create_msg_idx.get(&msg.to) == Some(&ev_idx)','create_msg_idx.get(&msg.to).is_some_and(|indices| indices.contains(&ev_idx))')
s=s.replace('create_msg_idx.get(&participant.id).map_or(0.0, |&index| {','''// LivingSpace#goCreate(y) overwrites createY on every callback.
                    create_msg_idx.get(&participant.id).and_then(|indices| indices.last()).map_or(0.0, |&index| {''')
a=s.index('    let created_lifeline_top: HashMap<String, f64> = create_msg_idx');b=s.index('\n    // Frames are popped',a)
s=s[:a]+'''    let creation_line_top = |id: &str, idx: usize| {
        if diagram.teoz {
            let (has_text, height) = event_message_text_height(&diagram.events[idx]);
            event_y(idx)
                - (msg_step(has_text, height) + 2.0 * global_padding - 2.0 * ARROW_HALF_H)
                - 2.0
        } else {
            // Step1Message#createArrowCreate records origin+composite/2;
            // DrawableSet adds ParticipantBox#magicMargin (line height /2).
            let head = &participants[id_to_idx[id]];
            let (has_text, h) = event_message_text_height(&diagram.events[idx]);
            let arrow_height = msg_step(has_text, h) + 2.0 * global_padding;
            let contact = (if has_text { h } else { 0.0 })
                + 2.0 + ROSE_ARROW_PADDING_Y + 2.0 * global_padding;
            event_y(idx) - contact
                + arrow_height.max(head.preferred_height + ROSE_LIFELINE_PREFERRED_HEIGHT / 2.0) / 2.0
                + ROSE_LIFELINE_PREFERRED_HEIGHT / 2.0
        }
    };
    // LifeLine#setCreate and LivingSpace#goCreate overwrite the participant's
    // line start; every earlier message still owns its separately drawn head.
    let created_lifeline_top: HashMap<String, f64> = create_msg_idx
        .iter()
        .filter_map(|(id, indices)| indices.last().map(|&idx| (id.clone(), creation_line_top(id, idx))))
        .collect();
''' +s[b:]
s=s.replace('if let Some(&ev_idx) = create_msg_idx.get(&p.id) {','if let Some(indices) = create_msg_idx.get(&p.id) {\n            for &ev_idx in indices {')
s=s.replace('''            created_inline.insert(ev_idx, (i, fill_color.clone(), draw_control_glyph));
        } else {''','''            created_inline.insert(ev_idx, (i, fill_color.clone(), draw_control_glyph));
            }
        } else {''')
s=s.replace('''                shifted.box_x += if reverse {''','''                let shift = if reverse {''')
s=s.replace('''                };
                &shifted
            } else {''','''                };
                // CommunicationTile applies UTranslate to the entire head:
                // symbol center, label anchor and box bounds share the move.
                shifted.box_x += shift;
                shifted.center_x += shift;
                &shifted
            } else {''')
s=s.replace('''                created_lifeline_top[&p.id]
            } else {''','''                creation_line_top(&p.id, ev_idx)
            } else {''')
p.write_text(s)
