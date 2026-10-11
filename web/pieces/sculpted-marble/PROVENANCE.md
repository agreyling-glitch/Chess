# Sculpted Marble

Transparent piece sprites rendered by Ironwood's CPU renderer from the bundled
Poly Haven Chess Set by Riley Queen, including its white and black diffuse and
normal maps. The source geometry and textures are CC0 1.0.

See [source provenance](../../../assets/3d/polyhaven-chess-set/README.md) for
source URLs and verified asset hashes. No Lichess artwork is used.

Regenerate from the repository root with:

    cargo test --lib generate_sculpted_marble_sprites -- --ignored

All twelve sprites share a camera. Each visible silhouette is tightly cropped,
scaled by piece type, and centered in its transparent square canvas.
Rendering uses a 3072-pixel source canvas before downsampling for smooth edges.
On the board the sprites rise above the square center to anchor their bases.
