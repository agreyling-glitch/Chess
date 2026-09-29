# Omie's Low Poly 3D Chess/Checkers Set

The Wood theme uses piece and board geometry and textures from
[Omie's Assets](https://omies-assets.itch.io/3d-chesscheckers-set).
The publisher marks the asset pack **CC0 1.0 Universal**, free for personal
and commercial use without attribution. The original `ChessSet.zip` was
downloaded September 29, 2026 from the public itch.io listing (upload 14685802).

`scripts/convert-omies-chess.mjs` converts the white FBX meshes to self-contained
glTF geometry and normalizes them for the Ironwood board. The black pieces use
the same geometry with the pack's black texture atlas. Seven source PNG maps
were converted to quality-83 JPEG for the WebAssembly bundle. The board color
and normal maps are lightly softened for the Wood finish. Other files in
the archive, including its promotional videos and Blender backups, are omitted.

Original archive SHA-256: `c6737d2f0d8ce5e8011f01bf801346250d61ce060013982722aff3d3f029ff7b`
