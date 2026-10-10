const KEY = 'ironwood-background-image';
const SELECTED = 'ironwood-background-selected';
const DEFAULT = { id: 'ironwood-default', name: 'Ironwood (default)', source: '/brand/ironwood-background-no-ring.png' };
let library = [], selected = '', generation = 0;
const database = new Promise((resolve, reject) => {
  const request = indexedDB.open('ironwood-backgrounds', 1);
  request.onupgradeneeded = () => request.result.createObjectStore('images', { keyPath: 'id' });
  request.onsuccess = () => resolve(request.result);
  request.onerror = () => reject(request.error);
});
async function transact(mode, operation) {
  const db = await database;
  return new Promise((resolve, reject) => {
    const transaction = db.transaction('images', mode);
    const request = operation(transaction.objectStore('images'));
    transaction.oncomplete = () => resolve(request.result);
    transaction.onerror = () => reject(transaction.error);
    transaction.onabort = () => reject(transaction.error);
  });
}
async function refreshLibrary() {
  library = [DEFAULT, ...await transact('readonly', store => store.getAll())];
}
async function select(id) {
  const item = library.find(item => item.id === id);
  if (id && !item) return;
  if (item) await decode(item.source);
  else { generation++; width = height = 0; pixels = new Uint8Array([0]); }
  selected = id;
  localStorage.setItem(SELECTED, id);
}
window.ironwoodBackgroundLibrary = () => JSON.stringify({ selected, images: library.map(({id, name}) => ({id, name})) });
window.ironwoodSelectBackground = id => select(id).catch(() => alert('Could not load this background.'));
let pixels = new Uint8Array();
let width = 0, height = 0;
async function decode(source, save = false, name = "Uploaded image") {
  const version = ++generation;
  const image = new Image();
  image.src = source;
  await image.decode();
  const scale = Math.min(1, 1600 / Math.max(image.naturalWidth, image.naturalHeight));
  const canvas = document.createElement('canvas');
  canvas.width = Math.max(1, Math.round(image.naturalWidth * scale));
  canvas.height = Math.max(1, Math.round(image.naturalHeight * scale));
  const context = canvas.getContext('2d');
  context.drawImage(image, 0, 0, canvas.width, canvas.height);
  if (version !== generation) return;
  if (save) {
    const id = crypto.randomUUID();
    await transact('readwrite', store => store.put({id, name, source: canvas.toDataURL('image/jpeg', 0.85)}));
    await refreshLibrary();
    selected = id;
    localStorage.setItem(SELECTED, id);
  }
  if (version !== generation) return;
  width = canvas.width; height = canvas.height;
  pixels = new Uint8Array(context.getImageData(0, 0, width, height).data);
}
window.ironwoodUploadBackground = () => {
  const input = document.createElement('input');
  input.type = 'file'; input.accept = 'image/png,image/jpeg,image/webp';
  input.onchange = async () => {
    const file = input.files?.[0];
    if (!file) return;
    const url = URL.createObjectURL(file);
    try { await decode(url, true, file.name); }
    catch { alert('Could not save this background. Try a smaller PNG, JPEG, or WebP image.'); }
    finally { URL.revokeObjectURL(url); }
  };
  input.click();
};
window.ironwoodRemoveBackground = async () => {
  try {
    if (selected && selected !== DEFAULT.id) await transact('readwrite', store => store.delete(selected));
    await refreshLibrary();
    await select('');
  } catch { alert('Could not remove the saved background.'); }
};
window.ironwoodBackgroundWidth = () => width;
window.ironwoodBackgroundHeight = () => height;
window.ironwoodTakeBackground = () => { const result = pixels; pixels = new Uint8Array(); return result; };
(async () => {
  await refreshLibrary();
  const legacy = localStorage.getItem(KEY);
  if (legacy && library.length === 1) {
    await decode(legacy, true);
    localStorage.removeItem(KEY);
  } else {
    const saved = localStorage.getItem(SELECTED);
    await select(saved === null ? '' : saved);
  }
})().catch(() => {});
