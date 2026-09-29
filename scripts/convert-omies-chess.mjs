// Convert the CC0 Omie's ChessSet.zip FBX assets to Ironwood's small glTF files.
// Usage: node scripts/convert-omies-chess.mjs <extracted ChessSet directory>
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { JSDOM } from 'jsdom';
import { FBXLoader } from 'three/examples/jsm/loaders/FBXLoader.js';
import { Matrix3, Vector3 } from 'three';
import sharp from 'sharp';

const source = process.argv[2];
if (!source) throw new Error('Pass the extracted ChessSet directory.');
const destination = join(import.meta.dirname, '..', 'assets', '3d', 'omies-chess-set');
mkdirSync(destination, { recursive: true });
const dom = new JSDOM('');
globalThis.document = dom.window.document;
globalThis.window = dom.window;
const loader = new FBXLoader();

function glbFromFbx(name) {
  const bytes = readFileSync(join(source, 'Models', name));
  const group = loader.parse(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength), '');
  group.updateMatrixWorld(true);
  const mesh = group.children.find(child => child.isMesh);
  if (!mesh) throw new Error(`No mesh in ${name}`);
  const geometry = mesh.geometry;
  const positions = geometry.attributes.position;
  const normals = geometry.attributes.normal;
  const uvs = geometry.attributes.uv;
  if (!normals || !uvs || positions.count > 65535) throw new Error(`Unsupported geometry in ${name}`);
  const normalMatrix = new Matrix3().getNormalMatrix(mesh.matrixWorld);
  const position = new Float32Array(positions.count * 3);
  const normal = new Float32Array(positions.count * 3);
  const uv = new Float32Array(positions.count * 2);
  const index = new Uint16Array(positions.count);
  const point = new Vector3();
  const direction = new Vector3();
  for (let i = 0; i < positions.count; i++) {
    point.fromBufferAttribute(positions, i).applyMatrix4(mesh.matrixWorld);
    direction.fromBufferAttribute(normals, i).applyMatrix3(normalMatrix).normalize();
    position.set(point.toArray(), i * 3);
    normal.set(direction.toArray(), i * 3);
    uv.set([uvs.getX(i), 1 - uvs.getY(i)], i * 2);
    index[i] = i;
  }
  const chunks = [Buffer.from(position.buffer), Buffer.from(normal.buffer), Buffer.from(uv.buffer), Buffer.from(index.buffer)];
  const views = [];
  let offset = 0;
  for (const chunk of chunks) {
    views.push({ buffer: 0, byteOffset: offset, byteLength: chunk.length });
    offset += chunk.length;
    offset = (offset + 3) & ~3;
  }
  const binary = Buffer.alloc(offset);
  chunks.forEach((chunk, i) => chunk.copy(binary, views[i].byteOffset));
  const accessors = [
    { bufferView: 0, componentType: 5126, count: positions.count, type: 'VEC3' },
    { bufferView: 1, componentType: 5126, count: positions.count, type: 'VEC3' },
    { bufferView: 2, componentType: 5126, count: positions.count, type: 'VEC2' },
    { bufferView: 3, componentType: 5123, count: positions.count, type: 'SCALAR' },
  ];
  const json = Buffer.from(JSON.stringify({
    asset: { version: '2.0', generator: 'Ironwood Omie converter' },
    buffers: [{ byteLength: binary.length }], bufferViews: views, accessors,
    meshes: [{ primitives: [{ attributes: { POSITION: 0, NORMAL: 1, TEXCOORD_0: 2 }, indices: 3 }] }],
  }));
  const paddedJson = Buffer.alloc((json.length + 3) & ~3, 0x20);
  json.copy(paddedJson);
  const header = Buffer.alloc(12);
  header.write('glTF', 0); header.writeUInt32LE(2, 4);
  header.writeUInt32LE(12 + 8 + paddedJson.length + 8 + binary.length, 8);
  const jsonHeader = Buffer.alloc(8);
  jsonHeader.writeUInt32LE(paddedJson.length, 0); jsonHeader.write('JSON', 4);
  const binHeader = Buffer.alloc(8);
  binHeader.writeUInt32LE(binary.length, 0); binHeader.write('BIN\0', 4);
  return Buffer.concat([header, jsonHeader, paddedJson, binHeader, binary]);
}

for (const piece of ['Pawn', 'Rook', 'Knight', 'Bishop', 'Queen', 'King']) {
  const name = `SM_White${piece === 'Bishop' ? 'bishop' : piece}.fbx`;
  writeFileSync(join(destination, `${piece.toLowerCase()}.glb`), glbFromFbx(name));
}
writeFileSync(join(destination, 'board.glb'), glbFromFbx('SM_ChessBoard.fbx'));
const maps = [
  ['WhitePieces', 'pieces_white', 'BaseColor', 'diff'],
  ['BlackPieces', 'pieces_black', 'BaseColor', 'diff'],
  ['ChessBoard_CheckerPieces', 'board', 'BaseColor', 'diff'],
  ['ChessBoard_CheckerPieces', 'board', 'Normal', 'nor'],
  ['ChessBoard_CheckerPieces', 'board', 'Roughness', 'rough'],
  ['WhitePieces', 'pieces_white', 'Normal', 'nor'],
  ['BlackPieces', 'pieces_black', 'Normal', 'nor'],
];
for (const [folder, prefix, sourceKind, outputKind] of maps) {
  const file = join(source, 'Textures', folder,
    `Mat_${folder === 'ChessBoard_CheckerPieces' ? 'Board_Checkers' : 'ChessPieces'}_${sourceKind}.png`);
  let image = sharp(file);
  if (prefix === 'board' && outputKind === 'diff') {
    // The atlas prints coordinates around the squares. Ironwood draws its own
    // labels, so cover the four atlas margins with the supplied dark wood grain.
    const { width, height } = await image.metadata();
    if (width !== height) throw new Error('Unexpected Omie board atlas size');
    const half = width / 2;
    const margin = Math.round(half * 0.095);
    const grain = async (left, top, width, height) => sharp(file)
      .extract({ left, top, width, height }).png().toBuffer();
    image = image.composite([
      { input: await grain(0, half, half, margin), left: 0, top: 0 },
      { input: await grain(0, half + margin, half, margin), left: 0, top: half - margin },
      { input: await grain(0, half + margin * 2, margin, half - margin * 2), left: 0, top: margin },
      { input: await grain(margin, half + margin * 2, margin, half - margin * 2), left: half - margin, top: margin },
    ]);
  }
  image = image.flatten({ background: '#808080' });
  if (prefix === 'board' && outputKind === 'diff') image = image.blur(0.3);
  if (prefix === 'board' && outputKind === 'nor') image = image.blur(0.5);
  await image.jpeg({ quality: 83, mozjpeg: true })
    .toFile(join(destination, `${prefix}_${outputKind}.jpg`));
}
console.log(`Converted Omie's CC0 chess set to ${destination}`);
