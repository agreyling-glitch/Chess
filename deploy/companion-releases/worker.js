// Internal private-preview distribution. No public bucket domain or listing.
const root = 'private-preview/windows-x64/';
const headers = {'Cache-Control':'private, no-store','X-Content-Type-Options':'nosniff','X-Robots-Tag':'noindex, nofollow, noarchive'};
const reply = (status,text,extra = {}) => new Response(text,{status,headers:{...headers,...extra}});
async function matches(actual,expected) {
  const digest = value => crypto.subtle.digest('SHA-256',new TextEncoder().encode(value));
  const [a,b] = await Promise.all([digest(actual),digest(expected)]);
  const left = new Uint8Array(a), right = new Uint8Array(b);
  let different = 0;
  for (let i = 0; i < left.length; i++) different |= left[i] ^ right[i];
  return different === 0;
}
async function release(request,env) {
    if (!env.RELEASE_READ_TOKEN || env.RELEASE_READ_TOKEN.length < 32) return reply(503,'Channel unavailable');
    const authorization = request.headers.get('Authorization') || '';
    if (authorization.length > 1024 || !await matches(authorization,`Bearer ${env.RELEASE_READ_TOKEN}`)) return reply(401,'Authentication required',{'WWW-Authenticate':'Bearer'});
    if (!['GET','HEAD'].includes(request.method)) return reply(405,'Read only',{'Allow':'GET, HEAD'});
    const url = new URL(request.url);
    if (url.protocol !== 'https:' || url.search || url.pathname.includes('%')) return reply(400,'Invalid release request');
    const key = url.pathname.slice(1);
    if (key !== `${root}manifest.json` && !/^private-preview\/windows-x64\/\d+\.\d+\.\d+\/ironwood-companion-windows-x64\.zip(?:\.sha256)?$/.test(key)) return reply(404,'Not found');
    try {
      const object = request.method === 'HEAD' ? await env.RELEASES.head(key) : await env.RELEASES.get(key);
      if (!object) return reply(404,'Not found');
      return new Response(request.method === 'HEAD' ? null : object.body,{headers:{...headers,
        'Content-Type':key.endsWith('.json') ? 'application/json' : key.endsWith('.zip') ? 'application/zip' : 'text/plain',
        'Content-Length':String(object.size),'ETag':object.httpEtag}});
    } catch { return reply(503,'Channel unavailable'); }
}
const origins = new Set(['https://ironwoodchess.com','https://www.ironwoodchess.com','http://127.0.0.1:8788','http://localhost:8788']);
export default {
 async fetch(request,env) {
  const origin = request.headers.get('Origin');
  if (origin && !origins.has(origin)) return reply(403,'Origin not allowed');
  let response;
  if (request.method === 'OPTIONS') {
   const method = request.headers.get('Access-Control-Request-Method');
   const requested = (request.headers.get('Access-Control-Request-Headers') || '').toLowerCase().split(',').map(v=>v.trim()).filter(Boolean);
   response = origin && ['GET','HEAD'].includes(method) && requested.every(v=>v==='authorization')
    ? reply(204,null) : reply(403,'Preflight denied');
  } else response = await release(request,env);
  if (origin) {
   response.headers.set('Access-Control-Allow-Origin',origin);
   response.headers.set('Vary','Origin');
   response.headers.set('Access-Control-Allow-Methods','GET, HEAD');
   response.headers.set('Access-Control-Allow-Headers','Authorization');
  }
  return response;
 }
};
