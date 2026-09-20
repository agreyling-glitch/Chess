/* Dedicated UCI boundary. The Stockfish glue creates its pthread workers here,
   leaving the browser UI worker-free and responsive. */
let engine;
let ready = false;
const pending = [];

function emit(line) { postMessage({ type: 'uci', line }); }
function send(command) {
  if (!ready) pending.push(command);
  else engine.postMessage(command);
}

onmessage = ({ data }) => {
  if (data?.type === 'init') {
    if (!crossOriginIsolated || typeof SharedArrayBuffer === 'undefined') {
      postMessage({ type: 'error', message: 'Cross-origin isolation is required for multi-thread Stockfish.' });
      return;
    }
    const engineWasm = new URL('./engine/stockfish-19.wasm', self.location.href);
    engine = new Worker(`./engine/stockfish-19.js#${encodeURIComponent(engineWasm.href)}`);
    engine.onerror = event => postMessage({ type: 'error', message: `Stockfish worker failed: ${event.message}` });
    const progressChannel = new MessageChannel();
    progressChannel.port1.onmessage = ({ data: progress }) => {
      postMessage({ type: 'progress', ...progress });
    };
    engine.postMessage({ progressPort: progressChannel.port2 }, [progressChannel.port2]);
    engine.onmessage = event => {
      const line = typeof event === 'string' ? event : event.data;
      emit(line);
      if (line === 'uciok') {
        ready = true;
        engine.postMessage(`setoption name Threads value ${Math.max(1, Math.min(data.threads || 2, 8))}`);
        engine.postMessage(`setoption name Hash value ${data.hash || 64}`);
        while (pending.length) engine.postMessage(pending.shift());
      }
    };
    engine.postMessage('uci');
  } else if (data?.type === 'command') send(data.command);
};
