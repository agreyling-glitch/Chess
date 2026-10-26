//! Parse and validate PGN while retaining tags, comments, NAGs and variations.
use crate::archive::{Document, Result, invalid};
use pgn_reader::{Nag, Outcome, RawComment, RawTag, Reader, SanPlus, Skip, Visitor};
use shakmaty::{CastlingMode, Chess, Position, fen::Fen};
use std::{collections::BTreeMap, ops::ControlFlow};

pub struct Parsed {
    pub moves: usize,
    pub sans: Vec<String>,
    pub final_fen: String,
    pub tags: BTreeMap<String, String>,
    pub pgn: String,
}
struct BoundaryCounter;
impl Visitor for BoundaryCounter {
    type Tags = ();
    type Movetext = ();
    type Output = ();
    fn begin_tags(&mut self) -> ControlFlow<(), ()> {
        ControlFlow::Continue(())
    }
    fn begin_movetext(&mut self, _: ()) -> ControlFlow<(), ()> {
        ControlFlow::Continue(())
    }
    fn end_game(&mut self, _: ()) {}
}
// Count boundaries without validating moves or constructing game documents.
pub(crate) fn count_games(
    reader: impl std::io::Read,
    check: &mut impl FnMut() -> std::result::Result<(), String>,
) -> std::result::Result<u64, String> {
    let mut reader = Reader::new(reader);
    let mut total = 0;
    loop {
        check()?;
        if reader
            .read_game(&mut BoundaryCounter)
            .map_err(|_| "Could not count PGN games")?
            .is_none()
        {
            break;
        }
        total += 1;
    }
    Ok(total)
}
struct Parser;
pub(crate) struct ImportReader(Reader<Box<dyn std::io::Read + Send>>);
impl ImportReader {
    pub(crate) fn skip_games(&mut self, count: u64) -> Result<u64> {
        let mut skipped = 0;
        while skipped < count {
            if self
                .0
                .read_game(&mut BoundaryCounter)
                .map_err(|_| invalid("Malformed PGN checkpoint"))?
                .is_none()
            {
                break;
            }
            skipped += 1;
        }
        Ok(skipped)
    }
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        Self::from_reader(std::io::Cursor::new(bytes))
    }
    pub(crate) fn from_reader(reader: impl std::io::Read + Send + 'static) -> Self {
        Self(Reader::new(Box::new(reader)))
    }
    pub(crate) fn next(&mut self) -> Result<Option<Result<Parsed>>> {
        self.0
            .read_game(&mut Parser)
            .map_err(|_| invalid("Malformed PGN stream"))
    }
}
pub struct Text {
    tags: BTreeMap<String, String>,
    tokens: Vec<String>,
    position: Chess,
    before: Chess,
    stack: Vec<(Chess, Chess)>,
    ended: bool,
    plies: usize,
    mainline_plies: usize,
    sans: Vec<String>,
}
impl Visitor for Parser {
    type Tags = BTreeMap<String, String>;
    type Movetext = Text;
    type Output = Result<Parsed>;
    fn begin_tags(&mut self) -> ControlFlow<Self::Output, Self::Tags> {
        ControlFlow::Continue(BTreeMap::new())
    }
    fn tag(
        &mut self,
        tags: &mut Self::Tags,
        name: &[u8],
        value: RawTag<'_>,
    ) -> ControlFlow<Self::Output> {
        let Ok(name) = std::str::from_utf8(name) else {
            return ControlFlow::Break(Err(invalid("Invalid PGN tag encoding")));
        };
        let Ok(value) = value.decode_utf8() else {
            return ControlFlow::Break(Err(invalid("Invalid PGN tag encoding")));
        };
        if tags.len() >= 64 || name.len() > 128 || value.len() > 4096 {
            return ControlFlow::Break(Err(invalid("PGN tags exceed supported limits")));
        }
        tags.insert(name.to_owned(), value.into_owned());
        ControlFlow::Continue(())
    }
    fn begin_movetext(&mut self, tags: Self::Tags) -> ControlFlow<Self::Output, Self::Movetext> {
        let variant = tags
            .get("Variant")
            .map(String::as_str)
            .unwrap_or("Standard");
        let mode = match variant {
            "Standard" | "Chess" | "From Position" => CastlingMode::Standard,
            "Chess960" | "Fischerandom" | "Fischer Random" => CastlingMode::Chess960,
            _ => return ControlFlow::Break(Err(invalid("Unsupported PGN variant"))),
        };
        let position = if let Some(fen) = tags.get("FEN") {
            if fen.split_whitespace().count() != 6 {
                return ControlFlow::Break(Err(invalid("PGN FEN requires all six fields")));
            }
            match fen
                .parse::<Fen>()
                .ok()
                .and_then(|fen| fen.into_position::<Chess>(mode).ok())
            {
                Some(p) => p,
                None => return ControlFlow::Break(Err(invalid("Invalid PGN starting position"))),
            }
        } else {
            Chess::default()
        };
        ControlFlow::Continue(Text {
            tags,
            tokens: vec![],
            before: position.clone(),
            position,
            stack: vec![],
            ended: false,
            plies: 0,
            mainline_plies: 0,
            sans: Vec::new(),
        })
    }
    fn san(&mut self, text: &mut Text, san: SanPlus) -> ControlFlow<Self::Output> {
        if text.ended || text.plies >= 10000 {
            return ControlFlow::Break(Err(invalid("Invalid PGN termination or move limit")));
        }
        let Ok(m) = san.san.to_move(&text.position) else {
            return ControlFlow::Break(Err(invalid("Illegal PGN move")));
        };
        text.before = text.position.clone();
        text.tokens.push(format!(
            "{}{}",
            text.position.fullmoves(),
            if text.position.turn().is_white() {
                "."
            } else {
                "..."
            }
        ));
        text.tokens.push(san.to_string());
        text.position.play_unchecked(m);
        text.plies += 1;
        if text.stack.is_empty() {
            text.mainline_plies += 1;
            text.sans.push(san.to_string());
        }
        ControlFlow::Continue(())
    }
    fn nag(&mut self, text: &mut Text, nag: Nag) -> ControlFlow<Self::Output> {
        text.tokens.push(nag.to_string());
        ControlFlow::Continue(())
    }
    fn comment(&mut self, text: &mut Text, comment: RawComment<'_>) -> ControlFlow<Self::Output> {
        let Ok(comment) = std::str::from_utf8(comment.as_bytes()) else {
            return ControlFlow::Break(Err(invalid("Invalid PGN comment encoding")));
        };
        text.tokens.push(format!("{{{comment}}}"));
        ControlFlow::Continue(())
    }
    fn begin_variation(&mut self, text: &mut Text) -> ControlFlow<Self::Output, Skip> {
        if text.stack.len() >= 32 {
            return ControlFlow::Break(Err(invalid("PGN variations exceed nesting limit")));
        }
        text.stack
            .push((text.position.clone(), text.before.clone()));
        text.position = text.before.clone();
        text.tokens.push("(".into());
        ControlFlow::Continue(Skip(false))
    }
    fn end_variation(&mut self, text: &mut Text) -> ControlFlow<Self::Output> {
        let Some((position, before)) = text.stack.pop() else {
            return ControlFlow::Break(Err(invalid("Unmatched PGN variation")));
        };
        text.position = position;
        text.before = before;
        text.tokens.push(")".into());
        ControlFlow::Continue(())
    }
    fn outcome(&mut self, text: &mut Text, outcome: Outcome) -> ControlFlow<Self::Output> {
        if !text.stack.is_empty() {
            return ControlFlow::Break(Err(invalid("Unclosed PGN variation")));
        }
        let result = outcome.to_string();
        if text.tags.get("Result").is_some_and(|v| v != &result) {
            return ControlFlow::Break(Err(invalid("PGN result tag and termination disagree")));
        }
        text.tags.insert("Result".into(), result.clone());
        text.tokens.push(result);
        text.ended = true;
        ControlFlow::Continue(())
    }
    fn end_game(&mut self, text: Text) -> Self::Output {
        if !text.ended || !text.stack.is_empty() {
            return Err(invalid(
                "PGN requires a termination marker and balanced variations",
            ));
        }
        let tags = text.tags;
        let headers = tags
            .iter()
            .map(|(name, value)| {
                format!(
                    "[{name} \"{}\"]",
                    value.replace('\\', "\\\\").replace('"', "\\\"")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok(Parsed {
            moves: text.mainline_plies,
            sans: text.sans,
            final_fen: Fen::from_position(&text.position, shakmaty::EnPassantMode::Legal)
                .to_string(),
            tags,
            pgn: format!("{headers}\n\n{}\n", text.tokens.join(" ")),
        })
    }
}
#[cfg(test)]
thread_local! { pub(crate) static PARSE_COUNT:std::cell::Cell<usize>=const {std::cell::Cell::new(0)}; }
pub fn parse(input: &str) -> Result<Vec<Parsed>> {
    #[cfg(test)]
    PARSE_COUNT.with(|count| count.set(count.get() + 1));
    if input.len() > crate::archive::MAX_BODY {
        return Err(invalid("PGN exceeds import limit"));
    }
    let mut reader = Reader::new(input.as_bytes());
    let mut games = Vec::new();
    while let Some(game) = reader
        .read_game(&mut Parser)
        .map_err(|_| invalid("Malformed PGN"))?
    {
        games.push(game?);
        if games.len() > crate::archive::MAX_BATCH {
            return Err(invalid("Import exceeds 1000 games"));
        }
    }
    if games.is_empty() {
        return Err(invalid("No PGN games found"));
    }
    Ok(games)
}
pub fn documents(input: &str) -> Result<Vec<Document>> {
    Ok(parse(input)?
        .into_iter()
        .map(|game| Document {
            analysis_runs: Vec::new(),
            format: "ironwood-companion-game".into(),
            version: 1,
            record: None,
            pgn: game.pgn,
            metadata: game.tags,
        })
        .collect())
}
