//! Resolved name tables (players, tournaments, sites, rounds) — a NEUTRAL
//! structure, independent of the original disk format (`.sn4` front-coded/
//! sorted, vs `.sn5` append-only journal/varint — see `si4::namebase`/
//! `si5::namebase` for the format-specific readers), for the same reason
//! as `entry::IndexEntry`: `pgn_build::build_pgn` only knows this
//! structure, never the original disk format.

/// The 4 reconstructed name tables, indexed by `idNumberT` (the ID stored
/// in the `IndexEntry` records of the index file).
#[derive(Debug, Clone, Default)]
pub struct NameTables {
    pub players: Vec<String>,
    pub events:  Vec<String>,
    pub sites:   Vec<String>,
    pub rounds:  Vec<String>,
}

impl NameTables {
    #[must_use]
    pub fn player(&self, id: u32) -> &str {
        self.players.get(id as usize).map_or("?", String::as_str)
    }
    #[must_use]
    pub fn event(&self, id: u32) -> &str {
        self.events.get(id as usize).map_or("?", String::as_str)
    }
    #[must_use]
    pub fn site(&self, id: u32) -> &str {
        self.sites.get(id as usize).map_or("?", String::as_str)
    }
    #[must_use]
    pub fn round(&self, id: u32) -> &str {
        self.rounds.get(id as usize).map_or("?", String::as_str)
    }

    /// Returns the ID of `name` in `list` if already present, otherwise
    /// appends it and returns the newly assigned ID (its index) — the
    /// write-side counterpart of `player`/`event`/`site`/`round` above.
    /// Shared implementation for the four tables below.
    fn id_or_insert(list: &mut Vec<String>, name: &str) -> u32 {
        if let Some(pos) = list.iter().position(|s| s == name) {
            // `pos < list.len()`, itself always representable in a `u32`
            // in practice (a database with over 4 billion distinct names
            // is not a realistic target); `unwrap_or(u32::MAX)` is a safe
            // fallback rather than a panic on that purely theoretical case.
            return u32::try_from(pos).unwrap_or(u32::MAX);
        }
        list.push(name.to_owned());
        u32::try_from(list.len() - 1).unwrap_or(u32::MAX)
    }

    /// Finds or creates a player ID for `name` — see [`Self::id_or_insert`].
    pub fn player_id_or_insert(&mut self, name: &str) -> u32 {
        Self::id_or_insert(&mut self.players, name)
    }

    /// Finds or creates an event ID for `name` — see [`Self::id_or_insert`].
    pub fn event_id_or_insert(&mut self, name: &str) -> u32 {
        Self::id_or_insert(&mut self.events, name)
    }

    /// Finds or creates a site ID for `name` — see [`Self::id_or_insert`].
    pub fn site_id_or_insert(&mut self, name: &str) -> u32 {
        Self::id_or_insert(&mut self.sites, name)
    }

    /// Finds or creates a round ID for `name` — see [`Self::id_or_insert`].
    pub fn round_id_or_insert(&mut self, name: &str) -> u32 {
        Self::id_or_insert(&mut self.rounds, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_or_insert_assigns_sequential_ids() {
        let mut t = NameTables::default();
        assert_eq!(t.player_id_or_insert("Carlsen"), 0);
        assert_eq!(t.player_id_or_insert("Nepomniachtchi"), 1);
        assert_eq!(t.players, vec!["Carlsen", "Nepomniachtchi"]);
    }

    #[test]
    fn test_id_or_insert_reuses_existing_id() {
        let mut t = NameTables::default();
        let first = t.player_id_or_insert("Carlsen");
        let second = t.player_id_or_insert("Carlsen");
        assert_eq!(first, second);
        assert_eq!(t.players.len(), 1, "aucun doublon");
    }

    #[test]
    fn test_id_or_insert_tables_are_independent() {
        let mut t = NameTables::default();
        let player_id = t.player_id_or_insert("Same Name");
        let event_id = t.event_id_or_insert("Same Name");
        assert_eq!(player_id, 0);
        assert_eq!(event_id, 0, "chaque table a sa propre numérotation");
    }
}
