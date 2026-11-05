from pathlib import Path
p=Path('crates/rustuml-render/src/sequence.rs');s=p.read_text();needle='    #[test]\n    fn enhanced_note_body_paints_body_before_centered_title()'
insert='''    #[test]
    fn repeated_creation_keeps_each_message_owner() {
        for source in ["Seed", "Other"] {
            let input = format!("@startuml\\nparticipant Seed\\nparticipant Other\\nactor Born\\ncreate Born\\n{source} -> Born: first\\nBorn -> {source}: reply\\ncreate Born\\n{source} -> Born ++: second\\nBorn --> {source} --: release\\ncreate Born\\n{source} -> Born: third\\n@enduml");
            let diagram = rustuml_parser::parse::parse(&input).unwrap();
            let rustuml_parser::diagram::Diagram::Sequence(sequence) = diagram else {
                panic!("expected sequence");
            };
            let owners = sequence_create_message_indices(&sequence.events);
            let indices = owners.get("Born").unwrap();
            assert_eq!(indices.len(), 3);
            assert!(indices.windows(2).all(|pair| pair[0] < pair[1]));
            for index in indices {
                assert!(matches!(&sequence.events[*index], Event::Message(message) if message.to == "Born"));
                assert!(matches!(&sequence.events[*index - 1], Event::Create(id) if id == "Born"));
            }
        }
    }

    #[test]
    fn participant_display_measures_and_paints_each_stripe() {
        for size in [11, 19] {
            let block = ParticipantTextBlock::new("Long label\\n\\nEnd", size as f64, "sans-serif", false);
            assert_eq!(block.lines.len(), 3);
            assert_eq!(block.lines[1].0, " ");
            assert_eq!(block.height, block.lines.iter().map(|line| line.2.height).sum::<f64>());
            let mut output = String::new();
            block.draw(&mut output, &TextBase { x: 0.0, y: block.lines[0].2.ascent,
                font_size: size, font_family: "sans-serif", fill: "#000000", bold: false,
                italic: false, underline: false, skip_underline: false });
            assert_eq!(output.matches("<text ").count(), 3);
            assert!(output.contains("> </text>"));
            assert!(output.find(">Long label</text>").unwrap() < output.find(">End</text>").unwrap());
        }
    }

'''
assert needle in s;s=s.replace(needle,insert+needle);p.write_text(s)
p=Path('crates/rustuml-parser/src/parse/sequence.rs');s=p.read_text();needle='    #[test]\n    fn inline_sequence_display_normalizes_before_model_storage()'
insert='''    #[test]
    fn participant_labels_keep_display_lines_separate_from_identity() {
        for kind in ["participant", "actor", "queue"] {
            let diagram = parse(&format!(r#"{kind} "Long\\nShort\\nLast" as Mail
Gate -> Mail: enqueue"#));
            let participant = diagram.participants.iter().find(|p| p.id == "Mail").unwrap();
            assert_eq!(participant.label, "Long\\nShort\\nLast");
            assert!(matches!(&diagram.events[0], Event::Message(message) if message.to == "Mail"));
        }
    }

'''
assert needle in s;s=s.replace(needle,insert+needle);p.write_text(s)
