import { CompanionArchiveClient } from './companion-archive-client.js';
let pending = null, revision = 0;
const listeners = new Set();
const registryListeners = new Set();
const storageKey = 'ironwood.companion.connection.v1';
const enabled = () => globalThis.window?.ironwoodCompanionFeatureEnabled?.() !== false;
function restore() {
  try {
    const value = JSON.parse(window.localStorage.getItem(storageKey));
    const entries = value?.version === 1 ? [{...value,name:'Companion',engines:true}] : value?.version === 2 ? value.connections : [];
    const connections = [];
    for (const entry of entries || []) {
      if (typeof entry.credential !== 'string' || !entry.credential.trim()) continue;
      const client = new CompanionArchiveClient(entry.endpoint,entry.credential);
      if (connections.some(item => item.endpoint === client.endpoint)) continue;
      connections.push({endpoint:client.endpoint,credential:client.credential,name:String(entry.name || new URL(client.endpoint).hostname).slice(0,80),engines:entry.engines !== false,remember:true,connected:true});
    }
    return {connections,archiveId:value?.version === 1 ? connections[0]?.endpoint || null : value?.archiveId || null};
  } catch { return {connections:[],archiveId:null}; }
}
let registry = restore();
let lastArchive = companionConnection();
function persist(next) {
  try {
    const connections = next.connections.filter(item => item.remember).map(({endpoint,credential,name,engines}) => ({endpoint,credential,name,engines}));
    if (connections.length) window.localStorage.setItem(storageKey,JSON.stringify({version:2,connections,archiveId:connections.some(item => item.endpoint === next.archiveId) ? next.archiveId : null}));
    else if (window.localStorage?.getItem(storageKey)) window.localStorage.removeItem(storageKey);
  } catch { throw new Error('Browser storage is unavailable. Uncheck “Remember this connection” to connect for this tab only.'); }
}
function publish(next) {
  registry = next;
  const archive = companionConnection();
  if (archive?.endpoint !== lastArchive?.endpoint || archive?.credential !== lastArchive?.credential) {
    lastArchive = archive;
    if (globalThis.window?.dispatchEvent && window.Event) window.dispatchEvent(new window.Event('ironwood-companion-connection-changed'));
    for (const listener of listeners) listener(archive);
  }
  if (globalThis.window?.dispatchEvent && window.Event) window.dispatchEvent(new window.Event('ironwood-companion-connections-changed'));
  for (const listener of registryListeners) listener(companionConnections());
}
export const companionConnections = () => enabled() ? registry.connections.map(item => ({...item,archive:item.endpoint === registry.archiveId})) : [];
export const companionEngineConnections = () => companionConnections().filter(item => item.connected && item.engines);
export function companionConnection() {
  const item = enabled() && registry.connections.find(item => item.connected && item.endpoint === registry.archiveId);
  return item ? {endpoint:item.endpoint,credential:item.credential,...(item.capabilities ? {capabilities:item.capabilities} : {})} : null;
}
export function onCompanionConnectionChange(listener) { listeners.add(listener); return () => listeners.delete(listener); }
export function onCompanionConnectionsChange(listener) { registryListeners.add(listener); return () => registryListeners.delete(listener); }
export function disconnectCompanion(endpoint) {
  revision++;
  publish({...registry,connections:registry.connections.map(item => !endpoint || item.endpoint === endpoint ? {...item,connected:false} : item)});
}
export function removeCompanionConnection(endpoint) {
  const next = {connections:registry.connections.filter(item => item.endpoint !== endpoint),archiveId:registry.archiveId === endpoint ? null : registry.archiveId};
  persist(next); revision++; publish(next);
}
export function forgetCompanionConnection() { const next = {connections:[],archiveId:null}; persist(next); revision++; publish(next); }
export async function connectCompanion(endpoint, credential, options = {}) {
  if (!enabled()) throw new Error('Companion testing is disabled.');
  const {remember = false,engines = true,archive = true,name} = options;
  if (name != null && !String(name).trim()) throw new Error('Enter a Companion name.');
  const attempt = ++revision;
  if (!credential.trim()) throw new Error('Enter your Companion connection token.');
  const client = new CompanionArchiveClient(endpoint,credential.trim());
  const capabilities = await client.request('capabilities');
  if (capabilities.protocol_version !== '1' || capabilities.product !== 'Ironwood Companion') throw new Error('Unsupported Companion server or protocol.');
  if (archive && capabilities.services?.archive === false) throw new Error('This Companion does not provide an archive. Uncheck Default archive.');
  if (attempt !== revision || !enabled()) throw new Error('Connection changed. Try connecting again.');
  const previous = registry.connections.find(item => item.endpoint === client.endpoint);
  const item = {endpoint:client.endpoint,credential:client.credential,capabilities,name:String(name || previous?.name || new URL(client.endpoint).hostname).trim().slice(0,80),engines,remember,connected:true};
  const next = {connections:[...registry.connections.filter(item => item.endpoint !== client.endpoint),item],archiveId:archive ? client.endpoint : registry.archiveId === client.endpoint ? null : registry.archiveId};
  persist(next); publish(next);
  return {endpoint:item.endpoint,credential:item.credential,capabilities};
}
export function manageCompanionConnection() {
  if (!enabled()) return Promise.resolve(null);
  if (pending) return pending;
  pending = new Promise(resolve => {
    if (!document.querySelector('#companion-connection-style')) {
      const style = document.createElement('link'); style.id = 'companion-connection-style'; style.rel = 'stylesheet'; style.href = '/companion-archive.css'; document.head.append(style);
    }
    const dialog = document.createElement('dialog'); dialog.id = 'companion-connection'; dialog.dataset.companionConnection = 'true';
    dialog.innerHTML = `<form><p class="eyebrow">IRONWOOD COMPANION</p><h2>Companion connections</h2><p>Choose one default archive and any number of engine providers. Research and tablebases use the default archive.</p><section><label>Connections<select name="connection" aria-label="Named Companion connections"></select></label><button type="button" data-new>Add connection</button></section><section><label>Companion name<input name="connectionName" maxlength="80" required placeholder="Windows Engines or QNAP Archive"></label><label>Companion address<input name="endpoint" type="url" required></label><label>Connection token<input name="credential" type="password" autocomplete="off" required></label><label><input name="archive" type="checkbox"> Default archive (including research and tablebases)</label><label><input name="engines" type="checkbox"> Use as an engine provider</label><label><input name="remember" type="checkbox"> Remember this connection on this device</label><p>Remembering saves this address and token in this browser. Tokens stay masked and are never stored in games.</p></section><button type="button" data-backup-folder>Backup folder…</button><p role="status" aria-live="polite"></p><div class="actions"><button type="submit">Connect / Save</button><button type="button" data-disconnect>Disconnect selected</button><button type="button" data-forget>Remove selected</button><button type="button" data-close>Done</button></div></form>`;
    const form = dialog.querySelector('form'), status = dialog.querySelector('[role=status]'), submit = form.querySelector('[type=submit]');
    const picker = form.elements.connection;
    let editing = null, alive = true;
    function fill(endpoint) {
      const item = registry.connections.find(item => item.endpoint === endpoint);
      editing = item?.endpoint || null;
      form.elements.connectionName.value = item?.name || '';
      form.elements.endpoint.value = item?.endpoint || 'http://127.0.0.1:8790';
      form.elements.endpoint.readOnly = !!item;
      form.elements.credential.value = item?.credential || '';
      form.elements.archive.checked = item ? item.endpoint === registry.archiveId : !registry.archiveId;
      form.elements.engines.checked = item?.engines ?? true;
      form.elements.remember.checked = item?.remember ?? false;
      form.querySelector('[data-disconnect]').disabled = !item?.connected;
      form.querySelector('[data-forget]').disabled = !item;
      form.querySelector('[data-backup-folder]').disabled = !item?.connected;
      status.textContent = item ? `${item.name}: ${item.connected ? 'Configured' : 'Disconnected'}. Connect / Save checks the server and applies changes.` : 'Enter the name, address and token for this Companion.';
    }
    function render(endpoint) {
      picker.replaceChildren();
      for (const item of registry.connections) {
        const option = document.createElement('option'); option.value = item.endpoint;
        option.textContent = `${item.name}${item.endpoint === registry.archiveId ? ' · Default archive' : ''}${item.engines ? ' · Engines' : ''}${item.connected ? '' : ' · Disconnected'}`;
        picker.append(option);
      }
      const option = document.createElement('option'); option.value = ''; option.textContent = 'New connection'; picker.append(option);
      picker.value = endpoint || ''; fill(picker.value);
    }
    picker.onchange = () => fill(picker.value);
    form.querySelector('[data-new]').onclick = () => { picker.value = ''; fill(null); };
    form.querySelector('[data-disconnect]').onclick = () => { disconnectCompanion(editing); render(editing); };
    form.querySelector('[data-forget]').onclick = () => {
      try { removeCompanionConnection(editing); render(null); } catch (error) { status.textContent = error.message; }
    };
    form.querySelector('[data-backup-folder]').onclick = async () => {
      try { const module = await import('./history-backup.js'); await module.configureBackupFolder(window, editing); }
      catch(error) { status.textContent = error.message; }
    };
    form.querySelector('[data-close]').onclick = () => dialog.close();
    form.onsubmit = async event => {
      event.preventDefault(); submit.disabled = true; status.textContent = 'Checking connection…';
      try {
        const item = await connectCompanion(form.elements.endpoint.value,form.elements.credential.value,{name:form.elements.connectionName.value,remember:form.elements.remember.checked,archive:form.elements.archive.checked,engines:form.elements.engines.checked});
        if (alive) { render(item.endpoint); status.textContent = 'Connection saved and checked.'; }
      } catch (error) { if (alive) status.textContent = error.message; }
      finally { submit.disabled = false; }
    };
    dialog.addEventListener('close', () => { alive = false; revision++; form.elements.credential.value = ''; dialog.remove(); pending = null; resolve(companionConnection()); },{once:true});
    render(registry.archiveId || registry.connections[0]?.endpoint);
    document.body.append(dialog); dialog.showModal();
  });
  return pending;
}
export async function requireCompanionConnection() { return companionConnection() || await manageCompanionConnection(); }
// Engine identity is persisted with the game; credentials belong only to the
// shared connection on this browser origin, never to the game or engine picker.
export function companionEngineConfiguration(identity, overrides = {}) {
  const connections = companionEngineConnections();
  const connection = connections.find(item => identity.startsWith(`${item.endpoint}/`));
  if (!connection) throw new Error(connections.length ? 'The selected engine belongs to a different Companion address. Manage the connection or select an engine again.' : 'Connect to Companion in Engine settings, then choose Retry connection.');
  const engineId = identity.slice(connection.endpoint.length + 1);
  if (!engineId || engineId.includes('/')) throw new Error('The selected engine belongs to a different Companion address. Select an engine again.');
  return {endpoint:connection.endpoint,credential:connection.credential,engineId,overrides};
}
if (typeof window !== 'undefined') {
  window.addEventListener?.('ironwood-companion-feature-changed', () => {
    revision++;
    publish({...registry});
  });
  window.ironwoodCompanionConnection = companionConnection;
  window.ironwoodCompanionConnections = companionConnections;
  window.ironwoodManageCompanionConnection = manageCompanionConnection;
}
