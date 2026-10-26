import { companionConnection, requireCompanionConnection, manageCompanionConnection, onCompanionConnectionChange } from './companion-connection.js';
import { CompanionArchiveClient, browserDocuments, transferDocuments } from './companion-archive-client.js';
import { companionBrowserRecords, receiveCompanionDocuments, gameDetails } from './game-storage.js';
import {mountCompanionImportProgress} from './companion-import-progress.js';
let client;
window.ironwoodCompanionArchiveConfiguration = companionConnection;
onCompanionConnectionChange(() => { client = undefined; });
function node(tag, text, parent) { const item = document.createElement(tag); if (text) item.textContent = text; parent?.append(item); return item; }
function field(parent, title, type = 'text') { const label = node('label', title, parent), input = node('input', '', label); input.type = type; return input; }
function select(parent, title, values) { const label = node('label', title, parent), input = node('select', '', label); input.setAttribute('aria-label', title); for (const [value, text] of values) { const option = node('option', text, input); option.value = value; } return input; }
function download(value, name, type = 'application/json') { const blob = new Blob([typeof value === 'string' ? value : JSON.stringify(value, null, 2)], { type }), url = URL.createObjectURL(blob), link = node('a', '', document.body); link.href = url; link.download = name; link.click(); link.remove(); setTimeout(() => URL.revokeObjectURL(url), 30000); }
function confirmation(parent, text, proceed) {
  const box = node('section', '', parent); box.className = 'archive-confirm'; node('p', text, box);
  const cancel = node('button', 'Cancel', box), accept = node('button', 'Confirm', box); accept.className = 'danger';
  cancel.onclick = () => box.remove(); accept.onclick = () => { box.remove(); proceed(); }; box.scrollIntoView({block:'nearest'});
}
window.ironwoodOpenCompanionArchive = async ({host, onClose, isActive} = {}) => {
  if (isActive && !isActive()) return;
  if (window.ironwoodCompanionFeatureEnabled?.() === false) return;
  if (document.querySelector('#companion-archive')) return;
  if (!document.querySelector('#companion-archive-style')) { const style = node('link', '', document.head); style.id = 'companion-archive-style'; style.rel = 'stylesheet'; style.href = '/companion-archive.css'; }
  if (host) host.replaceChildren();
  const dialog = node('dialog', '', host || document.body); dialog.id = 'companion-archive';
  if (!host) { node('p', 'IRONWOOD COMPANION', dialog).className = 'eyebrow'; node('h2', 'Your game archive', dialog); }
  node('p', 'Keep a collection on your Companion server. Transfers make copies; browser games, notes and training ratings stay intact.', dialog);
  const connection = node('section', '', dialog);
  const connectionSummary = node('p', 'Uses your shared Companion connection.', connection);
  const status = node('p', '', dialog); status.setAttribute('role', 'status');
  const stopFolderProgress=mountCompanionImportProgress(window,dialog);
  dialog.addEventListener('close',stopFolderProgress,{once:true});
  const content = node('div', '', dialog); content.hidden = true;
  const tabs = node('div', '', content); tabs.className = 'actions archive-tabs';
  let busy = false, connected = false, cursor = '', next = null, local = [], localPage = 0, groups = [], selected = new Map();
  const actions = [];
  function button(parent, text, action, needsConnection = true) {
    const control = node('button', text, parent); control.type = 'button'; actions.push([control, needsConnection]);
    control.onclick = () => void perform(action); return control;
  }
  function enabled() { for (const [control, needs] of actions) control.disabled = busy || (needs && !connected); }
  async function perform(action) { if (busy) return; busy = true; enabled(); try { await action(); } catch (error) { status.textContent = error.message; } finally { busy = false; enabled(); } }
  const openArchive = button(connection, 'Open archive', async () => {
    const shared = await requireCompanionConnection(); if (!shared || !dialog.open) return;
    const candidate = new CompanionArchiveClient(shared.endpoint, shared.credential);
    const capabilities = await candidate.request('capabilities');
    const active = companionConnection();
    if (!dialog.open || active?.endpoint !== shared.endpoint || active?.credential !== shared.credential) return;
    if (capabilities.protocol_version !== '1' || !capabilities.services?.archive || ![2,3,4].includes(capabilities.archive_schema_version)) throw new Error('This Companion does not provide a supported archive.');
    client = candidate; connectionSummary.textContent = `Connected to ${candidate.endpoint}`; connected = true; content.hidden = false; cursor = ''; selected.clear();
    await refreshGroups(); await refreshLocal(); await refreshServer();
    status.textContent = 'Connected. Choose browser games to upload or archive games to copy back.';
  }, false);
  button(connection, 'Manage connection', async () => { await manageCompanionConnection(); }, false);

  const tools = node('section', '', content); node('h3', 'Collections and files', tools);
  const collection = select(content, 'Collection', [['', 'All games / no collection']]), collectionName = field(tools, 'Collection name');
  content.insertBefore(collection.parentElement, tools);
  const collectionActions = node('div', '', tools); collectionActions.className = 'actions';
  button(collectionActions, 'Create collection', async () => { await client.request('archive/collections', { action:'create',name:collectionName.value }); collectionName.value = ''; await refreshGroups(); status.textContent = 'Collection created.'; });
  const chosen = () => { const group = groups.find(g => g.id === collection.value); if (!group) throw new Error('Choose a collection first.'); return group; };
  button(collectionActions, 'Rename collection', async () => { const group = chosen(); await client.request(`archive/collections/${group.id}`, {action:'rename',revision:group.revision,name:collectionName.value}); await refreshGroups(); status.textContent = 'Collection renamed.'; });
  button(collectionActions, 'Remove collection', async () => { const group = chosen(); confirmation(content, `Remove collection “${group.name}”? Its games stay in the archive.`, () => void perform(async () => {await client.request(`archive/collections/${group.id}`,{action:'delete',revision:group.revision});collection.value = '';await refreshGroups();await refreshServer();status.textContent = 'Collection removed; games preserved.';})); });
  const fileActions = node('div', '', tools); fileActions.className = 'actions';
  const file = field(tools, 'Import PGN or Companion transfer', 'file'); file.accept = '.pgn,.json';
  button(fileActions, 'Import file to archive', async () => {
    const input = file.files[0]; if (!input) throw new Error('Choose a PGN or Companion JSON file.'); if (input.size > 16*1024*1024) throw new Error('Import exceeds 16 MiB; use smaller batches.');
    const text = await input.text(), payload = text.trimStart().startsWith('{') ? {documents:transferDocuments(JSON.parse(text))} : {pgn:text};
    const result = await client.request('archive/import', {...payload,collection:collection.value || null}); await refreshGroups(); await refreshServer(); status.textContent = `${result.imported} imported; ${result.duplicates} duplicates already archived.`;
  });
  button(fileActions, 'Download archive backup', async () => { download(await client.request('archive/backup'),'ironwood-companion-backup.json'); status.textContent = 'Archive backup downloaded. Keep a copy on another device.'; });
  const restoreFile = field(tools, 'Restore backup into an empty archive', 'file'); restoreFile.accept = '.json';
  button(fileActions, 'Restore archive backup', async () => { const input = restoreFile.files[0]; if (!input) throw new Error('Choose a Companion archive backup.'); if (input.size>16*1024*1024) throw new Error('Backup exceeds browser restore limit. Use native archive-restore.'); const result = await client.request('archive/restore',JSON.parse(await input.text()));await refreshGroups();await refreshServer();status.textContent = `${result.restored} games restored. Browser data was not changed.`; });

  const browser = node('section', '', content); node('h3', 'Browser library → archive', browser);
  const localList = node('div', '', browser); localList.className = 'game-list';
  const localActions = node('div', '', browser); localActions.className = 'actions';
  button(localActions, 'Previous browser page', async () => {localPage = Math.max(0,localPage-1); renderLocal();});
  button(localActions, 'Next browser page', async () => {if ((localPage+1)*50<local.length)localPage++;renderLocal();});
  button(localActions, 'Select this browser page', async () => { for (const input of localList.querySelectorAll('input')) input.checked = true; });
  button(localActions, 'Upload selected browser games', async () => {
    const ids = new Set([...localList.querySelectorAll('input:checked')].map(input=>input.value));
    const records = local.filter(record=>ids.has(record.id)); if (!records.length) throw new Error('Select browser games first.');
    const result = await client.request('archive/import',{documents:browserDocuments(records,window.ironwoodExportSavedGame,gameDetails),collection:collection.value || null}); await refreshGroups(); await refreshServer(); status.textContent = `${result.imported} copied to archive; ${result.duplicates} duplicates. Browser originals kept.`;
  });
  async function refreshLocal() { local = await companionBrowserRecords(); local.sort((a,b)=>b.updatedAt-a.updatedAt); renderLocal(); }
  function renderLocal() { localList.replaceChildren(); node('p', `${local.length} browser games · page ${localPage+1}`, localList); for (const record of local.slice(localPage*50,(localPage+1)*50)) { const details = gameDetails(record.json), label = node('label', '', localList), input = node('input', '', label); input.type = 'checkbox'; input.value = record.id; label.append(document.createTextNode(`${details.title} · ${record.category || details.category} · ${details.result}`)); } }

  const server = node('section', '', content); node('h3', 'Archive → browser library', server);
  const searchOptions = node('details', '', server); node('summary', 'Search filters', searchOptions);
  const filters = node('div', '', searchOptions); filters.className = 'filters';
  const white = field(filters,'White player'), black = field(filters,'Black player'), opening = field(filters,'Opening or ECO'), profile = field(filters,'Training profile ID');
  const dateFrom = field(filters,'Date from','date'), dateTo = field(filters,'Date to','date');
  const resultFilter = select(filters,'Result',[['','All results'],['1-0','White won'],['0-1','Black won'],['1/2-1/2','Draw'],['*','Unfinished']]);
  const serverActions = node('div', '', server); serverActions.className = 'actions';
  button(serverActions, 'Search archive', async () => {cursor = '';selected.clear();await refreshServer();});
  button(serverActions, 'First archive page', async () => {cursor = '';await refreshServer();});
  button(serverActions, 'Next archive page', async () => {if (next) {cursor = next;await refreshServer();}});
  const serverList = node('div', '', server); serverList.className = 'game-list';
  const selectionActions = node('div', '', server); selectionActions.className = 'actions';
  const selectedIds = () => {const ids = [...selected.keys()];if (!ids.length) throw new Error('Select archived games first.');return ids;};
  button(selectionActions,'Copy selected to browser',async()=>{const result=await client.request('archive/export',{ids:selectedIds(),format:'json'});const received=await receiveCompanionDocuments(transferDocuments(result));await refreshLocal();status.textContent=`${received.received} copied to browser; ${received.duplicates} already present. Archive originals kept.`;});
  for (const format of ['json','pgn']) button(selectionActions,`Export selected ${format.toUpperCase()}`,async()=>{const result=await client.request('archive/export',{ids:selectedIds(),format});download(format==='pgn'?result.pgn:result,`ironwood-archive.${format}`,format==='pgn'?'application/x-chess-pgn':'application/json');status.textContent='Selected games exported.';});
  for (const action of ['add','remove']) button(selectionActions,action==='add'?'Add selected to collection':'Remove selected from collection',async()=>{const group=chosen();await client.request(`archive/collections/${group.id}`,{action,revision:group.revision,games:selectedIds()});await refreshGroups();await refreshServer();status.textContent='Collection membership updated; games preserved.';});
  button(selectionActions,'Delete selected archive game',async()=>{
    const ids=selectedIds();if(ids.length!==1)throw new Error('Select one archive game to delete.');const game=selected.get(ids[0]);
    confirmation(server,`Permanently delete this game from the server archive and all its collections? Browser copies and training ratings stay intact. This cannot be undone.`,()=>void perform(async()=>{await client.request(`archive/games/${game.id}/delete`,{revision:game.revision});selected.delete(game.id);await refreshGroups();await refreshServer();status.textContent='Archive game deleted. Browser copies kept.';}));
  }).className='danger';
  const metadata = node('textarea','',server); metadata.setAttribute('aria-label','Archive metadata JSON');metadata.rows=5;metadata.hidden=true;
  let editing;
  button(selectionActions,'Edit archive metadata',async()=>{const ids=selectedIds();if(ids.length!==1)throw new Error('Select one game to edit.');editing=await client.request(`archive/games/${ids[0]}`);metadata.value=JSON.stringify(editing.document.metadata,null,2);metadata.hidden=false;status.textContent='Edit archive search metadata below. Original game data and PGN stay intact.';});
  button(selectionActions,'Save archive metadata',async()=>{if(!editing)throw new Error('Open a game for metadata editing first.');const document={...editing.document,metadata:JSON.parse(metadata.value)};await client.request(`archive/games/${editing.id}/update`,{revision:editing.revision,document});editing=null;metadata.hidden=true;selected.clear();await refreshServer();status.textContent='Archive metadata saved.';});
  async function refreshGroups() { groups=await client.request('archive/collections'); const previous=collection.value;collection.replaceChildren();for(const item of [{id:'',name:'All games / no collection'},...groups]){const option=node('option',item.name,collection);option.value=item.id;}collection.value=groups.some(g=>g.id===previous)?previous:''; }
  async function refreshServer() {
    const query=new URLSearchParams({after:cursor,limit:'50',white:white.value,black:black.value,opening:opening.value,profile:profile.value,date_from:dateFrom.value.replaceAll('-','.'),date_to:dateTo.value.replaceAll('-','.'),result:resultFilter.value,collection:collection.value});
    const page=await client.request(`archive/games?${query}`);next=page.next;serverList.replaceChildren();const count=node('p','',serverList), updateCount=()=>{count.textContent=`${page.games.length} games on this page${next?' · more available':''} · ${selected.size} selected`;};updateCount();
    for(const game of page.games){const label=node('label','',serverList),input=node('input','',label);input.type='checkbox';input.checked=selected.has(game.id);input.onchange=()=>{if(input.checked)selected.set(game.id,game);else selected.delete(game.id);updateCount();};label.append(document.createTextNode(`${game.metadata.White||'Unknown'} vs ${game.metadata.Black||'Unknown'} · ${game.metadata.Result||'*'} · ${game.metadata.Date||'Date unknown'} · revision ${game.revision}`));}
  }
  const sections = [[server,'Archive library'],[browser,'Browser uploads'],[tools,'Collections & files']];
  for(const [section,title] of sections) button(tabs,title,async()=>{for(const [other] of sections)other.hidden=other!==section;});
  tools.hidden=true;browser.hidden=true;
  const close=node('button',host ? 'Back to browser' : 'Close archive',dialog);close.onclick=()=>dialog.close();dialog.addEventListener('close',()=>{dialog.remove();onClose?.();},{once:true});
  const unsubscribe = onCompanionConnectionChange(() => { connected = false; content.hidden = true; selected.clear(); editing = null; connectionSummary.textContent = 'Connection changed. Open archive to continue.'; enabled(); });
  dialog.addEventListener('close', unsubscribe, {once:true});
  enabled();if (host) dialog.show(); else dialog.showModal();openArchive.click();
};
