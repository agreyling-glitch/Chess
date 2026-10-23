//! Release-time validation of downloaded opening lines; never used by the app.
use shakmaty::{Chess, Position, san::San};
fn validate(folder: &std::path::Path) -> Result<usize, Box<dyn std::error::Error>> {
    let mut total=0;
    for volume in ['a','b','c','d','e'] {
        let text=std::fs::read_to_string(folder.join(format!("{volume}.tsv")))?;
        for (index,line) in text.lines().enumerate().skip(1) {
            let fields:Vec<_>=line.split('\t').collect();
            if fields.len()!=3 {return Err(format!("{volume}.tsv:{}: expected three columns",index+1).into());}
            let mut position=Chess::default();let mut moves=0;
            for token in fields[2].split_whitespace().filter(|token|!token.ends_with('.')) {
                let san:San=token.parse().map_err(|_|format!("{volume}.tsv:{}: invalid SAN {token}",index+1))?;
                let mv=san.to_move(&position).map_err(|_|format!("{volume}.tsv:{}: illegal move {token}",index+1))?;
                position.play_unchecked(mv);moves+=1;
            }
            if moves==0 {return Err(format!("{volume}.tsv:{}: missing moves",index+1).into());}
            total+=1;
        }
    }
    if total==0 {return Err("Empty opening dataset".into());}
    Ok(total)
}
fn main() {
    let path=std::env::args_os().nth(1).expect("Usage: validate-openings <dataset directory>");
    match validate(std::path::Path::new(&path)) {
        Ok(rows)=>println!("Validated {rows} legal opening lines."),
        Err(error)=>{eprintln!("Opening data rejected: {error}");std::process::exit(1);}
    }
}
