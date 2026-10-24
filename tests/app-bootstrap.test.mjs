import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { JSDOM } from 'jsdom';

const source = readFileSync(new URL('../web/app-bootstrap.js', import.meta.url), 'utf8');
const { startChessApp, installDeferredTools, compileAppWasm, install3dCompanion } = await import(new URL('../web/app-bootstrap.js', import.meta.url));
const flush = () => new Promise(resolve => setImmediate(resolve));
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
function environment(url = 'http://localhost/play/') {
  const dom = new JSDOM('<body><canvas id="chess_canvas"></canvas><div id="loading">Loading</div></body>', { url });
  const warnings = [];
  const target = {
    document: dom.window.document, location: dom.window.location, localStorage: dom.window.localStorage,
    performance: { now: () => 123.4 }, setTimeout: () => 0,
    console: { warn: (...args) => warnings.push(args), error: (...args) => warnings.push(args) },
    alert: message => warnings.push(message), ironwoodPlayChessSound: () => {},
  };
  return { dom, target, warnings };
}

test('3D companion follows only its opener and reuses an open window', () => {
  const main = environment(), other = environment(), view = environment('http://localhost/play/?view=3d-companion');
  let opened = 0, focused = 0;
  const child = { closed: false, focus: () => focused++ };
  main.target.open = url => {
    assert.equal(url, 'http://localhost/play/?view=3d-companion');
    opened++;
    return child;
  };
  for (const env of [main, other, view]) install3dCompanion(env.target);
  view.target.opener = main.target;
  main.target.ironwoodPublish3dPosition('first position');
  other.target.ironwoodPublish3dPosition('unrelated game');
  assert.equal(view.target.ironwoodCompanionMode(), true);
  assert.equal(view.target.ironwoodRead3dPosition(), 'first position');
  main.target.ironwoodPublish3dPosition('history position');
  assert.equal(view.target.ironwoodRead3dPosition(), 'history position');
  assert.equal(main.target.ironwoodOpen3dCompanion(), true);
  assert.equal(main.target.ironwoodOpen3dCompanion(), true);
  assert.equal(opened, 1);
  assert.equal(focused, 1);
  child.closed = true;
  main.target.ironwoodOpen3dCompanion();
  assert.equal(opened, 2);
  main.target.closed = true;
  assert.equal(view.target.ironwoodRead3dPosition(), '');
  for (const env of [main, other, view]) env.dom.window.close();
});

test('blocked 3D companion displays a useful message', () => {
  const { target, dom } = environment();
  install3dCompanion(target);
  target.open = () => null;
  assert.equal(target.ironwoodOpen3dCompanion(), false);
  assert.match(target.document.querySelector('[role=alert]').textContent, /Allow pop-ups/);
  dom.window.close();
});

test('FICS pop-out reuses its window, follows its opener and queues commands without loading another connection', () => {
  const main = environment(), view = environment('http://localhost/play/?view=fics-companion');
  let opened = 0, focused = 0;
  const child = { closed: false, focus: () => focused++ };
  main.target.open = url => { assert.equal(url, 'http://localhost/play/?view=fics-companion'); opened++; return child; };
  install3dCompanion(main.target);
  const requests = [];
  const deferred = installDeferredTools(view.target, path => { requests.push(path); return Promise.resolve({}); });
  view.target.opener = main.target;
  main.target.ironwoodPublishFicsCompanion('{"connected":true,"log":["Welcome"]}');
  assert.equal(view.target.ironwoodCompanionMode(), true);
  assert.equal(view.target.ironwoodFicsCompanionMode(), true);
  assert.equal(view.target.ironwoodReadFicsCompanion(), '{"connected":true,"log":["Welcome"]}');
  assert.equal(view.target.ironwoodFicsSendCommand('help'), true);
  assert.equal(view.target.ironwoodFicsSendCommand('help\nquit'), false);
  assert.deepEqual(JSON.parse(main.target.ironwoodTakeFicsCompanionCommands()), ['help']);
  assert.deepEqual(JSON.parse(main.target.ironwoodTakeFicsCompanionCommands()), []);
  deferred.afterPaint();
  assert.deepEqual(requests, []);
  assert.equal(main.target.ironwoodOpenFicsCompanion(), true);
  assert.equal(main.target.ironwoodOpenFicsCompanion(), true);
  assert.equal(opened, 1); assert.equal(focused, 1);
  assert.equal(main.target.ironwoodFicsCompanionOpen(), true);
  child.closed = true;
  assert.equal(main.target.ironwoodFicsCompanionOpen(), false);
  main.target.closed = true;
  assert.equal(view.target.ironwoodReadFicsCompanion(), '');
  main.dom.window.close(); view.dom.window.close();
});

test('companion startup leaves account restoration and background tools idle', () => {
  const { target, dom } = environment('http://localhost/play/?view=3d-companion');
  const requests = [];
  const tools = installDeferredTools(target, path => { requests.push(path); return Promise.resolve({}); });
  tools.afterPaint();
  assert.deepEqual(requests, []);
  dom.window.close();
});

test('core fetch/compile overlaps restoration; optional modules wait for the painted board or use', async () => {
  const { dom, target } = environment();
  const restored = deferred(), compiled = deferred(), painted = deferred();
  const requests = [], module = {}, app = { export_saved_game() {}, validate_scoresheet_pgn() {}, preview_scoresheet() {} };
  let initialized = false, storageReady = false, clipboardReady = false;
  app.default = async options => {
    assert.equal(options.module_or_path, module);
    assert.ok(storageReady, 'restored game must exist before Rust starts');
    assert.ok(clipboardReady, 'clipboard writes must retain the original user gesture');
    const bindings = [...readFileSync(new URL('../web/pkg/battle_chess.js', import.meta.url), 'utf8').matchAll(/window\.(ironwood\w+)\(/g)].map(match => match[1]);
    for (const binding of bindings) assert.equal(typeof target[binding], 'function', `${binding} must be synchronously callable`);
    assert.equal(target.ironwoodLichessPoll(), '');
    assert.equal(target.ironwoodPollPositionEditor(), '');
    assert.equal(target.ironwoodTakeBackground().length, 0);
    initialized = true;
  };
  const start = startChessApp({ target,
    compile: url => { requests.push(new URL(url, target.location.href).pathname); return compiled.promise; },
    afterPaint: () => painted.promise,
    loadModule: async url => {
      const path = new URL(url, target.location.href).pathname;
      requests.push(path);
      if (path === '/pkg/battle_chess.js') return app;
      if (path === '/game-storage.js') return { initializeGameStorage: async () => { await restored.promise; storageReady = true; } };
      if (path === '/board-clipboard.js') { clipboardReady = true; return {}; }
      return {};
    },
  });
  await flush();
  assert.deepEqual(new Set(requests), new Set(['/pkg/battle_chess.js', '/pkg/battle_chess_bg.wasm', '/game-storage.js', '/board-clipboard.js']));
  compiled.resolve(module);
  await flush();
  assert.equal(initialized, false);
  restored.resolve();
  assert.equal(await start, true);
  assert.equal(target.document.getElementById('loading'), null);
  assert.equal(target.ironwoodExportSavedGame, app.export_saved_game);
  assert.equal(requests.length, 4);
  painted.resolve();
  await flush();
  assert.deepEqual(requests.slice(4).sort(), ['/background-image.js', '/lichess/bridge.js', '/pwa.js']);
  assert.equal(target.document.body.dataset.startupReadyMs, '123.4');
  dom.window.close();
});

test('new WASM waits for the actual first app frame before optional work, beyond init readiness', async () => {
  const { dom, target } = environment();
  const name = new TextEncoder().encode('__wbg_ironwoodFirstFrame_test');
  const imports = [1, 3, 119, 98, 103, name.length, ...name, 0, 0];
  const module = new WebAssembly.Module(new Uint8Array([
    0, 97, 115, 109, 1, 0, 0, 0, // WASM header
    1, 4, 1, 96, 0, 0, // () -> () function type
    2, imports.length, ...imports,
  ]));
  const requests = [], frames = [];
  const success = await startChessApp({ target, compile: async () => module,
    afterPaint: () => { const frame = deferred(); frames.push(frame); return frame.promise; },
    loadModule: async url => {
      const path = new URL(url, target.location.href).pathname;
      requests.push(path);
      if (path === '/game-storage.js') return { initializeGameStorage: async () => {} };
      if (path === '/pkg/battle_chess.js') return { default: async () => {} };
      return {};
    },
  });
  assert.equal(success, true);
  assert.equal(frames.length, 1);
  frames[0].resolve(); await flush();
  assert.equal(target.document.body.dataset.startupReadyMs, '123.4');
  assert.equal(target.document.body.dataset.boardReadyMs, undefined);
  assert.equal(requests.length, 3, 'async painter initialization must not race optional scripts');
  target.ironwoodFirstFrame(); await flush();
  assert.equal(frames.length, 2);
  assert.equal(requests.length, 3);
  frames[1].resolve(); await flush();
  assert.equal(target.document.body.dataset.boardReadyMs, '123.4');
  assert.deepEqual(requests.slice(3).sort(), ['/background-image.js', '/lichess/bridge.js', '/pwa.js']);
  dom.window.close();
});

test('OAuth callback bridge initializes before Rust; normal sessions reconnect after paint', async () => {
  for (const query of ['?code=one-use&state=secret', '?error=access_denied&state=secret']) {
    const { dom, target } = environment(`https://ironwood.test/play/${query}`);
    const oauth = deferred();
    let initialized = false, authStarted = false;
    const start = startChessApp({ target, compile: async () => ({}), afterPaint: () => new Promise(() => {}),
      loadModule: async url => {
        if (url === '/lichess/bridge.js') {
          authStarted = true;
          await oauth.promise;
          dom.window.history.replaceState(null, '', '/play/');
        }
        if (url === '/game-storage.js') return { initializeGameStorage: async () => {} };
        if (url === '/pkg/battle_chess.js') return { default: async () => {
          assert.equal(target.location.search, ''); initialized = true;
        } };
        return {};
      },
    });
    await flush();
    assert.equal(authStarted, true);
    assert.equal(initialized, false);
    oauth.resolve();
    assert.equal(await start, true);
    dom.window.close();
  }
});

test('optional storage/clipboard failures do not prevent board startup; core failure remains visible', async () => {
  for (const failCore of [false, true]) {
    const { dom, target, warnings } = environment();
    let initialized = false;
    const success = await startChessApp({ target,
      compile: async () => { if (failCore) throw new Error('offline'); return {}; },
      afterPaint: () => new Promise(() => {}),
      loadModule: async url => {
        if (url.startsWith('/pkg/')) return { default: async () => { initialized = true; } };
        throw new Error('Unavailable optional file');
      },
    });
    assert.equal(success, !failCore);
    assert.equal(initialized, !failCore);
    assert.ok(warnings.length >= 2);
    if (failCore) assert.match(target.document.getElementById('loading').textContent, /could not start/);
    else assert.equal(target.ironwoodBeginBoardPngCopy(), false);
    dom.window.close();
  }
});

test('deferred actions preserve arguments and order with one load; idle polls do not fetch tools', async () => {
  const { dom, target } = environment();
  const gate = deferred(), paths = [], calls = [];
  installDeferredTools(target, async path => {
    paths.push(path);
    await gate.promise;
    target.ironwoodFicsConnectRegistered = (...args) => calls.push(['connect', ...args]);
    target.ironwoodFicsDisconnect = () => calls.push(['disconnect']);
  });
  for (let index = 0; index < 20; index++) {
    target.ironwoodFicsPoll(); target.ironwoodLichessPoll();
    target.ironwoodPollPositionEditor(); target.ironwoodPollScoresheet(); target.ironwoodTakeBackground();
  }
  assert.deepEqual(paths, []);
  target.ironwoodFicsConnectRegistered('Alice', 'session-password');
  target.ironwoodFicsDisconnect();
  await flush();
  assert.deepEqual(paths, ['/fics/bridge.js']);
  gate.resolve();
  await flush();
  assert.deepEqual(calls, [['connect', 'Alice', 'session-password'], ['disconnect']]);
  dom.window.close();
});

test('piece artwork bridge loads its helper only when requested and preserves the async byte contract', async () => {
  const { dom, target } = environment();
  const paths = [], calls = [], bytes = Array.from({ length: 12 }, () => new Uint8Array([137, 80]));
  installDeferredTools(target, async path => {
    paths.push(path);
    return { loadPieceTheme: async theme => { calls.push(theme); return bytes; } };
  });
  assert.deepEqual(paths, []);
  const first = target.ironwoodLoadPieceTheme('hybrid-wood');
  const second = target.ironwoodLoadPieceTheme('sculpted-marble');
  assert.deepEqual(paths, [], 'module fetch begins asynchronously only after an explicit theme request');
  assert.equal(await first, bytes);
  assert.equal(await second, bytes);
  assert.deepEqual(paths, ['/piece-themes.js']);
  assert.deepEqual(calls, ['hybrid-wood', 'sculpted-marble']);
  dom.window.close();
});

test('3D artwork bridge stays idle until requested and shares one helper for Wood and Marble/Glass', async () => {
  const { dom, target } = environment();
  const paths = [], calls = [], bytes = Array.from({ length: 14 }, () => new Uint8Array([103, 108, 84, 70]));
  installDeferredTools(target, async path => {
    paths.push(path);
    return { loadBoardTheme: async theme => { calls.push(theme); return bytes; } };
  });
  assert.deepEqual(paths, []);
  const first = target.ironwoodLoadBoardTheme('wood');
  const second = target.ironwoodLoadBoardTheme('marble');
  assert.deepEqual(paths, []);
  assert.equal(await first, bytes);
  assert.equal(await second, bytes);
  assert.deepEqual(paths, ['/board-themes.js']);
  assert.deepEqual(calls, ['wood', 'marble']);
  dom.window.close();
});

test('failed deferred action can be retried without reloading the board', async () => {
  const { dom, target } = environment();
  let attempts = 0, opened = '';
  const urls = [];
  installDeferredTools(target, async url => {
    urls.push(url);
    if (++attempts === 1) throw new Error('temporarily offline');
    target.ironwoodOpenPositionEditor = fen => { opened = fen; };
  });
  target.ironwoodOpenPositionEditor('first');
  await flush();
  assert.match(target.document.querySelector('[role=alert]').textContent, /Could not load Position Editor/);
  target.ironwoodOpenPositionEditor('second');
  await flush();
  assert.equal(attempts, 2);
  assert.notEqual(urls[0], urls[1], 'retry must bypass a failed browser module-map entry');
  assert.equal(opened, 'second');
  dom.window.close();
});

test('tablebase first lookup is replayed once for the latest position while the module loads', async () => {
  const { dom, target } = environment();
  const gate = deferred(), calls = [];
  let loads = 0;
  installDeferredTools(target, async path => {
    assert.equal(path, '/tablebase.js'); loads++;
    await gate.promise;
    target.ironwoodTablebase = (...args) => { calls.push(args); return '{"category":"draw"}'; };
  });
  assert.equal(target.ironwoodTablebase('first', false, true), '');
  assert.equal(target.ironwoodTablebase('latest', false, true), '');
  target.ironwoodTablebase('latest', false, false);
  await flush();
  assert.equal(loads, 1);
  gate.resolve(); await flush();
  assert.deepEqual(calls, [['latest', false, false]]);
  assert.equal(JSON.parse(target.ironwoodTablebase('latest', false, false)).category, 'draw');
  dom.window.close();
});

test('puzzle history stays synchronous before lazy client load and requests are not duplicated', async () => {
  const { dom, target } = environment();
  const gate = deferred(), calls = [];
  let loads = 0;
  target.localStorage.setItem('ironwood.puzzles.history.v1', '[{"id":"saved"}]');
  installDeferredTools(target, async path => {
    assert.equal(path, '/puzzles.js'); loads++;
    await gate.promise;
    target.ironwoodPuzzles = (...args) => { calls.push(args); return '{}'; };
  });
  assert.equal(target.ironwoodPuzzles('history'), '[{"id":"saved"}]');
  assert.equal(target.ironwoodPuzzles('save', '[{"id":"updated"}]'), '');
  assert.equal(target.ironwoodPuzzles('history'), '[{"id":"updated"}]');
  assert.equal(loads, 0);
  for (let i = 0; i < 10; i++) assert.equal(JSON.parse(target.ironwoodPuzzles('request', '{"daily":true}')).loading, true);
  await flush(); assert.equal(loads, 1);
  gate.resolve(); await flush();
  assert.deepEqual(calls, [['request', '{"daily":true}']]);
  dom.window.close();
});

test('background picker opens synchronously before loading, then passes the selected file', async () => {
  const { dom, target } = environment();
  const gate = deferred(), file = new dom.window.File(['image'], 'background.png', { type: 'image/png' });
  let picker = false, loadStarted = false, uploaded;
  dom.window.HTMLInputElement.prototype.click = function () {
    picker = true;
    assert.equal(loadStarted, false, 'picker must retain user activation');
    Object.defineProperty(this, 'files', { value: [file] });
    this.onchange();
  };
  installDeferredTools(target, async path => {
    assert.equal(path, '/background-image.js'); loadStarted = true;
    await gate.promise;
    return { uploadBackgroundFile: selected => { uploaded = selected; } };
  });
  target.ironwoodUploadBackground();
  assert.equal(picker, true);
  assert.equal(loadStarted, false);
  await flush(); gate.resolve(); await flush();
  assert.equal(uploaded, file);
  dom.window.close();
});

test('WASM compilation streams only valid WASM MIME responses and preserves byte fallback', async () => {
  const calls = [], module = {};
  const wasm = { compileStreaming: async response => { calls.push('stream'); return module; },
    compile: async bytes => { calls.push(bytes.byteLength); return module; } };
  assert.equal(await compileAppWasm('/app.wasm', async () => new Response('test', { headers: { 'Content-Type': 'application/wasm' } }), wasm), module);
  assert.equal(await compileAppWasm('/app.wasm', async () => new Response('test', { headers: { 'Content-Type': 'application/octet-stream' } }), wasm), module);
  assert.deepEqual(calls, ['stream', 4]);
  await assert.rejects(compileAppWasm('/app.wasm', async () => new Response('', { status: 404 }), wasm), /404/);
});


test('player profile pop-outs preserve independent selections and share only their opener connection', () => {
  const main = environment(), view = environment('http://localhost/play/?view=profile-companion&profile=7');
  let opened = 0, focused = 0;
  main.target.open = url => {
    assert.match(url, /view=profile-companion&profile=(7|8)$/);
    opened++; return { closed: false, focus: () => focused++ };
  };
  install3dCompanion(main.target);
  const requested = [], commands = [];
  main.target.ironwoodLichessCommand = value => commands.push(JSON.parse(value));
  const tools = installDeferredTools(view.target, path => { requested.push(path); return Promise.resolve({}); });
  view.target.opener = main.target;
  const selection = JSON.stringify({ id: 7, username: 'cutemouse83', control: 1, range: 4 });
  assert.equal(main.target.ironwoodOpenProfileCompanion(selection), true);
  assert.equal(main.target.ironwoodOpenProfileCompanion(selection), true);
  assert.equal(main.target.ironwoodOpenProfileCompanion(JSON.stringify({ id: 8, username: 'other' })), true);
  assert.equal(opened, 2); assert.equal(focused, 1);
  assert.equal(view.target.ironwoodProfileCompanionMode(), true);
  assert.equal(view.target.ironwoodProfileCompanionInitial(), selection);
  main.target.ironwoodPublishProfileState('{"connected":true}');
  assert.equal(view.target.ironwoodReadProfileState(), '{"connected":true}');
  view.target.ironwoodLichessCommand('{"type":"playerProfile","windowId":"7","username":"other"}');
  view.target.ironwoodLichessCommand('{"type":"seek"}');
  assert.equal(commands.length, 1);
  assert.equal(view.target.ironwoodReviewProfileInMain('1. e4 *'), true);
  assert.deepEqual(JSON.parse(main.target.ironwoodTakeProfileReviews()), ['1. e4 *']);
  assert.deepEqual(JSON.parse(main.target.ironwoodTakeProfileReviews()), []);
  tools.afterPaint(); assert.deepEqual(requested, []);
  main.target.closed = true;
  assert.equal(view.target.ironwoodReadProfileState(), '');
  assert.equal(view.target.ironwoodReviewProfileInMain('1. d4 *'), false);
  main.dom.window.close(); view.dom.window.close();
});

test('blocked player profile pop-out leaves the original available', () => {
  const { target, dom, warnings } = environment();
  install3dCompanion(target); target.open = () => null;
  assert.equal(target.ironwoodOpenProfileCompanion('{"id":1}'), false);
  assert.equal(target.ironwoodProfileCompanionOpen(), false);
  assert.match(warnings[0], /Allow pop-ups/);
  dom.window.close();
});

test('Lichess lobby pop-out shares the main connection and game controls', () => {
  const main = environment(), view = environment('http://localhost/play/?view=lobby-companion');
  let opens = 0, focus = 0;
  const child = { closed: false, focus: () => focus++ };
  main.target.open = url => { assert.equal(url, 'http://localhost/play/?view=lobby-companion'); opens++; return child; };
  install3dCompanion(main.target); install3dCompanion(view.target);
  view.target.opener = main.target;
  const requested = [];
  installDeferredTools(view.target, path => { requested.push(path); return Promise.resolve({}); });
  const commands = [];
  main.target.ironwoodLichessCommand = raw => commands.push(JSON.parse(raw));
  let signs = 0; main.target.ironwoodLichessSignIn = () => signs++;
  assert.equal(main.target.ironwoodOpenLobbyCompanion(), true);
  assert.equal(main.target.ironwoodOpenLobbyCompanion(), true);
  assert.equal(opens, 1); assert.equal(focus, 1);
  assert.equal(view.target.ironwoodCompanionMode(), true);
  assert.equal(view.target.document.title, 'Ironwood — Lichess lobby');
  main.target.ironwoodPublishLobby('{"lobby":{"connected":true}}');
  assert.equal(view.target.ironwoodReadLobby(), '{"lobby":{"connected":true}}');
  view.target.ironwoodLichessCommand('{"type":"seek"}');
  view.target.ironwoodLichessSignIn();
  assert.deepEqual(commands, [{ type: 'seek' }]); assert.equal(signs, 1);
  assert.deepEqual(requested, []);
  assert.equal(view.target.ironwoodLobbyActionInMain('resign'), true);
  assert.deepEqual(JSON.parse(main.target.ironwoodTakeLobbyActions()), ['resign']);
  assert.deepEqual(JSON.parse(main.target.ironwoodTakeLobbyActions()), []);
  assert.equal(view.target.ironwoodLobbyActionInMain('invalid'), false);
  main.target.closed = true;
  assert.equal(view.target.ironwoodReadLobby(), '');
  assert.equal(view.target.ironwoodLobbyActionInMain('leave'), false);
  main.dom.window.close(); view.dom.window.close();
});

test('Background pop-out shares image controls and sends appearance changes to its main window', () => {
  const main = environment(), view = environment('http://localhost/play/?view=background-companion');
  let opens = 0, focus = 0, uploads = 0;
  const child = { closed: false, focus: () => focus++ };
  main.target.open = url => { assert.equal(url, 'http://localhost/play/?view=background-companion'); opens++; return child; };
  install3dCompanion(main.target); install3dCompanion(view.target);
  view.target.opener = main.target;
  const requested = [];
  installDeferredTools(view.target, path => { requested.push(path); return Promise.resolve({}); });
  main.target.ironwoodBackgroundLibrary = () => '{"selected":"ironwood-default","images":[]}';
  main.target.ironwoodUploadBackground = () => uploads++;
  const selected = [];
  main.target.ironwoodSelectBackground = id => selected.push(id);
  assert.equal(main.target.ironwoodOpenBackgroundCompanion(), true);
  assert.equal(main.target.ironwoodOpenBackgroundCompanion(), true);
  assert.equal(opens, 1); assert.equal(focus, 1);
  assert.equal(view.target.ironwoodCompanionMode(), true);
  assert.equal(view.target.document.title, 'Ironwood — Background');
  assert.equal(JSON.parse(view.target.ironwoodBackgroundLibrary()).selected, 'ironwood-default');
  view.target.ironwoodUploadBackground(); assert.equal(uploads, 1);
  view.target.ironwoodSelectBackground(''); assert.deepEqual(selected, ['']);
  assert.deepEqual(requested, []);
  assert.equal(view.target.ironwoodChangeBackgroundInMain('[30,40,50]'), true);
  assert.deepEqual(JSON.parse(main.target.ironwoodTakeBackgroundChanges()), ['[30,40,50]']);
  assert.deepEqual(JSON.parse(main.target.ironwoodTakeBackgroundChanges()), []);
  main.target.closed = true;
  assert.equal(view.target.ironwoodChangeBackgroundInMain('[30,40,50]'), false);
  assert.equal(JSON.parse(view.target.ironwoodBackgroundLibrary()).selected, '');
  main.dom.window.close(); view.dom.window.close();
});

test('Board Theme pop-out reuses its window and sends selections to the main board', () => {
  const main = environment(), view = environment('http://localhost/play/?view=theme-companion');
  let opens = 0, focus = 0;
  const child = { closed: false, focus: () => focus++ };
  main.target.open = url => { assert.equal(url, 'http://localhost/play/?view=theme-companion'); opens++; return child; };
  install3dCompanion(main.target); install3dCompanion(view.target);
  view.target.opener = main.target;
  assert.equal(main.target.ironwoodOpenThemeCompanion(), true);
  assert.equal(main.target.ironwoodOpenThemeCompanion(), true);
  assert.equal(opens, 1); assert.equal(focus, 1);
  assert.equal(view.target.ironwoodCompanionMode(), true);
  assert.equal(view.target.ironwoodThemeCompanionMode(), true);
  assert.equal(view.target.document.title, 'Ironwood — Board Theme');
  const selection = JSON.stringify(['Cburnett', null, false]);
  assert.equal(view.target.ironwoodSelectThemeInMain(selection), true);
  assert.deepEqual(JSON.parse(main.target.ironwoodTakeThemeSelections()), [selection]);
  assert.deepEqual(JSON.parse(main.target.ironwoodTakeThemeSelections()), []);
  main.target.closed = true;
  assert.equal(view.target.ironwoodSelectThemeInMain(selection), false);
  child.closed = true;
  assert.equal(main.target.ironwoodThemeCompanionOpen(), false);
  main.target.open = () => null;
  assert.equal(main.target.ironwoodOpenThemeCompanion(), false);
  assert.match(main.warnings[0], /Allow pop-ups/);
  main.dom.window.close(); view.dom.window.close();
});

test('Training Profiles pop-out reuses its window, shares activity, and sends starts to the main board', () => {
  const main = environment(), view = environment('http://localhost/play/?view=training-companion');
  let opens = 0, focus = 0;
  const child = {closed:false,focus:()=>focus++};
  main.target.open = url => { assert.equal(url,'http://localhost/play/?view=training-companion'); opens++; return child; };
  main.target.addEventListener = main.dom.window.addEventListener.bind(main.dom.window);
  install3dCompanion(main.target); install3dCompanion(view.target);
  view.target.opener = main.target;
  assert.equal(main.target.ironwoodOpenTrainingCompanion(),true);
  assert.equal(main.target.ironwoodOpenTrainingCompanion(),true);
  assert.equal(opens,1); assert.equal(focus,1);
  assert.equal(view.target.ironwoodCompanionMode(),true);
  assert.equal(view.target.ironwoodTrainingCompanionMode(),true);
  main.target.ironwoodPublishTrainingLive(true);
  assert.equal(view.target.ironwoodReadTrainingLive(),true);
  assert.equal(view.target.ironwoodStartTrainingInMain('profile-id'),true);
  assert.deepEqual(JSON.parse(main.target.ironwoodTakeTrainingStarts()),['profile-id']);
  assert.deepEqual(JSON.parse(main.target.ironwoodTakeTrainingStarts()),[]);
  main.dom.window.dispatchEvent(new main.dom.window.StorageEvent('storage',{key:'ironwood.chess.training.v1'}));
  assert.equal(main.target.ironwoodTakeTrainingChanged(),true);
  assert.equal(main.target.ironwoodTakeTrainingChanged(),false);
});

test('Companion add-on is hidden by default and its persistent diagnostics switch gates deferred tools',async()=>{
  const {installCompanionFeature,companionEnabled}=await import('../web/companion-feature.js');
  const dom=new JSDOM('<body></body>',{url:'http://localhost/play/'}), target=dom.window;
  const loaded=[];
  installDeferredTools(target,async path=>{loaded.push(path);target.ironwoodRefreshCompanionEngines=()=>{};});
  assert.equal(companionEnabled(target),false);
  target.ironwoodRefreshCompanionEngines();target.ironwoodArchiveExplorer('fen');await flush();
  assert.equal(target.ironwoodCompanionDiagnostics(),'{}');
  target.ironwoodRefreshCompanionDiagnostics();target.ironwoodCheckCompanionUpdates();target.ironwoodInstallCompanionUpdate();await flush();
  assert.deepEqual(loaded,[]);
  target.ironwoodSetCompanionFeatureEnabled(true);
  assert.equal(companionEnabled(target),true);
  target.ironwoodRefreshCompanionEngines();await flush();await flush();
  assert.deepEqual(loaded,['/companion-engines.js']);
  let closed=0;const dialog=target.document.createElement('dialog');dialog.id='companion-archive';dialog.close=()=>{closed++;dialog.remove();};target.document.body.append(dialog);
  target.ironwoodSetCompanionFeatureEnabled(false);
  assert.equal(closed,1);assert.equal(companionEnabled(target),false);
  installCompanionFeature(target);assert.equal(target.ironwoodCompanionFeatureEnabled(),false);
  dom.window.close();
});

test('analysis catalog loads automatically after reload only when Companion is enabled', async () => {
  const dom = new JSDOM('<body></body>', {url:'http://localhost/play/'});
  const target = dom.window, loaded = [];
  let reads = 0;
  installDeferredTools(target, async path => {
    loaded.push(path);
    target.ironwoodCompanionEngineCatalog = () => { reads++; return '{"engines":[{"label":"Lc0"}]}'; };
  });
  target.ironwoodCompanionEngineCatalog();
  await flush();
  assert.deepEqual(loaded, []);
  target.ironwoodSetCompanionFeatureEnabled(true);
  assert.equal(JSON.parse(target.ironwoodCompanionEngineCatalog()).busy,true);
  target.ironwoodCompanionEngineCatalog();
  await flush(); await flush();
  assert.deepEqual(loaded, ['/companion-engines.js']);
  assert.equal(reads,1);
  assert.equal(JSON.parse(target.ironwoodCompanionEngineCatalog()).engines[0].label,'Lc0');
  dom.window.close();
});

test('restored engine initializes through the shared connection without loading the engine picker', async () => {
  const {dom, target} = environment();
  const loaded = [], messages = [];
  installDeferredTools(target, async path => {
    loaded.push(path);
    return {companionEngineConfiguration(identity, overrides) {
      assert.equal(identity, 'http://localhost:8790/lc0');
      return {endpoint:'http://localhost:8790', credential:'shared-token', engineId:'lc0', overrides};
    }};
  });
  const worker = {postMessage: message => messages.push(message)};
  await target.ironwoodInitializeCompanionEngine(worker, {type:'init'}, 'unused');
  assert.deepEqual(loaded, []);
  assert.match(messages.pop().companionError, /disabled/);
  target.ironwoodCompanionFeatureEnabled = () => true;
  await target.ironwoodInitializeCompanionEngine(worker, {type:'init',overrides:{Backend:'cuda'}}, 'http://localhost:8790/lc0');
  assert.deepEqual(loaded, ['/companion-connection.js']);
  assert.equal(messages[0].companion.engineId, 'lc0');
  assert.deepEqual(messages[0].companion.overrides, {Backend:'cuda'});
  dom.window.close();
});

test('connection restoration errors reach the worker and disabling during load prevents connection', async () => {
  const {dom, target} = environment();
  const loading = deferred(), messages = [];
  let enabled = true, resolved = 0;
  installDeferredTools(target, () => loading.promise);
  target.ironwoodCompanionFeatureEnabled = () => enabled;
  const start = target.ironwoodInitializeCompanionEngine({postMessage: value => messages.push(value)}, {type:'init'}, 'saved-engine');
  enabled = false;
  loading.resolve({companionEngineConfiguration() { resolved++; }});
  await start;
  assert.equal(resolved, 0);
  assert.match(messages.pop().companionError, /disabled/);
  enabled = true;
  // A missing credential is an actionable connection error, not a startup failure.
  installDeferredTools(target, async () => ({companionEngineConfiguration() { throw new Error('Connect, then Retry connection'); }}));
  target.ironwoodCompanionFeatureEnabled = () => true;
  await target.ironwoodInitializeCompanionEngine({postMessage: value => messages.push(value)}, {type:'init'}, 'saved-engine');
  assert.equal(messages.pop().companionError, 'Connect, then Retry connection');
  dom.window.close();
});

test('Browse fills import text without importing and handles cancel, size and read failures',async()=>{
 const {installImportFilePicker}=await import('../web/app-bootstrap.js');
 const {dom,target}=environment();installImportFilePicker(target);
 dom.window.HTMLInputElement.prototype.click=function(){};
 target.ironwoodBrowseImportFile();let input=target.document.querySelector('input[type=file]');assert.match(input.accept,/pgn/);
 Object.defineProperty(input,'files',{value:[{size:25,text:async()=> '1. e4 e5 *'}]});input.dispatchEvent(new dom.window.Event('change'));await flush();
 assert.deepEqual(JSON.parse(target.ironwoodPollImportFile()),{text:'1. e4 e5 *'});assert.equal(target.ironwoodPollImportFile(),'');assert.equal(input.isConnected,false);
 target.ironwoodBrowseImportFile();input=target.document.querySelector('input');input.dispatchEvent(new dom.window.Event('cancel'));assert.equal(target.ironwoodPollImportFile(),'');assert.equal(input.isConnected,false);
 target.ironwoodBrowseImportFile();input=target.document.querySelector('input');Object.defineProperty(input,'files',{value:[{size:65*1024*1024}]});input.dispatchEvent(new dom.window.Event('change'));await flush();assert.match(JSON.parse(target.ironwoodPollImportFile()).error,/64 MiB/);
 target.ironwoodBrowseImportFile();input=target.document.querySelector('input');Object.defineProperty(input,'files',{value:[{size:1,text:async()=>{throw Error('Unreadable');}}]});input.dispatchEvent(new dom.window.Event('change'));await flush();assert.match(JSON.parse(target.ironwoodPollImportFile()).error,/Unreadable/);
 dom.window.close();
});

test('Browse reads a ZIP of PGNs into the paste field and reports archive errors without importing',async()=>{
 const {installImportFilePicker}=await import('../web/app-bootstrap.js');const {zip}=await import('./helpers/zip-fixture.mjs');
 const {dom,target}=environment();installImportFilePicker(target);let saves=0;target.ironwoodSaveGame=()=>saves++;
 const choose=async archive=>{
  target.ironwoodBrowseImportFile();const input=target.document.querySelector('input[type=file]');assert.match(input.accept,/\.zip/);
  Object.defineProperty(input,'files',{value:[{name:'games.zip',type:'application/zip',size:archive.length,arrayBuffer:async()=>archive}]});input.dispatchEvent(new dom.window.Event('change'));
  let result;for(let i=0;i<100&&!result;i++){await new Promise(resolve=>setTimeout(resolve,10));result=target.ironwoodPollImportFile();}assert.ok(result,'ZIP selection must complete');assert.equal(input.isConnected,false);return JSON.parse(result);
 };
 const pgn='[White "Alice"]\n[Black "Bob"]\n\n1. e4 e5 *';assert.deepEqual(await choose(zip([{name:'game.pgn',text:pgn}])),{text:pgn});
 assert.match((await choose(zip([{name:'readme.txt'}]))).error,/does not contain any PGN/);assert.equal(saves,0);dom.window.close();
});

test('dropped ZIP bridge shares Browse extraction and rejects damaged archives without saving',async()=>{
 const {installImportFilePicker}=await import('../web/app-bootstrap.js');const {zip}=await import('./helpers/zip-fixture.mjs');
 const {dom,target}=environment();installImportFilePicker(target);let saved=0;target.ironwoodSaveGame=()=>saved++;
 const first='[White "Alice"]\n[Black "Bob"]\n\n1. e4 e5 *',second='[White "Carol"]\n[Black "Dan"]\n\n1. d4 d5 *';
 assert.equal(await target.ironwoodReadPgnZip(new Uint8Array(zip([{name:'first.pgn',text:first},{name:'folder/second.PGN',text:second}]))),first+'\n\n'+second);
 await assert.rejects(target.ironwoodReadPgnZip(new Uint8Array([1,2,3])),/damaged/);
 await assert.rejects(target.ironwoodReadPgnZip(zip([{name:'large.pgn',declaredSize:65*1024*1024}])),/64 MiB/);
 assert.equal(target.ironwoodPollImportFile(),'');assert.equal(saved,0);dom.window.close();
});
