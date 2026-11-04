from pathlib import Path
p=Path('crates/rustuml-parser/src/parse/sequence.rs');s=p.read_text()
s=s.replace('''    fn validate_colors(
''','''    // CODE capture position in CommandParticipantA/A2/A3/A4#getRegexConcat.
    // Match the already selected command's prefix, then project its byte range
    // onto the original (case-preserving) input. ASCII folding preserves offsets.
    fn participant_code<'a>(&self, text: &'a str, color: Option<&str>) -> Option<&'a str> {
        let code_index = match self.class {
            "CommandParticipantA" | "CommandParticipantA3" => 4,
            "CommandParticipantA2" | "CommandParticipantA4" => 3,
            _ => return None,
        };
        let RegisteredPattern::Color { prefix, .. } = &self.pattern else {
            return None;
        };
        let prefix_text = &text[..text.len() - color.map_or(0, str::len)];
        let folded = prefix_text.to_ascii_lowercase();
        let caps = prefix.captures(&folded)?;
        let code = caps.get(code_index)?;
        Some(&text[code.range()])
    }

    fn validate_colors(
''',1)
s=s.replace('''        if let Some(token) = token {
            // Colors#Colors''','''        // CommandParticipantMultilines#executeNow never reads its COLOR
        // capture. Single-line CommandParticipant#executeArg instead passes the
        // entire token to HColorSet#getColor, not to Colors.
        let token = if self.class == "CommandParticipantMultilines" {
            None
        } else if matches!(self.class, "CommandParticipantA" | "CommandParticipantA2" | "CommandParticipantA3" | "CommandParticipantA4") {
            if let Some(token) = token && !valid_sequence_color(token) {
                return Err(bad(token));
            }
            None
        } else {
            token
        };
        if let Some(token) = token {
            // Colors#Colors''',1)
s=s.replace('''        command.validate_colors(&folded, color, source_line)?;
        if color.is_some_and(|token| token.contains([':', ';', '/', '\\\\', '|'])) {''','''        // CommandParticipant#executeArg returns before color/stereotype/create
        // execution when CODE already exists. CommandActivate's separate create
        // path does not take this return and retains repeated creation events.
        let existing_participant = command.participant_code(text, color)
            .is_some_and(|code| parser.participant_ids.iter().any(|id| id == code));
        if !existing_participant {
            command.validate_colors(&folded, color, source_line)?;
        }
        if !existing_participant && command.class != "CommandParticipantMultilines"
            && color.is_some_and(|token| token.contains([':', ';', '/', '\\\\', '|', '-'])) {''',1)
# Skip duplicate execution, but retain command warning not needed for supported no-op.
s=s.replace('''        } else if command.class != "CommandNope" {
            let supported''','''        } else if command.class != "CommandNope" && !existing_participant {
            let supported''',1)
old='''r#"^(participant|actor|boundary|control|entity|database|collections|queue)[%s]+(?:"([^"]+)"[%s]+as[%s]+(\\w+)|(\\w+)[%s]+as[%s]+"([^"]+)"|(\\w+)[%s]+as[%s]+(\\w+)|"([^"]+)"|(\\w+))(?:[%s]+<<([^>]+)>>)?(?:[%s]+(#[^%s]+))?(?:[%s]+order[%s]+(\\d+))?"#'''
new='''r#"^(?i-u:(participant|actor|boundary|control|entity|database|collections|queue))[%s]+(?:"([^"]+)"[%s]+(?i-u:as)[%s]+([\\p{L}\\p{N}_.@]+)|([\\p{L}\\p{N}_.@]+)[%s]+(?i-u:as)[%s]+"([^"]+)"|([\\p{L}\\p{N}_.@]+)[%s]+(?i-u:as)[%s]+([\\p{L}\\p{N}_.@]+)|"([^"]+)"|([\\p{L}\\p{N}_.@]+))[%s]*(?:<<(.+?)>>)?[%s]*(?:(?i-u:order)[%s]+(?P<order>-?[0-9]{1,7}))?[%s]*(?P<color>#[^%s]+)?$"#'''
assert old in s;s=s.replace(old,new,1)
s=s.replace('''        static RE: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(
                '''+new,'''        // CommandParticipantA/A2/A3/A4: complete identity, optional touching
        // stereotype, then ORDER, URL (extracted above), COLOR and end-of-line.
        // Negative order is admitted but cannot be represented by usize: report
        // unsupported execution before mutating the model instead of truncating.
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(
                '''+new,1)
s=s.replace('''            let kind = parse_participant_kind(&caps[1]);''','''            if caps.name("order").is_some_and(|m| m.as_str().starts_with('-')) {
                return false;
            }
            let kind = parse_participant_kind(&caps[1].to_ascii_lowercase());''',1)
s=s.replace('''let color = caps.get(11).map(|m| m.as_str().to_string());''','''let color = caps.name("color").map(|m| m.as_str().to_string());''',1)
s=s.replace('''let explicit_order = caps.get(12).and_then(|m| m.as_str().parse::<usize>().ok());''','''let explicit_order = caps.name("order").and_then(|m| m.as_str().parse::<usize>().ok());''',1)
# Participant CREATE declarations must not take the partial activate handler.
s=s.replace('''                self.try_create_destroy(line) || self.try_participant_decl(line)
            }
            "CommandArrow"''','''                // CommandParticipant#getRegexType includes typed CREATE and
                // bare CREATE declarations with options; only a complete
                // declaration can create a new participant and creation event.
                static CREATE: LazyLock<Regex> = LazyLock::new(|| sequence_regex(r"^(?i-u:create)[%s]+(.*)$").unwrap());
                if let Some(caps) = CREATE.captures(line) {
                    let tail = &caps[1];
                    let declaration = if ["participant", "actor", "boundary", "control", "entity", "database", "collections", "queue"].iter().any(|kind| tail.get(..kind.len()).is_some_and(|prefix| prefix.eq_ignore_ascii_case(kind)) && tail[kind.len()..].starts_with(|c: char| matches!(c, ' ' | '\\t' | '\\r' | '\\n' | '\\u{000b}' | '\\u{000c}' | '\\u{00a0}'))) {
                        tail.to_owned()
                    } else {
                        format!("participant {tail}")
                    };
                    let before = self.participants.len();
                    if self.try_participant_decl_in_command(&declaration, line) {
                        if let Some(participant) = self.participants.get(before) {
                            self.events.push(Event::Create(participant.id.clone()));
                        }
                        true
                    } else {
                        false
                    }
                } else {
                    self.try_participant_decl(line)
                }
            }
            "CommandArrow"''',1)
p.write_text(s)
