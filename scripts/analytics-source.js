const local = ['localhost', '127.0.0.1'].includes(location.hostname);
const testing = local;
const enabled = location.hostname === 'ironwoodchess.com' || location.hostname === 'www.ironwoodchess.com' || testing;
const consentKey = testing ? 'ironwood.analytics.test-consent' : 'ironwood.analytics.consent';
let accepted = false;
let client;
let loading;
let lastInput = performance.now();
let lastTick = performance.now();
const allowedEvents = new Set(['$pageview', 'app_opened', 'game_started', 'game_analysis_started', 'active_usage']);

function preference() {
  try { return localStorage.getItem(consentKey); } catch { return null; }
}
function capture(event, properties = {}) {
  if (!accepted || !client || !allowedEvents.has(event)) return;
  client.capture(event, { ...properties, environment: testing ? 'local_test' : 'production' });
}
window.ironwoodTrackUsage = event => capture(event);

async function start() {
  if (!enabled || !accepted) return;
  try {
    loading ||= import('../node_modules/posthog-js/dist/module.mjs');
    const { default: posthog } = await loading;
    if (!accepted) return;
    if (!client) {
      client = posthog.init('phc_xGARmvBxRvxAqazaoPAn7vzvhfuAveYFjhDGXvNtC2Wy', {
        api_host: 'https://us.i.posthog.com',
        defaults: '2026-05-30',
        person_profiles: 'identified_only',
        persistence: 'localStorage+cookie',
        persistence_name: testing ? 'ironwood_test' : 'ironwood',
        autocapture: false,
        capture_pageview: false,
        capture_pageleave: false,
        disable_session_recording: true,
        capture_exceptions: false,
        capture_performance: false,
        disable_surveys: true,
        advanced_disable_flags: true,
        disable_external_dependency_loading: true,
        save_referrer: false,
        save_campaign_params: false,
        before_send: event => {
          if (!accepted || !allowedEvents.has(event.event)) return null;
          // Page titles can contain player names; URLs can contain imported data.
          function clean(value) {
            if (!value || typeof value !== 'object') return;
            for (const key of Object.keys(value)) {
              if (/url|pathname|referrer|title|utm_|initial_/i.test(key)) delete value[key];
              else clean(value[key]);
            }
          }
          clean(event.properties);
          event.properties.$current_url = location.origin + (location.pathname.startsWith('/play') ? '/play/' : '/');
          return event;
        },
      });
    }
    client.opt_in_capturing({ captureEventName: false });
    lastInput = lastTick = performance.now();
    capture('$pageview');
    if (location.pathname.startsWith('/play')) capture('app_opened');
  } catch (error) {
    console.warn('Ironwood usage statistics unavailable.', error);
  }
}

function choose(value) {
  accepted = value === 'accepted';
  try { localStorage.setItem(consentKey, value); } catch { /* Current page still respects the choice. */ }
  if (accepted) void start();
  else client?.opt_out_capturing();
}

function settings() {
  let panel = document.querySelector('[data-analytics-panel]');
  if (panel) { panel.hidden = false; return; }
  panel = document.createElement('section');
  panel.dataset.analyticsPanel = '';
  panel.setAttribute('aria-label', 'Usage statistics');
  panel.innerHTML = `<strong>Optional usage statistics${testing ? ' — local test' : ''}</strong>
    <p>Allow PostHog cookies and browser storage to measure visitors and active usage time? These statistics will be used to improve Ironwood Chess. Game content and notes are excluded.</p>
    <details style="margin:12px 0">
      <summary style="color:#d3ad62;cursor:pointer">Privacy details</summary>
      <div style="max-height:35vh;overflow-y:auto;padding-right:8px">
        <p>With your permission, selected usage events go to PostHog Cloud in the United States: app visits, local games started, full-game analyses, and active usage time. Cookies and browser storage recognize returning browsers and sessions.</p>
        <p>Active time counts while the app is visible, focused, and has received input within the last two minutes. Offline or blocked requests may not be counted.</p>
        <p>Game positions, move lists, notes, imported games, player names, page titles, and full URLs are excluded. Session recording and automatic interaction capture are disabled. PostHog receives network information such as your IP address and browser information.</p>
        <p>Choosing No thanks leaves the app fully usable. Analytics settings lets you change your choice later. Declining stops future collection and clears analytics persistence; previously submitted events remain. Your preference is stored locally.</p>
        <p>Your chess engine and local analysis run on your device. Online features contact their respective services independently of analytics.</p>
      </div>
    </details>
    <button type="button" data-accept>Allow statistics</button>
    <button type="button" data-decline>No thanks</button>`;
  panel.style.cssText = 'position:fixed;bottom:36px;left:16px;z-index:10000;max-width:440px;max-height:calc(100vh - 80px);overflow-y:auto;padding:18px;background:#181c20;color:#f3f0e4;border:1px solid #d3ad62;border-radius:8px;font:15px/1.5 system-ui;box-shadow:0 4px 20px #0008';
  for (const button of panel.querySelectorAll('button')) button.style.cssText = 'padding:8px 12px;margin:4px;cursor:pointer';
  panel.querySelector('[data-accept]').onclick = () => { choose('accepted'); panel.hidden = true; };
  panel.querySelector('[data-decline]').onclick = () => { choose('declined'); panel.hidden = true; };
  document.body.append(panel);
}

if (enabled) {
  const button = document.createElement('button');
  button.textContent = 'Analytics settings';
  button.style.cssText = 'position:fixed;right:8px;bottom:3px;z-index:9999;padding:3px 8px;background:#181c20;color:#d3ad62;border:1px solid #d3ad6266;border-radius:4px;cursor:pointer;font:12px system-ui';
  button.onclick = settings;
  document.body.append(button);
  accepted = preference() === 'accepted';
  if (accepted) void start();
  else if (preference() !== 'declined') settings();
  window.addEventListener('storage', event => {
    if (event.key === consentKey) {
      accepted = event.newValue === 'accepted';
      if (accepted) void start(); else client?.opt_out_capturing();
    }
  });
  for (const event of ['pointerdown', 'keydown', 'wheel']) {
    window.addEventListener(event, () => { lastInput = performance.now(); }, { passive: true });
  }
  document.addEventListener('visibilitychange', () => { lastTick = performance.now(); });
  setInterval(() => {
    const now = performance.now();
    const seconds = Math.min(15, (now - lastTick) / 1000);
    lastTick = now;
    if (location.pathname.startsWith('/play') && !document.hidden && document.hasFocus() && now - lastInput < 120_000) {
      capture('active_usage', { active_seconds: Math.round(seconds) });
    }
  }, 15_000);
}
