# Hybrid Marble: Egyptian pieces

Sprites rendered by Ironwood from its bundled Egyptian 3D models and textures.
Source assets are licensed CC0; see the [source provenance](../../../assets/3d/polyy-chess-pack-1/README.md).
These are original renders, not downloaded preview images.

Regenerate with:

    cargo test --lib generate_sculpted_marble_sprites -- --ignored

The hybrid styles share the same top-down camera, silhouette centering, and
piece-type sizing. Source rendering is supersampled before downsampling to PNG.
