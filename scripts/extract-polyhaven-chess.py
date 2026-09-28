"""Extract the six white piece meshes from Poly Haven's CC0 chess_set glTF.

Usage: python scripts/extract-polyhaven-chess.py chess_set.gltf chess_set.bin output_dir
The source files are available from https://api.polyhaven.com/files/chess_set.
"""

import json
import struct
import sys
from pathlib import Path


SOURCE_NAMES = {
    "board": "board",
    "pawn": "piece_pawn_white_01",
    "rook": "piece_rook_white_01",
    "knight": "piece_knight_white_01",
    "bishop": "piece_bishop_white_01",
    "queen": "piece_queen_white",
    "king": "piece_king_white",
}


def values(gltf, binary, accessor_index, fmt):
    accessor = gltf["accessors"][accessor_index]
    view = gltf["bufferViews"][accessor["bufferView"]]
    at = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
    size = struct.calcsize(fmt)
    stride = view.get("byteStride", size)
    return [struct.unpack_from(fmt, binary, at + n * stride) for n in range(accessor["count"])]


def pad(blob):
    return blob + b" " * (-len(blob) % 4)


def glb(name, positions, normals, texcoords, indices):
    pos_bytes = b"".join(struct.pack("<fff", *p) for p in positions)
    normal_bytes = b"".join(struct.pack("<fff", *p) for p in normals)
    uv_bytes = b"".join(struct.pack("<ff", *uv) for uv in texcoords)
    idx_bytes = b"".join(struct.pack("<H", i) for i in indices)
    data = pad(pos_bytes) + pad(normal_bytes) + pad(uv_bytes) + pad(idx_bytes)
    document = {
        "asset": {"version": "2.0", "generator": "Ironwood Poly Haven extractor"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": name}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2}, "indices": 3}]}],
        "buffers": [{"byteLength": len(data)}],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": len(pos_bytes)},
            {"buffer": 0, "byteOffset": len(pad(pos_bytes)), "byteLength": len(normal_bytes)},
            {"buffer": 0, "byteOffset": len(pad(pos_bytes)) + len(pad(normal_bytes)), "byteLength": len(uv_bytes)},
            {"buffer": 0, "byteOffset": len(pad(pos_bytes)) + len(pad(normal_bytes)) + len(pad(uv_bytes)), "byteLength": len(idx_bytes)},
        ],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": len(positions), "type": "VEC3",
             "min": [min(p[i] for p in positions) for i in range(3)],
             "max": [max(p[i] for p in positions) for i in range(3)]},
            {"bufferView": 1, "componentType": 5126, "count": len(normals), "type": "VEC3"},
            {"bufferView": 2, "componentType": 5126, "count": len(texcoords), "type": "VEC2"},
            {"bufferView": 3, "componentType": 5123, "count": len(indices), "type": "SCALAR"},
        ],
    }
    encoded = pad(json.dumps(document, separators=(",", ":")).encode())
    return (b"glTF" + struct.pack("<II", 2, 12 + 8 + len(encoded) + 8 + len(data))
            + struct.pack("<I4s", len(encoded), b"JSON") + encoded
            + struct.pack("<I4s", len(data), b"BIN\0") + data)


def main():
    gltf = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    binary = Path(sys.argv[2]).read_bytes()
    output = Path(sys.argv[3])
    output.mkdir(parents=True, exist_ok=True)
    nodes = {node["name"]: node for node in gltf["nodes"]}
    for name, node_name in SOURCE_NAMES.items():
        node = nodes[node_name]
        positions = []
        normals = []
        texcoords = []
        indices = []
        for primitive in gltf["meshes"][node["mesh"]]["primitives"]:
            assert primitive.get("mode", 4) == 4
            base = len(positions)
            positions.extend(values(gltf, binary, primitive["attributes"]["POSITION"], "<fff"))
            normals.extend(values(gltf, binary, primitive["attributes"]["NORMAL"], "<fff"))
            texcoords.extend(values(gltf, binary, primitive["attributes"]["TEXCOORD_0"], "<ff"))
            index_accessor = gltf["accessors"][primitive["indices"]]
            fmt = {5121: "<B", 5123: "<H", 5125: "<I"}[index_accessor["componentType"]]
            indices.extend(base + item[0] for item in values(gltf, binary, primitive["indices"], fmt))
        assert len(positions) == len(normals) == len(texcoords) <= 65535 and max(indices) < len(positions)
        (output / f"{name}.glb").write_bytes(glb(name, positions, normals, texcoords, indices))
        print(f"{name}: {len(positions)} vertices, {len(indices) // 3} triangles")


if __name__ == "__main__":
    main()
