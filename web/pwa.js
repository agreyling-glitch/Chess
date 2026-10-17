import './analytics.js';
import { isMobileDevice } from './device-support.js';
window.ironwoodHasDeferredUpdate = () => false;
window.ironwoodUpdateAndReload = () => {};
if (isMobileDevice()) {
  for (const button of document.querySelectorAll('[data-install-app], [data-offline-engine]')) button.hidden = true;
  const status = document.querySelector('[data-offline-status]');
  if (status) status.textContent = 'App installation and offline play require a desktop or laptop computer.';
} else {
const installButton = document.querySelector('[data-install-app]');
const offlineButton = document.querySelector('[data-offline-engine]');
const offlineProgress = document.querySelector('[data-offline-progress]');
const offlineStatus = document.querySelector('[data-offline-status]');
const runningAppVersion = '__APP_SHELL_VERSION__';
const runningPackageVersion = '__PACKAGE_VERSION__';
const runningEngineVersion = '__ENGINE_CACHE_VERSION__';
const isLocalhost = ['localhost', '127.0.0.1'].includes(window.location.hostname);
const previewUpdate = isLocalhost && new URLSearchParams(location.search).get('preview-update') === '1';
let installPrompt;
let registration;
let reloadForUpdate = false;
let pendingEngineUpdate = false;
let pendingAppVersion = '';
let waitingWorker;
let updateDeferred = false;
window.ironwoodHasDeferredUpdate = () => updateDeferred && Boolean(waitingWorker || registration?.waiting) && !reloadForUpdate;
window.ironwoodUpdateAndReload = () => {
  const waiting = waitingWorker || registration?.waiting;
  if (!waiting || reloadForUpdate) return;
  reloadForUpdate = true;
  updateNotice.querySelector('[data-update-reload]').disabled = true;
  updateNotice.querySelector('[data-update-message]').textContent = 'Finishing the update…';
  waiting.postMessage({ type: 'ACTIVATE_UPDATE' });
};

const versionDiagnostics = {
  status: 'Checking versions…',
  environment: isLocalhost ? 'Local development' : location.origin,
  app_version: runningAppVersion,
  package_version: runningPackageVersion,
  engine_version: runningEngineVersion,
  server_app_version: null,
  server_package_version: null,
  server_engine_version: null,
  service_worker_version: null,
  service_worker_engine_version: null,
  service_worker_state: isLocalhost ? 'Disabled for local development' : 'Checking…',
  online: navigator.onLine,
  cross_origin_isolated: window.crossOriginIsolated,
  browser: navigator.userAgent,
};

function summarizeVersionDiagnostics() {
  const serverKnown = Boolean(versionDiagnostics.server_app_version);
  const appMatches = versionDiagnostics.server_app_version === runningAppVersion;
  const packageMatches = versionDiagnostics.server_package_version === runningPackageVersion;
  const engineMatches = versionDiagnostics.server_engine_version === runningEngineVersion;
  const workerMatches = !versionDiagnostics.service_worker_version
    || versionDiagnostics.service_worker_version === runningAppVersion;
  if (!serverKnown) return navigator.onLine ? 'Server version unavailable' : 'Offline — server version unavailable';
  return appMatches && packageMatches && engineMatches && workerMatches
    ? 'All loaded components match the server'
    : 'Version mismatch detected — update or reload may be required';
}

async function refreshVersionDiagnostics() {
  versionDiagnostics.online = navigator.onLine;
  versionDiagnostics.cross_origin_isolated = window.crossOriginIsolated;
  try {
    const response = await fetch('/app-version.json', { cache: 'no-store' });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const latest = await response.json();
    versionDiagnostics.server_app_version = latest.version || null;
    versionDiagnostics.server_package_version = latest.package_version || null;
    versionDiagnostics.server_engine_version = latest.engine?.cache_version || null;
  } catch {
    versionDiagnostics.server_app_version = null;
    versionDiagnostics.server_package_version = null;
    versionDiagnostics.server_engine_version = null;
  }

  if (!isLocalhost && 'serviceWorker' in navigator) {
    const worker = registration?.active || navigator.serviceWorker.controller;
    versionDiagnostics.service_worker_state = registration?.waiting
      ? 'Update waiting'
      : registration?.installing
        ? `Installing (${registration.installing.state})`
        : worker
          ? worker.state || 'Active'
          : 'Not controlling this page';
    if (worker) {
      const details = await workerVersion(worker);
      versionDiagnostics.service_worker_version = details?.version || null;
      versionDiagnostics.service_worker_engine_version = details?.engine_cache_version || null;
    }
  }
  versionDiagnostics.status = summarizeVersionDiagnostics();
}

window.ironwoodVersionDiagnostics = () => JSON.stringify(versionDiagnostics);
window.ironwoodRefreshVersionDiagnostics = () => { void refreshVersionDiagnostics(); };
window.addEventListener('online', refreshVersionDiagnostics);
window.addEventListener('offline', refreshVersionDiagnostics);
void refreshVersionDiagnostics();

function reminderKey(version) {
  return `ironwood-update-remind-${version}`;
}

function remindAfter(version) {
  return Number(localStorage.getItem(reminderKey(version))) || 0;
}

function createUpdateNotice() {
  const notice = document.createElement('dialog');
  notice.className = 'ironwood-update';
  notice.setAttribute('aria-labelledby', 'ironwood-update-title');
  notice.innerHTML = `
    <div class="ironwood-update-emblem" aria-hidden="true">↻</div>
    <div class="ironwood-update-eyebrow">IRONWOOD CHESS</div>
    <h2 id="ironwood-update-title">An update is ready</h2>
    <p data-update-message>A new version is ready. Your current game is saved locally.</p>
    <div class="ironwood-update-actions">
      <button type="button" data-update-later>Later</button>
      <button type="button" data-update-reload>Update and reload</button>
    </div>`;
  const style = document.createElement('style');
  style.textContent = `
    .ironwood-update{box-sizing:border-box;width:min(32rem,calc(100vw - 32px));max-height:calc(100dvh - 32px);overflow:auto;padding:32px 30px 30px;border:3px solid #b99859;border-top-color:#f0d595;border-left-color:#d9ba79;border-bottom-color:#63502e;border-right-color:#80673b;border-radius:14px;color:#f3f0e4;background:radial-gradient(ellipse at 50% 0%,#303941 0%,#1b2228 55%,#141a1f 100%);box-shadow:inset 0 0 0 1px #17130d,inset 0 0 0 4px #d4ad6230,inset 0 5px 10px #ffffff0b,0 0 0 1px #0a0b0c,0 5px 0 #31291c,0 8px 0 1px #090b0d,0 28px 80px #000c;font:16px/1.5 Inter,system-ui,sans-serif;text-align:center}
    .ironwood-update::backdrop{background:#080a0cb8;backdrop-filter:blur(3px)}
    .ironwood-update-emblem{display:grid;place-items:center;width:54px;height:54px;margin:0 auto 17px;border:1px solid #d8b878;border-radius:50%;background:linear-gradient(145deg,#42484a,#161c20);box-shadow:inset 0 2px 2px #ffffff25,inset 0 -3px 5px #0009,0 3px 0 #080b0e,0 0 24px #d3ad6214;color:#eed49b;font-size:36px;line-height:1}
    .ironwood-update-eyebrow{color:#c7ab73;font-size:10px;font-weight:700;letter-spacing:.22em;margin-bottom:8px}
    .ironwood-update h2{margin:0 0 13px;font:600 26px/1.2 Inter,system-ui,sans-serif;letter-spacing:-.025em;color:#fff6df;text-shadow:0 2px 1px #000}
    .ironwood-update p{max-width:25rem;margin:0 auto;color:#bec6ca;font-size:15px;line-height:1.65;text-wrap:pretty}
    .ironwood-update-actions{display:flex;justify-content:center;align-items:stretch;gap:12px;margin-top:26px;padding-top:23px;border-top:1px solid #d3ad622c;box-shadow:0 -1px 0 #0008}
    .ironwood-update button{flex:1;min-width:0;min-height:46px;padding:11px 12px;border:1px solid #78858d;border-top-color:#a2adb3;border-bottom-color:#303a42;border-radius:7px;color:#edf0ef;background:linear-gradient(#505d66 0%,#36434d 48%,#2b3741 51%,#35424b 100%);box-shadow:inset 0 1px 0 #ffffff30,inset 0 -1px 0 #0005,0 4px 0 #11171c,0 5px 0 #6d7a8140,0 7px 10px #0006;font:600 14px/1.3 Inter,system-ui,sans-serif;cursor:pointer;text-align:center;text-shadow:0 1px 1px #0008;transition:filter .15s,transform .1s,box-shadow .1s}
    .ironwood-update [data-update-reload]{border-color:#bc9144;border-top-color:#ffebaf;border-bottom-color:#8b6427;background:linear-gradient(#f5dfa2 0%,#dfbe72 48%,#caa052 51%,#dbb96c 100%);color:#302312;text-shadow:0 1px 0 #fff7;box-shadow:inset 0 1px 0 #fff8,inset 0 -1px 0 #8b642766,0 4px 0 #755323,0 5px 0 #17110b,0 7px 10px #0006}
    .ironwood-update button:hover{filter:brightness(1.1)}
    .ironwood-update button:focus-visible{outline:2px solid #fff0c7;outline-offset:5px}
    .ironwood-update button:active{transform:translateY(3px);box-shadow:inset 0 2px 4px #0005,0 1px 0 #11171c}
    @media(max-width:380px){.ironwood-update{padding:25px 18px}.ironwood-update-actions{gap:9px}.ironwood-update button{font-size:13px;padding:10px 7px}}
`;
  document.head.append(style);
  document.body.append(notice);
  notice.querySelector('[data-update-later]').addEventListener('click', () => {
    if (pendingAppVersion && !previewUpdate) {
      localStorage.setItem(reminderKey(pendingAppVersion), String(Date.now() + 30 * 60_000));
    }
    updateDeferred = true;
    notice.close();
  });
  notice.addEventListener('cancel', event => {
    event.preventDefault();
    notice.querySelector('[data-update-later]').click();
  });
  notice.querySelector('[data-update-reload]').addEventListener('click', () => {
    window.ironwoodUpdateAndReload();
  });
  return notice;
}

const updateNotice = createUpdateNotice();

// Exercise the real dialog and deferred Help action without installing a worker.
if (previewUpdate) {
  waitingWorker = {
    postMessage() {
      const url = new URL(location.href);
      url.searchParams.delete('preview-update');
      location.replace(url.href);
    },
  };
  showUpdateNotice('local-update-preview');
}


function showUpdateNotice(version, engineChanged = false) {
  if (!version || version === runningAppVersion) return;
  pendingAppVersion = version;
  if (!previewUpdate && Date.now() < remindAfter(version)) {
    updateDeferred = true;
    return;
  }
  if (updateNotice.open) return;
  updateNotice.querySelector('[data-update-message]').textContent = engineChanged
    ? 'A new app and engine version is ready. Your game and existing offline engine remain safe. After reloading, you can explicitly download the new engine for offline play.'
    : 'A new version is ready. Your current game is saved locally, and your offline Stockfish engine will remain installed.';
  updateNotice.showModal();
}

function workerVersion(worker) {
  return new Promise(resolve => {
    const channel = new MessageChannel();
    const timeout = setTimeout(() => resolve(null), 3000);
    channel.port1.onmessage = ({ data }) => {
      clearTimeout(timeout);
      resolve(data?.type === 'APP_VERSION' ? data : null);
    };
    worker.postMessage({ type: 'GET_APP_VERSION' }, [channel.port2]);
  });
}

async function offerWaitingUpdate(worker = registration?.waiting) {
  if (!worker || !navigator.serviceWorker.controller) return;
  const version = await workerVersion(worker);
  if (!version) return;
  waitingWorker = worker;
  pendingEngineUpdate = version.engine_cache_version !== runningEngineVersion;
  showUpdateNotice(version.version, pendingEngineUpdate);
}

function formatBytes(value) {
  if (!value) return '';
  return `${(value / 1024 / 1024).toFixed(1)} MiB`;
}

function setOfflineState(cached) {
  if (!offlineButton || !offlineStatus) return;
  offlineButton.disabled = false;
  offlineButton.dataset.cached = String(cached);
  offlineButton.textContent = cached ? 'Remove offline engine' : 'Enable offline play';
  offlineStatus.textContent = cached
    ? 'Stockfish 19 is stored for offline games on this device.'
    : 'Store the 94.5 MiB engine on this device for play without a connection.';
  if (offlineProgress) {
    offlineProgress.hidden = true;
    offlineProgress.removeAttribute('value');
  }
}

window.addEventListener('beforeinstallprompt', event => {
  event.preventDefault();
  installPrompt = event;
  if (installButton) installButton.hidden = false;
});

window.addEventListener('appinstalled', () => {
  installPrompt = undefined;
  if (installButton) installButton.hidden = true;
});

installButton?.addEventListener('click', async () => {
  if (!installPrompt) return;
  await installPrompt.prompt();
  installPrompt = undefined;
  installButton.hidden = true;
});

offlineButton?.addEventListener('click', async () => {
  const active = registration?.active || navigator.serviceWorker.controller;
  if (!active) return;
  offlineButton.disabled = true;
  if (offlineButton.dataset.cached === 'true') {
    active.postMessage({ type: 'REMOVE_OFFLINE_ENGINE' });
    return;
  }
  offlineButton.textContent = 'Downloading Stockfish…';
  offlineStatus.textContent = 'Keep this page open while the engine is stored.';
  if (offlineProgress) {
    offlineProgress.hidden = false;
    offlineProgress.removeAttribute('value');
  }
  if (navigator.storage?.persist) await navigator.storage.persist();
  active.postMessage({ type: 'CACHE_OFFLINE_ENGINE' });
});

if ('serviceWorker' in navigator && isLocalhost) {
  const reloadKey = 'ironwood.local-service-worker-disabled';
  Promise.all([
    navigator.serviceWorker.getRegistrations().then(registrations =>
      Promise.all(registrations
        .filter(item => new URL(item.scope).origin === location.origin)
        .map(item => item.unregister()))
    ),
    'caches' in window ? caches.keys().then(names =>
      Promise.all(names
        .filter(name => name.startsWith('ironwood-shell-'))
        .map(name => caches.delete(name)))
    ) : Promise.resolve(),
  ]).then(() => {
    if (navigator.serviceWorker.controller && !sessionStorage.getItem(reloadKey)) {
      sessionStorage.setItem(reloadKey, '1');
      location.reload();
    } else if (!navigator.serviceWorker.controller) {
      sessionStorage.removeItem(reloadKey);
    }
  }).catch(error => console.warn('Ironwood could not disable local PWA caching:', error));
}

if ('serviceWorker' in navigator && !isLocalhost) {
  navigator.serviceWorker.addEventListener('controllerchange', () => {
    void refreshVersionDiagnostics();
    if (reloadForUpdate) window.location.reload();
  });

  navigator.serviceWorker.addEventListener('message', ({ data }) => {
    if (data?.type === 'OFFLINE_ENGINE_STATUS') setOfflineState(data.cached);
    if (data?.type === 'OFFLINE_ENGINE_PROGRESS' && offlineProgress) {
      if (data.total > 0) {
        offlineProgress.max = data.total;
        offlineProgress.value = data.loaded;
        offlineStatus.textContent = `${formatBytes(data.loaded)} of ${formatBytes(data.total)} stored`;
      }
    }
    if (data?.type === 'OFFLINE_ENGINE_READY') setOfflineState(true);
    if (data?.type === 'OFFLINE_ENGINE_ERROR') {
      setOfflineState(false);
      offlineStatus.textContent = `Offline download failed: ${data.message}`;
    }
  });

  navigator.serviceWorker.register('/service-worker.js', {
    scope: '/',
    updateViaCache: 'none',
  }).then(async value => {
    registration = value;
    void refreshVersionDiagnostics();
    const watchInstallingWorker = worker => worker?.addEventListener('statechange', () => {
      if (worker.state === 'installed' && navigator.serviceWorker.controller) {
        offerWaitingUpdate(worker);
      }
    });
    registration.addEventListener('updatefound', () => watchInstallingWorker(registration.installing));
    if (registration.waiting) {
      await offerWaitingUpdate();
    }
    await navigator.serviceWorker.ready;
    (registration.active || navigator.serviceWorker.controller)?.postMessage({
      type: 'CHECK_OFFLINE_ENGINE',
    });
    const checkForUpdate = async () => {
      if (document.hidden) return;
      try {
        const response = await fetch('/app-version.json', { cache: 'no-store' });
        if (!response.ok) return;
        const latest = await response.json();
        versionDiagnostics.server_app_version = latest.version || null;
        versionDiagnostics.server_package_version = latest.package_version || null;
        versionDiagnostics.server_engine_version = latest.engine?.cache_version || null;
        versionDiagnostics.status = summarizeVersionDiagnostics();
        if (latest.version !== runningAppVersion) {
          pendingEngineUpdate = latest.engine?.cache_version !== runningEngineVersion;
          await registration.update();
          if (registration.waiting) {
            await offerWaitingUpdate(registration.waiting);
          }
        }
      } catch { /* Offline players keep using the installed version. */ }
    };
    document.addEventListener('visibilitychange', checkForUpdate);
    window.addEventListener('focus', checkForUpdate);
    setInterval(checkForUpdate, 5 * 60_000);
    checkForUpdate();
  }).catch(error => {
    if (offlineStatus) offlineStatus.textContent = `Offline support is unavailable: ${error.message}`;
  });
}
}
