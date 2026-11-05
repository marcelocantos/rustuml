from pathlib import Path
p=Path('crates/rustuml-parser/src/diagram/sequence.rs');s=p.read_text();s+='''
/// Java SequenceDiagram#activate overwrites one pending CREATE. addMessage
/// consumes that exact raw event, keeping earlier attached creations intact.
/// Owners are indexed by raw event, so overwritten CREATEs stay unowned.
pub fn sequence_create_attachments(events: &[Event]) -> Result<Vec<Option<usize>>, (u32, String)> {
    let mut owners = vec![None; events.len()];
    let mut pending: Option<(usize, &str)> = None;
    for (index, event) in events.iter().enumerate() {
        match event {
            Event::Create(id) => pending = Some((index, id)),
            Event::Message(message) => {
                if let Some((create, participant)) = pending {
                    // Message#compatibleForCreate excludes self/source;
                    // MessageExo accepts its sole real participant in either direction.
                    let compatible = if matches!(message.from.as_str(), "[" | "]") {
                        message.to == participant
                    } else if matches!(message.to.as_str(), "[" | "]") {
                        message.from == participant
                    } else {
                        message.from != participant && message.to == participant
                    };
                    if !compatible {
                        return Err((message.source_line, format!(
                            "After create command, you have to send a message to \\\"{participant}\\\"")));
                    }
                    owners[create] = Some(index);
                    pending = None;
                }
            }
            _ => {}
        }
    }
    Ok(owners)
}
''';p.write_text(s)
p=Path('crates/rustuml-parser/src/parse/sequence.rs');s=p.read_text();old='''    resolve_sequence_returns(events)
        .map(|_| ())
        .map_err(|(line, message)| ParseError { line, message })''';new='''    let lowered = resolve_sequence_returns(events)
        .map_err(|(line, message)| ParseError { line, message })?;
    sequence_create_attachments(&lowered)
        .map(|_| ())
        .map_err(|(line, message)| ParseError { line, message })''';assert old in s;s=s.replace(old,new);p.write_text(s)
p=Path('crates/rustuml-render/src/sequence.rs');s=p.read_text();a=s.index('fn sequence_create_message_indices(');b=s.index('\nfn sequence_lifecycle_layout',a)
s=s[:a]+'''fn sequence_create_message_indices(events: &[Event]) -> HashMap<String, Vec<usize>> {
    let mut result: HashMap<String, Vec<usize>> = HashMap::new();
    // Source parsing validates compatibility after return lowering. A caller
    // supplying an invalid model directly gets no invented creation ownership.
    if let Ok(owners) = sequence_create_attachments(events) {
        for (index, owner) in owners.into_iter().enumerate() {
            if let (Event::Create(id), Some(message)) = (&events[index], owner) {
                result.entry(id.clone()).or_default().push(message);
            }
        }
    }
    result
}
''' + s[b:]
s=s.replace('''        let creates = sequence_create_message_indices(events);''','''        let creates = sequence_create_attachments(events).unwrap_or_default();''',1)
a=s.index('''                owner: if let Event::Create(id) = event {''');b=s.index('''                } else {
                    owners[index]''',a)
s=s[:a]+'''                owner: if matches!(event, Event::Create(_)) {
                    // Use the actual raw event identity. An overwritten pending
                    // CREATE must not borrow a later event for the same participant.
                    creates.get(index).copied().flatten()
'''+s[b:]
s=s.replace('''    let mut spacing_pending_create: Vec<String> = Vec::new();\n''','')
s=s.replace('''            Event::Create(id) => spacing_pending_create.push(id.clone()),\n''','')
s=s.replace('''                        let create_extra = if let Some(pos) =
                            spacing_pending_create.iter().position(|id| *id == msg.to)
                        {
                            spacing_pending_create.remove(pos);''','''                        let create_extra = if create_msg_idx.get(&msg.to)
                            .is_some_and(|indices| indices.contains(&event_idx))
                        {''')
p.write_text(s)
