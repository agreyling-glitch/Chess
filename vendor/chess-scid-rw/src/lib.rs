//! `chess-scid-rw` — pure-Rust reader/writer for [SCID](https://scid.sourceforge.net/)
//! chess database files, formats **si4** (`.si4`/`.sn4`/`.sg4`) and
//! **si5** (`.si5`/`.sn5`/`.sg5`).
//!
//! Given the three files of a database, this crate decodes the index, the
//! name tables (players/events/sites/rounds) and each game's move stream
//! (main line, nested variations, comments, NAGs, non-standard starting
//! positions), and reassembles standard PGN text — see [`Si4Database`]/
//! [`Si5Database`] and their `game_pgn` method.
//!
//! # Dependencies
//!
//! Zero external crate dependencies. The chess representation needed to
//! decode SCID's compact move encoding (position, legal move generation,
//! SAN notation, a variation tree) is vendored in [`chess`] rather than
//! pulled from an external crate, so this crate only ever depends on
//! `std`.
//!
//! # Format coverage
//!
//! - Both si4 and si5 are supported: [`si4`]/[`si5`] contain only the
//!   readers specific to each on-disk format (index, name table);
//!   everything else (move decoder [`moves`]/[`game_blob`], PGN assembly
//!   [`pgn_build`], neutral structures [`entry`]/[`names`]) is shared
//!   between the two, since a game's content (moves/tags/comments/
//!   variations/NAGs) is encoded identically in both formats — see
//!   `docs/scid-si5-specification.md`, section "Historical context".
//! - Non-standard starting positions (custom FEN) are decoded AND encoded:
//!   see [`game_blob`]'s module doc (`build_piece_lists`) for the
//!   piece-index assignment algorithm, replayed from the reference
//!   implementation's `Position::AddPiece` and reused on both the read
//!   and write side.
//! - Variations, nested to any depth, are decoded: see [`game_blob`]'s
//!   module doc (`decode_variation_tree`). The source file's main line
//!   stays the main line (`children[0]`) of the resulting tree, and
//!   variations are exported as PGN RAV `(...)` groups.
//!
//! # Status / roadmap
//!
//! Reading and writing are both complete for si4 and si5:
//! [`Si4Database::create`]/[`Si4Database::add_game`]/
//! [`Si4Database::update_game`]/[`Si4Database::delete_game`]/
//! [`Si4Database::save`] and their [`Si5Database`] equivalents create a
//! new database, append games, edit or delete an already-added game
//! (encoded from PGN text — see [`pgn_ingest`]) — see the project's
//! `README.md` for the current scope (append-only on disk) and the
//! remaining roadmap.
//!
//! # Origin
//!
//! The decoder was originally written for, and is used by, the
//! [Vendetta Chess GUI](https://github.com/Fab2bprog/Rust-Vendetta-Chess-GUI)
//! project, then extracted here as a standalone, dependency-free crate for
//! the wider Rust chess community.

pub mod bytes;
pub mod chess;
pub mod dates;
pub mod eco;
pub mod entry;
pub mod error;
pub mod game_blob;
pub mod moves;
pub mod names;
pub mod pgn_build;
#[cfg(feature = "chess-extras")]
pub mod pgn_ingest;
mod scid_common;
pub mod si4;
pub mod si5;

pub use error::{GameDecodeError, GameEncodeError, ScidError, StorageLimitError};
#[cfg(feature = "chess-extras")]
pub use error::AddGameError;
pub use si4::database::{Si4Database, Si4Paths};
pub use si5::database::{Si5Database, Si5Paths};
