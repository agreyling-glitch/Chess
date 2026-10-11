// Stockfish.js 19's synchronous Threads option can wait for a pthread whose
// asynchronous "loaded" message cannot run until that same command returns.
// Prepare idle workers first, while the engine event loop can still yield.
export function patchThreadPool(source) {
  const boundary = 'else if(e.data.progressPort)c=e.data.progressPort;';
  if (source.split(boundary).length !== 2 || !source.includes('Aa:function()') || !source.includes('Ha:function(')) {
    throw new Error('Stockfish.js thread-pool adapter needs review for this engine version');
  }
  return '/* Modified by Ironwood Chess, 2026-10-11: asynchronous pthread pool preparation.\n' +
    '   Patch source: scripts/stockfish-thread-pool.mjs in the Ironwood source repository. */\n' +
    source.replace(boundary, `${boundary}else if(e.data.prepareThreads){
    const pool=f.PThread;
    const count=Math.max(1,Math.min(Number(e.data.prepareThreads),8));
    const tasks=[];
    while(pool.ea.length+pool.ja.length<count){
      pool.Aa();
      const worker=pool.ea[pool.ea.length-1];
      tasks.push(new Promise(resolve=>pool.Ha(worker,resolve)));
    }
    Promise.all(tasks).then(()=>postMessage('ironwood-thread-pool-ready'));
  }`);
}
