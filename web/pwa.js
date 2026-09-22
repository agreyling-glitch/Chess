const installButton = document.querySelector('[data-install-app]');
const offlineButton = document.querySelector('[data-offline-engine]');
const offlineProgress = document.querySelector('[data-offline-progress]');
const offlineStatus = document.querySelector('[data-offline-status]');
let installPrompt;
let registration;

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

if ('serviceWorker' in navigator) {
  const isLocalhost = ['localhost', '127.0.0.1'].includes(window.location.hostname);
  const hadController = Boolean(navigator.serviceWorker.controller);
  let reloadingForUpdate = false;
  navigator.serviceWorker.addEventListener('controllerchange', () => {
    // Local development deliberately installs a fresh worker between sessions.
    // Never auto-reload there: doing so can turn worker activation into a loop.
    if (isLocalhost || !hadController || reloadingForUpdate) return;
    reloadingForUpdate = true;
    window.location.reload();
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

  const localWorkerRevision = isLocalhost ? String(Date.now()) : '';
  const serviceWorkerUrl = isLocalhost
    ? `/service-worker.js?dev=${localWorkerRevision}`
    : '/service-worker.js';
  navigator.serviceWorker.register(serviceWorkerUrl, {
    scope: '/',
    updateViaCache: 'none',
  }).then(async value => {
    registration = value;
    await navigator.serviceWorker.ready;
    (registration.active || navigator.serviceWorker.controller)?.postMessage({
      type: 'CHECK_OFFLINE_ENGINE',
    });
  }).catch(error => {
    if (offlineStatus) offlineStatus.textContent = `Offline support is unavailable: ${error.message}`;
  });
}
