//! Versioned, lossless PGN move packets using shakmaty's two-byte UCI encoding.
//! Literal packets retain spacing, tags and annotations; move packets retain SAN
//! spelling only when it differs from the SAN reconstructed from the position.
use crate::archive::{Document, Result, invalid};
use shakmaty::{Chess, Position, CastlingMode, fen::Fen, packed::PackedUciMove, san::SanPlus, uci::UciMove};
use rusqlite::{Connection, OptionalExtension, params};
use std::io::{Read, Write};

const MAGIC:&[u8]=b"IWPG\x01";
const LIMIT:usize=8*1024*1024;
pub fn strip_opening(pgn:&str)->String{
    let mut out=String::new();let mut rest=pgn;
    loop {
        let offset=rest.len()-rest.trim_start().len();out.push_str(&rest[..offset]);rest=&rest[offset..];
        if !rest.starts_with('['){out.push_str(rest);return out;}
        let mut quoted=false;let mut escaped=false;let mut end=None;
        for (i,c) in rest.char_indices().skip(1){if escaped{escaped=false;continue;}if c=='\\'&&quoted{escaped=true;continue;}if c=='"'{quoted=!quoted;}if c==']'&&!quoted{end=Some(i+1);break;}}
        let Some(end)=end else{out.push_str(rest);return out;};let header=&rest[..end];
        let remove=header[1..].trim_start().strip_prefix("Opening").is_some_and(|tail|tail.starts_with(char::is_whitespace));
        if !remove{out.push_str(header);}rest=&rest[end..];
        if remove{rest=rest.strip_prefix("\r\n").or_else(||rest.strip_prefix('\n')).unwrap_or(rest);}
    }
}
fn clean_record(value:&mut serde_json::Value){
    if let Some(o)=value.as_object_mut(){o.remove("Opening");o.remove("opening");for (key,v) in o.iter_mut(){if key=="review_pgn" {if let Some(s)=v.as_str(){*v=strip_opening(s).into();}}else if key=="json"{if let Some(s)=v.as_str(){if let Ok(mut nested)=serde_json::from_str::<serde_json::Value>(s){let original=nested.clone();clean_record(&mut nested);if original!=nested{*v=nested.to_string().into();}}}}else{clean_record(v);}}}else if let Some(a)=value.as_array_mut(){for v in a{clean_record(v);}}
}
pub fn normalize(doc:&mut Document,eco:&str){doc.metadata.remove("Opening");if !eco.is_empty(){doc.metadata.insert("ECO".into(),eco.into());}doc.pgn=strip_opening(&doc.pgn);if let Some(record)=&mut doc.record{clean_record(record);}}
pub fn initialize(db:&Connection)->Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS game_payloads(game_id TEXT PRIMARY KEY REFERENCES games(id) ON DELETE CASCADE,moves BLOB NOT NULL,record BLOB);
        CREATE TABLE IF NOT EXISTS eco_openings(eco TEXT PRIMARY KEY,description TEXT NOT NULL);")?;
    for (eco,name) in crate::opening_classification::eco_descriptions(){db.execute("INSERT OR IGNORE INTO eco_openings VALUES(?1,?2)",params![eco,name])?;}
    Ok(())
}
fn put(out:&mut Vec<u8>,bytes:&[u8]){out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());out.extend_from_slice(bytes);}
fn take<'a>(bytes:&mut &'a [u8],n:usize)->Result<&'a [u8]>{if n>bytes.len(){return Err(invalid("Truncated binary game"));}let (a,b)=bytes.split_at(n);*bytes=b;Ok(a)}
fn field<'a>(bytes:&mut &'a [u8])->Result<&'a [u8]>{let n=u32::from_le_bytes(take(bytes,4)?.try_into().unwrap()) as usize;if n>LIMIT{return Err(invalid("Binary game exceeds limit"));}take(bytes,n)}
fn text(bytes:&[u8])->Result<&str>{std::str::from_utf8(bytes).map_err(|_|invalid("Invalid binary game text"))}
fn initial(fen:&str,mode:CastlingMode)->Result<Chess>{if fen.is_empty(){Ok(Chess::default())}else{fen.parse::<Fen>().ok().and_then(|f|f.into_position(mode).ok()).ok_or(invalid("Invalid binary starting position"))}}
pub fn encode(pgn:&str,tags:&std::collections::BTreeMap<String,String>)->Result<Vec<u8>> {
    let mode=if tags.get("Variant").is_some_and(|s|matches!(s.as_str(),"Chess960"|"Fischerandom"|"Fischer Random")){CastlingMode::Chess960}else{CastlingMode::Standard};
    let fen=tags.get("FEN").map(String::as_str).unwrap_or("");
    let mut pos=initial(fen,mode)?;let mut before=pos.clone();let mut stack=Vec::new();
    let mut out=MAGIC.to_vec();out.push(u8::from(mode==CastlingMode::Chess960));put(&mut out,fen.as_bytes());
    let bytes=pgn.as_bytes();let mut i=0;let mut literal=0;
    while i<bytes.len(){
        match bytes[i] {
            b'['=>{i+=1;let mut quoted=false;while i<bytes.len(){let b=bytes[i];i+=1;if b==b'\\'&&quoted {i=(i+1).min(bytes.len());continue;}if b==b'"'{quoted=!quoted;}if b==b']'&&!quoted{break;}}continue;},
            b'{'=>{i+=1;while i<bytes.len()&&bytes[i]!=b'}'{i+=1;}i=(i+1).min(bytes.len());continue;},
            b';'|b'%'=>{while i<bytes.len()&&bytes[i]!=b'\n'{i+=1;}continue;},
            b'(' | b')'=>{
                if literal<i{out.push(0);put(&mut out,&bytes[literal..i]);}
                if bytes[i]==b'(' {stack.push((pos.clone(),before.clone()));pos=before.clone();out.push(2);}else{let (p,b)=stack.pop().ok_or(invalid("Unbalanced binary variation"))?;pos=p;before=b;out.push(3);}
                i+=1;literal=i;continue;
            },
            b if b.is_ascii_whitespace()=>{i+=1;continue;},
            _=>{}
        }
        let start=i;while i<bytes.len()&&!bytes[i].is_ascii_whitespace()&&!b"{}();[]".contains(&bytes[i]){i+=1;}
        if i==start{i+=1;continue;}
        let mut move_start=start;
        if bytes[start].is_ascii_digit(){while move_start<i&&(bytes[move_start].is_ascii_digit()||bytes[move_start]==b'.'){move_start+=1;}if move_start==i||!bytes[start..move_start].contains(&b'.'){continue;}}
        let token=text(&bytes[move_start..i])?;
        let Ok(san)=token.trim_end_matches(['!','?']).parse::<SanPlus>() else{continue;};
        let Ok(m)=san.san.to_move(&pos) else{continue;};
        if literal<move_start{out.push(0);put(&mut out,&bytes[literal..move_start]);}
        out.push(1);out.extend_from_slice(&PackedUciMove::pack(UciMove::from_move(m,mode)).to_bytes());
        let canonical=SanPlus::from_move(pos.clone(),m).to_string();
        put(&mut out,if token==canonical{b""}else{token.as_bytes()});
        before=pos.clone();pos.play_unchecked(m);literal=i;
    }
    if literal<bytes.len(){out.push(0);put(&mut out,&bytes[literal..]);}
    if !stack.is_empty(){return Err(invalid("Unbalanced binary variation"));}
    let mut encoder=flate2::write::ZlibEncoder::new(Vec::new(),flate2::Compression::fast());encoder.write_all(&out).map_err(|_|invalid("Cannot encode binary moves"))?;
    let mut bytes=b"IWZ\x01".to_vec();bytes.extend(encoder.finish().map_err(|_|invalid("Cannot encode binary moves"))?);Ok(bytes)
}
pub fn decode(bytes:&[u8])->Result<String>{
    let expanded=expand(bytes)?;decode_packets(&expanded)
}
fn expand(bytes:&[u8])->Result<Vec<u8>>{
    if !bytes.starts_with(b"IWZ\x01"){return Err(invalid("Unsupported binary game version"));}
    let mut expanded=Vec::new();flate2::read::ZlibDecoder::new(&bytes[4..]).take((LIMIT*4+1) as u64).read_to_end(&mut expanded).map_err(|_|invalid("Invalid binary move compression"))?;
    if expanded.len()>LIMIT*4{return Err(invalid("Binary game exceeds limit"));}Ok(expanded)
}
/// Replay packed mainline moves directly, without rendering or reparsing PGN.
pub fn visit_mainline(bytes:&[u8],mut visit:impl FnMut(&Chess,Option<shakmaty::Move>))->Result<()>{
    let expanded=expand(bytes)?;let mut bytes=expanded.as_slice();
    if take(&mut bytes,MAGIC.len())?!=MAGIC{return Err(invalid("Unsupported binary game version"));}
    let mode=match take(&mut bytes,1)?[0]{0=>CastlingMode::Standard,1=>CastlingMode::Chess960,_=>return Err(invalid("Invalid binary castling mode"))};
    let mut pos=initial(text(field(&mut bytes)?)?,mode)?;let mut before=pos.clone();let mut stack=Vec::new();let mut plies=0;visit(&pos,None);
    while !bytes.is_empty(){match take(&mut bytes,1)?[0]{
        0=>{field(&mut bytes)?;},
        1=>{plies+=1;if plies>10000{return Err(invalid("Binary game exceeds move limit"));}let uci=PackedUciMove::from_bytes(take(&mut bytes,2)?.try_into().unwrap()).unpack();field(&mut bytes)?;let m=uci.to_move(&pos).map_err(|_|invalid("Illegal binary move"))?;if stack.is_empty(){visit(&pos,Some(m));}before=pos.clone();pos.play_unchecked(m);},
        2=>{if stack.len()>=32{return Err(invalid("Binary variation nesting limit"));}stack.push((pos.clone(),before.clone()));pos=before.clone();},
        3=>{let(p,b)=stack.pop().ok_or(invalid("Unbalanced binary variation"))?;pos=p;before=b;},
        _=>return Err(invalid("Invalid binary game packet")),
    }}if !stack.is_empty(){return Err(invalid("Unbalanced binary variation"));}Ok(())
}
fn decode_packets(mut bytes:&[u8])->Result<String>{
    if take(&mut bytes,MAGIC.len())?!=MAGIC{return Err(invalid("Unsupported binary game version"));}
    let mode=match take(&mut bytes,1)?[0]{0=>CastlingMode::Standard,1=>CastlingMode::Chess960,_=>return Err(invalid("Invalid binary castling mode"))};
    let mut pos=initial(text(field(&mut bytes)?)?,mode)?;let mut before=pos.clone();let mut stack=Vec::new();let mut out=String::new();let mut plies=0;
    while !bytes.is_empty(){match take(&mut bytes,1)?[0]{
        0=>out.push_str(text(field(&mut bytes)?)?),
        1=>{plies+=1;if plies>10000{return Err(invalid("Binary game exceeds move limit"));}let uci=PackedUciMove::from_bytes(take(&mut bytes,2)?.try_into().unwrap()).unpack();let m=uci.to_move(&pos).map_err(|_|invalid("Illegal binary move"))?;let original=text(field(&mut bytes)?)?;out.push_str(&if original.is_empty(){SanPlus::from_move(pos.clone(),m).to_string()}else{original.to_owned()});before=pos.clone();pos.play_unchecked(m);},
        2=>{if stack.len()>=32{return Err(invalid("Binary variation nesting limit"));}stack.push((pos.clone(),before.clone()));pos=before.clone();out.push('(');},
        3=>{let (p,b)=stack.pop().ok_or(invalid("Unbalanced binary variation"))?;pos=p;before=b;out.push(')');},
        _=>return Err(invalid("Invalid binary game packet")),
    }if out.len()>LIMIT{return Err(invalid("Decoded game exceeds limit"));}}
    if !stack.is_empty(){return Err(invalid("Unbalanced binary variation"));}Ok(out)
}
pub struct Stored {pub json:String,pub moves:Vec<u8>,pub record:Option<Vec<u8>>}
pub fn prepare(doc:&Document,parsed:&crate::archive_pgn::Parsed)->Result<Stored>{
    let mut document=doc.clone();normalize(&mut document,doc.metadata.get("ECO").map(String::as_str).unwrap_or(""));
    let moves=encode(&document.pgn,&parsed.tags)?;
    document.pgn.clear();
    let record=if let Some(record)=&mut document.record{
        let mut encoder=flate2::write::ZlibEncoder::new(Vec::new(),flate2::Compression::fast());
        let bytes=serde_json::to_vec(record)?;if bytes.len()>LIMIT{return Err(invalid("Game record exceeds binary storage limit"));}
        encoder.write_all(&bytes).map_err(|_|invalid("Cannot encode game record"))?;
        let payload=encoder.finish().map_err(|_|invalid("Cannot encode game record"))?;
        if let Some(record)=record.as_object_mut(){record.remove("json");}
        Some(payload)
    }else{None};
    Ok(Stored{json:serde_json::to_string(&document)?,moves,record})
}
pub fn store(db:&Connection,id:&str,moves:&[u8],record:Option<&[u8]>)->Result<()>{
    db.execute("INSERT INTO game_payloads VALUES(?1,?2,?3) ON CONFLICT(game_id) DO UPDATE SET moves=excluded.moves,record=excluded.record",params![id,moves,record])?;Ok(())
}
pub fn load(db:&Connection,id:&str,raw:&str)->Result<Document>{
    let mut doc:Document=serde_json::from_str(raw)?;
    if !doc.pgn.is_empty(){return Ok(doc);}
    let (moves,record):(Vec<u8>,Option<Vec<u8>>)=db.query_row("SELECT moves,record FROM game_payloads WHERE game_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?.ok_or(invalid("Missing binary game payload"))?;
    doc.pgn=decode(&moves)?;
    if let Some(bytes)=record {let mut out=Vec::new();flate2::read::ZlibDecoder::new(bytes.as_slice()).take((LIMIT+1) as u64).read_to_end(&mut out).map_err(|_|invalid("Invalid binary record"))?;if out.len()>LIMIT{return Err(invalid("Binary record exceeds limit"));}doc.record=Some(serde_json::from_slice(&out)?);}
    if let Some(eco)=doc.metadata.get("ECO"){if let Some(name)=db.query_row("SELECT description FROM eco_openings WHERE eco=?1",[eco],|r|r.get::<_,String>(0)).optional()?{doc.metadata.insert("Opening".into(),name.clone());let tag=format!("[Opening \"{}\"]\n",name.replace('\\',"\\\\").replace('"',"\\\""));doc.pgn=format!("{tag}{}",doc.pgn);}}
    Ok(doc)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn packed_moves_round_trip_annotations_variations_and_special_positions(){
        let cases=[
            "[White \"Éva \\\"quoted\\\"\"]\n\n1.e4!? {keep [Opening \"inside comment\"] ♞} (1. d4 d5 (1... Nf6)) e5 $1 2.Nf3 Nc6 3.Bb5 a6 *",
            "1. e4 e5 2. Nf3 Nc6 3. Bc4 Nf6 4. O-O Be7 *",
            "1. e4 a6 2. e5 d5 3. exd6 *",
            "[SetUp \"1\"]\n[FEN \"7k/P7/8/8/8/8/8/7K w - - 0 1\"]\n\n1. a8=Q+ *",
            "[SetUp \"1\"]\n[FEN \"7k/P7/8/8/8/8/8/7K w - - 0 1\"]\n\n1. a8=N *",
            "1. f3 e5 2. g4 Qh4# 0-1",
            "[Variant \"Chess960\"]\n[SetUp \"1\"]\n[FEN \"4k3/8/8/8/8/8/8/RK5R w AH - 0 1\"]\n\n1. O-O *",
        ];
        for input in cases {
            let parsed=crate::archive_pgn::parse(input).unwrap().remove(0);
            let bytes=encode(input,&parsed.tags).unwrap();assert_eq!(decode(&bytes).unwrap(),input);
            let mut moves=0;visit_mainline(&bytes,|_,m|moves+=usize::from(m.is_some())).unwrap();assert_eq!(moves,parsed.moves);
            let exported=crate::archive_pgn::parse(&decode(&bytes).unwrap()).unwrap().remove(0);
            assert_eq!(exported.final_fen,parsed.final_fen);assert_eq!(exported.sans,parsed.sans);
            for cut in [0,4,bytes.len()/2,bytes.len()-3]{assert!(decode(&bytes[..cut]).is_err());}
        }
    }
    #[test]
    fn normalized_storage_contains_only_codes_and_binary_payloads(){
        let mut db=Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE games(id TEXT PRIMARY KEY,revision INTEGER,created_at INTEGER,updated_at INTEGER,document TEXT,fingerprint TEXT UNIQUE);CREATE TABLE collections(id TEXT PRIMARY KEY,name TEXT,revision INTEGER);CREATE TABLE memberships(collection_id TEXT,game_id TEXT);").unwrap();
        crate::analysis_runs::initialize(&db).unwrap();crate::explorer::initialize(&db).unwrap();crate::archive_library::initialize(&db).unwrap();
        let input="[ECO \"A85\"]\n[Opening \"Wrong repeated description\"]\n\n1. d4 f5 2. c4 Nf6 3. Nc3 *";
        let result=crate::archive::import(&mut db,crate::archive_pgn::documents(input).unwrap(),None).unwrap();let id=result["ids"][0].as_str().unwrap();
        let raw:String=db.query_row("SELECT document FROM games",[],|r|r.get(0)).unwrap();assert!(!raw.contains("Wrong repeated"));assert!(!raw.contains("Dutch Defense"));assert!(!raw.contains("d4"));
        let card:String=db.query_row("SELECT summary FROM library_cards",[],|r|r.get(0)).unwrap();assert!(!card.contains("opening"));
        let kind:String=db.query_row("SELECT typeof(moves) FROM game_payloads",[],|r|r.get(0)).unwrap();assert_eq!(kind,"blob");
        let loaded=crate::archive::get(&db,id).unwrap();assert_eq!(loaded.document.metadata["ECO"],"A85");assert!(loaded.document.pgn.contains("Dutch Defense"));assert!(!loaded.document.pgn.contains("Wrong repeated"));
        assert_eq!(crate::archive::import(&mut db,vec![loaded.document],None).unwrap()["duplicates"],1);
        let backup=crate::archive::backup(&mut db).unwrap();crate::archive::delete(&mut db,id,1).unwrap();assert_eq!(db.query_row::<u64,_,_>("SELECT count(*) FROM game_payloads",[],|r|r.get(0)).unwrap(),0);
        crate::archive::restore(&mut db,backup).unwrap();assert!(crate::archive::get(&db,id).unwrap().document.pgn.contains("d4"));
    }
}
