//! Readers and writers specific to the si5 disk format (`.si5`/`.sn5`) —
//! see the crate root doc for the overview. A game's content (`.sg5`) is
//! decoded/encoded by [`crate::game_blob`], shared as-is with si4 — no
//! `si5::game_blob` module here.
//!
//! Each of [`index`]/[`namebase`] holds both directions (`read_*`/
//! `write_*`) for its file, right next to each other — read and write are
//! exact mirrors of one another, so keeping them in the same module (not a
//! separate `si5::writer`) is what makes that symmetry easiest to see and
//! keep correct. [`database`] then orchestrates all three files, in both
//! directions, behind [`database::Si5Database`]'s public API.

pub mod database;
pub mod index;
pub mod namebase;
