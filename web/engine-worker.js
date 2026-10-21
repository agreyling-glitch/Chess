/* Dedicated UCI boundary. The Stockfish glue creates its pthread workers here,
   leaving the browser UI worker-free and responsive. */
let engine;
let ready = false;
const pending = [];
const resourceOptions = new Map();
const advertisedExtras = new Set();
const managedOptions = new Set(['Threads','Hash','MultiPV','Ponder','Skill Level','UCI_LimitStrength','UCI_Elo','UCI_Chess960']);

function emit(line) { postMessage({ type: 'uci', line }); }
function dispatch(command) {
  // Stockfish rebuilds threads and clears its hash even when these values
  // have not changed. Keep explicit search/reset commands in their UCI order.
  const option = /^setoption name (Threads|Hash) value ([1-9]\d*)$/.exec(command);
  if (option && resourceOptions.get(option[1]) === option[2]) return;
  engine.postMessage(command);
  if (option) resourceOptions.set(option[1], option[2]);
}
function send(command) {
  if (!ready) pending.push(command);
  else dispatch(command);
}

onmessage = ({ data }) => {
  if (data?.type === 'init') {
    ready = false;
    resourceOptions.clear();
    advertisedExtras.clear();
    if (!crossOriginIsolated || typeof SharedArrayBuffer === 'undefined') {
      postMessage({ type: 'error', message: 'Cross-origin isolation is required for multi-thread Stockfish.' });
      return;
    }
    const engineWasm = new URL('./engine/stockfish-19.wasm', self.location.href);
    const engineScript = new URL('./engine/stockfish-19-ironwood-pool-v1.js', self.location.href);
    // Local previews can be restarted mid-download; never reuse a partial HTTP entry.
    if (['localhost', '127.0.0.1'].includes(engineWasm.hostname)) {
      engineWasm.searchParams.set('local', Date.now().toString());
      engineScript.searchParams.set('local', Date.now().toString());
    }
    engineScript.hash = encodeURIComponent(engineWasm.href);
    engine = new Worker(engineScript.href);
    engine.onerror = event => postMessage({ type: 'error', message: `Stockfish worker failed: ${event.message}` });
    const progressChannel = new MessageChannel();
    progressChannel.port1.onmessage = ({ data: progress }) => {
      postMessage({ type: 'progress', ...progress });
    };
    engine.postMessage({ progressPort: progressChannel.port2 }, [progressChannel.port2]);
    engine.onmessage = event => {
      const line = typeof event === 'string' ? event : event.data;
      const advertised = /^option name (.+?) type (spin|check|combo)\b/.exec(line);
      if (advertised && !managedOptions.has(advertised[1])) advertisedExtras.add(advertised[1]);
      if (line === 'uciok') {
        // Do not publish readiness or apply synchronous thread settings until
        // every supported pthread has loaded its WASM module.
        engine.postMessage({ prepareThreads: 8 });
        return;
      }
      if (line === 'ironwood-thread-pool-ready') {
        ready = true;
        send(`setoption name Threads value ${Math.max(1, Math.min(data.threads || 2, 8))}`);
        send(`setoption name Hash value ${data.hash || 64}`);
        send(`setoption name Skill Level value ${Math.max(0, Math.min(data.skillLevel ?? 20, 20))}`);
        send(`setoption name UCI_LimitStrength value ${data.limitStrength ? 'true' : 'false'}`);
        send(`setoption name UCI_Elo value ${Math.max(1320, Math.min(data.elo || 1800, 3190))}`);
        send('setoption name Ponder value false');
        for (const [name, value] of Object.entries(data.uciOptions || {})) {
          if (advertisedExtras.has(name) && !/[\r\n\0]/.test(name + value)) send(`setoption name ${name} value ${value}`);
        }
        while (pending.length) send(pending.shift());
        emit('uciok');
        return;
      }
      emit(line);
    };
    engine.postMessage('uci');
  } else if (data?.type === 'command') send(data.command);
};
