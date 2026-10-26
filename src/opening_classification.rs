//! Offline opening classification using the CC0 Lichess opening dataset.
use shakmaty::{Chess, Position, EnPassantMode, fen::Fen, san::San};
use std::{collections::HashMap, sync::OnceLock};
pub fn eco_descriptions()-> &'static std::collections::BTreeMap<String,String>{
    static DESCRIPTIONS:OnceLock<std::collections::BTreeMap<String,String>>=OnceLock::new();
    DESCRIPTIONS.get_or_init(||{
        let mut entries=std::collections::BTreeMap::<String,(usize,String)>::new();
        for data in [include_str!("../assets/openings/a.tsv"),include_str!("../assets/openings/b.tsv"),include_str!("../assets/openings/c.tsv"),include_str!("../assets/openings/d.tsv"),include_str!("../assets/openings/e.tsv")]{for line in data.lines().skip(1){let fields:Vec<_>=line.split('\t').collect();if fields.len()<3{continue;}let length=fields[2].split_whitespace().filter(|t|!t.ends_with('.')).count();let candidate=(length,fields[1].to_owned());let entry=entries.entry(fields[0].into()).or_insert(candidate.clone());if (candidate.0,candidate.1.len())<(entry.0,entry.1.len()){*entry=candidate;}}}
        entries.into_iter().map(|(code,(_,name))|(code,name)).collect()
    })
}
pub fn names_by_eco() -> &'static std::collections::BTreeMap<String, Vec<String>> {
    static NAMES: OnceLock<std::collections::BTreeMap<String, Vec<String>>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut names=std::collections::BTreeMap::<String,Vec<String>>::new();
        for data in [include_str!("../assets/openings/a.tsv"),include_str!("../assets/openings/b.tsv"),include_str!("../assets/openings/c.tsv"),include_str!("../assets/openings/d.tsv"),include_str!("../assets/openings/e.tsv")] {
            for line in data.lines().skip(1) {
                let mut fields=line.split('\t');
                if let (Some(eco),Some(name))=(fields.next(),fields.next()) {let entries=names.entry(eco.into()).or_default();if !entries.iter().any(|v|v==name){entries.push(name.into());}}
            }
        }
        names
    })
}
/// Preserve source ECO codes; never attach a fallback name from another code.
pub fn consistent_labels(eco:&str, name:&str, classified:Option<(String,String)>)->(String,String) {
    let missing=|s:&str|matches!(s.trim(),""|"?"|"-");
    let code=eco.trim().to_uppercase();
    if missing(&code) {
        return match classified {Some((code,label))=>consistent_labels(&code,name,Some((code.clone(),label))),None=>(String::new(),if missing(name){String::new()}else{name.into()})};
    }
    let Some(names)=names_by_eco().get(&code) else{return (code,name.into());};
    let mismatched= !names.iter().any(|n|n==name) && names_by_eco().values().any(|list|list.iter().any(|n|n==name));
    let label=if missing(name)||mismatched {
        classified.filter(|(c,_)|c==&code).map(|(_,n)|n).unwrap_or_else(||names[0].clone())
    }else{name.into()};
    (code,label)
}
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
    #[test] fn source_codes_and_fallback_names_stay_consistent(){
        for (code,names) in names_by_eco(){
            let fallback=Some(("D06".into(),"Queen's Gambit".into()));
            let (eco,name)=consistent_labels(code,"",fallback);
            assert_eq!(&eco,code);assert!(names.contains(&name));
        }
        assert!(consistent_labels("A85","Queen's Gambit",None).1.starts_with("Dutch Defense"));
        assert_eq!(consistent_labels("A85","Custom source label",None).1,"Custom source label");
    }
    #[test] fn recognizes_families_and_transpositions(){
        assert_eq!(opening("e4 c5").0,"B20");
        assert!(opening("e4 e5 Nf3 Nc6 Bb5").1.contains("Ruy Lopez"));
        assert_eq!(opening("e4 e6 d4 d5"),opening("d4 e6 e4 d5"));
        assert!(classify(&[]).is_none());
    }
}
