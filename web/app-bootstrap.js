// Keep optional tools off the board's startup path. wasm-bindgen calls these
// window functions synchronously, including polls before a tool has loaded.
export function installDeferredTools(target, loadModule, asset = path => path) {
  install3dCompanion(target);
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
  if (target.ironwoodFicsCompanionMode()) {
    target.ironwoodFicsSendCommand = command => {
      try { return target.opener?.ironwoodQueueFicsCompanionCommand(command) ?? false; }
      catch { return false; }
    };
  }

  const lichess = tool('/lichess/bridge.js', 'Lichess');
  for (const method of ['Connect', 'SignIn', 'SignOut', 'Disconnect', 'Move', 'Command'])
    lichess.action(`ironwoodLichess${method}`);
  target.ironwoodLichessPoll = () => '';
  if (target.ironwoodLobbyCompanionMode?.()) {
    for (const method of ['Connect', 'SignIn', 'SignOut', 'Disconnect', 'Command']) {
      const name = `ironwoodLichess${method}`;
      target[name] = (...args) => {
        try { if (target.opener && !target.opener.closed) return target.opener[name](...args); } catch {}
      };
    }
  }
  if (target.ironwoodProfileCompanionMode()) {
    target.ironwoodLichessCommand = raw => {
      try {
        const cmd = JSON.parse(raw);
        if (!['playerProfile', 'following', 'setFollowing', 'performance', 'activity', 'closeProfileWindow'].includes(cmd.type)) return;
        if (!target.opener || target.opener.closed) return;
        target.opener.ironwoodLichessCommand(raw);
      } catch { /* The main window may have closed. */ }
    };
  }

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
  if (target.ironwoodBackgroundCompanionMode?.()) {
    for (const name of ['ironwoodBackgroundLibrary', 'ironwoodSelectBackground', 'ironwoodRemoveBackground', 'ironwoodUploadBackground']) {
      target[name] = (...args) => {
        try {
          if (target.opener && !target.opener.closed) return target.opener[name](...args);
        } catch {}
        return name === 'ironwoodBackgroundLibrary' ? '{"selected":"","images":[]}' : undefined;
      };
    }
  }
  return {
    lichess,
    afterPaint() {
      if (target.ironwoodCompanionMode()) return;
      // Restore signed-in sessions and saved backgrounds without holding the
      // board's first paint. The other tools load only when explicitly opened.
      void lichess.load().catch(error => target.console.warn('Ironwood Lichess is unavailable:', error));
      void background.load().catch(error => target.console.warn('Ironwood background images are unavailable:', error));
    },
  };
}

export function install3dCompanion(target) {
  const view = new URL(target.location.href).searchParams.get('view');
  const companion = view === '3d-companion' || view === 'fics-companion' || view === 'profile-companion' || view === 'training-companion' || view === 'theme-companion' || view === 'background-companion' || view === 'lobby-companion';
  let lobbyChild, lobbyState = '', lobbyActions = [];
  target.ironwoodLobbyCompanionMode = () => view === 'lobby-companion';
  target.ironwoodLobbyCompanionOpen = () => !!lobbyChild && !lobbyChild.closed;
  target.ironwoodCloseLobbyCompanion = () => { if (view === 'lobby-companion') target.close?.(); };
  target.ironwoodPublishLobby = value => { if (!companion) lobbyState = value; };
  target.ironwoodReadLobby = () => {
    try { return companion ? (target.opener && !target.opener.closed ? target.opener.ironwoodReadLobby() : '') : lobbyState; } catch { return ''; }
  };
  target.ironwoodQueueLobbyAction = action => {
    if (companion || !['leave','profile','resign'].includes(action) || lobbyActions.length >= 16) return false;
    lobbyActions.push(action); target.focus?.(); return true;
  };
  target.ironwoodLobbyActionInMain = action => {
    try { return !!target.opener && !target.opener.closed && target.opener.ironwoodQueueLobbyAction(action); } catch { return false; }
  };
  target.ironwoodTakeLobbyActions = () => { const actions = lobbyActions; lobbyActions = []; return JSON.stringify(actions); };
  target.ironwoodOpenLobbyCompanion = () => {
    if (lobbyChild && !lobbyChild.closed) { lobbyChild.focus(); return true; }
    const url = new URL(target.location.href); url.search = '?view=lobby-companion'; url.hash = '';
    lobbyChild = target.open(url.href, '_blank', 'popup,width=900,height=900,resizable=yes,scrollbars=no');
    if (!lobbyChild) target.alert('Allow pop-ups for Ironwood to open the Lichess lobby.');
    return !!lobbyChild;
  };
  let backgroundChild, backgroundChanges = [];
  target.ironwoodBackgroundCompanionMode = () => view === 'background-companion';
  target.ironwoodBackgroundCompanionOpen = () => !!backgroundChild && !backgroundChild.closed;
  target.ironwoodCloseBackgroundCompanion = () => { if (view === 'background-companion') target.close?.(); };
  target.ironwoodQueueBackgroundChange = value => {
    if (companion || typeof value !== 'string' || value.length > 100 || backgroundChanges.length >= 32) return false;
    backgroundChanges.push(value); return true;
  };
  target.ironwoodChangeBackgroundInMain = value => {
    try { return !!target.opener && !target.opener.closed && target.opener.ironwoodQueueBackgroundChange(value); } catch { return false; }
  };
  target.ironwoodTakeBackgroundChanges = () => { const changes = backgroundChanges; backgroundChanges = []; return JSON.stringify(changes); };
  target.ironwoodOpenBackgroundCompanion = () => {
    if (backgroundChild && !backgroundChild.closed) { backgroundChild.focus(); return true; }
    const url = new URL(target.location.href); url.search = '?view=background-companion'; url.hash = '';
    backgroundChild = target.open(url.href, '_blank', 'popup,width=780,height=800,resizable=yes,scrollbars=no');
    if (!backgroundChild) target.alert('Allow pop-ups for Ironwood to open Background.');
    return !!backgroundChild;
  };
  let themeChild, themeSelections = [];
  target.ironwoodThemeCompanionMode = () => view === 'theme-companion';
  target.ironwoodThemeCompanionOpen = () => !!themeChild && !themeChild.closed;
  target.ironwoodCloseThemeCompanion = () => { if (view === 'theme-companion') target.close?.(); };
  target.ironwoodQueueThemeSelection = value => {
    if (companion || typeof value !== 'string' || value.length > 1000 || themeSelections.length >= 16) return false;
    themeSelections.push(value);
    try { if (JSON.parse(value)[2] === true) target.focus?.(); } catch {}
    return true;
  };
  target.ironwoodSelectThemeInMain = value => {
    try { return !!target.opener && !target.opener.closed && target.opener.ironwoodQueueThemeSelection(value); } catch { return false; }
  };
  target.ironwoodTakeThemeSelections = () => { const selections = themeSelections; themeSelections = []; return JSON.stringify(selections); };
  target.ironwoodOpenThemeCompanion = () => {
    if (themeChild && !themeChild.closed) { themeChild.focus(); return true; }
    const url = new URL(target.location.href); url.search = '?view=theme-companion'; url.hash = '';
    themeChild = target.open(url.href, '_blank', 'popup,width=980,height=800,resizable=yes,scrollbars=no');
    if (!themeChild) target.alert('Allow pop-ups for Ironwood to open Board Theme.');
    return !!themeChild;
  };
  let trainingChild, trainingLive = false, trainingChanged = false, trainingStarts = [];
  target.ironwoodTrainingCompanionMode = () => view === 'training-companion';
  target.ironwoodTrainingCompanionOpen = () => !!trainingChild && !trainingChild.closed;
  target.ironwoodCloseTrainingCompanion = () => { if (view === 'training-companion') target.close?.(); };
  target.ironwoodPublishTrainingLive = value => { if (!companion) trainingLive = !!value; };
  target.ironwoodReadTrainingLive = () => {
    try { return view === 'training-companion' ? !!target.opener?.ironwoodReadTrainingLive() : trainingLive; } catch { return true; }
  };
  target.addEventListener?.('storage', event => { if (event.key === 'ironwood.chess.training.v1') trainingChanged = true; });
  target.ironwoodTakeTrainingChanged = () => { const changed = trainingChanged; trainingChanged = false; return changed; };
  target.ironwoodQueueTrainingStart = id => {
    if (companion || typeof id !== 'string' || id.length > 100 || trainingStarts.length >= 8) return false;
    trainingStarts.push(id); target.focus?.(); return true;
  };
  target.ironwoodStartTrainingInMain = id => {
    try { return target.opener && !target.opener.closed ? target.opener.ironwoodQueueTrainingStart(id) : false; } catch { return false; }
  };
  target.ironwoodTakeTrainingStarts = () => { const commands = trainingStarts; trainingStarts = []; return JSON.stringify(commands); };
  target.ironwoodOpenTrainingCompanion = () => {
    if (trainingChild && !trainingChild.closed) { trainingChild.focus(); return true; }
    const url = new URL(target.location.href); url.search = '?view=training-companion'; url.hash = '';
    trainingChild = target.open(url.href, '_blank', 'popup,width=900,height=850,resizable=yes,scrollbars=no');
    if (!trainingChild) target.alert('Allow pop-ups for Ironwood to open Training Profiles.');
    return !!trainingChild;
  };
  let child, ficsChild, state = '', ficsState = '', commands = [];
  let profileState = '', profileReviews = [];
  const profiles = new Map();
  target.ironwoodProfileCompanionMode = () => view === 'profile-companion';
  target.ironwoodCloseProfileCompanion = () => { if (view === 'profile-companion') target.close?.(); };
  const profileId = new URL(target.location.href).searchParams.get('profile');
  target.ironwoodPublishProfileState = value => { if (!companion) profileState = value; };
  target.ironwoodReadProfileState = () => {
    if (!companion) return profileState;
    try { return target.opener && !target.opener.closed ? target.opener.ironwoodReadProfileState() : ''; } catch { return ''; }
  };
  target.ironwoodProfileCompanionInitial = () => {
    try { return target.opener && !target.opener.closed ? target.opener.ironwoodProfileInitial(profileId) : ''; } catch { return ''; }
  };
  target.ironwoodProfileInitial = id => profiles.get(id)?.initial || '';
  target.ironwoodProfileCompanionOpen = () => [...profiles.values()].some(p => !p.child.closed);
  target.ironwoodOpenProfileCompanion = raw => {
    if (companion) return false;
    let data; try { data = JSON.parse(raw); } catch { return false; }
    if (!Number.isSafeInteger(data.id)) return false;
    const id = String(data.id), existing = profiles.get(id);
    if (existing && !existing.child.closed) { existing.child.focus(); return true; }
    const url = new URL(target.location.href);
    url.search = '?view=profile-companion&profile=' + encodeURIComponent(id); url.hash = '';
    const child = target.open(url.href, '_blank', 'popup,width=1100,height=850,resizable=yes,scrollbars=no');
    if (!child) { target.alert('Allow pop-ups for Ironwood to open the player profile.'); return false; }
    profiles.set(id, { child, initial: raw }); return true;
  };
  target.ironwoodQueueProfileReview = pgn => {
    if (companion || typeof pgn !== 'string' || pgn.length > 200000 || profileReviews.length >= 8) return false;
    profileReviews.push(pgn); target.focus?.(); return true;
  };
  target.ironwoodReviewProfileInMain = pgn => {
    try { return target.opener && !target.opener.closed ? target.opener.ironwoodQueueProfileReview(pgn) : false; } catch { return false; }
  };
  target.ironwoodTakeProfileReviews = () => { const pending = profileReviews; profileReviews = []; return JSON.stringify(pending); };
  target.ironwoodCompanionMode = () => companion;
  target.ironwoodFicsCompanionMode = () => view === 'fics-companion';
  target.ironwoodPublishFicsCompanion = value => { if (!companion) ficsState = value; };
  target.ironwoodReadFicsCompanion = () => {
    if (!companion) return ficsState;
    try { return target.opener && !target.opener.closed ? target.opener.ironwoodReadFicsCompanion() : ''; }
    catch { return ''; }
  };
  target.ironwoodQueueFicsCompanionCommand = command => {
    if (companion || typeof command !== 'string' || !/^[\x20-\x7e]{1,256}$/.test(command) || commands.length >= 32) return false;
    commands.push(command);
    return true;
  };
  target.ironwoodTakeFicsCompanionCommands = () => {
    const pending = commands; commands = []; return JSON.stringify(pending);
  };
  target.ironwoodFicsCompanionOpen = () => !!ficsChild && !ficsChild.closed;
  target.ironwoodOpenFicsCompanion = () => {
    if (ficsChild && !ficsChild.closed) { ficsChild.focus(); return true; }
    const url = new URL(target.location.href);
    url.search = '?view=fics-companion'; url.hash = '';
    ficsChild = target.open(url.href, '_blank', 'popup,width=760,height=620,resizable=yes,scrollbars=no');
    if (!ficsChild) target.alert('Allow pop-ups for Ironwood to open the FICS console.');
    return !!ficsChild;
  };
  target.ironwoodPublish3dPosition = value => { if (!companion) state = value; };
  target.ironwoodRead3dPosition = () => {
    if (!companion) return state;
    try {
      return target.opener && !target.opener.closed
        ? target.opener.ironwoodRead3dPosition() : '';
    } catch { return ''; }
  };
  target.ironwoodOpen3dCompanion = () => {
    if (child && !child.closed) { child.focus(); return true; }
    const url = new URL(target.location.href);
    url.search = '?view=3d-companion';
    url.hash = '';
    child = target.open(url.href, '_blank', 'popup,width=1000,height=900,resizable=yes,scrollbars=no');
    if (!child) {
      const notice = target.document.createElement('div');
      notice.setAttribute('role', 'alert');
      notice.textContent = 'Allow pop-ups for Ironwood to open the 3D view.';
      notice.style.cssText = 'position:fixed;bottom:24px;left:24px;z-index:9999;padding:16px;background:#282f35;color:#ecebe4;font:16px system-ui';
      target.document.body.append(notice);
      target.setTimeout(() => notice.remove(), 8000);
    }
    return !!child;
  };
  if (companion) target.document.title = view === 'lobby-companion' ? 'Ironwood — Lichess lobby' : view === 'background-companion' ? 'Ironwood — Background' : view === 'theme-companion' ? 'Ironwood — Board Theme' : view === 'training-companion' ? 'Ironwood — Training profiles' : view === 'profile-companion' ? 'Ironwood — Player profile' : view === 'fics-companion' ? 'Ironwood — FICS console' : 'Ironwood — 3D view';
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
  target.ironwoodShowProfileGamePreview ??= () => target.alert('Move preview is unavailable. Please reload Ironwood.');
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
      if (target.ironwoodCompanionMode())
        target.document.title = target.ironwoodLobbyCompanionMode() ? 'Ironwood — Lichess lobby' : target.ironwoodBackgroundCompanionMode() ? 'Ironwood — Background' : target.ironwoodThemeCompanionMode() ? 'Ironwood — Board Theme' : target.ironwoodTrainingCompanionMode() ? 'Ironwood — Training profiles' : target.ironwoodProfileCompanionMode() ? 'Ironwood — Player profile' : target.ironwoodFicsCompanionMode() ? 'Ironwood — FICS console' : 'Ironwood — 3D view';
      if (local && signalsFrame) {
        target.document.body.dataset.boardReadyMs = target.performance.now().toFixed(1);
        target.document.body.dataset.boardReadyResources = recordResources();
      }
      tools.afterPaint();
      if (!target.ironwoodCompanionMode())
        void loadModule(asset('/pwa.js')).catch(error => target.console.warn('Ironwood offline support is unavailable:', error));
    });
    return true;
  } catch (error) {
    target.console.error('Ironwood could not start:', error);
    if (loading) loading.textContent = 'Ironwood could not start. Please reload the page.';
    return false;
  }
}
