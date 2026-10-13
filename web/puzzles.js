// Puzzle data: Lichess, https://database.lichess.org/#puzzles (CC0).
const HISTORY_KEY = 'ironwood.puzzles.history.v1';
let state = { loading: false };
let nextRequestAt = 0;
window.ironwoodPuzzles = (command, payload = '') => {
  if (command === 'history') {
    try { return localStorage.getItem(HISTORY_KEY) || '[]'; }
    catch { return '[]'; }
  }
  if (command === 'save') {
    try { localStorage.setItem(HISTORY_KEY, payload); return ''; }
    catch { return 'Progress could not be saved on this device. It remains available for this session.'; }
  }
  if (command === 'request') {
    if (state.loading) return JSON.stringify(state);
    if (Date.now() < nextRequestAt) {
      state = { error: 'Please wait a minute before requesting another puzzle.' };
      return JSON.stringify(state);
    }
    let options;
    try { options = JSON.parse(payload); } catch { return JSON.stringify({ error: 'Invalid puzzle request.' }); }
    const url = new URL(options.daily ? '/api/puzzle/daily' : '/api/puzzle/next', 'https://lichess.org');
    if (!options.daily) {
      if (options.theme && options.theme !== 'mix') url.searchParams.set('angle', options.theme);
      url.searchParams.set('difficulty', options.difficulty || 'normal');
    }
    state = { loading: true };
    nextRequestAt = Date.now() + 1000;
    fetch(url, { credentials: 'omit', signal: AbortSignal.timeout(20000) })
      .then(async response => {
        if (response.status === 429) { nextRequestAt = Date.now() + 60000; throw new Error('Lichess is rate limiting requests. Wait a minute and try again.'); }
        if (!response.ok) throw new Error(`Lichess could not load the puzzle (${response.status}). Try again later.`);
        state = { data: await response.json() };
      })
      .catch(error => { state = { error: error.name === 'TimeoutError' ? 'Puzzle request timed out. Try again.' : (error.message || 'Could not reach Lichess. Check your connection.') }; });
  }
  return JSON.stringify(state);
};
