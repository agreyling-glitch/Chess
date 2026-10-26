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
  window.open('/downloads/companion/', '_blank', 'noopener,noreferrer');
}
