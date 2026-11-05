// One finite-request lane for this app window. Live streams use fetch directly
// through this coordinator so they share cooldowns without occupying the lane.
export function createRequestCoordinator({now = () => Date.now()} = {}) {
  let tail = Promise.resolve(), retryAt = 0;
  function check() {
    if (now() < retryAt) {
      const error = new Error('Lichess rate limit: wait a full minute or the indicated Retry-After before retrying');
      error.status = 429;
      throw error;
    }
  }
  async function fetchResponse(url, options = {}, request = globalThis.fetch) {
    check();
    const response = await request(url, options);
    if (response.status === 429) {
      const header = response.headers?.get('Retry-After');
      const seconds = header == null ? NaN : Number(header);
      const deadline = Number.isFinite(seconds) ? now() + Math.max(0, seconds) * 1000 : Date.parse(header);
      retryAt = Math.max(retryAt, now() + 60000, Number.isFinite(deadline) ? deadline : 0);
    }
    return response;
  }
  function run(operation, {signal} = {}) {
    const task = tail.then(() => {
      signal?.throwIfAborted();
      check();
      return operation();
    });
    tail = task.catch(() => {});
    return task;
  }
  return {run, fetch: fetchResponse, check, retryAt: () => retryAt};
}

export const lichessRequests = createRequestCoordinator();
