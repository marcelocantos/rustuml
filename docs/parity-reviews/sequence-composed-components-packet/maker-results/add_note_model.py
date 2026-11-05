from pathlib import Path
p=Path('crates/rustuml-render/src/sequence.rs');s=p.read_text();at=s.index('fn note_text_metrics_with_family(')
s=s[:at]+'''/// BodyEnhanced2 segments Display before BodyEnhancedAbstract decorates each
/// body. This one component carries preferred bounds and its paint traversal.
struct EnhancedNoteBody<'a> {
    blocks: Vec<EnhancedNoteBlock<'a>>,
    width: f64,
    height: f64,
}
struct EnhancedNoteBlock<'a> {
    body: Vec<NoteVisualLine<'a>>,
    title: Option<&'a str>,
    separator: Option<char>,
    body_width: f64,
    width: f64,
    height: f64,
    title_height: f64,
}
impl<'a> EnhancedNoteBody<'a> {
    fn new(text: &'a str, size: f64, family: &str) -> Option<Self> {
        fn separator(line: NoteVisualLine<'_>) -> Option<(char, Option<&str>)> {
            if line.kind != NoteLineKind::Normal { return None; }
            let text = line.text.trim();
            for token in ["--", "==", "..", "__"] {
                if text.starts_with(token) && text.ends_with(token) && text != "..." {
                    return Some((token.chars().next().unwrap(),
                        (text.len()>4).then(|| text[2..text.len()-2].trim())));
                }
            }
            None
        }
        let lines=note_visual_lines(text);
        // Preserve the existing SheetBlock/table path when no block wrapper is
        // required. A separator is grammar, never a fixture or label predicate.
        if !lines.iter().any(|line| separator(*line).is_some()) { return None; }
        let mut blocks=Vec::new();
        let mut body=Vec::new();
        let mut heading=None;
        let mut title=None;
        for line in lines {
            if let Some((next, next_title))=separator(line) {
                blocks.push(Self::block(std::mem::take(&mut body),heading,title,size,family));
                heading=Some(next);title=next_title;
            } else { body.push(line); }
        }
        blocks.push(Self::block(body,heading,title,size,family));
        let width=blocks.iter().map(|b|b.width).fold(0.0,f64::max);
        let height=blocks.iter().map(|b|b.height).sum();
        Some(Self{blocks,width,height})
    }
    fn block(body: Vec<NoteVisualLine<'a>>, separator: Option<char>, title: Option<&'a str>,
        size:f64,family:&str) -> EnhancedNoteBlock<'a> {
        let mut counters=Vec::new();
        let body_width=body.iter().map(|line|note_visual_line_width_with_family(*line,size,family,&mut counters))
            .fold(0.0,f64::max);
        let body_height:f64=body.iter().map(|line|note_visual_line_metrics_with_family(*line,size,family).height).sum();
        // BodyEnhancedAbstract#decorate: marginX=0; titled body has right6,
        // top H/2, bottom4, then LineBefore minimum(titleWidth+8,H), outer top H/2.
        let (width,height,title_height)=if let Some(title)=title {
            let h=rendered_line_metrics_with_family(title,size,family).height;
            ((body_width+6.0).max(text_width_with_family(title,size,family)+8.0),
                (body_height+h/2.0+4.0).max(h)+h/2.0,h)
        } else if separator.is_some() { (body_width,body_height+8.0,0.0) }
        else { (body_width,body_height,0.0) };
        EnhancedNoteBlock{body,title,separator,body_width,width,height,title_height}
    }
    #[allow(clippy::too_many_arguments)]
    fn draw(&self,buf:&mut String,base:&TextBase<'_>,stroke:&str,align:MessageAlign,
        rule_left:f64,rule_right:f64) {
        let alignment=|outer:f64,inner:f64|match align {
            MessageAlign::Left=>0.0,MessageAlign::Center=>(outer-inner)/2.0,MessageAlign::Right=>outer-inner,
        };
        let mut top=base.y;
        for block in &self.blocks {
            let x=base.x+alignment(self.width,block.width);
            let rule_y=top+block.title_height/2.0;
            let draw_rule=|buf:&mut String| {
                let Some(separator)=block.separator else {return;};
                let thickness=if separator=='_' {"0.5"} else {"1"};
                let dash=if separator=='.' {"stroke-dasharray:1,2;"} else {""};
                let line=|buf:&mut String,x1:f64,x2:f64| {
                    for dy in if separator=='=' {vec![0.0,2.0]} else {vec![0.0]} {
                        write!(buf,r#"<line style="stroke:{stroke};stroke-width:{thickness};{dash}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                            fmt_coord(x1),fmt_coord(x2),fmt_coord(rule_y+dy),fmt_coord(rule_y+dy)).unwrap();
                    }
                };
                if let Some(title)=block.title {
                    let w=text_width_with_family(title,base.font_size as f64,base.font_family);
                    let x=(rule_left+rule_right-w)/2.0;
                    line(buf,rule_left,x);
                    let metrics=rendered_line_metrics_with_family(title,base.font_size as f64,base.font_family);
                    text_render::emit_text(buf,title,&TextBase{x,y:rule_y-block.title_height/2.0-0.5+metrics.ascent,
                        underline:false,skip_underline:true,..base.clone()});
                    line(buf,x+w,rule_right);
                } else { line(buf,rule_left,rule_right); }
            };
            // TextBlockLineBefore: untitled rule precedes the body; titled rule
            // follows it. UHorizontalLine centers its title on the note stencil.
            if block.title.is_none() {draw_rule(buf);}
            let mut y=top+if block.title.is_some(){block.title_height}
                else if block.separator.is_some(){4.0}else{0.0};
            let mut widths=Vec::new();let mut numbers=Vec::new();
            for line in &block.body {
                let metrics=note_visual_line_metrics_with_family(*line,base.font_size as f64,base.font_family);
                let width=note_visual_line_width_with_family(*line,base.font_size as f64,base.font_family,&mut widths);
                let sub=if line.kind==NoteLineKind::Normal && line_has_subscript_after_plain_first(line.text.trim()){3.0}else{0.0};
                emit_note_visual_line(buf,*line,&TextBase{x:x+alignment(block.body_width,width),
                    y:y+metrics.ascent-sub,..base.clone()},stroke,&mut numbers);
                y+=metrics.height;
            }
            if block.title.is_some(){draw_rule(buf);}
            top+=block.height;
        }
    }
}

''' +s[at:]
needle='fn note_text_metrics_with_family(text: &str, font_size: f64, font_family: &str) -> NoteTextMetrics {\n';s=s.replace(needle,needle+'''    if let Some(body)=EnhancedNoteBody::new(text,font_size,font_family) {
        let line_heights=body.blocks.iter().map(|b|b.height).filter(|h|*h>0.0).collect::<Vec<_>>();
        return NoteTextMetrics{first_height:line_heights.first().copied().unwrap_or(0.0),
            line_heights,total_height:body.height};
    }
''',1)
needle='fn note_max_line_width_with_family(text: &str, font_size: f64, font_family: &str) -> f64 {\n';s=s.replace(needle,needle+'''    if let Some(body)=EnhancedNoteBody::new(text,font_size,font_family) {return body.width;}
''',1)
needle='''                } else {
                    let mut line_top = note_top;
                    let mut note_number_counters''';repl='''                } else if let Some(body)=EnhancedNoteBody::new(&note.text,note_font_size_f,&note_font_family) {
                    let component_shift=match note_text_align {
                        MessageAlign::Left=>0.0,
                        MessageAlign::Center=>ROSE_NOTE_MARGIN_X2-NOTE_TEXT_X_PAD,
                        MessageAlign::Right=>2.0*ROSE_NOTE_PADDING_X+note_shadow_delta-NOTE_TEXT_X_PAD,
                    };
                    body.draw(&mut svg.buf,&TextBase{x:text_x+component_shift,y:note_top+text_y_offset,
                        font_size:svg.note_font_size,font_family:&svg.note_font_family,fill:&note_font_color,
                        bold:false,italic:false,underline:false,skip_underline:false},note_stroke,note_block_align,
                        note_left+1.0,note_left+max_text_w+2.0*note_global_padding+20.0);
                } else {
                    let mut line_top = note_top;
                    let mut note_number_counters''';assert needle in s;s=s.replace(needle,repl,1);p.write_text(s)
