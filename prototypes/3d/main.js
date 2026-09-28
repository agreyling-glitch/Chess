import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

const START = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
const names = { p: 'pawn', r: 'rook', n: 'knight', b: 'bishop', q: 'queen', k: 'king' };
const container = document.querySelector('#scene');
const status = document.querySelector('#status');
const fenInput = document.querySelector('#fen');
const error = document.querySelector('#fen-error');
fenInput.value = new URLSearchParams(location.search).get('fen') || START;

const scene = new THREE.Scene();
scene.background = new THREE.Color('#1b2921');
const camera = new THREE.PerspectiveCamera(42, 1, 0.1, 100);
camera.position.set(6.7, 9.1, 10.3);
const renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'high-performance' });
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
renderer.outputColorSpace = THREE.SRGBColorSpace;
renderer.toneMapping = THREE.ACESFilmicToneMapping;
container.append(renderer.domElement);
const controls = new OrbitControls(camera, renderer.domElement);
controls.target.set(0, 0.25, 0);
controls.minDistance = 7.5;
controls.maxDistance = 24;
controls.minPolarAngle = 0.3;
controls.maxPolarAngle = 1.38;
controls.enableDamping = true;
controls.update();

scene.add(new THREE.HemisphereLight('#d8e6fa', '#57432e', 2.2));
const sun = new THREE.DirectionalLight('#fff2d7', 3.3);
sun.position.set(-4, 10, 6);
sun.castShadow = true;
sun.shadow.mapSize.set(2048, 2048);
sun.shadow.camera.left = sun.shadow.camera.bottom = -7;
sun.shadow.camera.right = sun.shadow.camera.top = 7;
sun.shadow.normalBias = 0.025;
scene.add(sun);
const plane = new THREE.Mesh(new THREE.PlaneGeometry(200, 200), new THREE.MeshStandardMaterial({ color: '#202d24', roughness: 1 }));
plane.rotation.x = -Math.PI / 2;
plane.position.y = -0.27;
plane.receiveShadow = true;
scene.add(plane);

const board = new THREE.Group();
scene.add(board);

const pieces = new THREE.Group();
scene.add(pieces);
const whiteMaterial = new THREE.MeshStandardMaterial({ color: '#f3ead7', roughness: 0.34, metalness: 0.05 });
const blackMaterial = new THREE.MeshStandardMaterial({ color: '#b7c8c0', roughness: 0.35, metalness: 0.12 });
const prototypes = new Map();
const loader = new GLTFLoader();
const textureLoader = new THREE.TextureLoader();

function parseFen(value) {
  const fields = value.trim().split(/\s+/);
  const ranks = fields[0]?.split('/');
  if (ranks?.length !== 8 || (fields[1] && !/^[wb]$/.test(fields[1]))) throw Error('Enter a valid FEN position.');
  const result = [];
  for (let row = 0; row < 8; row++) {
    let file = 0;
    for (const symbol of ranks[row]) {
      if (/^[1-8]$/.test(symbol)) file += Number(symbol);
      else if (/^[prnbqkPRNBQK]$/.test(symbol)) {
        if (file >= 8) throw Error('Each FEN rank must contain eight squares.');
        result.push({ symbol, file, rank: 7 - row });
        file++;
      } else throw Error('FEN contains an unknown piece or square.');
    }
    if (file !== 8) throw Error('Each FEN rank must contain eight squares.');
  }
  return result;
}

function showPosition(value) {
  const position = parseFen(value);
  pieces.clear();
  for (const { symbol, file, rank } of position) {
    const piece = prototypes.get(symbol.toLowerCase()).clone(true);
    const material = symbol === symbol.toUpperCase() ? whiteMaterial : blackMaterial;
    piece.traverse(object => {
      if (object.isMesh) {
        object.material = material;
        object.castShadow = true;
        object.receiveShadow = true;
      }
    });
    piece.position.set(file - 3.5, 0.075, 3.5 - rank);
    // Face both knights toward their opponent.
    if (symbol.toLowerCase() === 'n' && symbol === symbol.toLowerCase()) piece.rotation.y = Math.PI;
    pieces.add(piece);
  }
  error.textContent = '';
  history.replaceState(null, '', `?fen=${encodeURIComponent(value.trim())}`);
}

function prepareModel(root, name) {
  root.updateMatrixWorld(true);
  const bounds = new THREE.Box3().setFromObject(root);
  const size = bounds.getSize(new THREE.Vector3());
  const center = bounds.getCenter(new THREE.Vector3());
  const dimensions = {
    pawn: [0.51, 0.90],
    rook: [0.58, 0.98],
    knight: [0.65, 1.15],
    bishop: [0.63, 1.30],
    queen: [0.63, 1.43],
    king: [0.66, 1.52],
  };
  const [footprint, height] = dimensions[name];
  root.position.set(-center.x, -bounds.min.y, -center.z);
  const normalized = new THREE.Group();
  normalized.scale.set(
    footprint / Math.max(size.x, size.z),
    height / size.y,
    footprint / Math.max(size.x, size.z),
  );
  normalized.add(root);
  return normalized;
}

function resize() {
  const { width, height } = container.getBoundingClientRect();
  if (!width || !height) return;
  camera.aspect = width / height;
  camera.updateProjectionMatrix();
  renderer.setSize(width, height, false);
}
new ResizeObserver(resize).observe(container);
renderer.setAnimationLoop(() => { controls.update(); renderer.render(scene, camera); });
renderer.domElement.addEventListener('webglcontextlost', () => { status.textContent = 'Graphics context lost. Reload to restore the board.'; });

document.querySelector('#fen-form').addEventListener('submit', event => {
  event.preventDefault();
  try { showPosition(fenInput.value); } catch (cause) { error.textContent = cause.message; }
});
document.querySelector('#flip').addEventListener('click', () => {
  camera.position.set(-camera.position.x, camera.position.y, -camera.position.z);
  controls.update();
});
document.querySelector('#reset').addEventListener('click', () => {
  fenInput.value = START;
  showPosition(START);
});

try {
  const loadTexture = async (name, color) => {
    const texture = await textureLoader.loadAsync(`./models/chess_set_pieces_${name}_1k.jpg`);
    texture.flipY = false;
    if (color) texture.colorSpace = THREE.SRGBColorSpace;
    return texture;
  };
  const [whiteMap, blackMap, whiteNormal, blackNormal] = await Promise.all([
    loadTexture('white_diff', true), loadTexture('black_diff', true),
    loadTexture('white_nor_gl', false), loadTexture('black_nor_gl', false),
  ]);
  whiteMaterial.map = whiteMap;
  blackMaterial.map = blackMap;
  whiteMaterial.normalMap = whiteNormal;
  blackMaterial.normalMap = blackNormal;
  whiteMaterial.needsUpdate = blackMaterial.needsUpdate = true;
  const [boardGltf, boardMap, boardNormal, boardArm] = await Promise.all([
    loader.loadAsync('./models/board.glb'),
    textureLoader.loadAsync('./models/chess_set_board_diff_1k.jpg'),
    textureLoader.loadAsync('./models/chess_set_board_nor_gl_1k.jpg'),
    textureLoader.loadAsync('./models/chess_set_board_arm_1k.jpg'),
  ]);
  boardMap.flipY = false;
  boardMap.colorSpace = THREE.SRGBColorSpace;
  boardNormal.flipY = false;
  boardArm.flipY = false;
  const boardMesh = boardGltf.scene;
  const bounds = new THREE.Box3().setFromObject(boardMesh);
  const center = bounds.getCenter(new THREE.Vector3());
  // The source playing grid uses 0.057888 units per square.
  const boardScale = 1 / 0.057888;
  boardMesh.scale.setScalar(boardScale);
  boardMesh.position.set(-center.x * boardScale, 0.055 - bounds.max.y * boardScale, -center.z * boardScale);
  boardMesh.traverse(object => {
    if (object.isMesh) {
      object.material = new THREE.MeshStandardMaterial({
        map: boardMap, normalMap: boardNormal, normalScale: new THREE.Vector2(0.7, 0.7),
        roughnessMap: boardArm, roughness: 1, side: THREE.DoubleSide,
      });
      object.castShadow = object.receiveShadow = true;
    }
  });
  board.add(boardMesh);
  await Promise.all(Object.entries(names).map(async ([symbol, name]) => {
    const gltf = await loader.loadAsync(`./models/${name}.glb`);
    prototypes.set(symbol, prepareModel(gltf.scene, name));
  }));
  showPosition(fenInput.value);
  status.textContent = '';
} catch (cause) {
  status.textContent = `Could not load 3D board: ${cause.message}`;
}
