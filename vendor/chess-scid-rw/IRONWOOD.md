# Vendored Scid reader

Source: https://github.com/Fab2bprog/chess-scid-rw
Commit: 6253187d211ed872a3b545fc251e85fec88dd8f8
License: GPL-3.0-or-later (see LICENSE and upstream source attribution).

Ironwood disables the default chess-extras feature and uses in-memory si4
conversion and bounded si5 index/game decoding with a shared name table.
Local change in src/game_blob.rs: reject truncated
comment blocks instead of silently dropping comments. No database assets
or model weights are included. The crate includes decoder code ported from
Scid and chess code from Vendetta; upstream explicitly distributes these under
GPL-3.0-or-later, matching Ironwood.
