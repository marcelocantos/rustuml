# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
p=Path('crates/rustuml-render/src/sequence.rs');s=p.read_text();s=s.replace('sequence_create_message_indices','sequence_ordinary_creation_messages')
a='''            if let (Event::Create(id), Some(message)) = (&events[index], owner) {
                result.entry(id.clone()).or_default().push(message);
            }'''
b='''            if let (Event::Create(id), Some(message)) = (&events[index], owner) {
                // DrawableSetInitializer dispatches MessageExo to Step1MessageExo;
                // Teoz dispatches it to CommunicationExoTile. Neither consumer
                // draws an ordinary creation head or calls setCreate/goCreate.
                // Attachment ownership remains in sequence_create_attachments
                // and TeozHistory; this map is solely the ordinary head consumer.
                if let Event::Message(message_event) = &events[message]
                    && !matches!(message_event.from.as_str(), "[" | "]")
                    && !matches!(message_event.to.as_str(), "[" | "]")
                {
                    result.entry(id.clone()).or_default().push(message);
                }
            }'''
assert a in s;s=s.replace(a,b)
# Only queue's component receives its complete Display here; figure contracts remain separate.
a='''                queue_head_offset,
                stereo_ref,
                figure_line_h,
            );
        }
        ParticipantKind::Participant'''
b='''                queue_head_offset,
                // Participant#getDisplay includes stereotype regardless of box;
                // ComponentRoseQueue passes that complete block as its label.
                p.stereotype.as_ref().map(|s| (s.as_str(), p.stereotype_width)),
                figure_line_h,
            );
        }
        ParticipantKind::Participant'''
assert a in s;s=s.replace(a,b)
a='''                    let h = label_height + 5.0 + 5.0 - 1.0;''';b='''                    // ComponentRoseQueue uses Participant#getDisplay, including
                    // the stereotype stripe, for both preferred size and paint.
                    let h = stereotype_height + label_height + 5.0 + 5.0 - 1.0;''';assert a in s;s=s.replace(a,b)
a='''            let bh = if st.is_some() && in_box && p.kind != ParticipantKind::Participant {
                bh + stereotype_height
            } else {
                bh
            };'''
b='''            let bh = match p.kind {
                // These components already compose the complete Display above.
                ParticipantKind::Participant | ParticipantKind::Queue => bh,
                _ if st.is_some() && in_box => bh + stereotype_height,
                _ => bh,
            };''';assert a in s;s=s.replace(a,b)
s=s.replace('// USymbolQueue composes the stereotype and name Display vertically.', '// Participant#getDisplay supplies stereotype and name as one text block;\n        // ComponentRoseQueue passes it to USymbolQueue as the label.')
s=s.replace('// USymbolQueue#asSmall translates mergeTB(stereotype,label) by its\n        // margin. Each child contributes its own first-stripe ascent.', '// ComponentRoseQueue supplies the complete Display label to USymbolQueue;\n        // its margin translates both stripes, each with its own ascent.')
marker='''    #[test]
    fn repeated_creation_keeps_each_message_owner()'''
tests='''    #[test]
    fn external_creation_keeps_attachment_without_ordinary_head_consumer() {
        for teoz in [false, true] {
            for arrow in ["[-> Born", "Born <-]", "[<- Born", "Born ->]"] {
                let input = format!(
                    "@startuml\\n{}participant Seed\\nactor Born\\ncreate Born\\n{arrow}: edge\\ncreate Born\\nSeed -> Born: internal\\n@enduml",
                    if teoz { "!pragma teoz true\\n" } else { "" }
                );
                let diagram = rustuml_parser::parse::parse(&input).unwrap();
                let rustuml_parser::diagram::Diagram::Sequence(sequence) = &diagram else {
                    panic!("expected sequence");
                };
                let attached = sequence_create_attachments(&sequence.events).unwrap();
                assert_eq!(attached.iter().flatten().count(), 2);
                let external = attached.iter().flatten().next().copied().unwrap();
                let heads = sequence_ordinary_creation_messages(&sequence.events);
                assert_eq!(heads["Born"].len(), 1);
                assert_ne!(heads["Born"][0], external);
                let history = TeozHistory::new(&sequence.events, &vec![None; sequence.events.len()]);
                assert!(history.items.iter().any(|item| {
                    matches!(&item.life, Some((id, TeozLifeKind::Create, _)) if id == "Born")
                        && item.owner == Some(external)
                }));
                let svg = crate::render_svg(&diagram).unwrap();
                assert!(!svg.is_empty());
            }
        }
    }

    #[test]
    fn queue_stereotype_display_survives_bare_and_created_consumers() {
        for teoz in [false, true] {
            for declaration in ["queue", "create queue"] {
                let input = format!(
                    "@startuml\\n{}participant Seed\\n{declaration} \\\"Mail\\\\nroom\\\" as Mail <<dispatch>>\\nSeed -> Mail: send\\n@enduml",
                    if teoz { "!pragma teoz true\\n" } else { "" }
                );
                let diagram = rustuml_parser::parse::parse(&input).unwrap();
                let svg = crate::render_svg(&diagram).unwrap();
                assert_eq!(svg.matches("«dispatch»").count(), 2, "{svg}");
            }
        }
    }

'''
assert marker in s;s=s.replace(marker,tests+marker);p.write_text(s)
