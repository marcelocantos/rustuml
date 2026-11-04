from pathlib import Path
p=Path('crates/rustuml-parser/src/parse/sequence.rs');s=p.read_text();needle='    #[test]\n    fn participant_labels_keep_display_lines_separate_from_identity()';insert='''    #[test]
    fn pending_create_overwrites_only_unattached_events() {
        for earlier in ["Birch", "Willow"] {
            let input = format!("participant Ash\\nparticipant Birch\\nparticipant Willow\\ncreate {earlier}\\ncreate Birch\\nAsh -> Birch: one\\nAsh -> {earlier}: ordinary\\ncreate Birch\\nAsh -> Birch: two");
            let diagram = parse(&input);
            let owners = sequence_create_attachments(&diagram.events).unwrap();
            let creates: Vec<_> = diagram.events.iter().enumerate().filter_map(|(i, event)| matches!(event, Event::Create(_)).then_some(i)).collect();
            assert_eq!(creates.len(), 3);
            assert_eq!(owners[creates[0]], None);
            assert_eq!(owners[creates[1]], Some(creates[1] + 1));
            assert_eq!(owners[creates[2]], Some(creates[2] + 1));
        }
        for message in ["Birch -> Ash", "Birch -> Birch"] {
            let lines = format!("participant Ash\\nparticipant Birch\\ncreate Birch\\n{message}: invalid").lines().map(str::to_owned).collect::<Vec<_>>();
            assert!(parse_sequence(&lines).is_err(), "{message}");
        }
        for message in ["[-> Birch", "Birch ->]"] {
            let diagram = parse(&format!("participant Birch\\ncreate Birch\\n{message}: external"));
            assert_eq!(sequence_create_attachments(&diagram.events).unwrap().iter().filter(|x| x.is_some()).count(), 1);
        }
    }

''';assert needle in s;s=s.replace(needle,insert+needle);p.write_text(s)
p=Path('crates/rustuml-render/src/sequence.rs');s=p.read_text();needle='    #[test]\n    fn repeated_creation_keeps_each_message_owner()';insert='''    fn text_baseline(svg: &str, label: &str) -> f64 {
        let tag = svg.split("<text ").find(|tag| tag.contains(&format!(">{label}</text>"))).unwrap();
        tag.split(" y=\\\"").nth(1).unwrap().split('"').next().unwrap().parse().unwrap()
    }

    #[test]
    fn message_stripes_use_current_ascent_after_prior_height() {
        for pragma in ["", "!pragma teoz true\\n"] {
            let input = format!("@startuml\\n{pragma}Ash -> Birch: one\\\\n<size:23>two</size>\\\\nthree\\n@enduml");
            let svg = crate::render_svg(&rustuml_parser::parse::parse(&input).unwrap());
            let small = rendered_line_metrics_with_family("one", 13.0, "sans-serif");
            let large = rendered_line_metrics_with_family("<size:23>two</size>", 13.0, "sans-serif");
            let first = text_baseline(&svg, "one");
            let second = text_baseline(&svg, "two");
            let third = text_baseline(&svg, "three");
            assert!((second-first-(small.height+large.ascent-small.ascent)).abs() < 0.001, "{svg}");
            assert!((third-second-(large.height+small.ascent-large.ascent)).abs() < 0.001, "{svg}");
        }
    }

    #[test]
    fn queue_label_starts_at_component_margin_with_own_ascent() {
        for size in [9, 23] {
            let input = format!("@startuml\\nqueue \\\"<size:{size}>Tall</size>\\\\nend\\\" as Mail\\nMail -> Mail: loop\\n@enduml");
            let svg = crate::render_svg(&rustuml_parser::parse::parse(&input).unwrap());
            let path = svg.split("d=\\\"M").nth(1).unwrap();
            let top: f64 = path.split(',').nth(1).unwrap().split(' ').next().unwrap().parse().unwrap();
            let ascent = rendered_line_metrics_with_family(&format!("<size:{size}>Tall</size>"), 14.0, "sans-serif").ascent;
            assert!((text_baseline(&svg, "Tall")-top-5.0-ascent).abs() < 0.001, "{svg}");
        }
    }

''';assert needle in s;s=s.replace(needle,insert+needle);p.write_text(s)
