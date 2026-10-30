import { companionConnections, onCompanionConnectionsChange } from './companion-connection.js';
import { CompanionArchiveClient } from './companion-archive-client.js';

export function openHistoryBackup(target = window, { connections = companionConnections, subscribe = onCompanionConnectionsChange, Client = CompanionArchiveClient } = {}) {
  const doc = target.document;
  const existing = doc.getElementById('history-backup');
  if (existing) { existing.focus(); return existing; }
  const dialog = doc.createElement('dialog'); dialog.id = 'history-backup';
  dialog.className = 'history-backup-dialog';
  const add = (tag, text, parent = dialog) => { const element = doc.createElement(tag); if (text) element.textContent = text; parent.append(element); return element; };
  add('h2', 'Backup and Restore');
  const inner = add('section'); inner.className = 'history-backup-fields';
  const actionLabel = add('label', 'Action', inner);
  const action = add('select', '', actionLabel);
  for (const name of ['Backup', 'Restore']) { const option = add('option', name, action); option.value = name.toLowerCase(); }
  const locationLabel = add('label', 'Game History Location', inner);
  const location = add('select', '', locationLabel);

  const folder = add('p', '', inner); folder.style.cssText = 'overflow-wrap:anywhere;color:#d3ad62';
  const settings = add('button', 'Configure backup folder…', inner);
  const fileLabel = add('label', 'Backup in Companion folder', inner);
  const file = add('select', '', fileLabel);
  const progress = add('progress', '', inner); progress.max = 100; progress.style.width = '100%'; progress.hidden = true;
  let files = [], folderReady = false, generation = 0, pollTimer;
  const guidance = add('p', '', inner); const status = add('p', '', inner); status.setAttribute('role', 'status');
  const controls = add('div'); controls.className = 'history-backup-actions';
  const close = add('button', 'Close', controls), run = add('button', 'Download backup', controls);
  run.className = 'primary';
  let busy = false;
  const update = () => {
    const restore = action.value === 'restore', browser = location.value === 'browser';
    fileLabel.hidden = !restore || browser;
    folder.hidden = settings.hidden = browser;
    guidance.textContent = browser
      ? (restore ? 'Choose a browser backup, then review Merge or Replace before restoring.' : 'Includes browser games, training profiles and preferences.')
      : (restore ? 'Restore a database backup from this Companion’s backup folder. Its archive must be empty. Browser data and other Companions stay unchanged.' : 'Save a database snapshot directly in this Companion’s backup folder. Games do not pass through the browser.');
    run.textContent = restore ? (browser ? 'Choose browser backup…' : 'Restore to selected Companion') : (browser ? 'Download backup' : 'Back up on Companion');
    settings.disabled = busy;
    run.disabled = busy || !location.value || (!browser && (!folderReady || (restore && !file.value)));
  };
  const refresh = () => {
    const selected = location.value;
    location.replaceChildren();
    const browser = add('option', 'Browser', location); browser.value = 'browser';
    for (const item of connections()) {
      if (item.capabilities?.services?.archive === false) continue;
      const option = add('option', item.name + (item.connected ? '' : ' (disconnected)'), location);
      option.value = item.endpoint; option.disabled = !item.connected;
    }
    location.value = [...location.options].some(option => option.value === selected && !option.disabled) ? selected : 'browser';
    update();
  };
  const unsubscribe = subscribe(() => { if (!busy) { refresh(); void load(); } });
  dialog.addEventListener('close', () => { unsubscribe(); dialog.remove(); generation++; target.clearTimeout(pollTimer); }, { once: true });
  close.onclick = () => dialog.close();
  action.onchange = () => { status.textContent = ''; update(); };
  location.onchange = () => { status.textContent = ''; busy = false; void load(); update(); };
  settings.onclick = async () => { await configureBackupFolder(target, location.value, {connections, Client}); void load(); };
  file.onchange = update;
  run.onclick = async () => {
    if (location.value === 'browser') {
      if (action.value === 'restore') { dialog.close(); target.ironwoodRestoreStorage(); }
      else { busy = true; update(); await target.ironwoodBackupStorage(); busy = false; update(); }
      return;
    }
    const selected = connections().find(item => item.endpoint === location.value && item.connected);
    if (!selected) { status.textContent = 'Reconnect the selected Companion and try again.'; refresh(); return; }
    const client = new Client(selected.endpoint, selected.credential);
    busy = true; update();
    try {
      await client.request('archive/backups', {action:action.value, ...(action.value === 'restore' ? {file:file.value} : {})});
      await load();
    } catch (error) { status.textContent = error.message || 'Could not start backup or restore.'; busy = false; update(); }
  };
  async function load() {
    const revision = ++generation, endpoint = location.value;
    target.clearTimeout(pollTimer);
    folderReady = false;
    if (endpoint === 'browser') { progress.hidden = true; update(); return; }
    const selected = connections().find(item => item.endpoint === endpoint && item.connected);
    if (!selected) { update(); return; }
    try {
      const result = await new Client(selected.endpoint, selected.credential).request('archive/backups');
      if (revision !== generation) return;
      folderReady = true; folder.textContent = `Backup folder on ${selected.name}: ${result.folder}`;
      files = result.files || []; const previous = file.value; file.replaceChildren();
      for (const entry of files) { const option = add('option', `${entry.name} · ${(entry.bytes / 1024 / 1024).toFixed(1)} MiB`, file); option.value = entry.name; }
      if (files.some(entry => entry.name === previous)) file.value = previous;
      const job = result.job || {}; busy = job.state === 'running';
      progress.hidden = !busy; progress.value = Number(job.percent) || 0;
      status.textContent = busy ? `${job.action === 'restore' ? 'Restoring' : 'Backing up'} on ${selected.name}: ${job.phase || ''} · ${Math.floor(job.percent || 0)}%`
        : job.state === 'failed' ? job.message : job.state === 'complete' ? `Completed on ${selected.name}: ${job.file}` : files.length ? `${files.length} backup files available.` : 'No backups in this folder yet.';
      if (job.state === 'complete' && job.action === 'restore') target.dispatchEvent?.(new target.CustomEvent('ironwood-game-library-changed', {detail:{source:'archive',endpoint}}));
      if (busy) pollTimer = target.setTimeout(() => void load(), 1500);
    } catch (error) {
      if (revision !== generation) return;
      busy = false; folder.textContent = '';
      status.textContent = error.status === 404 ? 'Update this Companion to use server-side backups.' : error.message;
    }
    if (revision === generation) update();
  }
  dialog.addEventListener('cancel', event => { if (busy) event.preventDefault(); });
  refresh(); doc.body.append(dialog); dialog.showModal(); void load(); return dialog;
}

window.ironwoodOpenHistoryBackup = () => openHistoryBackup();

export async function configureBackupFolder(target = window, endpoint, {connections = companionConnections, Client = CompanionArchiveClient} = {}) {
  const selected = connections().find(item => item.endpoint === endpoint && item.connected);
  if (!selected) return;
  const doc = target.document, dialog = doc.createElement('dialog');
  dialog.style.cssText = 'width:min(560px,calc(100vw - 48px));padding:24px;background:#171d22;color:#ecebe4;border:2px solid #d3ad62;border-radius:10px;font:16px system-ui';
  const heading = doc.createElement('h2'); heading.textContent = `Backup folder — ${selected.name}`;
  const label = doc.createElement('label'); label.textContent = 'Folder path on the Companion machine';
  const input = doc.createElement('input'); input.style.cssText = 'display:block;width:95%;padding:10px;margin:12px 0'; label.append(input);
  const hint = doc.createElement('p'); hint.textContent = 'Use an absolute local path or an accessible NAS/network share. Companion’s operating-system account needs read and write access.';
  const status = doc.createElement('p'); status.setAttribute('role','status');
  const save = doc.createElement('button'); save.textContent = 'Save backup folder'; save.disabled = true;
  const close = doc.createElement('button'); close.textContent = 'Close'; close.onclick = () => dialog.close();
  dialog.append(heading,label,hint,status,save,close); doc.body.append(dialog); dialog.showModal();
  dialog.addEventListener('close', () => dialog.remove(), {once:true});
  const client = new Client(selected.endpoint, selected.credential);
  try { const settings = await client.request('archive/backups/settings'); input.value = settings.folder || ''; save.disabled = false; }
  catch(error) { status.textContent = error.status === 404 ? 'Update this Companion to configure its backup folder.' : error.message; }
  save.onclick = async () => {
    save.disabled = true;
    try { const result = await client.request('archive/backups/settings', {folder:input.value}); input.value = result.folder; status.textContent = `Backup folder saved on ${selected.name}.`; }
    catch(error) { status.textContent = error.message; }
    finally { save.disabled = false; }
  };
  await new Promise(resolve => dialog.addEventListener('close', resolve, {once:true}));
}
