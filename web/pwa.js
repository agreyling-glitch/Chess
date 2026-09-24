const installButton = document.querySelector('[data-install-app]');
const offlineButton = document.querySelector('[data-offline-engine]');
const offlineProgress = document.querySelector('[data-offline-progress]');
const offlineStatus = document.querySelector('[data-offline-status]');
const runningAppVersion = '__APP_SHELL_VERSION__';
const runningPackageVersion = '__PACKAGE_VERSION__';
const runningEngineVersion = '__ENGINE_CACHE_VERSION__';
const isLocalhost = ['localhost', '127.0.0.1'].includes(window.location.hostname);
let installPrompt;
let registration;
let reloadForUpdate = false;
let pendingEngineUpdate = false;
let pendingAppVersion = '';
let waitingWorker;

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
    <h2 id="ironwood-update-title">Ironwood update available</h2>
    <p data-update-message>A new version is ready. Your current game is saved locally.</p>
    <div class="ironwood-update-actions">
      <button type="button" data-update-later>Later</button>
      <button type="button" data-update-reload>Update and reload</button>
    </div>`;
  const style = document.createElement('style');
  style.textContent = `
    .ironwood-update{max-width:31rem;padding:1.4rem;border:1px solid #d3ad6266;border-radius:8px;color:#f3f0e4;background:#181c20;box-shadow:0 20px 70px #000b;font:16px/1.5 Inter,system-ui,sans-serif}
    .ironwood-update::backdrop{background:#080a0cc0}
    .ironwood-update h2{margin:0 0 .65rem;font:600 1.5rem/1.2 Inter,system-ui,sans-serif;letter-spacing:0;color:#f3f0e4}
    .ironwood-update p{margin:0;color:#bdc1bc}
    .ironwood-update-actions{display:flex;justify-content:flex-end;gap:.7rem;margin-top:1.35rem}
    .ironwood-update button{padding:.65rem .9rem;border:1px solid #ffffff26;border-radius:4px;color:#f3f0e4;background:#292e32;font:600 .9rem Inter,system-ui,sans-serif;cursor:pointer}
    .ironwood-update [data-update-reload]{border-color:#d3ad62;background:#d3ad62;color:#17140e}`;
  document.head.append(style);
  document.body.append(notice);
  notice.querySelector('[data-update-later]').addEventListener('click', () => {
    if (pendingAppVersion) {
      localStorage.setItem(reminderKey(pendingAppVersion), String(Date.now() + 30 * 60_000));
    }
    notice.close();
  });
  notice.addEventListener('cancel', event => {
    event.preventDefault();
    notice.querySelector('[data-update-later]').click();
  });
  notice.querySelector('[data-update-reload]').addEventListener('click', () => {
    const waiting = waitingWorker || registration?.waiting;
    if (!waiting) return;
    reloadForUpdate = true;
    notice.querySelector('[data-update-reload]').disabled = true;
    notice.querySelector('[data-update-message]').textContent = 'Finishing the update…';
    waiting.postMessage({ type: 'ACTIVATE_UPDATE' });
  });
  return notice;
}

const updateNotice = createUpdateNotice();

function showUpdateNotice(version, engineChanged = false) {
  if (!version || version === runningAppVersion || Date.now() < remindAfter(version) || updateNotice.open) return;
  pendingAppVersion = version;
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
