from pathlib import Path
p=Path('crates/rustuml-parser/src/parse/sequence.rs');s=p.read_text();at=s.index('    #[test]\n    fn registered_sequence_commands')
s=s[:at]+r'''    #[test]
    fn sprite_blocks_own_only_their_command_body() {
        for keyword in ["sprite ", "SPRITE\t", "SpRiTe\u{00a0}"] {
            for (body, pixel) in [
                ("$mark <svg width=\"2\" height=\"2\">\n}\n<rect width=\"2\" height=\"2\"/>\n</svg>", false),
                ("$mark <svg width=\"2\" height=\"2\"></svg>", false),
                ("$mark [2x2/16] {\nFF\n00\n}", true),
                ("$mark [2x2/16] {\nFF\n00\nEND SPRITE", true),
                ("$mark [2x2/16] {\nFF\n00\nendsprite", true),
            ] {
                let source = format!("@startuml\n{keyword}{body}\nA -> B: retained\n@enduml");
                let pre = crate::preprocess::preprocess_full(&source, None);
                let diagram = parse_sequence(&pre.lines).unwrap();
                assert_eq!(diagram.events.len(), 1, "{source}");
                assert_eq!(pre.sprites.contains_key("mark"), pixel);
                if pixel {
                    assert_eq!(pre.sprites["mark"].rows, ["FF", "00"]);
                }
            }
        }
    }

    #[test]
    fn complete_participant_declarations_preserve_identity_and_options() {
        let d = parse("participant \"Tall tower\" as Top.Node<<edge>> order 7#Wheat\nqueue @Port as \"Mail room\"<<service>>#Olive\nA -> B");
        assert_eq!(d.participants[0].id, "Top.Node");
        assert_eq!(d.participants[0].label, "Tall tower");
        assert_eq!(d.participants[0].stereotype.as_deref(), Some("edge"));
        assert_eq!(d.participants[0].order, Some(7));
        assert_eq!(d.participants[0].color.as_deref(), Some("#Wheat"));
        assert_eq!(d.participants[1].id, "@Port");
        assert_eq!(d.participants[1].label, "Mail room");
        // An unsupported negative order consumes no truncated participant.
        let d = parse("participant West order -7\nA -> B");
        assert!(!d.participants.iter().any(|p| p.id == "West"));
        let mut parser = SeqParser::new();
        assert!(!parser.try_participant_decl("participant West unsupported"));
        assert!(parser.participants.is_empty());
    }

    #[test]
    fn command_specific_color_execution_respects_early_and_unused_paths() {
        for color in ["#back:Wheat;line:Olive", "#line.dotted", "#shadowing:true", "#text:application"] {
            assert!(parse_sequence(&[format!("participant West {color}")]).is_err(), "{color}");
        }
        for separator in ['-', '/', '\\', '|'] {
            assert!(parse_sequence(&[format!("participant West #Wheat{separator}Olive")]).is_ok());
        }
        for declaration in [
            "participant West #NotAColour",
            "participant West as \"Another label\" #NotAColour",
            "participant \"Another label\" as West #NotAColour",
            "participant Else as West #NotAColour",
            "participant \"West\" #NotAColour",
        ] {
            let d = parse_sequence(&["participant West".into(), declaration.into()]).unwrap();
            assert_eq!(d.participants.len(), 1);
            assert_eq!(d.participants[0].label, "West");
        }
        assert!(parse_sequence(&["participant West #NotAColour [".into(), "Label".into(), "]".into()]).is_ok());
        assert!(parse_sequence(&["box #back:Wheat;line:Olive".into(), "participant West".into(), "end box".into()]).is_ok());
    }

'''+s[at:];p.write_text(s)
