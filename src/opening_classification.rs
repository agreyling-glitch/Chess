//! Offline opening classification using the CC0 Lichess opening dataset.
use shakmaty::{Chess, Position, EnPassantMode, fen::Fen, san::San};
use std::{collections::HashMap, sync::OnceLock};
fn key(position: &Chess) -> String {
    Fen::from_position(position, EnPassantMode::Legal).to_string().split_whitespace().take(4).collect::<Vec<_>>().join(" ")
}
fn catalog() -> &'static HashMap<String, (String,String)> {
    static CATALOG: OnceLock<HashMap<String,(String,String)>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut result=HashMap::new();
        for data in [include_str!("../assets/openings/a.tsv"),include_str!("../assets/openings/b.tsv"),include_str!("../assets/openings/c.tsv"),include_str!("../assets/openings/d.tsv"),include_str!("../assets/openings/e.tsv")] {
            for line in data.lines().skip(1) {
                let mut fields=line.split('\t');
                let (Some(eco),Some(name),Some(pgn))=(fields.next(),fields.next(),fields.next()) else {continue;};
                let mut position=Chess::default();
                let mut valid=true;
                for token in pgn.split_whitespace().filter(|token|!token.ends_with('.')) {
                    let Ok(san)=token.parse::<San>() else {valid=false;break;};
                    let Ok(mv)=san.to_move(&position) else {valid=false;break;};
                    position.play_unchecked(mv);
                }
                if valid {result.entry(key(&position)).or_insert_with(||(eco.into(),name.into()));}
            }
        }
        result
    })
}
pub fn classify(moves: &[String]) -> Option<(String,String)> {
    let mut position=Chess::default();
    let mut found=None;
    for token in moves.iter().take(80) {
        let Ok(san)=token.trim_end_matches(['+','#','!','?']).parse::<San>() else {break;};
        let Ok(mv)=san.to_move(&position) else {break;};
        position.play_unchecked(mv);
        if let Some(opening)=catalog().get(&key(&position)) {found=Some(opening.clone());}
    }
    found
}
#[cfg(test)]
mod tests {
    use super::*;
    fn opening(line:&str)->(String,String){classify(&line.split_whitespace().map(str::to_owned).collect::<Vec<_>>()).unwrap()}
    #[test] fn recognizes_families_and_transpositions(){
        assert_eq!(opening("e4 c5").0,"B20");
        assert!(opening("e4 e5 Nf3 Nc6 Bb5").1.contains("Ruy Lopez"));
        assert_eq!(opening("e4 e6 d4 d5"),opening("d4 e6 e4 d5"));
        assert!(classify(&[]).is_none());
    }
}
