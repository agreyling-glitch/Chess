# Poly Haven Chess Set geometry and textures

The board and six piece shapes were extracted from [Chess Set by Riley Queen on Poly Haven](https://polyhaven.com/a/chess_set).
Poly Haven publishes the asset under [CC0 1.0](https://polyhaven.com/license).

The source was the official 1K glTF and its geometry buffer, downloaded September 28, 2026 from
`https://api.polyhaven.com/files/chess_set`. Their MD5 hashes match the official API records:

- `chess_set_1k.gltf`: `ceac1016348f797f3db112e3e70a96a4`
- `chess_set.bin`: `815ae6d5becbcdd836b29b0d644bba95`

`scripts/extract-polyhaven-chess.py` extracts the board and first white pawn, rook, knight, bishop, queen,
and king mesh into separate GLBs. Geometry, vertex normals, and UV coordinates are unchanged apart from repacking
their buffers. The two 1K diffuse and normal maps are also bundled from the official API; their
MD5 hashes match the API records:

- White diffuse: `d71daf8e6465bf04c63992eaa185e1d1`
- Black diffuse: `cc4bbdc39ed8877c74dfad206284a106`
- White normal: `4cacfbb2e652cd1fd68ad599fda9c7b7`
- Black normal: `c0a6bf5918cc47fbc7ef70e7bcd44b1a`
- Board diffuse: `6387bd6ddb53994016f0463e83e2c50a`
- Board OpenGL normal: `521b8b19abf4f1aa3cc20105f00eac97`
- Board ARM (green channel roughness): `dbd91f4091f0eccadbde324cae25c159`

The egui board samples diffuse maps for marble detail and tints them to Ironwood's existing piece
colors. The board geometry and its marble diffuse, normal, and roughness maps are rendered inside
egui, with translucent move highlights and soft piece contact shadows above the marble surface.
The source glTF contains no lights; Ironwood approximates the Poly Haven preview's studio key,
fill, and glossy reflections in its egui renderer. Both the egui renderer and standalone viewer
use the board and piece normal maps. The camera, input and game logic remain in place.
