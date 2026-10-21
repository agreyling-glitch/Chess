import { openCompanionDownload } from './companion-download.js';
import { companionConnection, onCompanionConnectionChange } from './companion-connection.js';
import { CompanionArchiveClient } from './companion-archive-client.js';

// No credential, address, or update download URL enters the diagnostic snapshot.
export function createCompanionDiagnostics({ connection = companionConnection, enabled = () => window.ironwoodCompanionFeatureEnabled(), client = value => new CompanionArchiveClient(value.endpoint, value.credential), clock = Date.now } = {}) {
  let state = { connection_status: 'Disconnected', version: '', message: 'Connect to Companion to view its version and updates.', can_check: false, can_install: false, busy: false };
  let generation = 0, last = 0, restarting = 0;
  const reset = () => { generation++; last = 0; restarting = 0; state = { connection_status: 'Disconnected', version: '', message: 'Connect to Companion to view its version and updates.', can_check: false, can_install: false, busy: false }; };
  const stillCurrent = (attempt, current) => enabled() && attempt === generation && connection()?.endpoint === current.endpoint && connection()?.credential === current.credential;
  async function refresh(action) {
    if (!enabled() || state.busy) return;
    const current = connection();
    if (!current) { reset(); last = clock(); return; }
    const attempt = generation;
    state = { ...state, busy: true, can_check: false, can_install: false };
    last = clock();
    try {
      const api = client(current);
      const capabilities = await api.request('capabilities');
      if (!stillCurrent(attempt, current)) return;
      if (capabilities.product !== 'Ironwood Companion' || capabilities.protocol_version !== '1') throw Error('Unsupported Companion server or protocol.');
      state = { ...state, connection_status: 'Connected', version: capabilities.server_version || 'Unknown' };
      let updates;
      try {
        if (action === 'install') restarting = clock();
        updates = await api.request(action ? `updates/${action}` : 'updates', action ? {} : undefined);
      }
      catch (error) {
        if (!error.status) throw error;
        if (action === 'install') restarting = 0;
        if (error.status === 404) {
          if (stillCurrent(attempt, current)) state = { ...state, message: 'This Companion version does not support update controls. Update it manually first.', busy: false, can_check: false, can_install: false };
          return;
        }
        // An idle conflict is an update error, not a disconnected server.
        if (stillCurrent(attempt, current)) state = { ...state, message: error.message, busy: false, can_check: !action || action !== 'install', can_install: false };
        return;
      }
      if (!stillCurrent(attempt, current)) return;
      if (action === 'install') restarting = clock();
      else if (updates.state !== 'installing') restarting = 0;
      state = { ...state, message: updates.message || updates.state, update_state: updates.state, management: updates.management, available_version: updates.available_version || '', can_check: !!updates.can_check, can_install: !!updates.can_install, busy: false };
    } catch {
      if (!stillCurrent(attempt, current)) return;
      const waiting = restarting && clock() - restarting < 120000;
      state = { ...state, connection_status: waiting ? 'Restarting' : 'Disconnected', message: waiting ? 'Companion is restarting. Waiting for it to reconnect…' : 'Companion is unavailable. Start it or manage the connection, then refresh.', busy: false, can_check: false, can_install: false };
      if (!waiting) restarting = 0;
    } finally { if (stillCurrent(attempt, current)) state.busy = false; }
  }
  function snapshot() {
    if (!enabled()) return '{}';
    const interval = restarting || ['checking', 'installing'].includes(state.update_state) ? 1000 : 15000;
    if (!last || clock() - last >= interval) void refresh();
    return JSON.stringify(state);
  }
  return { snapshot, refresh: () => refresh(), check: () => refresh('check'), install: () => refresh('install'), reset };
}
if (typeof window !== 'undefined') {
  const diagnostics = createCompanionDiagnostics();
  onCompanionConnectionChange(diagnostics.reset);
  window.addEventListener('ironwood-companion-feature-changed', diagnostics.reset);
  window.ironwoodDownloadCompanion = openCompanionDownload;
  window.ironwoodCompanionDiagnostics = diagnostics.snapshot;
  window.ironwoodRefreshCompanionDiagnostics = diagnostics.refresh;
  window.ironwoodCheckCompanionUpdates = diagnostics.check;
  window.ironwoodInstallCompanionUpdate = diagnostics.install;
}
