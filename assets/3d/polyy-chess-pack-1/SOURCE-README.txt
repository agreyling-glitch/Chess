3D Chess Sets Pack 1 - low-poly LOD
Generated with polyy.ai

48 chess pieces in GLB format. Decimated LOD meshes with JPEG-compressed color maps.
Four complete sets, each with king, queen, bishop, knight, rook and pawn
in a light and a dark material. The light and dark piece of each pair
share one mesh, so both sides of a set match exactly.

  Bauhaus: stacked geometric primitives in matte lacquered wood, cream with a cobalt band and charcoal with a mustard band.
  Art Deco: stepped, fluted columns with brass trim, ivory marble and black onyx.
  Sci-fi: smooth capsule shells on hexagonal bases with one light line, white with cyan and gunmetal with orange.
  Egyptian: carved stone with gold bands, pale sandstone with a lapis inlay and black basalt with a turquoise inlay.

Contents
  models/<set>/   GLB meshes, one file per piece (<piece>_<light|dark>.glb)
  previews/<set>/ animated turntable GIF for each piece, same filename
  LICENSE.txt

Engine notes
  Godot 4:   drag the .glb into the FileSystem dock and instance the scene.
  Unity:     GLB needs glTFast or UnityGLTF; both read these files directly.
  Unreal 5:  Import the .glb through the Content Browser.
  Blender:   File > Import > glTF 2.0.

Mesh details
  One mesh and one PBR material per file, with base color and
  metallic/roughness textures embedded (JPEG/PNG). No normal maps.
  Roughly 1,285 to 6,652 triangles per piece.
  Y-up, standing upright with the base at the bottom. The origin sits at
  the center of the bounding box and every piece is normalized to fit a
  1 unit box, so scale each type in the engine. Tournament proportions
  relative to the king: queen 0.87, bishop 0.70, knight 0.67, rook 0.60,
  pawn 0.53; a king stands about 1.6 squares tall on a standard board.
  The Art Deco, sci-fi and Egyptian knights face +Z; the Bauhaus knight's
  head points -X. A 180 degree turn about Y faces the other side of the board.

File list
  models/bauhaus/king_light.glb
  models/bauhaus/king_dark.glb
  models/bauhaus/queen_light.glb
  models/bauhaus/queen_dark.glb
  models/bauhaus/bishop_light.glb
  models/bauhaus/bishop_dark.glb
  models/bauhaus/knight_light.glb
  models/bauhaus/knight_dark.glb
  models/bauhaus/rook_light.glb
  models/bauhaus/rook_dark.glb
  models/bauhaus/pawn_light.glb
  models/bauhaus/pawn_dark.glb
  models/deco/king_light.glb
  models/deco/king_dark.glb
  models/deco/queen_light.glb
  models/deco/queen_dark.glb
  models/deco/bishop_light.glb
  models/deco/bishop_dark.glb
  models/deco/knight_light.glb
  models/deco/knight_dark.glb
  models/deco/rook_light.glb
  models/deco/rook_dark.glb
  models/deco/pawn_light.glb
  models/deco/pawn_dark.glb
  models/scifi/king_light.glb
  models/scifi/king_dark.glb
  models/scifi/queen_light.glb
  models/scifi/queen_dark.glb
  models/scifi/bishop_light.glb
  models/scifi/bishop_dark.glb
  models/scifi/knight_light.glb
  models/scifi/knight_dark.glb
  models/scifi/rook_light.glb
  models/scifi/rook_dark.glb
  models/scifi/pawn_light.glb
  models/scifi/pawn_dark.glb
  models/egypt/king_light.glb
  models/egypt/king_dark.glb
  models/egypt/queen_light.glb
  models/egypt/queen_dark.glb
  models/egypt/bishop_light.glb
  models/egypt/bishop_dark.glb
  models/egypt/knight_light.glb
  models/egypt/knight_dark.glb
  models/egypt/rook_light.glb
  models/egypt/rook_dark.glb
  models/egypt/pawn_light.glb
  models/egypt/pawn_dark.glb
