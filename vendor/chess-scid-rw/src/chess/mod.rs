pub mod game;
pub mod game_tree;
pub mod history;
pub mod movegen;
pub mod notation;
pub mod pgn;
/// Polyglot opening-book (`.bin`) reader — never used by the SCID
/// decoder itself, gated behind the `chess-extras` feature (on by
/// default). See `README.md`.
#[cfg(feature = "chess-extras")]
pub mod polyglot;
pub mod rules;
pub mod types;
