import { releaseKey } from './companion-release-key.js';
const host = 'https://companion-releases.ironwoodchess.com';
const root = `${host}/private-preview/windows-x64/`;
const enabled = () => globalThis.window?.ironwoodCompanionFeatureEnabled?.() === true;
const bytes = value => Uint8Array.from(atob(value), c => c.charCodeAt(0));

export async function downloadPrivateCompanion(token, {fetcher = fetch, cryptoApi = crypto, now = Date.now, allowed = enabled} = {}) {
  if (!allowed()) throw Error('Enable Companion in Version diagnostics first.');
  if (typeof token !== 'string' || !/^[A-Za-z0-9]{32,512}$/.test(token)) throw Error('Enter your private release-access token.');
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 120000);
  const get = async (url, limit) => {
    if (!allowed()) throw Error('Companion was disabled. Download cancelled.');
    const response = await fetcher(url, {headers: {Authorization: `Bearer ${token}`}, credentials: 'omit', cache: 'no-store', redirect: 'error', referrerPolicy: 'no-referrer', signal: controller.signal});
    if (!response.ok) throw Error(response.status === 401 ? 'Release access was rejected. Check your release-access token.' : 'The private release service is unavailable. Please retry.');
    if (+response.headers.get('Content-Length') > limit) throw Error('Release exceeds its download size limit.');
    const reader = response.body.getReader();
    const parts = []; let size = 0;
    try {
      for (;;) {
        const {value, done} = await reader.read(); if (done) break;
        size += value.length;
        if (size > limit || !allowed()) throw Error('Download cancelled or exceeds its size limit.');
        parts.push(value);
      }
    } finally { await reader.cancel(); reader.releaseLock(); }
    const result = new Uint8Array(size); let offset = 0;
    for (const part of parts) {result.set(part, offset); offset += part.length;}
    return result;
  };
  try {
    const envelope = JSON.parse(new TextDecoder().decode(await get(`${root}manifest.json`, 65536)));
    const payload = bytes(envelope.payload);
    const prefix = new TextEncoder().encode('Ironwood Companion private-preview release v1\n');
    const signed = new Uint8Array(prefix.length + payload.length); signed.set(prefix); signed.set(payload, prefix.length);
    const key = await cryptoApi.subtle.importKey('jwk', releaseKey, {name:'RSASSA-PKCS1-v1_5', hash:'SHA-256'}, false, ['verify']);
    if (!await cryptoApi.subtle.verify('RSASSA-PKCS1-v1_5', key, bytes(envelope.signature), signed)) throw Error('Release signature verification failed. Nothing was downloaded for installation.');
    const release = JSON.parse(new TextDecoder().decode(payload));
    if (release.format !== 2 || release.product !== 'Ironwood Companion' || release.platform !== 'windows-x64' || release.signing_mode !== 'private-preview-key' || release.protocol !== 1 || release.data_schema !== 1 || !/^\d+\.\d+\.\d+$/.test(release.version) || !Number.isFinite(Date.parse(release.expires)) || Date.parse(release.expires) <= now() || !/^[a-f0-9]{64}$/.test(release.sha256)) throw Error('Release metadata is incompatible or expired.');
    if (release.url !== `${root}${release.version}/ironwood-companion-windows-x64.zip`) throw Error('Unexpected release download address.');
    const archive = await get(release.url, 256 * 1024 * 1024);
    const hash = Array.from(new Uint8Array(await cryptoApi.subtle.digest('SHA-256', archive)), b => b.toString(16).padStart(2,'0')).join('');
    if (hash !== release.sha256) throw Error('Package integrity verification failed. Nothing was saved.');
    if (!allowed()) throw Error('Companion was disabled. Download cancelled.');
    return {version:release.version, archive};
  } finally { clearTimeout(timeout); controller.abort(); token = ''; }
}

export function openCompanionDownload() {
  if (!enabled()) return;
  if (!document.getElementById('companion-connection-style')) {
    const style = document.createElement('link'); style.id = 'companion-connection-style';
    style.rel = 'stylesheet'; style.href = '/companion-archive.css'; document.head.append(style);
  }
  let dialog = document.getElementById('companion-download');
  if (dialog) { if (!dialog.open) dialog.showModal(); return; }
  dialog = document.createElement('dialog'); dialog.id = 'companion-download';
  dialog.innerHTML = `<form><h2>Private Companion download</h2><p>Windows x64 · signed private preview</p><section><label>Release-access token<input name="access" type="password" autocomplete="off" spellcheck="false" required></label><p>Use the private release token supplied by your preview administrator. This is separate from the Companion connection token. It is used only for this download and is not saved in your browser.</p></section><p role="status" aria-live="polite"></p><div class="actions"><button type="submit">Download for Windows</button><button type="button" data-close>Close</button></div><section><h3>Setup</h3><ol><li>Extract the ZIP into a permanent folder. Keep all its files together.</li><li>Run <code>current/ironwood-companion.exe</code>. If Windows displays an unknown-publisher warning, proceed only with this verified private-preview download. Our release signature does not establish Windows publisher trust.</li><li>In the Companion tray menu, select <strong>Show connection token</strong>. In Ironwood, enable Companion and use <strong>Manage connection</strong> to enter its address and connection token.</li><li>Your games and settings are stored separately from the installation. Stop Companion before moving or replacing its application folder.</li></ol><p>Automatic updates also require release access configured for the Windows account running Companion. The browser download does not configure or persist that credential.</p></section></form>`;
  document.body.append(dialog);
  const form = dialog.querySelector('form'), input = form.elements.access;
  const status = dialog.querySelector('[role=status]'), button = dialog.querySelector('[type=submit]');
  let closed = false;
  const close = () => { closed = true; input.value = ''; dialog.close(); dialog.remove(); window.removeEventListener('ironwood-companion-feature-changed', featureChanged); };
  const featureChanged = () => { if (!enabled()) close(); };
  window.addEventListener('ironwood-companion-feature-changed', featureChanged);
  dialog.querySelector('[data-close]').onclick = close;
  dialog.addEventListener('cancel', event => {event.preventDefault(); close();});
  form.onsubmit = async event => {
    event.preventDefault(); button.disabled = true;
    const token = input.value.trim(); input.value = '';
    status.textContent = 'Downloading and verifying the signed Windows package…';
    try {
      const result = await downloadPrivateCompanion(token, {allowed:() => !closed && enabled()});
      const url = URL.createObjectURL(new Blob([result.archive], {type:'application/zip'}));
      const link = document.createElement('a'); link.href = url; link.download = `ironwood-companion-${result.version}-windows-x64.zip`; link.click();
      setTimeout(() => URL.revokeObjectURL(url), 60000);
      status.textContent = `Verified Companion ${result.version}. Your Windows download is ready. Follow the setup steps below.`;
    } catch (error) {
      status.textContent = error instanceof TypeError ? 'Cannot reach the private release service. Check your connection or DNS and retry.' : error.message;
    } finally {button.disabled = false;}
  };
  dialog.showModal(); input.focus();
}
