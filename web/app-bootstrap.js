// Keep optional tools off the board's startup path. wasm-bindgen calls these
// window functions synchronously, including polls before a tool has loaded.
export function installDeferredTools(target, loadModule, asset = path => path) {
  const document = target.document;
  const notice = (message, role) => {
    const status = document.createElement('div');
    status.setAttribute('role', role);
    status.textContent = message;
    status.style.cssText = 'position:fixed;bottom:24px;left:50%;transform:translateX(-50%);z-index:9999;padding:14px 20px;background:#282f35;color:#ecebe4;border:1px solid #d3ad62;border-radius:6px;font:16px system-ui';
    document.body.append(status);
    return status;
  };
  const report = (name, error) => {
    target.console.warn(`Ironwood ${name} is unavailable:`, error);
    const status = notice(`Could not load ${name}. Try opening it again, or reload Ironwood.`, 'alert');
    target.setTimeout(() => status.remove(), 8000);
  };
  const tool = (path, name) => {
    let pending, attempts = 0;
    const load = () => pending ||= Promise.resolve().then(() => {
      const url = asset(path);
      // Use a fresh URL on retry for browsers that retain failed module loads;
      // successful loads stay shared and never initialize a second client.
      const retry = ++attempts > 1 ? `${url.includes('?') ? '&' : '?'}retry=${attempts}` : '';
      return loadModule(url + retry);
    }).catch(error => {
      pending = undefined;
      throw error;
    });
    const action = method => {
      const deferred = (...args) => {
        const status = notice(`Loading ${name}…`, 'status');
        void load().then(() => {
          if (target[method] === deferred) throw new Error(`Missing ${method}`);
          return target[method](...args);
        }).catch(error => report(name, error)).finally(() => status.remove());
      };
      target[method] = deferred;
    };
    return { load, action };
  };
  const fics = tool('/fics/bridge.js', 'FICS');
  for (const method of ['Connect', 'ConnectRegistered', 'Disconnect', 'Send', 'SendCommand'])
    fics.action(`ironwoodFics${method}`);
  target.ironwoodFicsPoll = () => '';

  const lichess = tool('/lichess/bridge.js', 'Lichess');
  for (const method of ['Connect', 'SignIn', 'SignOut', 'Disconnect', 'Move', 'Command'])
    lichess.action(`ironwoodLichess${method}`);
  target.ironwoodLichessPoll = () => '';

  const editor = tool('/position-editor/editor.js', 'Position Editor');
  editor.action('ironwoodOpenPositionEditor');
  target.ironwoodPollPositionEditor = () => '';
  const scoresheet = tool('/scoresheet-import.js', 'scoresheet entry');
  scoresheet.action('ironwoodOpenScoresheet');
  target.ironwoodPollScoresheet = () => '';

  const pieces = tool('/piece-themes.js', 'piece artwork');
  target.ironwoodLoadPieceTheme = theme => pieces.load().then(module => module.loadPieceTheme(theme));
  const boards = tool('/board-themes.js', '3D board artwork');
  target.ironwoodLoadBoardTheme = theme => boards.load().then(module => module.loadBoardTheme(theme));

  const tablebase = tool('/tablebase.js', 'tablebases');
  let tablebasePending = false, tablebaseArgs, tablebaseError = '';
  target.ironwoodTablebase = (...args) => {
    tablebaseArgs = args;
    if (tablebaseError && !args[1]) return tablebaseError;
    if (!tablebasePending) {
      tablebasePending = true;
      tablebaseError = '';
      void tablebase.load().then(() => target.ironwoodTablebase(...tablebaseArgs)).catch(() => {
        tablebaseError = JSON.stringify({ error: 'Could not load tablebases. Choose Retry to try again.' });
      }).finally(() => { tablebasePending = false; });
    }
    return '';
  };

  const puzzles = tool('/puzzles.js', 'puzzles');
  let puzzleState = {}, puzzlePending = false;
  target.ironwoodPuzzles = (command, payload = '') => {
    // Rust reads history once when the puzzle panel is created. Preserve that
    // synchronous read (and writes) even while the network client is unloaded.
    if (command === 'history') {
      try { return target.localStorage.getItem('ironwood.puzzles.history.v1') || '[]'; }
      catch { return '[]'; }
    }
    if (command === 'save') {
      try { target.localStorage.setItem('ironwood.puzzles.history.v1', payload); return ''; }
      catch { return 'Progress could not be saved on this device. It remains available for this session.'; }
    }
    if (command === 'request' && !puzzlePending) {
      puzzlePending = true;
      puzzleState = { loading: true };
      void puzzles.load().then(() => target.ironwoodPuzzles(command, payload)).catch(() => {
        puzzleState = { error: 'Could not load puzzles. Request a puzzle to try again.' };
      }).finally(() => { puzzlePending = false; });
    }
    return JSON.stringify(puzzleState);
  };

  const background = tool('/background-image.js', 'background images');
  target.ironwoodBackgroundLibrary = () => '{"selected":"","images":[]}';
  target.ironwoodTakeBackground = () => new Uint8Array();
  target.ironwoodBackgroundWidth = target.ironwoodBackgroundHeight = () => 0;
  background.action('ironwoodSelectBackground');
  background.action('ironwoodRemoveBackground');
  target.ironwoodUploadBackground = () => {
    // File pickers, like clipboard writes, must start during the original click.
    const input = document.createElement('input');
    input.type = 'file'; input.accept = 'image/png,image/jpeg,image/webp';
    input.onchange = () => {
      const file = input.files?.[0];
      if (file) void background.load().then(module => module.uploadBackgroundFile(file))
        .catch(error => report('background images', error));
    };
    input.click();
  };
  return {
    lichess,
    afterPaint() {
      // Restore signed-in sessions and saved backgrounds without holding the
      // board's first paint. The other tools load only when explicitly opened.
      void lichess.load().catch(error => target.console.warn('Ironwood Lichess is unavailable:', error));
      void background.load().catch(error => target.console.warn('Ironwood background images are unavailable:', error));
    },
  };
}

function installFallbacks(target) {
  for (const name of ['StoreCurrentGame', 'StoreObservedGame', 'StoreImportedGame', 'StartNewStoredGame',
    'EnsureImportedIndex', 'RefreshStorageStatus', 'BeginBatchImport', 'EndBatchImport',
    'RefreshVersionDiagnostics', 'UpdateAndReload', 'TrackUsage', 'ExplorerRetry', 'ExplorerRefresh'])
    target[`ironwood${name}`] ??= () => {};
  for (const name of ['IsImportedDuplicate', 'ImportedIndexReady', 'HasDeferredUpdate', 'ExplorerSignedIn'])
    target[`ironwood${name}`] ??= () => false;
  target.ironwoodStorageStatus ??= () => 'Browser storage estimate unavailable';
  target.ironwoodTrainingProfilesAction ??= () => JSON.stringify({error:'Training profile storage is unavailable. Reload and try again.'});
  target.ironwoodBatchImportStatus ??= () => '{"pending":0,"saved":0,"failed":0,"firstError":null}';
  target.ironwoodVersionDiagnostics ??= () => '{"status":"Version diagnostics are loading"}';
  target.ironwoodExplorer ??= () => '{"error":"Opening Explorer is unavailable. Please reload Ironwood."}';
  target.ironwoodCopyBoard ??= () => target.alert('Image clipboard is unavailable. Please reload Ironwood.');
  target.ironwoodBeginBoardPngCopy ??= () => false;
  target.ironwoodFinishBoardPngCopy ??= () => {};
  target.ironwoodFailBoardPngCopy ??= () => {};
  for (const name of ['FetchLichess', 'FetchChessCom']) target[`ironwood${name}`] ??= () => {};
  for (const name of ['PollLichess', 'PollChessCom']) target[`ironwood${name}`] ??= () => '';
  for (const name of ['LichessStatus', 'ChessComStatus'])
    target[`ironwood${name}`] ??= () => 'Game import is unavailable. Please reload Ironwood.';
  for (const name of ['OpenGameLibrary', 'BackupStorage', 'RestoreStorage', 'OpenStorageInfo', 'ClearSavedGames', 'ResetLocalData'])
    target[`ironwood${name}`] ??= () => target.alert('Saved game storage is unavailable. Please reload Ironwood.');
}

export async function compileAppWasm(url, fetcher = fetch, wasm = WebAssembly) {
  const response = await fetcher(url);
  if (!response.ok) throw new Error(`Could not fetch the chess app (${response.status})`);
  // Compilation has no app side effects; instantiation must wait for restored
  // storage and all synchronous bindings. Retain non-WASM-MIME server support.
  if (typeof wasm.compileStreaming === 'function' && response.headers.get('Content-Type')?.split(';')[0].trim() === 'application/wasm')
    return wasm.compileStreaming(response);
  return wasm.compile(await response.arrayBuffer());
}

export async function startChessApp({
  target = window,
  loadModule = url => import(url),
  compile = url => compileAppWasm(url),
  afterPaint = () => new Promise(resolve => target.requestAnimationFrame(() => target.requestAnimationFrame(resolve))),
} = {}) {
  const local = ['localhost', '127.0.0.1'].includes(target.location.hostname);
  const token = Date.now();
  const asset = path => local ? `${path}?v=${token}` : path;
  const loading = target.document.getElementById('loading');
  installFallbacks(target);
  const tools = installDeferredTools(target, loadModule, asset);
  const firstFrame = new Promise(resolve => { target.ironwoodFirstFrame = resolve; });
  const recordResources = () => JSON.stringify(
    (target.performance.getEntriesByType?.('resource') || []).map(entry => ({
      path: new URL(entry.name, target.location.href).pathname,
      transferBytes: entry.transferSize, bodyBytes: entry.decodedBodySize,
    })));
  try {
    const callback = new URLSearchParams(target.location.search);
    const [app, wasm] = await Promise.all([
      loadModule(asset('/pkg/battle_chess.js')),
      compile(asset('/pkg/battle_chess_bg.wasm')),
      loadModule(asset('/game-storage.js')).then(module => module.initializeGameStorage())
        .catch(error => target.console.warn('Ironwood game library is unavailable:', error)),
      // Clipboard entry points must be ready before any possible board click;
      // deferring them through import() would lose transient user activation.
      loadModule(asset('/board-clipboard.js'))
        .catch(error => target.console.warn('Ironwood image clipboard is unavailable:', error)),
      // OAuth initialization removes one-use URL credentials before Rust/PWA
      // diagnostics start. It does not wait for the remote token exchange.
      callback.has('code') || callback.has('error') ? tools.lichess.load() : undefined,
    ]);
    await app.default({ module_or_path: wasm });
    target.ironwoodExportSavedGame = app.export_saved_game;
    target.ironwoodValidateScoresheet = app.validate_scoresheet_pgn;
    target.ironwoodPreviewScoresheet = app.preview_scoresheet;
    loading?.remove();
    // wasm-bindgen's async Rust start returns before WebRunner has initialized
    // its painter. Keep the historical init checkpoint for local comparisons,
    // but wait for the first Rust UI frame before starting optional work.
    const initializedPaint = afterPaint().then(() => {
      if (local) {
        target.document.body.dataset.startupReadyMs = target.performance.now().toFixed(1);
        target.document.body.dataset.startupResources = recordResources();
      }
    });
    const signalsFrame = wasm instanceof WebAssembly.Module && WebAssembly.Module.imports(wasm)
      .some(binding => binding.name.includes('ironwoodFirstFrame'));
    // A controlling service worker can still serve the previous WASM package
    // during an update. Its legacy startup must still load PWA/update support.
    const paintedBoard = signalsFrame ? firstFrame.then(() => afterPaint()) : initializedPaint;
    void paintedBoard.then(() => {
      if (local && signalsFrame) {
        target.document.body.dataset.boardReadyMs = target.performance.now().toFixed(1);
        target.document.body.dataset.boardReadyResources = recordResources();
      }
      tools.afterPaint();
      void loadModule(asset('/pwa.js')).catch(error => target.console.warn('Ironwood offline support is unavailable:', error));
    });
    return true;
  } catch (error) {
    target.console.error('Ironwood could not start:', error);
    if (loading) loading.textContent = 'Ironwood could not start. Please reload the page.';
    return false;
  }
}
