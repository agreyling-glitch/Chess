use super::*;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct StudyNode {
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub san: String,
    pub fen: String,
    pub comment: String,
    pub nags: Vec<u8>,
    #[serde(default)]
    pub local_note: Option<String>,
    #[serde(default)]
    pub local_marks: Option<Vec<BoardMark>>,
    #[serde(default)]
    pub local_nags: Option<Vec<u8>>,
    #[serde(default)]
    pub local: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct StudyChapter {
    pub source: String,
    #[serde(default)]
    pub original_source: Option<String>,
    pub name: String,
    pub url: String,
    pub nodes: Vec<StudyNode>,
    #[serde(default)]
    pub retained: bool,
    #[serde(default)]
    pub analysis_lines: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct StudyWorkspace {
    pub name: String,
    pub chapters: Vec<StudyChapter>,
    pub chapter: usize,
    pub node: usize,
    #[serde(default)]
    pub line_end: usize,
    #[serde(default)]
    pub flipped: Option<bool>,
    #[serde(default)]
    pub visited: std::collections::BTreeSet<String>,
    #[serde(skip)]
    pub practice: bool,
    #[serde(default)]
    pub practice_black: bool,
    #[serde(default)]
    pub practice_side_chosen: bool,
    #[serde(skip)]
    pub attempts: usize,
    #[serde(skip)]
    pub correct: usize,
    #[serde(skip)]
    pub message: String,
    #[serde(skip)]
    pub refreshing: bool,
    #[serde(skip)]
    pub focus_panel: bool,
}

#[derive(Debug)]
enum Token {
    Move(String),
    Comment(String),
    Open,
    Close,
}

fn tokens(text: &str) -> Result<Vec<Token>, String> {
    let body = ChessApp::pgn_movetext(text);
    let mut chars = body.chars().peekable();
    let mut out = Vec::new();
    while let Some(ch) = chars.next() {
        match ch {
            c if c.is_whitespace() => {}
            ';' => {
                let comment: String = chars.by_ref().take_while(|c| *c != '\n').collect();
                out.push(Token::Comment(comment));
            }
            '{' => {
                let mut comment = String::new();
                let mut depth = 1;
                for c in chars.by_ref() {
                    if c == '{' {
                        depth += 1;
                    }
                    if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    comment.push(c);
                }
                if depth != 0 {
                    return Err("Study has an unclosed comment.".into());
                }
                out.push(Token::Comment(comment));
            }
            '(' => out.push(Token::Open),
            ')' => out.push(Token::Close),
            '}' => return Err("Study has an unmatched comment delimiter.".into()),
            _ => {
                let mut token = ch.to_string();
                while chars
                    .peek()
                    .is_some_and(|c| !c.is_whitespace() && !"{}();".contains(*c))
                {
                    token.push(chars.next().unwrap());
                }
                out.push(Token::Move(token));
            }
        }
    }
    Ok(out)
}

impl StudyChapter {
    fn opening_label(&self) -> Option<String> {
        let tag = |name| ChessApp::pgn_tag(&self.source, name)
            .filter(|text| !matches!(text.trim(), "" | "?" | "-"));
        let eco = tag("ECO").unwrap_or_default();
        let name = tag("Opening").unwrap_or_default();
        let classified = if eco.is_empty() || name.is_empty() {
            let standard = ChessApp::pgn_tag(&self.source, "Variant")
                .is_none_or(|variant| variant == "Standard")
                && ChessApp::pgn_tag(&self.source, "FEN")
                    .is_none_or(|fen| fen == Board::default().to_string());
            standard.then(|| ChessApp::cached_pgn_timeline(&self.source).ok()
                .and_then(|(_, moves)| crate::opening_classification::classify(&moves))).flatten()
        } else { None };
        let (eco, name) = crate::opening_classification::consistent_labels(&eco, &name, classified);
        let label = [eco, name].into_iter().filter(|text| !text.trim().is_empty()).collect::<Vec<_>>().join(" · ");
        (!label.is_empty()).then_some(label)
    }

    pub fn parse(source: &str) -> Result<Self, String> {
        ChessApp::validate_pgn_complexity(source)?;
        let variant = ChessApp::pgn_tag(source, "Variant").unwrap_or_else(|| "Standard".into());
        if !["Standard", "From Position", "Chess960"].contains(&variant.as_str()) {
            return Err(format!("Study variant {variant} is not supported."));
        }
        let board = ChessApp::pgn_tag(source, "FEN")
            .map(|fen| Board::from_str(&fen))
            .transpose()
            .map_err(|_| "Study has an invalid starting position.")?
            .unwrap_or_default();
        let mut nodes = vec![StudyNode {
            parent: None,
            children: vec![],
            san: String::new(),
            fen: board.to_string(),
            comment: String::new(),
            nags: vec![],
            local_note: None,
            local_marks: None,
            local_nags: None,
            local: false,
        }];
        let mut current = 0;
        let mut stack = Vec::new();
        for token in tokens(source)? {
            match token {
                Token::Open => {
                    if stack.len() >= 64 {
                        return Err("Study variation nesting exceeds 64 levels.".into());
                    }
                    stack.push(current);
                    current = nodes[current]
                        .parent
                        .ok_or("A variation must follow a move.")?;
                }
                Token::Close => {
                    current = stack
                        .pop()
                        .ok_or("Study has an unmatched variation delimiter.")?;
                }
                Token::Comment(comment) => {
                    let mut prose = comment;
                    while let Some(start) = prose.find("[%note ") {
                        let Some(end) = prose[start..].find(']') else {
                            break;
                        };
                        if let Ok(note) =
                            serde_json::from_str::<String>(prose[start + 7..start + end].trim())
                        {
                            nodes[current].local_note = Some(note);
                            prose.replace_range(start..start + end + 1, "");
                        } else {
                            break;
                        }
                    }
                    if !prose.trim().is_empty() {
                        if !nodes[current].comment.is_empty() {
                            nodes[current].comment.push_str("\n\n");
                        }
                        nodes[current].comment.push_str(prose.trim());
                    }
                }
                Token::Move(raw) => {
                    let token = raw
                        .rsplit_once('.')
                        .map(|(_, suffix)| suffix)
                        .unwrap_or(&raw);
                    if token.is_empty() || ["*", "1-0", "0-1", "1/2-1/2"].contains(&token) {
                        continue;
                    }
                    if let Some(nag) = token.strip_prefix('$') {
                        let nag = nag
                            .parse::<u8>()
                            .map_err(|_| "Study has an invalid annotation glyph.")?;
                        nodes[current].nags.push(nag);
                        continue;
                    }
                    let san = token.trim_end_matches(['!', '?']);
                    let suffix = &token[san.len()..];
                    let nag = match suffix {
                        "!" => Some(1),
                        "?" => Some(2),
                        "!!" => Some(3),
                        "??" => Some(4),
                        "!?" => Some(5),
                        "?!" => Some(6),
                        _ => None,
                    };
                    let before = Board::from_str(&nodes[current].fen)
                        .map_err(|_| "Invalid study position.")?;
                    let mv = ChessApp::parse_san_move(&before, san)
                        .ok_or_else(|| format!("Could not parse study move {san}."))?;
                    if nodes.len() >= 16000 {
                        return Err(
                            "A study chapter exceeds 16,000 moves including variations.".into()
                        );
                    }
                    let index = nodes.len();
                    nodes.push(StudyNode {
                        parent: Some(current),
                        children: vec![],
                        san: ChessApp::san_for_move(&before, mv),
                        fen: before.make_move_new(mv).to_string(),
                        comment: String::new(),
                        nags: nag.into_iter().collect(),
                        local_note: None,
                        local_marks: None,
                        local_nags: None,
                        local: false,
                    });
                    nodes[current].children.push(index);
                    current = index;
                }
            }
        }
        if !stack.is_empty() {
            return Err("Study has an unclosed variation.".into());
        }
        Ok(Self {
            source: source.into(),
            original_source: None,
            name: ChessApp::pgn_tag(source, "ChapterName")
                .or_else(|| ChessApp::pgn_tag(source, "Event"))
                .unwrap_or_else(|| "Chapter".into()),
            url: ChessApp::pgn_tag(source, "ChapterURL").unwrap_or_default(),
            nodes,
            retained: false,
            analysis_lines: Default::default(),
        })
    }

    pub fn path(&self, mut node: usize) -> Vec<usize> {
        let mut path = vec![node];
        while let Some(parent) = self.nodes[node].parent {
            path.push(parent);
            node = parent;
        }
        path.reverse();
        path
    }
    fn key(&self, node: usize) -> String {
        format!(
            "{}|{}",
            self.nodes[0].fen,
            self.path(node)
                .into_iter()
                .skip(1)
                .map(|id| self.nodes[id].san.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
    fn identity(&self) -> String {
        if self.url.is_empty() {
            self.name.clone()
        } else {
            self.url.clone()
        }
    }
    fn move_text(&self, node: usize) -> String {
        let starting = ChessApp::pgn_tag(&self.source, "FEN")
            .and_then(|fen| {
                fen.split_whitespace()
                    .nth(5)
                    .and_then(|n| n.parse::<usize>().ok())
            })
            .unwrap_or(1)
            .max(1);
        let black = self.nodes[0].fen.split_whitespace().nth(1) == Some("b");
        let ply = self.path(node).len().saturating_sub(2) + usize::from(black);
        format!(
            "{}{} {}",
            starting + ply / 2,
            if ply % 2 == 0 { "." } else { "..." },
            self.nodes[node].san
        )
    }
    pub fn path_pgn(&self, node: usize) -> String {
        let mut pgn = ChessApp::pgn_header_lines(&self.source).join("\n");
        pgn.push_str("\n\n");
        for i in self.path(node) {
            let n = &self.nodes[i];
            if i != 0 {
                pgn.push_str(&self.move_text(i));
                pgn.push(' ');
            }
            for nag in &n.nags {
                pgn.push_str(&format!("${nag} "));
            }
            if !n.comment.is_empty() {
                pgn.push_str(&format!("{{{}}} ", n.comment));
            }
        }
        pgn.push('*');
        pgn
    }
    pub fn export_pgn(&self) -> String {
        self.export_with_notes(true)
    }
    fn export_with_notes(&self, local: bool) -> String {
        enum Write {
            Node(usize),
            Children(usize),
            Open,
            Close,
        }
        let mut output = ChessApp::pgn_header_lines(&self.source).join("\n");
        output.push_str("\n\n");
        let mut tasks = vec![Write::Children(0), Write::Node(0)];
        while let Some(task) = tasks.pop() {
            match task {
                Write::Open => output.push_str("( "),
                Write::Close => output.push_str(") "),
                Write::Node(id) => {
                    let node = &self.nodes[id];
                    if id != 0 {
                        output.push_str(&self.move_text(id));
                        output.push(' ');
                    }
                    for nag in if local {
                        node.local_nags.as_ref().unwrap_or(&node.nags)
                    } else {
                        &node.nags
                    } {
                        output.push_str(&format!("${nag} "));
                    }
                    let mut comment = node.comment.clone();
                    if local && node.local_marks.is_some() {
                        for name in ["cal", "csl", "iw_ghosts", "iw_arrow_styles"] {
                            let marker = format!("[%{name} ");
                            while let Some(start) = comment.find(&marker) {
                                let Some(end) = comment[start..].find(']') else {
                                    break;
                                };
                                comment.replace_range(start..start + end + 1, "");
                            }
                        }
                    }
                    if !comment.trim().is_empty() {
                        output.push_str(&format!("{{{comment}}} "));
                    }
                    if local && let Some(marks) = &node.local_marks {
                        output.push_str(&ChessApp::pgn_board_marks(Some(marks)));
                    }
                    if local && let Some(note) = &node.local_note {
                        output.push_str(&ChessApp::pgn_note(note));
                    }
                }
                Write::Children(parent) => {
                    if let Some(main) = self.nodes[parent].children.first() {
                        tasks.push(Write::Children(*main));
                        for other in self.nodes[parent].children.iter().skip(1).rev() {
                            tasks.push(Write::Close);
                            tasks.push(Write::Children(*other));
                            tasks.push(Write::Node(*other));
                            tasks.push(Write::Open);
                        }
                        tasks.push(Write::Node(*main));
                    }
                }
            }
        }
        output.push_str(&ChessApp::pgn_tag(&self.source, "Result").unwrap_or_else(|| "*".into()));
        output
    }
    fn reading_rows(&self) -> Vec<(usize, usize)> {
        enum Task {
            Children(usize, usize),
            Node(usize, usize),
        }
        let mut tasks = vec![Task::Children(0, 0)];
        let mut rows = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Node(id, depth) => rows.push((id, depth)),
                Task::Children(parent, depth) => {
                    if let Some(main) = self.nodes[parent].children.first() {
                        tasks.push(Task::Children(*main, depth));
                        for other in self.nodes[parent].children.iter().skip(1).rev() {
                            tasks.push(Task::Children(*other, depth + 1));
                            tasks.push(Task::Node(*other, depth + 1));
                        }
                        tasks.push(Task::Node(*main, depth));
                    }
                }
            }
        }
        rows
    }
    fn author_note(&self, node: usize) -> String {
        let pgn = self.path_pgn(node);
        ChessApp::parse_pgn_mainline(&pgn)
            .ok()
            .and_then(|(positions, _)| ChessApp::parse_pgn_notes(&pgn, &positions).last().cloned())
            .unwrap_or_default()
    }
}

impl StudyWorkspace {
    pub fn parse(text: &str) -> Result<Self, String> {
        let games = ChessApp::split_pgn_games(text)?;
        if games.is_empty() || games.len() > 256 {
            return Err("Import between 1 and 256 study chapters at a time.".into());
        }
        let name = ChessApp::pgn_tag(&games[0], "StudyName").unwrap_or_else(|| "My study".into());
        if games
            .iter()
            .any(|pgn| ChessApp::pgn_tag(pgn, "StudyName").as_deref() != Some(name.as_str()))
        {
            return Err("Import one study at a time.".into());
        }
        let chapters = games
            .iter()
            .map(|pgn| StudyChapter::parse(pgn))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            name,
            chapters,
            chapter: 0,
            node: 0,
            line_end: 0,
            flipped: None,
            visited: Default::default(),
            practice: false,
            practice_black: ChessApp::pgn_tag(&games[0], "Orientation").as_deref() == Some("black"),
            practice_side_chosen: false,
            attempts: 0,
            correct: 0,
            message: String::new(),
            refreshing: false,
            focus_panel: true,
        })
    }
    pub fn validated(mut self) -> Result<Self, String> {
        if self.chapters.is_empty() || self.chapters.len() > 512 {
            return Err("Invalid saved study chapter count.".into());
        }
        let mut bytes = 0;
        for chapter in &mut self.chapters {
            bytes += chapter.source.len();
            if bytes > 64 * 1024 * 1024 {
                return Err("Saved study exceeds 64 MiB.".into());
            }
            let mut canonical = StudyChapter::parse(&chapter.source)?;
            let valid_topology = chapter.nodes.iter().enumerate().all(|(i, n)| {
                if i == 0 {
                    n.parent.is_none()
                } else {
                    n.parent.is_some_and(|parent| parent < i)
                }
            });
            if valid_topology {
                let overrides = chapter
                    .nodes
                    .iter()
                    .enumerate()
                    .map(|(i, n)| (chapter.key(i), n))
                    .collect::<std::collections::BTreeMap<_, _>>();
                for i in 0..canonical.nodes.len() {
                    let key = canonical.key(i);
                    let node = &mut canonical.nodes[i];
                    if let Some(old) = overrides.get(&key) {
                        node.local = old.local;
                        node.local_marks =
                            old.local_marks.clone().filter(|marks| marks.len() <= 1024);
                        node.local_nags = old.local_nags.clone().filter(|nags| nags.len() <= 256);
                        node.local_note = old
                            .local_note
                            .clone()
                            .filter(|note| note.len() <= 256 * 1024)
                            .or_else(|| node.local_note.take());
                    }
                }
            }
            canonical.original_source = chapter
                .original_source
                .clone()
                .filter(|text| text.len() <= 1024 * 1024);
            canonical.retained = chapter.retained;
            canonical.analysis_lines = chapter
                .analysis_lines
                .iter()
                .take(64)
                .filter(|(_, value)| value.to_string().len() <= 2 * 1024 * 1024)
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            *chapter = canonical;
        }
        self.chapter = self.chapter.min(self.chapters.len() - 1);
        self.node = self.node.min(self.chapters[self.chapter].nodes.len() - 1);
        let ch = &self.chapters[self.chapter];
        if self.line_end >= ch.nodes.len() || !ch.path(self.line_end).contains(&self.node) {
            self.line_end = self.node;
        }
        if !self.practice_side_chosen {
            self.practice_black =
                ChessApp::pgn_tag(&ch.source, "Orientation").as_deref() == Some("black");
        }
        Ok(self)
    }
    pub fn export_pgn(&self) -> String {
        self.chapters
            .iter()
            .map(StudyChapter::export_pgn)
            .collect::<Vec<_>>()
            .join("\n\n")
    }
    fn remember_notes(&mut self, notes: &[String]) {
        let chapter = &mut self.chapters[self.chapter];
        let path = chapter.path(self.line_end);
        let pgn = chapter.path_pgn(self.line_end);
        let original = ChessApp::parse_pgn_mainline(&pgn)
            .ok()
            .map(|(positions, _)| ChessApp::parse_pgn_notes(&pgn, &positions))
            .unwrap_or_default();
        for (ply, id) in path.into_iter().enumerate() {
            if let Some(note) = notes.get(ply) {
                let expected = chapter.nodes[id]
                    .local_note
                    .as_ref()
                    .or_else(|| original.get(ply))
                    .map(String::as_str)
                    .unwrap_or("");
                if note != expected {
                    chapter.nodes[id].local_note = Some(note.clone());
                }
            }
        }
    }
    fn remember_annotations(&mut self, marks: &[Vec<BoardMark>], nags: &[Vec<u8>]) {
        let ch = &mut self.chapters[self.chapter];
        let pgn = ch.path_pgn(self.line_end);
        if let Ok((positions, _)) = ChessApp::parse_pgn_mainline(&pgn) {
            let original_marks = ChessApp::parse_pgn_board_marks(&pgn, &positions);
            let original_nags = ChessApp::parse_pgn_nags(&pgn, &positions);
            for (ply, id) in ch.path(self.line_end).into_iter().enumerate() {
                if let Some(value) = marks.get(ply) {
                    if Some(value)
                        != ch.nodes[id]
                            .local_marks
                            .as_ref()
                            .or_else(|| original_marks.get(ply))
                    {
                        ch.nodes[id].local_marks = Some(value.clone());
                    }
                }
                if let Some(value) = nags.get(ply) {
                    if Some(value)
                        != ch.nodes[id]
                            .local_nags
                            .as_ref()
                            .or_else(|| original_nags.get(ply))
                    {
                        ch.nodes[id].local_nags = Some(value.clone());
                    }
                }
            }
        }
    }
    pub fn merge(&mut self, text: &str) -> Result<(), String> {
        let mut incoming = Self::parse(text)?;
        let old_chapter = self.chapters[self.chapter].identity();
        let old_key = self.chapters[self.chapter].key(self.node);
        for old in &self.chapters {
            if old.retained {
                incoming.chapters.push(old.clone());
                continue;
            }
            if let Some(new) = incoming
                .chapters
                .iter_mut()
                .find(|ch| ch.identity() == old.identity())
            {
                new.analysis_lines = old.analysis_lines.clone();
                let notes = old
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| {
                        n.local_note.is_some()
                            || n.local_marks.is_some()
                            || n.local_nags.is_some()
                            || n.local
                    })
                    .map(|(i, n)| {
                        (
                            old.key(i),
                            (
                                n.local_note.clone(),
                                n.local_marks.clone(),
                                n.local_nags.clone(),
                                n.local,
                            ),
                        )
                    })
                    .collect::<std::collections::BTreeMap<_, _>>();
                let mut matched = std::collections::BTreeSet::new();
                for i in 0..new.nodes.len() {
                    let key = new.key(i);
                    if let Some(note) = notes.get(&key) {
                        new.nodes[i].local_note = note.0.clone();
                        new.nodes[i].local_marks = note.1.clone();
                        new.nodes[i].local_nags = note.2.clone();
                        new.nodes[i].local = note.3;
                        matched.insert(key);
                    }
                }
                if notes.keys().any(|key| !matched.contains(key))
                    || old.nodes.iter().enumerate().any(|(id, n)| {
                        n.local
                            && !new
                                .nodes
                                .iter()
                                .enumerate()
                                .any(|(j, _)| new.key(j) == old.key(id))
                    })
                {
                    let mut retained = old.clone();
                    retained.retained = true;
                    incoming.chapters.push(retained);
                }
            } else {
                let mut retained = old.clone();
                retained.retained = true;
                incoming.chapters.push(retained);
            }
        }
        let mut unique = std::collections::BTreeSet::new();
        incoming
            .chapters
            .retain(|ch| unique.insert(serde_json::to_string(ch).unwrap_or_default()));
        incoming.chapter = incoming
            .chapters
            .iter()
            .position(|ch| ch.identity() == old_chapter)
            .unwrap_or(0);
        incoming.node = (0..incoming.chapters[incoming.chapter].nodes.len())
            .find(|i| incoming.chapters[incoming.chapter].key(*i) == old_key)
            .unwrap_or(0);
        incoming.visited = self.visited.clone();
        incoming.flipped = self.flipped;
        incoming.practice_black = self.practice_black;
        incoming.practice_side_chosen = self.practice_side_chosen;
        incoming.line_end = incoming.node;
        incoming.message="Study refreshed. Your notes were kept; removed material with notes is retained as a local chapter.".into();
        *self = incoming;
        Ok(())
    }
}

impl ChessApp {
    fn study_reading_visuals(ui: &mut egui::Ui) {
        let visuals = ui.visuals_mut();
        // Stroke widths participate in egui's widget sizing. A hover must only
        // change the color, otherwise wrapped move runs reflow under the pointer.
        for widget in [&mut visuals.widgets.inactive, &mut visuals.widgets.hovered,
            &mut visuals.widgets.active, &mut visuals.widgets.open, &mut visuals.widgets.noninteractive] {
            widget.bg_stroke.width = 1.0;
            widget.expansion = 0.0;
        }
        visuals.selection.stroke.width = 1.0;
        visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        visuals.widgets.inactive.bg_stroke.color = Color32::TRANSPARENT;
        visuals.override_text_color = Some(Color32::from_rgb(190, 194, 190));
        visuals.selection.bg_fill = Color32::from_rgb(106, 80, 38);
    }

    // Fold historical chapter saves into a single workspace. Callers keep the
    // original records alongside the consolidated save for lossless backups.
    pub fn merge_saved_studies(json: &str) -> Result<String, String> {
        let saves: Vec<serde_json::Value> = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if saves.is_empty() || saves.len() > 1024 { return Err("Invalid study save count.".into()); }
        let mut merged: Option<StudyWorkspace> = None;
        let mut latest = serde_json::Value::Null;
        let mut identity: Option<String> = None;
        for mut saved in saves {
            let mut study = if saved["study"].is_object() {
                serde_json::from_value::<StudyWorkspace>(saved["study"].clone())
                    .map_err(|e| e.to_string())?.validated()?
            } else {
                let source = saved["review_pgn"].as_str().ok_or("Missing study PGN.")?;
                let mut study = StudyWorkspace::parse(source)?;
                study.line_end = study.chapters[0].nodes.len() - 1;
                // Legacy records contain only a main line in their saved timeline.
                while let Some(next) = study.chapters[0].nodes[study.node].children.first().copied() {
                    study.node = next;
                }
                study.line_end = study.node;
                let path=study.chapters[0].path(study.line_end);
                study.node=path[saved["review_index"].as_u64().unwrap_or(0).min((path.len()-1) as u64) as usize];
                if let Ok(notes) = serde_json::from_value::<Vec<String>>(saved["move_notes"].clone()) {
                    study.remember_notes(&notes);
                }
                let marks: Vec<Vec<BoardMark>> = serde_json::from_value(saved["board_marks"].clone()).unwrap_or_default();
                let nags: Vec<Vec<u8>> = serde_json::from_value(saved["move_annotations"].clone()).unwrap_or_default();
                study.remember_annotations(&marks, &nags);
                if saved["game_analysis"].as_array().is_some_and(|v| v.iter().any(|a| !a.is_null())) || saved["analysis_run"].is_object() {
                    let ch = &mut study.chapters[0];
                    let positions = ch.path(study.line_end).iter().map(|i| ch.nodes[*i].fen.clone()).collect::<Vec<_>>();
                    ch.analysis_lines.insert(ch.key(study.line_end), serde_json::json!({
                        "analysis_run":saved["analysis_run"],"game_analysis":saved["game_analysis"],
                        "position_fens":positions,"paused":true,
                        "verification_targets":saved["verification_targets"],"verification_results":saved["verification_results"],
                        "verification_nodes":saved["verification_nodes"],"verification_time_ms":saved["verification_time_ms"]
                    }));
                }
                study
            };
            for chapter in &study.chapters {
                let id = chapter.url.strip_prefix("https://lichess.org/study/")
                    .and_then(|p| p.split('/').next()).filter(|id| id.len()==8 && id.chars().all(|c|c.is_ascii_alphanumeric()))
                    .ok_or("Study has no unambiguous Lichess identity.")?.to_owned();
                if identity.as_ref().is_some_and(|old| old != &id) { return Err("Cannot combine different studies.".into()); }
                identity = Some(id);
            }
            if let Some(old) = merged.take() {
                let selected = study.chapters[study.chapter].identity();
                let node_key = study.chapters[study.chapter].key(study.node);
                let end_key = study.chapters[study.chapter].key(study.line_end);
                let mut chapters = old.chapters;
                for incoming in &study.chapters {
                    if let Some(index) = chapters.iter().position(|ch| ch.identity()==incoming.identity() && ch.retained==incoming.retained) {
                        let mut combined = incoming.clone();
                        let mut unmatched = false;
                        for (i, n) in chapters[index].nodes.iter().enumerate() {
                            if n.local_note.is_none() && n.local_marks.is_none() && n.local_nags.is_none() && !n.local { continue; }
                            let key = chapters[index].key(i);
                            if let Some(j) = (0..combined.nodes.len()).find(|j| combined.key(*j)==key) {
                                let target = &mut combined.nodes[j];
                                if target.local_note.is_none() { target.local_note=n.local_note.clone(); }
                                if target.local_marks.is_none() { target.local_marks=n.local_marks.clone(); }
                                if target.local_nags.is_none() { target.local_nags=n.local_nags.clone(); }
                            } else { unmatched=true; }
                        }
                        let mut analysis=chapters[index].analysis_lines.clone();
                        analysis.extend(combined.analysis_lines.clone());combined.analysis_lines=analysis;
                        if unmatched {
                            let mut retained=chapters[index].clone();retained.retained=true;
                            if !chapters.iter().any(|ch| ch.export_pgn()==retained.export_pgn() && ch.retained) { chapters.push(retained); }
                        }
                        chapters[index]=combined;
                    } else { chapters.push(incoming.clone()); }
                }
                study.chapters=chapters;
                study.chapter=study.chapters.iter().position(|ch|ch.identity()==selected).unwrap_or(0);
                let ch=&study.chapters[study.chapter];
                study.node=(0..ch.nodes.len()).find(|i|ch.key(*i)==node_key).unwrap_or(0);
                study.line_end=(0..ch.nodes.len()).find(|i|ch.key(*i)==end_key).unwrap_or(study.node);
                study.visited.extend(old.visited);
            }
            if study.chapters.len()>512 { return Err("Combined study exceeds 512 chapters.".into()); }
            saved["study"]=serde_json::to_value(&study).map_err(|e|e.to_string())?;
            latest=saved;merged=Some(study);
        }
        serde_json::to_string(&latest).map_err(|e|e.to_string())
    }

    pub(super) fn open_study(&mut self, text: &str) -> Result<(), String> {
        self.open_study_with_storage(text, true)
    }
    fn open_study_with_storage(&mut self, text: &str, new_record: bool) -> Result<(), String> {
        let study = StudyWorkspace::parse(text)?;
        self.invalidate_external_3d_context();
        self.pgn_input.clear();
        self.review_positions.clear();
        self.move_notes.clear();
        self.board_marks.clear();
        self.move_annotations.clear();
        self.study = Some(study);
        #[cfg(target_arch = "wasm32")]
        if new_record {
            start_new_stored_game("imported");
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = new_record;
        self.study_select(0, 0);
        self.pgn_dialog_open = false;
        self.batch_pgn_dialog_open = false;
        self.import_input.clear();
        Ok(())
    }
    pub(super) fn study_snapshot(&self) -> Option<StudyWorkspace> {
        let mut study = self.study.clone()?;
        let ch = &study.chapters[study.chapter];
        if study.line_end < ch.nodes.len()
            && ch.path(study.line_end).len() == self.review_positions.len()
            && ch.path_pgn(study.line_end) == self.pgn_input
        {
            study.remember_notes(&self.move_notes);
            study.remember_annotations(&self.board_marks, &self.move_annotations);
            study.flipped = Some(self.flipped);
            if self.game_analysis.iter().any(Option::is_some) || self.analysis_run.is_some() {
                let ch = &mut study.chapters[study.chapter];
                let key = ch.key(study.line_end);
                let value = serde_json::json!({"analysis_run":self.analysis_run,"game_analysis":self.game_analysis,"verification_targets":self.verification_targets,"verification_results":self.verification_results,"verification_nodes":self.verification_nodes,"verification_time_ms":self.verification_time_ms,"position_fens":self.review_positions.iter().map(ToString::to_string).collect::<Vec<_>>(),"paused":self.game_analysis_running||self.game_analysis_paused});
                if ch.analysis_lines.len() >= 64 && !ch.analysis_lines.contains_key(&key) {
                    if let Some(first) = ch.analysis_lines.keys().next().cloned() {
                        ch.analysis_lines.remove(&first);
                    }
                }
                ch.analysis_lines.insert(key, value);
            }
        }
        Some(study)
    }
    pub(super) fn study_select(&mut self, chapter: usize, mut node: usize) {
        let Some(mut study) = self.study_snapshot() else {
            return;
        };
        if chapter >= study.chapters.len() || node >= study.chapters[chapter].nodes.len() {
            return;
        }
        let same_chapter = chapter == study.chapter;
        let ch = &study.chapters[chapter];
        // Enter practice positions on the chosen side for every navigation route.
        if study.practice
            && (Board::from_str(&ch.nodes[node].fen)
                .ok()
                .is_some_and(|board| board.side_to_move() == Color::Black)
                != study.practice_black)
        {
            if let Some(reply) = ch.nodes[node].children.first() {
                node = *reply;
            }
        }
        let mut end = if same_chapter
            && study.line_end < ch.nodes.len()
            && ch.path(study.line_end).contains(&node)
        {
            study.line_end
        } else {
            node
        };
        while let Some(next) = ch.nodes[end].children.first() {
            end = *next;
        }
        study.chapter = chapter;
        study.node = node;
        study.line_end = end;
        study
            .visited
            .insert(format!("{}:{}", ch.identity(), ch.key(node)));
        let pgn = ch.path_pgn(end);
        let path = ch.path(end);
        let index = path.iter().position(|id| *id == node).unwrap();
        let same_line = self.pgn_input == pgn && self.review_positions.len() == path.len();
        if !same_line {
            let previous_positions = self.review_positions.clone();
            let previous_analysis = self.game_analysis.clone();
            #[cfg(target_arch = "wasm32")]
            {
                self.ignore_next_bestmove = self.engine_searching;
            }
            self.study = None;
            self.load_pgn_with_storage(&pgn, false);
            if same_chapter {
                for (i, (old, new)) in previous_positions
                    .iter()
                    .zip(&self.review_positions)
                    .enumerate()
                {
                    if old != new {
                        break;
                    }
                    if let Some(value) = previous_analysis.get(i).cloned().flatten() {
                        self.game_analysis[i] = Some(value);
                    }
                }
            }
            if let Some(value) = ch.analysis_lines.get(&ch.key(end)) {
                let fens = self
                    .review_positions
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                if value["position_fens"] == serde_json::json!(fens) {
                    if let Ok(snapshot) =
                        serde_json::from_value::<ArchiveAnalysisSnapshot>(value.clone())
                    {
                        self.restore_archive_analysis_snapshot(snapshot);
                        self.game_analysis_paused = value["paused"] == true;
                    }
                }
            }
        } else if !self.game_analysis_running {
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(engine) = &self.engine {
                    engine.command("stop");
                }
                self.ignore_next_bestmove = self.engine_searching;
            }
            self.engine_searching = false;
            self.analysis_running = false;
            self.clear_analysis_result();
        }
        let mut notes = Self::parse_pgn_notes(&pgn, &self.review_positions);
        let mut marks = Self::parse_pgn_board_marks(&pgn, &self.review_positions);
        let mut nags = Self::parse_pgn_nags(&pgn, &self.review_positions);
        for (ply, id) in path.iter().enumerate() {
            let n = &ch.nodes[*id];
            if let Some(note) = &n.local_note {
                notes[ply] = note.clone();
            }
            if let Some(value) = &n.local_marks {
                marks[ply] = value.clone();
            }
            if let Some(value) = &n.local_nags {
                nags[ply] = value.clone();
            }
        }
        self.move_notes = notes;
        self.board_marks = marks;
        self.move_annotations = nags;
        self.review_index = Some(index);
        self.review_scroll_to_selected = true;
        self.board = self.review_positions[index];
        self.last_move = Self::review_move_at(&self.review_positions, &self.review_moves, index);
        self.engine_enabled = false;
        self.flipped = study.flipped.unwrap_or_else(|| {
            Self::pgn_tag(&ch.source, "Orientation").as_deref() == Some("black")
        });
        study.flipped = Some(self.flipped);
        self.study = Some(study);
        self.selected = None;
        self.legal_targets.clear();
        self.piece_drag = None;
        self.capture_animation = None;
        self.save_game();
    }
    pub(super) fn study_play(&mut self, mv: ChessMove) {
        let Some(study) = self.study.as_ref() else {
            return;
        };
        if study.practice && (self.board.side_to_move() == Color::Black) != study.practice_black {
            self.study_practice_reply();
            return;
        }
        let ch = &study.chapters[study.chapter];
        let next = ch.nodes[study.node]
            .children
            .iter()
            .find(|id| Self::parse_san_move(&self.board, &ch.nodes[**id].san) == Some(mv))
            .copied();
        let chapter = study.chapter;
        if let Some(next) = next {
            let explanation = ch.author_note(next);
            let study = self.study.as_mut().unwrap();
            if study.practice {
                study.attempts += 1;
                study.correct += 1;
                study.message = if explanation.is_empty() {
                    "Correct. Follow the next position.".into()
                } else {
                    format!("Correct. {explanation}")
                };
            }
            self.study_select(chapter, next);
            self.study_practice_reply();
        } else if !study.practice && MoveGen::new_legal(&self.board).any(|legal| legal == mv) {
            let san = Self::san_for_move(&self.board, mv);
            let fen = self.board.make_move_new(mv).to_string();
            let mut study = self.study_snapshot().unwrap();
            let parent = study.node;
            let ch = &mut study.chapters[chapter];
            if ch.nodes.len() >= 16000 {
                self.study.as_mut().unwrap().message =
                    "This chapter has reached the move limit.".into();
                return;
            }
            let id = ch.nodes.len();
            ch.nodes.push(StudyNode {
                parent: Some(parent),
                children: vec![],
                san,
                fen,
                comment: String::new(),
                nags: vec![],
                local_note: None,
                local_marks: None,
                local_nags: None,
                local: true,
            });
            ch.nodes[parent].children.push(id);
            if ch.original_source.is_none() {
                ch.original_source = Some(ch.source.clone());
            }
            ch.source = ch.export_with_notes(false);
            study.message = "Local variation added. It is included in study exports.".into();
            self.study = Some(study);
            self.study_select(chapter, id);
        } else if let Some(study) = self.study.as_mut() {
            if study.practice {
                study.attempts += 1;
                study.message = "Try again. That move is not in this study.".into();
            } else {
                study.message =
                    "That move is not in the chapter. Choose a continuation below.".into();
            }
            self.selected = None;
            self.legal_targets.clear();
        }
    }
    fn study_practice_reply(&mut self) {
        let Some(study) = self.study.as_ref() else {
            return;
        };
        if !study.practice {
            return;
        }
        let ch = &study.chapters[study.chapter];
        if ch.nodes[study.node].children.is_empty() {
            self.study.as_mut().unwrap().message =
                "Line complete. Choose another chapter or restart practice.".into();
            return;
        }
        let black = self.board.side_to_move() == Color::Black;
        if black != study.practice_black {
            let (chapter, node) = (study.chapter, ch.nodes[study.node].children[0]);
            self.study_select(chapter, node);
        }
    }
    fn study_text_ui(ui: &mut egui::Ui, text: &str) {
        for paragraph in text.split("\n\n").filter(|part| !part.trim().is_empty()) {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for word in paragraph.split_whitespace() {
                    let url = word.trim_end_matches(['.', ',', ';', ')', ']']);
                    if (url.starts_with("https://") || url.starts_with("http://"))
                        && url.split("://").nth(1).is_some_and(|host| !host.is_empty())
                    {
                        ui.add(egui::Hyperlink::from_label_and_url(url, url).open_in_new_tab(true));
                        if url.len() < word.len() {
                            ui.label(&word[url.len()..]);
                        }
                    } else {
                        ui.label(word);
                    }
                }
            });
            ui.add_space(4.0);
        }
    }
    pub(super) fn study_panel_ui(&mut self, ui: &mut egui::Ui) {
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(16.0));
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(15.0));
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Heading, FontId::proportional(23.0));
        ui.spacing_mut().button_padding = Vec2::new(9.0, 6.0);
        ui.visuals_mut().override_text_color = Some(Color32::from_rgb(232, 229, 214));
        ui.visuals_mut().selection.bg_fill = Color32::from_rgb(115, 87, 43);
        ui.visuals_mut().hyperlink_color = Color32::from_rgb(218, 181, 113);
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 7.0);
        ui.visuals_mut().widgets.inactive.bg_stroke =
            Stroke::new(1.0, Color32::from_rgb(108, 89, 53));
        if self.study.is_none() && Self::is_study_chapter(&self.pgn_input) {
            let text = self.pgn_input.clone();
            if let Err(error) = self.open_study_with_storage(&text, false) {
                ui.label(error);
                return;
            }
        }
        let chooser_id=egui::Id::new("study_choose_another");
        if self.study.is_some() && ui.button("Choose another study…").clicked() {
            self.save_game();
            ui.ctx().data_mut(|d|d.insert_temp(chooser_id,true));
            #[cfg(target_arch="wasm32")]
            refresh_study_picker();
        }
        if self.study.is_none() || ui.ctx().data(|d|d.get_temp::<bool>(chooser_id)).unwrap_or(false) {
            let ctx=ui.ctx().clone();
            let input_id=egui::Id::new("study_quick_url");
            let busy_id=egui::Id::new("study_quick_loading");
            let error_id=egui::Id::new("study_quick_error");
            let mut input=ctx.data(|d|d.get_temp::<String>(input_id)).unwrap_or_default();
            let mut busy=ctx.data(|d|d.get_temp::<bool>(busy_id)).unwrap_or(false);
            let mut error=ctx.data(|d|d.get_temp::<String>(error_id)).unwrap_or_default();
            #[cfg(target_arch="wasm32")]
            if busy {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
                if let Ok(reply)=serde_json::from_str::<serde_json::Value>(&poll_study_refresh()) {
                    busy=false;
                    if let Some(pgn)=reply["pgn"].as_str() {
                        match self.open_study(pgn) {
                            Err(message)=>error=message,
                            Ok(())=>{ctx.data_mut(|d|d.insert_temp(chooser_id,false));},
                        }
                    } else {error=reply["error"].as_str().unwrap_or("Could not import study.").into();}
                }
            }
            Self::stable_panel_button_style(ui);
            Self::grained_outer_section(ui, |ui| {
                let gold=Color32::from_rgb(215,185,121);
                ui.label(RichText::new("IMPORT A STUDY").size(11.0).color(gold));
                ui.heading("Learn from a Lichess study");
                ui.label("Paste a public study link to open its first chapter.");
                ui.add_space(6.0);
                let response=ui.add_enabled(!busy,egui::TextEdit::singleline(&mut input)
                    .hint_text("https://lichess.org/study/…").desired_width(ui.available_width()));
                let import=ui.add_enabled(!busy&&!input.trim().is_empty(),egui::Button::new(
                    RichText::new("Import study").color(Color32::from_rgb(30,26,20)))
                    .min_size(Vec2::new(140.0,38.0)).raised_gold()).clicked()
                    || (response.lost_focus()&&ui.input(|i|i.key_pressed(egui::Key::Enter))&&!busy);
                if import {
                    let parts:Vec<_>=input.trim().trim_end_matches('/').split('/').collect();
                    if parts.len()>=5&&parts.len()<=6&&parts[0]=="https:"&&parts[2]=="lichess.org"&&parts[3]=="study"
                        && parts[4].len()==8&&parts[4].bytes().all(|c|c.is_ascii_alphanumeric())
                        && (parts.len()==5||(parts[5].len()==8&&parts[5].bytes().all(|c|c.is_ascii_alphanumeric()))) {
                        #[cfg(target_arch="wasm32")]
                        {refresh_study(&format!("https://lichess.org/study/{}",parts[4]));busy=true;error.clear();}
                        #[cfg(not(target_arch="wasm32"))]
                        {error="URL imports are available in the browser version.".into();}
                    } else {error="Paste an https://lichess.org/study/ link.".into();}
                }
                if busy {ui.horizontal(|ui| {ui.spinner();ui.label("Importing study chapters…");});}
                if !error.is_empty(){ui.colored_label(Color32::LIGHT_RED,&error);}
                ui.small("The study must be public with downloads enabled. Notes and variations are kept.");
            });
            ctx.data_mut(|d|{d.insert_temp(input_id,input);d.insert_temp(busy_id,busy);d.insert_temp(error_id,error);});
            if self.study.is_some() && !ctx.data(|d|d.get_temp::<bool>(chooser_id)).unwrap_or(false){return;}
            ui.add_space(16.0);
            Self::grained_outer_section(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Saved studies").size(19.0).strong());
                    #[cfg(target_arch="wasm32")]
                    if ui.button("Refresh").clicked(){refresh_study_picker();}
                });
                ui.small("Choose where your studies are saved, then select one to open.");
                #[cfg(target_arch="wasm32")]
                {
                    let initialized=egui::Id::new("study_picker_initialized");
                    if !ctx.data(|d|d.get_temp::<bool>(initialized)).unwrap_or(false) {
                        refresh_study_picker();ctx.data_mut(|d|d.insert_temp(initialized,true));
                    }
                    if let Ok(catalog)=serde_json::from_str::<serde_json::Value>(&study_picker()) {
                        let selected=catalog["location"].as_str().unwrap_or("browser");
                        let locations=catalog["locations"].as_array();
                        let label=locations.and_then(|items|items.iter().find(|item|item["value"].as_str()==Some(selected)))
                            .and_then(|item|item["label"].as_str()).unwrap_or("Unavailable Companion");
                        ui.horizontal(|ui| {
                            ui.set_height(36.0);
                            ui.spacing_mut().interact_size.y = 36.0;
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.label("Storage location");
                            egui::ComboBox::from_id_salt("study_picker_location").selected_text(label).width(220.0).show_ui(ui,|ui| {
                                if let Some(locations)=locations {
                                    for location in locations {
                                        let value=location["value"].as_str().unwrap_or("browser");
                                        let name=location["label"].as_str().unwrap_or("Companion");
                                        let enabled=location["enabled"].as_bool().unwrap_or(false);
                                        if ui.add_enabled(enabled,egui::Button::new(if enabled {name.to_owned()} else {format!("{name} · offline")}).selected(value==selected)).clicked() {
                                            set_study_picker_location(value);ui.close();
                                        }
                                    }
                                }
                            });
                            if ui.button("Connections…").clicked(){manage_companion_connection();}
                            });
                        });
                        ui.add_space(6.0);
                        if catalog["loading"].as_bool()==Some(true){ui.spinner();ctx.request_repaint_after(std::time::Duration::from_millis(100));}
                        if let Some(message)=catalog["error"].as_str().filter(|s|!s.is_empty()){ui.colored_label(Color32::LIGHT_RED,message);}
                        if let Some(items)=catalog["items"].as_array() {
                            if items.is_empty()&&catalog["loading"].as_bool()!=Some(true){ui.label("No studies in this location. Choose another location or import a study above.");}
                            for item in items {
                                let name=item["name"].as_str().unwrap_or("Study");
                                let chapters=item["chapters"].as_u64().unwrap_or(1);
                                let label=format!("{name}\n{chapters} {} · Open study",if chapters==1 {"chapter"} else {"chapters"});
                                let response=ui.add_sized([ui.available_width(),68.0],egui::Button::new(
                                    RichText::new(label).color(Color32::from_rgb(225,218,197)))
                                    .fill(Color32::from_rgb(30,39,44)).stroke(Stroke::new(1.0,Color32::from_rgb(85,78,56)))
                                    .corner_radius(6.0));
                                let c=egui::pos2(response.rect.right()-18.0,response.rect.center().y);
                                let stroke=Stroke::new(1.5,Color32::from_rgb(215,185,121));
                                ui.painter().line_segment([c+Vec2::new(-3.0,-5.0),c+Vec2::new(2.0,0.0)],stroke);
                                ui.painter().line_segment([c+Vec2::new(2.0,0.0),c+Vec2::new(-3.0,5.0)],stroke);
                                if response.on_hover_text("Open this study in Study mode").clicked(){if let Some(id)=item["id"].as_str(){open_picked_study(id);}}
                            }
                        }
                    }
                }
                if ui.button("Browse full library…").clicked(){self.navigation_open_library();}
                if self.study.is_some() && ui.button("Return to current study").clicked(){ctx.data_mut(|d|d.insert_temp(chooser_id,false));}
            });
            return;
        }
        #[cfg(target_arch = "wasm32")]
        if self.study.as_ref().is_some_and(|s| s.refreshing) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
            let reply = poll_study_refresh();
            if !reply.is_empty() {
                if let Ok(reply) = serde_json::from_str::<serde_json::Value>(&reply) {
                    let mut study = self.study_snapshot().unwrap();
                    study.refreshing = false;
                    if let Some(text) = reply["pgn"].as_str() {
                        let expected = study.chapters[study.chapter]
                            .url
                            .split('/')
                            .take(5)
                            .collect::<Vec<_>>()
                            .join("/");
                        if reply["url"].as_str() != Some(expected.as_str()) {
                            study.message =
                                "Refresh belongs to a different study. Your local copy was kept."
                                    .into();
                        } else if let Err(error) = study.merge(text) {
                            study.message = error;
                        }
                    } else {
                        study.message = reply["error"]
                            .as_str()
                            .unwrap_or("Study refresh failed.")
                            .into();
                    }
                    let (chapter, node) = (study.chapter, study.node);
                    self.study = Some(study);
                    self.move_notes.clear();
                    self.board_marks.clear();
                    self.move_annotations.clear();
                    self.study_select(chapter, node);
                }
            }
        }
        let snapshot = self.study.as_ref().unwrap().clone();
        let gold = Color32::from_rgb(218, 181, 113);
        Frame::new()
            .fill(Color32::from_rgba_unmultiplied(24, 27, 29, 235))
            .stroke(Stroke::new(1.0, Color32::from_rgb(95, 79, 49)))
            .corner_radius(8)
            .inner_margin(14)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(
                    RichText::new("LICHESS STUDY")
                        .size(11.0)
                        .strong()
                        .color(gold),
                );
                ui.label(
                    RichText::new(&snapshot.name)
                        .size(25.0)
                        .strong()
                        .color(Color32::from_rgb(241, 230, 204)),
                );
                if let Some(author) =
                    Self::pgn_tag(&snapshot.chapters[snapshot.chapter].source, "Annotator")
                {
                    if let Some(name) =
                        author
                            .strip_prefix("https://lichess.org/@/")
                            .filter(|name| {
                                !name.is_empty()
                                    && name
                                        .chars()
                                        .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
                            })
                    {
                        ui.add(
                            egui::Hyperlink::from_label_and_url(
                                format!("By {name}"),
                                author.clone(),
                            )
                            .open_in_new_tab(true),
                        );
                    }
                }
                if let Some(opening) = snapshot.chapters[snapshot.chapter].opening_label() {
                    ui.label(RichText::new(opening).size(13.0)
                        .color(Color32::from_rgb(175, 175, 164)));
                }
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!(
                        "CHAPTER {} OF {}",
                        snapshot.chapter + 1,
                        snapshot.chapters.len()
                    ))
                    .size(11.0)
                    .strong()
                    .color(gold),
                );
                ui.horizontal(|ui| {
                    // Reserve the row height before placing any controls so that
                    // scopes, the selector, and buttons share the same center.
                    ui.set_height(36.0);
                    ui.spacing_mut().interact_size.y = 32.0;
                    if ui
                        .add_enabled(
                            snapshot.chapter > 0,
                            egui::Button::new("Previous")
                                .min_size(Vec2::new(70.0, 32.0))
                                .raised_gray(),
                        )
                        .on_hover_text("Previous chapter")
                        .clicked()
                    {
                        self.study_select(snapshot.chapter - 1, 0);
                    }
                    let width = (ui.available_width() - 100.0).max(80.0);
                    let selected = &snapshot.chapters[snapshot.chapter].name;
                    egui::ComboBox::from_id_salt("study_chapter")
                        .selected_text(RichText::new(selected).size(16.0).strong().color(gold))
                        .width(width)
                        .height(340.0)
                        .show_ui(ui, |ui| {
                            ui.visuals_mut().selection.bg_fill = Color32::from_rgb(106, 80, 38);
                            ui.visuals_mut().widgets.active.weak_bg_fill =
                                Color32::from_rgb(106, 80, 38);
                            ui.spacing_mut().button_padding = Vec2::new(12.0, 8.0);
                            ui.style_mut()
                                .text_styles
                                .insert(egui::TextStyle::Button, FontId::proportional(16.0));
                            for (i, ch) in snapshot.chapters.iter().enumerate() {
                                let read = snapshot
                                    .visited
                                    .iter()
                                    .any(|key| key.starts_with(&format!("{}:", ch.identity())));
                                let text = format!(
                                    "{:02}   {}{}{}",
                                    i + 1,
                                    ch.name,
                                    if ch.retained { " · local copy" } else { "" },
                                    if read && i != snapshot.chapter {
                                        " · viewed"
                                    } else {
                                        ""
                                    }
                                );
                                if ui
                                    .selectable_label(
                                        i == snapshot.chapter,
                                        RichText::new(text).size(16.0),
                                    )
                                    .clicked()
                                {
                                    self.study_select(i, 0);
                                }
                            }
                        })
                        .response
                        .on_hover_text(selected);
                    if ui
                        .add_enabled(
                            snapshot.chapter + 1 < snapshot.chapters.len(),
                            egui::Button::new("Next")
                                .min_size(Vec2::new(60.0, 32.0))
                                .raised_gray(),
                        )
                        .on_hover_text("Next chapter")
                        .clicked()
                    {
                        self.study_select(snapshot.chapter + 1, 0);
                    }
                });
                ui.add_space(8.0);
                ui.collapsing("Source and export", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.set_height(36.0);
                        let source = &snapshot.chapters[snapshot.chapter].url;
                        if source
                            .strip_prefix("https://lichess.org/study/")
                            .is_some_and(|path| {
                                let parts = path.split('/').collect::<Vec<_>>();
                                parts.len() == 2
                                    && parts.iter().all(|p| {
                                        p.len() == 8 && p.chars().all(|c| c.is_ascii_alphanumeric())
                                    })
                            })
                        {
                            ui.add(
                                egui::Hyperlink::from_label_and_url("View on Lichess", source)
                                    .open_in_new_tab(true),
                            );
                        }
                        if ui
                            .add(egui::Button::new("Copy study PGN").raised_gray())
                            .clicked()
                        {
                            if let Some(study) = self.study_snapshot() {
                                ui.ctx().copy_text(study.export_pgn());
                            }
                        }
                        #[cfg(target_arch = "wasm32")]
                        if ui
                            .add_enabled(
                                !snapshot.refreshing,
                                egui::Button::new(if snapshot.refreshing {
                                    "Refreshing…"
                                } else {
                                    "Refresh study"
                                })
                                .raised_gray(),
                            )
                            .clicked()
                        {
                            if let Some(id) = source
                                .strip_prefix("https://lichess.org/study/")
                                .and_then(|p| p.split('/').next())
                                .filter(|id| {
                                    id.len() == 8 && id.chars().all(|c| c.is_ascii_alphanumeric())
                                })
                            {
                                refresh_study(&format!("https://lichess.org/study/{id}"));
                                self.study.as_mut().unwrap().refreshing = true;
                            } else {
                                self.study.as_mut().unwrap().message =
                                    "This chapter has no Lichess source link.".into();
                            }
                        }
                    });
                });
            });
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            // Establish the row height before placing mixed button/selector widgets.
            ui.set_height(34.0);
            ui.spacing_mut().interact_size.y = 28.0;
            if Self::sidebar_tab_button(ui, "Read & explore", !snapshot.practice).clicked() {
                self.study.as_mut().unwrap().practice = false;
                self.study.as_mut().unwrap().message.clear();
            }
            if Self::sidebar_tab_button(ui, "Practice", snapshot.practice).clicked() {
                let s = self.study.as_mut().unwrap();
                s.practice = true;
                s.correct = 0;
                s.attempts = 0;
                s.message =
                    "Play the next move on the board. Any studied continuation is accepted.".into();
                self.study_select(snapshot.chapter, 0);
                self.study_practice_reply();
            }
            if snapshot.practice {
                let mut black = snapshot.practice_black;
                egui::ComboBox::from_id_salt("study_practice_side")
                    .selected_text(if black { "Play Black" } else { "Play White" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut black, false, "Play White");
                        ui.selectable_value(&mut black, true, "Play Black");
                    });
                if black != snapshot.practice_black {
                    self.study.as_mut().unwrap().practice_black = black;
                    self.study.as_mut().unwrap().practice_side_chosen = true;
                    self.study_select(snapshot.chapter, 0);
                    self.study_practice_reply();
                }
            }
        });
        if !snapshot.message.is_empty() {
            ui.label(&snapshot.message);
        }
        if snapshot.practice {
            ui.label(format!(
                "{} correct / {} attempts",
                snapshot.correct, snapshot.attempts
            ));
        }
        ui.add_space(10.0);
        let mut panel_grain = None;
        let lesson_panel = Frame::new()
            .fill(Color32::from_rgb(13, 18, 21))
            .stroke(Stroke::new(1.0, Color32::from_rgb(62, 61, 53)))
            .corner_radius(8)
            .inner_margin(14)
            .show(ui, |ui| {
                panel_grain = Some(ui.painter().add(egui::Shape::Noop));
                let study = self.study.as_ref().unwrap().clone();
                let ch = &study.chapters[study.chapter];
                let node = &ch.nodes[study.node];
                ui.label(
                    RichText::new(if study.node == 0 {
                        "Chapter introduction".to_string()
                    } else {
                        format!("Position after {}", node.san)
                    })
                    .size(13.0)
                    .strong()
                    .color(Color32::from_rgb(218, 181, 113)),
                );
                ui.add_space(3.0);
                if !study.practice || node.children.is_empty() {
                    let note = ch.author_note(study.node);
                    if !note.is_empty() {
                        Self::study_text_ui(ui, &note);
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            node.parent.is_some() && !study.practice,
                            egui::Button::new("Back").raised_gray(),
                        )
                        .clicked()
                    {
                        self.study_select(study.chapter, node.parent.unwrap());
                    }
                    if ui
                        .add(egui::Button::new("Chapter start").raised_gray())
                        .clicked()
                    {
                        self.study_select(study.chapter, 0);
                        self.study_practice_reply();
                    }
                    if ui
                        .add_enabled(
                            !study.practice,
                            egui::Button::new("Main line").raised_gray(),
                        )
                        .clicked()
                    {
                        let mut end = 0;
                        while let Some(next) = ch.nodes[end].children.first() {
                            end = *next;
                        }
                        self.study_select(study.chapter, end);
                    }
                    if study.practice
                        && !node.children.is_empty()
                        && ui.button("Reveal move").clicked()
                    {
                        let next = node.children[0];
                        self.study.as_mut().unwrap().message =
                            format!("Study continuation: {}", ch.nodes[next].san);
                    }
                });
                if !study.practice {
                    ui.add_space(7.0);
                    ui.label(
                        RichText::new("CONTINUATIONS")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(156, 155, 144)),
                    );
                    for (index, id) in node.children.iter().enumerate() {
                        if ui
                            .add(
                                egui::Button::new(format!(
                                    "{}{}",
                                    ch.move_text(*id),
                                    if index == 0 {
                                        " · main line"
                                    } else {
                                        " · variation"
                                    }
                                ))
                                .raised_gray(),
                            )
                            .clicked()
                        {
                            self.study_select(study.chapter, *id);
                        }
                    }
                    egui::CollapsingHeader::new("Moves and comments")
                        .default_open(true)
                        .show(ui, |ui| {
                            let rows = ch.reading_rows();
                            let mut cursor = 0;
                            let selection_id = egui::Id::new("study_reading_selection");
                            let reveal = ui
                                .ctx()
                                .data(|d| d.get_temp::<(usize, usize)>(selection_id))
                                != Some((study.chapter, study.node));
                            Self::study_reading_visuals(ui);
                            ui.spacing_mut().item_spacing = Vec2::new(8.0, 9.0);
                            egui::ScrollArea::vertical()
                                .id_salt(("study_reading_line", study.chapter))
                                .max_height(280.0)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    let start =
                                        ui.add(egui::Button::selectable(study.node == 0,
                                            RichText::new("Starting position").size(13.0)
                                                .color(Color32::from_rgb(163, 169, 164)))
                                                .frame_when_inactive(true));
                                    if reveal && study.node == 0 {
                                        start.scroll_to_me(Some(Align::Min));
                                    }
                                    if start.clicked() {
                                        self.study_select(study.chapter, 0);
                                    }
                                    while cursor < rows.len() {
                                        let depth = rows[cursor].1;
                                        let first = cursor;
                                        loop {
                                            let id = rows[cursor].0;
                                            cursor += 1;
                                            if cursor == rows.len()
                                                || rows[cursor].1 != depth
                                                || ch.nodes[rows[cursor].0].parent != Some(id)
                                                || !ch.author_note(id).is_empty()
                                            {
                                                break;
                                            }
                                        }
                                        ui.push_id(first, |ui| {
                                            let mut rect = ui.available_rect_before_wrap();
                                            rect.min.x += (depth as f32 * 18.0).min(90.0);
                                            ui.scope_builder(
                                                egui::UiBuilder::new().max_rect(rect),
                                                |ui| {
                                                    ui.horizontal_wrapped(|ui| {
                                                        for &(id, _) in &rows[first..cursor] {
                                                            let node = &ch.nodes[id];
                                                            let symbols = node
                                                                .local_nags
                                                                .as_ref()
                                                                .unwrap_or(&node.nags)
                                                                .iter()
                                                                .map(|nag| {
                                                                    crate::nags::symbol(*nag)
                                                                })
                                                                .collect::<Vec<_>>()
                                                                .join(" ");
                                                            let response = ui.add(egui::Button::selectable(
                                                                id == study.node,
                                                                RichText::new(format!(
                                                                    "{} {}",
                                                                    ch.move_text(id),
                                                                    symbols
                                                                )).size(16.0).strong().color(
                                                                    if id == study.node { Color32::from_rgb(255, 239, 204) }
                                                                    else { Color32::from_rgb(222, 188, 126) }),
                                                            ).frame_when_inactive(true));
                                                            if reveal && id == study.node {
                                                                response.scroll_to_me(Some(
                                                                    Align::Center,
                                                                ));
                                                            }
                                                            if response.clicked() {
                                                                self.study_select(
                                                                    study.chapter,
                                                                    id,
                                                                );
                                                            }
                                                        }
                                                    });
                                                    let comment =
                                                        ch.author_note(rows[cursor - 1].0);
                                                    if !comment.is_empty() {
                                                        ui.add_space(3.0);
                                                        Self::study_text_ui(ui, &comment);
                                                        ui.add_space(4.0);
                                                    }
                                                },
                                            );
                                        });
                                    }
                                });
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(selection_id, (study.chapter, study.node))
                            });
                        });
                }
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(
                    RichText::new("YOUR NOTE")
                        .size(11.0)
                        .strong()
                        .color(Color32::from_rgb(218, 181, 113)),
                );
                ui.label(
                    RichText::new("Personal observations for this position")
                        .size(13.0)
                        .color(Color32::from_rgb(157, 160, 157)),
                );
                let mut note = node.local_note.clone().unwrap_or_default();
                if ui
                    .add(
                        egui::TextEdit::multiline(&mut note)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3)
                            .margin(Vec2::new(12.0, 10.0))
                            .background_color(Color32::from_rgb(13, 16, 18))
                            .hint_text("Add a thought, a question, or a reminder…"),
                    )
                    .changed()
                {
                    let s = self.study.as_mut().unwrap();
                    s.chapters[s.chapter].nodes[s.node].local_note = Some(note.clone());
                    if let Some(target) = self.move_notes.get_mut(self.review_index.unwrap_or(0)) {
                        *target = note;
                    }
                    self.save_game();
                }
            });
        if let Some(shape) = panel_grain {
            let texture = Self::paper_grain_texture(ui.ctx());
            let surface = lesson_panel.response.rect.shrink(4.0);
            ui.painter().set(shape, egui::Shape::image(texture.id(), surface,
                egui::Rect::from_min_max(egui::Pos2::ZERO,
                    egui::pos2(surface.width()/256.0, surface.height()/256.0)),
                Color32::from_white_alpha(100)));
        }
        Self::paint_sunken_border(ui, lesson_panel.response.rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn chapter(name: &str, moves: &str) -> String {
        format!(
            "[Event \"Study: {name}\"]\n[StudyName \"Study\"]\n[ChapterName \"{name}\"]\n[ChapterURL \"https://lichess.org/study/abcd1234/{name}\"]\n[Result \"*\"]\n\n{moves} *"
        )
    }
    fn app() -> ChessApp {
        ChessApp::new(&eframe::CreationContext::_new_kittest(
            egui::Context::default(),
        ))
    }

    #[test]
    fn study_library_merges_legacy_chapters_notes_and_analysis_without_crossing_studies() {
        let first=chapter("FIRST001", "1.e4 {Author} e5");
        let second=chapter("SECOND02", "1.d4 d5");
        let mut one:serde_json::Value=serde_json::from_str(&ChessApp::prepare_archived_pgn(&first).unwrap()).unwrap();
        one["move_notes"]=serde_json::json!(["", "My observation", ""]);
        one["game_analysis"]=serde_json::json!([null,{"eval_cp":42},null]);
        let two:serde_json::Value=serde_json::from_str(&ChessApp::prepare_archived_pgn(&second).unwrap()).unwrap();
        let merged=ChessApp::merge_saved_studies(&serde_json::json!([one,two]).to_string()).unwrap();
        let value:serde_json::Value=serde_json::from_str(&merged).unwrap();
        let study:StudyWorkspace=serde_json::from_value(value["study"].clone()).unwrap();
        assert_eq!(study.chapters.len(),2);
        assert_eq!(study.chapters[0].nodes[1].local_note.as_deref(),Some("My observation"));
        assert_eq!(study.chapters[0].analysis_lines.len(),1);
        assert_eq!(study.chapter,1);
        let again=ChessApp::merge_saved_studies(&serde_json::json!([value.clone(),value]).to_string()).unwrap();
        assert_eq!(serde_json::from_str::<serde_json::Value>(&again).unwrap()["study"]["chapters"].as_array().unwrap().len(),2);
        let other=second.replace("abcd1234","other123");
        let other:serde_json::Value=serde_json::from_str(&ChessApp::prepare_archived_pgn(&other).unwrap()).unwrap();
        assert!(ChessApp::merge_saved_studies(&serde_json::json!([serde_json::from_str::<serde_json::Value>(&merged).unwrap(),other]).to_string()).is_err());
    }

    #[test]
    fn study_move_hover_keeps_geometry_and_wrapping_stable() {
        let ctx=egui::Context::default();
        let mut rects=Vec::new();
        let mut hovered=false;
        let mut draw=|pointer| {
            let mut input=egui::RawInput::default();
            input.events.push(egui::Event::PointerMoved(pointer));
            let mut rect=egui::Rect::NOTHING;
            let _=crate::app::run_test_ui(&ctx,input,|ui| {
                ChessApp::study_reading_visuals(ui);
                let response=ui.add(egui::Button::selectable(false,
                    RichText::new("8... Bxe3").size(16.0).strong())
                    .frame_when_inactive(true));
                rect=response.rect;hovered=response.hovered();
            });
            rect
        };
        let idle=draw(egui::pos2(1000.0,1000.0));
        for _ in 0..3 { rects.push(draw(idle.center())); }
        assert!(hovered);
        for rect in rects { assert_eq!(rect,idle,"Hover must not resize or move the move control"); }
    }

    #[test]
    fn study_header_opening_follows_chapter_tags_or_recognized_mainline() {
        let tagged = format!("[ECO \"C42\"]\n[Opening \"Petrov's Defense: Stafford Gambit\"]\n{}", chapter("Tagged", "1.e4"));
        assert_eq!(StudyChapter::parse(&tagged).unwrap().opening_label().as_deref(), Some("C42 · Petrov's Defense: Stafford Gambit"));
        let inferred = StudyChapter::parse(&chapter("Sicilian", "1.e4 c5")).unwrap();
        assert!(inferred.opening_label().unwrap().starts_with("B20 · Sicilian"));
        assert!(StudyChapter::parse(&chapter("Intro", "")).unwrap().opening_label().is_none());
        let position = format!("[FEN \"8/8/8/8/8/8/4K3/7k w - - 0 1\"]\n{}", chapter("Endgame", "1.Kf3"));
        assert!(StudyChapter::parse(&position).unwrap().opening_label().is_none());
    }

    #[test]
    fn study_tree_parses_nested_variations_and_restores_every_position() {
        let source = chapter(
            "One",
            "{Introduction} 1.e4! {King pawn} (1.d4 d5 (1...Nf6 {Indian}) 2.c4) e5 2.Nf3",
        );
        let ch = StudyChapter::parse(&source).unwrap();
        assert_eq!(ch.nodes[0].children.len(), 2);
        let e4 = ch.nodes[0].children[0];
        let d4 = ch.nodes[0].children[1];
        assert_eq!(ch.nodes[e4].nags, [1]);
        assert_eq!(ch.nodes[d4].children.len(), 2);
        assert_eq!(ch.nodes[ch.nodes[d4].children[1]].san, "Nf6");
        for id in 0..ch.nodes.len() {
            let (positions, _) = ChessApp::parse_pgn_mainline(&ch.path_pgn(id)).unwrap();
            assert_eq!(positions.last().unwrap().to_string(), ch.nodes[id].fen);
        }
        let exported = ch.export_pgn();
        let restored = StudyChapter::parse(&exported).unwrap();
        assert_eq!(restored.nodes.len(), ch.nodes.len());
        for node in &ch.nodes {
            assert!(restored.nodes.iter().any(|other| other.fen == node.fen
                && other.san == node.san
                && other.comment == node.comment));
        }
        for broken in ["1.e4 (1.d4", "1.e4 )", "(1.d4)", "1.e4 (1.e5)", "{Unclosed"] {
            assert!(
                StudyChapter::parse(&chapter("Bad", broken)).is_err(),
                "{broken}"
            );
        }
    }

    #[test]
    fn study_workspace_keeps_chapters_branch_notes_and_board_in_sync() {
        let source = format!(
            "{}\n\n{}",
            chapter("One", "1.e4 {Author} (1.d4 {Queen pawn}) e5"),
            chapter("Two", "{Introduction}")
        );
        let mut app = app();
        app.open_study(&source).unwrap();
        assert_eq!(app.study.as_ref().unwrap().chapters.len(), 2);
        assert_eq!(app.board, Board::default());
        app.study_select(0, 1);
        assert_eq!(
            app.move_notes.get(app.review_index.unwrap()).unwrap(),
            "Author"
        );
        *app.move_notes.get_mut(app.review_index.unwrap()).unwrap() = "My e4 note".into();
        app.study_select(0, 2);
        assert_eq!(
            app.move_notes.get(app.review_index.unwrap()).unwrap(),
            "Queen pawn"
        );
        app.study_select(1, 0);
        assert_eq!(app.move_notes, ["Introduction"]);
        app.study_select(0, 1);
        assert_eq!(
            app.move_notes.get(app.review_index.unwrap()).unwrap(),
            "My e4 note"
        );
        assert_eq!(
            app.board.to_string(),
            app.study.as_ref().unwrap().chapters[0].nodes[1].fen
        );
        app.review_to(0);
        assert_eq!(app.study.as_ref().unwrap().node, 0);
        assert_eq!(app.board, Board::default());
        let saved = app.persisted_game();
        let restored: PersistedGame =
            serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
        let restored = restored.study.unwrap().validated().unwrap();
        assert_eq!(
            restored.chapters[0].nodes[1].local_note.as_deref(),
            Some("My e4 note")
        );
        let exported =
            ChessApp::export_saved_game(&serde_json::to_string(&saved).unwrap(), "annotated")
                .unwrap();
        let exported = StudyWorkspace::parse(&exported).unwrap();
        assert_eq!(exported.chapters.len(), 2);
        assert_eq!(exported.chapters[0].nodes.len(), 4);
    }

    #[test]
    fn study_practice_accepts_branches_rejects_other_moves_and_replies() {
        let mut app = app();
        app.open_study(&chapter("One", "1.e4 e5 2.Nf3 (2.Bc4) Nc6"))
            .unwrap();
        app.study.as_mut().unwrap().practice = true;
        let before = app.board;
        app.play(ChessMove::from_str("d2d4").unwrap());
        assert_eq!(app.board, before);
        assert_eq!(app.study.as_ref().unwrap().attempts, 1);
        app.play(ChessMove::from_str("e2e4").unwrap());
        assert_eq!(&app.review_moves[..app.review_index.unwrap()], ["e4", "e5"]);
        app.play(ChessMove::from_str("f1c4").unwrap());
        assert_eq!(
            &app.review_moves[..app.review_index.unwrap()],
            ["e4", "e5", "Bc4"]
        );
        assert_eq!(app.study.as_ref().unwrap().correct, 2);
        assert!(
            app.study
                .as_ref()
                .unwrap()
                .message
                .contains("Line complete")
        );
        app.study.as_mut().unwrap().practice = false;
        app.study_select(0, 0);
        app.play(ChessMove::from_str("d2d4").unwrap());
        assert_eq!(app.review_moves, ["d4"]);
        assert!(
            app.study.as_ref().unwrap().chapters[0].nodes[app.study.as_ref().unwrap().node].local
        );
    }

    #[test]
    fn study_refresh_preserves_notes_on_removed_and_reordered_branches() {
        let mut study = StudyWorkspace::parse(&format!(
            "{}\n\n{}",
            chapter("One", "1.e4 (1.d4) e5"),
            chapter("Two", "1.c4")
        ))
        .unwrap();
        study.chapters[0].nodes[1].local_note = Some("Keep e4".into());
        study.chapters[0].nodes[2].local_note = Some("Keep d4".into());
        study
            .merge(&format!(
                "{}\n\n{}",
                chapter("Two", "1.c4 e5"),
                chapter("One", "1.e4 {Updated author comment} c5")
            ))
            .unwrap();
        assert_eq!(
            study.chapters[1].nodes[1].local_note.as_deref(),
            Some("Keep e4")
        );
        assert!(study.chapters.iter().any(|ch| {
            ch.retained
                && ch
                    .nodes
                    .iter()
                    .any(|n| n.local_note.as_deref() == Some("Keep d4"))
        }));
        let before = study.export_pgn();
        assert!(study.merge("bad data").is_err());
        assert_eq!(study.export_pgn(), before);
        // Saved topology is rebuilt from source, rather than trusting external indices.
        study.chapters[0].nodes[0].parent = Some(usize::MAX);
        assert!(study.validated().is_ok());
    }
    #[test]
    fn study_local_drawings_glyphs_and_variations_survive_export_and_reload() {
        let mut app = app();
        app.open_study(&chapter("One", "1.e4 {Author [%csl Ge4]} (1.d4) e5"))
            .unwrap();
        app.study_select(0, 1);
        app.board_marks[1].clear();
        app.move_annotations[1] = vec![3];
        app.study_select(0, 2);
        app.study_select(0, 1);
        assert!(app.board_marks[1].is_empty());
        assert_eq!(app.move_annotations[1], [3]);
        let exported = app.study_snapshot().unwrap().export_pgn();
        let ch = StudyChapter::parse(&exported).unwrap();
        let pgn = ch.path_pgn(1);
        let (positions, _) = ChessApp::parse_pgn_mainline(&pgn).unwrap();
        assert!(ChessApp::parse_pgn_board_marks(&pgn, &positions)[1].is_empty());
        assert_eq!(ChessApp::parse_pgn_nags(&pgn, &positions)[1], [3]);
        app.study_select(0, 0);
        app.play(ChessMove::from_str("c2c4").unwrap());
        *app.move_notes.get_mut(app.review_index.unwrap()).unwrap() = "My local line".into();
        let saved = app.persisted_game();
        let restored = saved.study.clone().unwrap().validated().unwrap();
        let node = restored.chapters[0]
            .nodes
            .iter()
            .find(|n| n.san == "c4")
            .unwrap();
        assert_eq!(node.local_note.as_deref(), Some("My local line"));
        assert!(node.local);
        let json =
            ChessApp::export_saved_game(&serde_json::to_string(&saved).unwrap(), "json").unwrap();
        let mut copy = super::tests::app();
        assert!(copy.load_analysis_json(&json));
        assert_eq!(copy.study.as_ref().unwrap().chapters.len(), 1);
        assert_eq!(copy.review_moves, ["c4"]);
    }
    #[test]
    fn study_black_start_keeps_numbering_in_branch_exports() {
        let source = chapter("Black", "17...e5 (17...c5) 18.Nf3").replace(
            "[Result",
            "[FEN \"rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 17\"]\n[Result",
        );
        let ch = StudyChapter::parse(&source).unwrap();
        assert!(ch.export_pgn().contains("17... e5"));
        assert!(ch.export_pgn().contains("17... c5"));
        assert_eq!(
            StudyChapter::parse(&ch.export_pgn()).unwrap().nodes.len(),
            ch.nodes.len()
        );
    }
    #[test]
    fn study_refresh_keeps_old_position_notes_when_the_starting_board_changes() {
        let mut study = StudyWorkspace::parse(&chapter("One", "1.e4")).unwrap();
        study.chapters[0].nodes[0].local_note = Some("Original start".into());
        let changed = chapter("One", "1.e4").replace(
            "[Result",
            "[FEN \"rnbqkbnr/ppp1pppp/8/3p4/3P4/8/PPP1PPPP/RNBQKBNR w KQkq - 0 2\"]\n[Result",
        );
        // A changed move tree must retain notes on positions which no longer exist.
        study.merge(&changed).unwrap();
        assert!(study.chapters[0].nodes[0].local_note.is_none());
        assert!(
            study.chapters.iter().any(
                |ch| ch.retained && ch.nodes[0].local_note.as_deref() == Some("Original start")
            )
        );
    }
    #[test]
    fn study_repeated_refresh_does_not_restore_stale_notes_from_retained_copies() {
        let original = chapter("One", "1.e4 (1.d4) e5");
        let updated = chapter("One", "1.e4 c5");
        let mut study = StudyWorkspace::parse(&original).unwrap();
        study.chapters[0].nodes[1].local_note = Some("Older main note".into());
        study.chapters[0].nodes[2].local_note = Some("Retain this branch".into());
        study.merge(&updated).unwrap();
        study.chapters[0].nodes[1].local_note = Some("Newer main note".into());
        let count = study.chapters.len();
        study.merge(&updated).unwrap();
        assert_eq!(study.chapters.len(), count);
        assert_eq!(
            study.chapters[0].nodes[1].local_note.as_deref(),
            Some("Newer main note")
        );
        assert!(study.chapters.iter().any(|ch| {
            ch.retained
                && ch
                    .nodes
                    .iter()
                    .any(|n| n.local_note.as_deref() == Some("Retain this branch"))
        }));
    }
    #[test]
    fn study_multiline_directives_and_personal_notes_round_trip() {
        let source = chapter(
            "One",
            r#"1.e4 {Author explanation.} {[%csl Ge4][%cal Ge2e4]} {[%note "My own observation"]} e5 {Second explanation.
[%cal Ge7e5]
More prose.}"#,
        );
        let study = StudyWorkspace::parse(&source).unwrap();
        let exported = study.export_pgn();
        let restored = StudyWorkspace::parse(&exported).unwrap();
        let ch = &restored.chapters[0];
        assert_eq!(
            ch.nodes[1].local_note.as_deref(),
            Some("My own observation")
        );
        assert_eq!(ch.author_note(1), "Author explanation.");
        let mut app = app();
        app.open_study(&exported).unwrap();
        app.study_select(0, 1);
        assert_eq!(app.board_marks[1].len(), 2);
        assert_eq!(app.move_notes[1], "My own observation");
        assert!(ch.path_pgn(1).matches("[ChapterName").count() == 1);
        // Valid directives starting on their own line belong to comments, not headers.
        let multiline = chapter(
            "One",
            "1.e4 {Author explanation.\n\n[%csl Ge4][%cal Ge2e4]} e5 *",
        );
        let ch = StudyChapter::parse(&multiline).unwrap();
        assert_eq!(ch.author_note(1), "Author explanation.");
        let (positions, _) = ChessApp::parse_pgn_mainline(&ch.path_pgn(2)).unwrap();
        assert_eq!(
            ChessApp::parse_pgn_board_marks(&ch.path_pgn(2), &positions)[1].len(),
            2
        );
    }

    #[test]
    fn study_black_practice_and_manual_orientation_survive_chapter_navigation() {
        let source = format!(
            "{}\n\n{}",
            chapter("One", "1.e4 e5 2.Nf3 Nc6"),
            chapter("Two", "1.d4 d5 2.c4 e6")
        )
        .replace("[Result", "[Orientation \"black\"]\n[Result");
        let mut app = app();
        app.open_study(&source).unwrap();
        assert!(app.study.as_ref().unwrap().practice_black);
        assert!(app.flipped);
        app.flipped = false;
        app.study_select(0, 1);
        assert!(!app.flipped);
        app.study_select(1, 0);
        assert!(!app.flipped);
        app.study.as_mut().unwrap().practice = true;
        app.study_select(0, 0);
        assert_eq!(app.board.side_to_move(), Color::Black);
        assert_eq!(app.review_index, Some(1));
        app.study_select(1, 0);
        assert_eq!(app.board.side_to_move(), Color::Black);
        assert_eq!(app.review_index, Some(1));
        app.play(ChessMove::from_str("d7d5").unwrap());
        assert_eq!(app.board.side_to_move(), Color::Black);
        assert_eq!(app.review_index, Some(3));
        assert_eq!(app.study.as_ref().unwrap().correct, 1);
    }

    #[test]
    fn study_analysis_remains_active_within_a_line_and_restores_between_branches() {
        let mut app = app();
        app.open_study(&chapter("One", "1.e4 e5 (1...c5) 2.Nf3 Nc6"))
            .unwrap();
        let end = app.study.as_ref().unwrap().line_end;
        let positions = app.review_positions.clone();
        app.game_analysis = positions
            .iter()
            .map(|_| {
                Some(PositionAnalysis {
                    eval_cp: Some(42),
                    ..Default::default()
                })
            })
            .collect();
        app.analysis_run = Some(
            serde_json::json!({"id":"study-run","position_fens":positions.iter().map(ToString::to_string).collect::<Vec<_>>()}),
        );
        app.game_analysis_running = true;
        app.study_select(0, 1);
        assert!(app.game_analysis_running);
        assert_eq!(app.game_analysis.len(), positions.len());
        assert_eq!(app.analysis_run.as_ref().unwrap()["id"], "study-run");
        app.study_select(0, end);
        assert!(app.game_analysis_running);
        app.game_analysis_running = false;
        let c5 = app.study.as_ref().unwrap().chapters[0]
            .nodes
            .iter()
            .position(|n| n.san == "c5")
            .unwrap();
        app.study_select(0, c5);
        app.study_select(0, end);
        assert_eq!(app.analysis_run.as_ref().unwrap()["id"], "study-run");
        assert!(
            app.game_analysis
                .iter()
                .all(|v| v.as_ref().unwrap().eval_cp == Some(42))
        );
        app.review_to(1);
        assert_eq!(app.review_positions, positions);
        assert_eq!(app.review_index, Some(1));
    }

    #[test]
    #[ignore = "Uses an explicitly downloaded, unbundled public study for comparison"]
    fn study_stafford_reference_full_fidelity() {
        let path = std::env::var("IRONWOOD_STUDY_COMPARISON_PGN")
            .expect("Set the ignored reference PGN path");
        let source = std::fs::read_to_string(path).unwrap();
        let reference = StudyWorkspace::parse(&source).unwrap();
        assert_eq!(reference.name, "Stafford Gambit Traps");
        assert_eq!(reference.chapters.len(), 10);
        let exported = reference.export_pgn();
        assert_eq!(exported.matches("[%cal ").count(), 19);
        assert_eq!(exported.matches("[%csl ").count(), 7);
        let restored = StudyWorkspace::parse(&exported).unwrap();
        assert_eq!(restored.chapters.len(), 10);
        let mut app = app();
        app.open_study(&source).unwrap();
        for (chapter, ch) in reference.chapters.iter().enumerate() {
            let other = &restored.chapters[chapter];
            assert_eq!(ch.nodes.len(), other.nodes.len());
            for (id, n) in ch.nodes.iter().enumerate() {
                let key = ch.key(id);
                let other_node = other
                    .nodes
                    .iter()
                    .enumerate()
                    .find(|(j, _)| other.key(*j) == key)
                    .unwrap()
                    .1;
                assert_eq!(n.fen, other_node.fen);
                assert_eq!(n.comment, other_node.comment);
                app.study_select(chapter, id);
                let ply = app.review_index.unwrap();
                assert_eq!(app.board.to_string(), n.fen);
                let mut expected_marks = 0;
                for marker in ["[%cal ", "[%csl "] {
                    for directive in n.comment.split(marker).skip(1) {
                        expected_marks += directive
                            .split(']')
                            .next()
                            .unwrap()
                            .split(',')
                            .filter(|item| !item.trim().is_empty())
                            .count();
                    }
                }
                assert_eq!(
                    app.board_marks[ply].len(),
                    expected_marks,
                    "{} / {}",
                    ch.name,
                    ch.move_text(id)
                );
                let mut prose = n.comment.clone();
                while let Some(start) = prose.find("[%") {
                    let end = prose[start..].find(']').unwrap();
                    prose.replace_range(start..start + end + 1, "");
                }
                assert_eq!(
                    app.move_notes[ply].split_whitespace().collect::<Vec<_>>(),
                    prose.split_whitespace().collect::<Vec<_>>(),
                    "{} / {}",
                    ch.name,
                    id
                );
            }
        }
    }
    #[test]
    fn study_future_checkmate_is_not_shown_as_a_current_loss() {
        let mut app = app();
        app.open_study(&chapter("One", "1.f3 e5 2.g4 Qh4#"))
            .unwrap();
        assert_eq!(app.game_result(), "*");
        app.study_select(0, 1);
        assert_eq!(app.game_result(), "*");
        let end = app.study.as_ref().unwrap().line_end;
        app.study_select(0, end);
        assert_eq!(app.game_result(), "0-1");
    }
}
