# Polyy.AI 3D Chess Sets Pack 1

The Art Deco and Egyptian piece geometry and textures come from
[Polyy.AI's 3D Chess Sets Pack 1](https://polyyai.itch.io/3d-chess-sets-pack-1),
downloaded September 29, 2026. Its bundled `LICENSE.txt` applies to these
models and maps and marks them CC0 1.0 Universal. The publisher describes the
pack as AI assisted. The models are rendered with Ironwood's existing Poly Haven
board.

`scripts/convert-polyy-chess.py` reads the low-poly archive, retains only indexed
mesh vertices, converts the indices to 16-bit, and combines the six embedded
color maps for each side of each set into a 3x2 JPEG atlas. Preview GIFs and
metallic/roughness maps are omitted. No source GLBs are shipped.

Original low-poly archive SHA-256:
`6c791adef55272b92ba87ce7d776d674b885e183589034029925ce4caa9c7713`
