import { CompanionEngineAdapter } from './companion-engine-adapter.js';
let adapter, ready, pending = [];
self.onmessage = ({ data }) => {
  if (data?.type === 'init') {
    adapter = new CompanionEngineAdapter(line => postMessage({ type: 'uci', line }), message => postMessage({ type: 'error', message }));
    if (!data.companion) { adapter.fatal(data.companionError || 'Connect to Companion in Engine settings, then choose Retry connection.'); return; }
    ready = adapter.connect(data.companion).then(() => {
      for (const command of pending) adapter.command(command);
      pending = [];
    }).catch(error => adapter.fatal(error.message));
  } else if (data?.type === 'command') {
    if (!adapter?.session) pending.push(data.command);
    else adapter.command(data.command);
  }
};
