const DATABASE = 'ironwood-scoresheet';
export function draftStore(indexedDB = window.indexedDB) {
 let database;
 async function open() {
  if (!indexedDB) throw new Error('Local storage unavailable');
  if (!database) database = new Promise((resolve,reject) => {
   const request = indexedDB.open(DATABASE,1);
   request.onupgradeneeded = () => request.result.createObjectStore('drafts');
   request.onsuccess = () => resolve(request.result);
   request.onerror = () => reject(request.error);
   request.onblocked = () => reject(new Error('Local storage is blocked'));
  });
  return database;
 }
 async function run(mode,value) {
  const db = await open();
  return new Promise((resolve,reject) => {
   const transaction = db.transaction('drafts',mode),store = transaction.objectStore('drafts');
   const request = mode === 'readonly' ? store.get('current') : value === null ? store.delete('current') : store.put(value,'current');
   transaction.oncomplete = () => resolve(request.result);
   transaction.onerror = transaction.onabort = () => reject(transaction.error || new Error('Draft could not be saved'));
  });
 }
 return {read:()=>run('readonly'),write:value=>run('readwrite',value),clear:()=>run('readwrite',null)};
}
