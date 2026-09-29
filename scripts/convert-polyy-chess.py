"""Convert Polyy.AI's low-poly pack into Ironwood's compact GLBs and atlases.

Run with the bundled workspace Python (Pillow required). The source zip is kept
outside the shipped assets; only referenced mesh vertices and color maps ship.
"""
import io
import json
import struct
import zipfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "3D_chess_sets_pack_1_lowpoly.zip"
OUTPUT = ROOT / "assets/3d/polyy-chess-pack-1"
PREFIX = "3D_Chess_Sets_Pack_1_LowPoly/"
SETS = ("deco", "egypt")
PIECES = ("pawn", "rook", "knight", "bishop", "queen", "king")
CELL = 512


def source_parts(data):
    json_length = struct.unpack_from("<I", data, 12)[0]
    document = json.loads(data[20:20 + json_length])
    bin_start = 20 + json_length + 8
    return document, data[bin_start:]


def view_bytes(document, binary, index):
    view = document["bufferViews"][index]
    start = view.get("byteOffset", 0)
    return binary[start:start + view["byteLength"]]


def accessor_values(document, binary, index, width):
    accessor = document["accessors"][index]
    view = document["bufferViews"][accessor["bufferView"]]
    types = {5123: "H", 5125: "I", 5126: "f"}
    code = types[accessor["componentType"]]
    fmt = "<" + code * width
    stride = view.get("byteStride", struct.calcsize(fmt))
    offset = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
    return [struct.unpack_from(fmt, binary, offset + i * stride)
            for i in range(accessor["count"])]


def pack_glb(positions, normals, uvs, indices):
    arrays = [
        (b"".join(struct.pack("<3f", *point) for point in positions), 5126, "VEC3"),
        (b"".join(struct.pack("<3f", *normal) for normal in normals), 5126, "VEC3"),
        (b"".join(struct.pack("<2f", *uv) for uv in uvs), 5126, "VEC2"),
        (b"".join(struct.pack("<H", index) for index in indices), 5123, "SCALAR"),
    ]
    payload = bytearray()
    views, accessors = [], []
    for content, component, kind in arrays:
        while len(payload) % 4:
            payload.append(0)
        views.append({"buffer": 0, "byteOffset": len(payload), "byteLength": len(content)})
        payload.extend(content)
        accessors.append({"bufferView": len(views) - 1, "componentType": component,
                          "count": len(indices) if kind == "SCALAR" else len(positions), "type": kind})
    result = {"asset": {"version": "2.0", "generator": "Ironwood Polyy converter"},
              "buffers": [{"byteLength": len(payload)}], "bufferViews": views,
              "accessors": accessors,
              "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1,
                                                       "TEXCOORD_0": 2}, "indices": 3}]}]}
    encoded = json.dumps(result, separators=(",", ":")).encode()
    encoded += b" " * (-len(encoded) % 4)
    payload.extend(b"\0" * (-len(payload) % 4))
    total = 12 + 8 + len(encoded) + 8 + len(payload)
    return (struct.pack("<4sII", b"glTF", 2, total) +
            struct.pack("<I4s", len(encoded), b"JSON") + encoded +
            struct.pack("<I4s", len(payload), b"BIN\0") + payload)


def make_glb(document, binary, column, row, repair_normals=False):
    primitive = document["meshes"][0]["primitives"][0]
    attributes = primitive["attributes"]
    indices = [v[0] for v in accessor_values(document, binary, primitive["indices"], 1)]
    positions = accessor_values(document, binary, attributes["POSITION"], 3)
    normals = accessor_values(document, binary, attributes["NORMAL"], 3)
    uvs = accessor_values(document, binary, attributes["TEXCOORD_0"], 2)
    if repair_normals:
        clean_indices, seen_faces = [], set()
        for n in range(0, len(indices), 3):
            tri = indices[n:n + 3]
            a, b, c = (positions[i] for i in tri)
            ab = [b[i] - a[i] for i in range(3)]
            ac = [c[i] - a[i] for i in range(3)]
            cross = (ab[1] * ac[2] - ab[2] * ac[1],
                     ab[2] * ac[0] - ab[0] * ac[2],
                     ab[0] * ac[1] - ab[1] * ac[0])
            key = tuple(sorted(tuple(round(v, 5) for v in positions[i]) for i in tri))
            if sum(v * v for v in cross) < 1e-12 or key in seen_faces:
                continue
            seen_faces.add(key)
            clean_indices.extend(tri)
        indices = clean_indices
        accumulated = {}
        for n in range(0, len(indices), 3):
            tri = indices[n:n + 3]
            a, b, c = (positions[i] for i in tri)
            ab = [b[i] - a[i] for i in range(3)]
            ac = [c[i] - a[i] for i in range(3)]
            face = [ab[1] * ac[2] - ab[2] * ac[1],
                    ab[2] * ac[0] - ab[0] * ac[2],
                    ab[0] * ac[1] - ab[1] * ac[0]]
            if sum(face[axis] * normals[i][axis] for i in tri for axis in range(3)) < 0:
                face = [-value for value in face]
            for index in tri:
                key = tuple(round(v, 5) for v in positions[index])
                total = accumulated.setdefault(key, [0.0, 0.0, 0.0])
                for axis in range(3):
                    total[axis] += face[axis]
        repaired = []
        for index, original in enumerate(normals):
            total = accumulated.get(tuple(round(v, 5) for v in positions[index]))
            if total and sum(v * v for v in total) > 1e-12:
                length = sum(v * v for v in total) ** 0.5
                repaired.append(tuple(v / length for v in total))
            else:
                repaired.append(original)
        normals = repaired
    corners = [(index, False) for index in indices]
    used = list(dict.fromkeys(corners))
    lookup = {key: new for new, key in enumerate(used)}
    assert len(used) < 65536
    return pack_glb(
        [positions[i] for i, _ in used],
        [tuple(-v for v in normals[i]) if flip else normals[i] for i, flip in used],
        [((uvs[i][0] + column) / 3, (uvs[i][1] + row) / 2) for i, _ in used],
        [lookup[key] for key in corners],
    )


def color_image(document, binary):
    primitive = document["meshes"][0]["primitives"][0]
    material = document["materials"][primitive["material"]]
    texture = document["textures"][material["pbrMetallicRoughness"]["baseColorTexture"]["index"]]
    image = document["images"][texture["source"]]
    return Image.open(io.BytesIO(view_bytes(document, binary, image["bufferView"]))).convert("RGB")


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(SOURCE) as archive:
        (OUTPUT / "LICENSE.txt").write_bytes(archive.read(PREFIX + "LICENSE.txt"))
        (OUTPUT / "SOURCE-README.txt").write_bytes(archive.read(PREFIX + "README.txt"))
        for set_name in SETS:
            directory = OUTPUT / set_name
            directory.mkdir(exist_ok=True)
            for side in ("light", "dark"):
                atlas = Image.new("RGB", (CELL * 3, CELL * 2))
                for number, piece in enumerate(PIECES):
                    data = archive.read(f"{PREFIX}models/{set_name}/{piece}_{side}.glb")
                    document, binary = source_parts(data)
                    column, row = number % 3, number // 3
                    image = color_image(document, binary)
                    image = image.resize((CELL, CELL), Image.Resampling.LANCZOS)
                    atlas.paste(image,
                                (column * CELL, row * CELL))
                    if side == "light":
                        (directory / f"{piece}.glb").write_bytes(
                            make_glb(document, binary, column, row))
                atlas.save(directory / f"{side}.jpg", quality=88, subsampling=0)


if __name__ == "__main__":
    main()
