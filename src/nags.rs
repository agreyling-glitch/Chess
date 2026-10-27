//! PGN NAG meanings (1994 specification, section 10).
//! Numeric assignments are format facts; descriptions are authored here.
//! https://github.com/mliebelt/pgn-spec-commented/blob/main/pgn-specification.md

pub fn symbol(nag: u8) -> String {
    match nag {
        1 => "!", 2 => "?", 3 => "!!", 4 => "??", 5 => "!?", 6 => "?!",
        8 => "□", 10 => "=", 13 => "∞", 14 => "+=", 15 => "=+",
        16 => "+/-", 17 => "-/+", 18 => "+−", 19 => "−+",
        _ => return format!("${nag}"),
    }.to_owned()
}

pub fn description(nag: u8) -> Option<String> {
    let direct = match nag {
        0 => "Empty annotation (placeholder)", 1 => "Good move", 2 => "Poor move",
        3 => "Very good move", 4 => "Very poor move", 5 => "Speculative move",
        6 => "Questionable move", 7 => "Forced move: alternatives lose quickly",
        8 => "Only reasonable move", 9 => "Worst move", 10 => "Drawish position",
        11 => "Balanced, quiet position", 12 => "Balanced, active position", 13 => "Unclear position",
        _ => "",
    };
    if !direct.is_empty() { return Some(direct.into()); }
    if nag > 139 { return None; }
    let side = if nag % 2 == 0 { "White" } else { "Black" };
    let level = |offset: u8| ["small", "moderate", "decisive"][usize::from(offset / 2)];
    let detail = match nag {
        14..=21 => format!("{} advantage", ["Small", "Moderate", "Decisive", "Overwhelming"][usize::from((nag - 14) / 2)]),
        22..=23 => "Zugzwang".into(),
        24..=29 => format!("{} space edge", level(nag - 24)),
        30..=35 => format!("{} development lead", level(nag - 30)),
        36..=37 => "Initiative".into(),
        38..=39 => "Sustained initiative".into(),
        40..=41 => "Attacking chances".into(),
        42..=47 => format!("{} compensation for sacrificed material", ["Insufficient", "Sufficient", "More than sufficient"][usize::from((nag - 42) / 2)]),
        48..=53 => format!("{} edge in center control", level(nag - 48)),
        54..=59 => format!("{} edge in kingside control", level(nag - 54)),
        60..=65 => format!("{} edge in queenside control", level(nag - 60)),
        66..=69 => format!("{} back rank", if nag < 68 { "Vulnerable" } else { "Secure" }),
        70..=73 => format!("{} king protection", if nag < 72 { "Weak" } else { "Strong" }),
        74..=77 => format!("{} king placement", if nag < 76 { "Poor" } else { "Good" }),
        78..=85 => format!("{} pawn structure", ["Very weak", "Somewhat weak", "Somewhat strong", "Very strong"][usize::from((nag - 78) / 2)]),
        86..=101 => {
            let piece = ["knight", "bishop", "rook", "queen"][usize::from((nag - 86) / 4)];
            let quality = if (nag - 86) % 4 < 2 { "Poor" } else { "Good" };
            format!("{quality} {piece} placement")
        }
        102..=105 => format!("{} piece coordination", if nag < 104 { "Poor" } else { "Good" }),
        106..=129 => {
            let phase = ["opening", "middlegame", "endgame"][usize::from((nag - 106) / 8)];
            let quality = ["Very poor", "Poor", "Good", "Very good"][usize::from(((nag - 106) % 8) / 2)];
            format!("{quality} {phase} play")
        }
        130..=135 => format!("{} counterplay", level(nag - 130)),
        136..=139 => format!("{} time pressure", if nag < 138 { "Moderate" } else { "Severe" }),
        _ => unreachable!(),
    };
    Some(format!("{side}: {detail}"))
}

pub fn label(nag: u8) -> String {
    match description(nag) {
        Some(text) => format!("{} · {text}", symbol(nag)),
        None => format!("${nag} · Unassigned or program-specific meaning"),
    }
}

pub fn matches(nag: u8, query: &str) -> bool {
    let text = format!("${nag} {}", label(nag)).to_lowercase();
    query.split_whitespace().all(|word| text.contains(&word.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_standard_descriptions_and_safe_extension_fallbacks() {
        for nag in 0..=255 {
            assert!(!symbol(nag).is_empty());
            assert_eq!(description(nag).is_some(), nag <= 139);
        }
        assert_eq!(description(22).as_deref(), Some("White: Zugzwang"));
        assert_eq!(description(37).as_deref(), Some("Black: Initiative"));
        assert_eq!(description(139).as_deref(), Some("Black: Severe time pressure"));
        assert_eq!(symbol(255), "$255");
        assert!(matches(37, "black initiative"));
        assert!(matches(255, "$255"));
        assert!(!matches(37, "white initiative"));
    }
}
