import { connect } from "cloudflare:sockets";

const ISOLATION_HEADERS = {
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Embedder-Policy": "require-corp",
  "Cross-Origin-Resource-Policy": "same-origin",
};

const FICS_COMMAND = /^(?:guest|set style 12|set seek 0|seek (?:[1-9]|[1-5]\d|60) (?:[0-9]|[1-5]\d|60) unrated|unseek|sought|play \d{1,4}|match [A-Za-z][A-Za-z0-9_-]{0,19} unrated (?:[1-9]|[1-5]\d|60) (?:[0-9]|[1-5]\d|60)|accept|decline|draw|resign|quit|[a-h][1-8][a-h][1-8][qrbn]?)$/;

// Only these packaged assets may cross the engine-origin trust boundary.
const ENGINE_ASSETS = new Map([
  ['/engine/stockfish-19.js', 'text/javascript; charset=utf-8'],
  ['/engine/stockfish-19.wasm', 'application/wasm'],
]);
const engineError=(message,status)=>new Response(message,{status,headers:{'Cache-Control':'no-store','Content-Type':'text/plain; charset=utf-8','X-Content-Type-Options':'nosniff'}});
async function proxyEngine(request,env,url) {
  const type=ENGINE_ASSETS.get(url.pathname);
  if(!type)return engineError('Engine asset not found.',404);
  if(!['GET','HEAD'].includes(request.method))return engineError('Method not allowed.',405);
  let upstream;
  try {
    const base=new URL(env.ENGINE_ORIGIN);
    if(!['https:','http:'].includes(base.protocol)||base.username||base.password||base.pathname!=='/'||base.search||base.hash)throw Error();
    // Assign a pathname, never resolve attacker input as a relative URL.
    upstream=new URL(base.origin);upstream.pathname=url.pathname.slice('/engine'.length);
  }catch{return engineError('The engine origin is not configured correctly.',503);}
  const headers=new Headers();
  for(const name of ['range','if-range','if-none-match','if-modified-since']){const value=request.headers.get(name);if(value!==null)headers.set(name,value);}
  let response;
  try {response=await fetch(new Request(upstream,{method:request.method,headers,redirect:'manual'}));}
  catch{return engineError('Engine asset is unavailable.',502);}
  // Redirects must never cross the configured origin, nor be reflected to clients.
  if(![200,206,304].includes(response.status)){
    await response.body?.cancel();
    return engineError('Engine asset is unavailable.',response.status===404?404:502);
  }
  const safe=new Headers({'Content-Type':type,'X-Content-Type-Options':'nosniff'});
  for(const name of ['content-length','content-encoding','content-range','accept-ranges','etag','last-modified']){const value=response.headers.get(name);if(value!==null)safe.set(name,value);}
  return new Response(request.method==='HEAD'||response.status===304?null:response.body,{status:response.status,headers:safe});
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname === "/discord" || url.pathname === "/discord/") {
      return new Response(null, {
        status: 302,
        headers: {
          Location: "https://discord.gg/Etq6rgxCjV",
          "Cache-Control": "no-store",
          "X-Robots-Tag": "noindex, nofollow, noarchive",
        },
      });
    }
    if (url.pathname === "/fics/socket") return bridgeFics(request);
    let response;

    if (url.pathname === "/robots.txt") {
      response = new Response(`User-agent: *\nAllow: /\nDisallow: /engine/\nDisallow: /pkg/\nDisallow: /engine-worker.js\n\nSitemap: ${url.origin}/sitemap.xml\n`, {
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      });
    } else if (url.pathname === "/engine/stockfish-19-ironwood-pool-v1.js") {
      response = await env.ASSETS.fetch(request);
    } else if (url.pathname.startsWith("/engine/")) {
      response = await proxyEngine(request,env,url);
    } else {
      response = await env.ASSETS.fetch(request);
    }

    const headers = new Headers(response.headers);
    for (const [name, value] of Object.entries(ISOLATION_HEADERS)) headers.set(name, value);
    if (url.hostname === "127.0.0.1" || url.hostname === "localhost") {
      headers.set("Cache-Control", "no-store");
    }
    if (url.pathname.startsWith("/engine/") && [200,206,304].includes(response.status) && url.hostname !== "127.0.0.1" && url.hostname !== "localhost") {
      headers.set("Cache-Control", "public, max-age=31536000, immutable");
    }
    if (url.pathname === "/app-version.json" || url.pathname === "/service-worker.js") {
      headers.set("Cache-Control", "no-store, max-age=0");
    }
    if (url.pathname.endsWith(".wasm") && [200,206,304].includes(response.status)) headers.set("Content-Type", "application/wasm");
    if (url.pathname.startsWith("/engine/") || url.pathname.startsWith("/pkg/") || url.pathname === "/engine-worker.js") {
      headers.set("X-Robots-Tag", "noindex, nofollow, noarchive");
    }
    return new Response(response.body, { status: response.status, headers });
  },
};

function bridgeFics(request) {
  if (request.headers.get("Upgrade")?.toLowerCase() !== "websocket") {
    return new Response("WebSocket required", { status: 426 });
  }
  if (request.headers.get("Origin") !== new URL(request.url).origin) {
    return new Response("Invalid origin", { status: 403 });
  }
  let socket;
  try {
    socket = connect({ hostname: "freechess.org", port: 5000 });
  } catch {
    return new Response("FICS is unavailable", { status: 502 });
  }
  const [client, server] = Object.values(new WebSocketPair());
  server.accept();
  const writer = socket.writable.getWriter();
  const encoder = new TextEncoder();
  let pending = Promise.resolve();
  let closed = false;
  let registeredLoginNameSent = false;
  let registeredPasswordSent = false;
  let awaitingPassword = false;
  let sessionReady = false;
  let serverPromptTail = '';
  const close = () => {
    if (closed) return;
    closed = true;
    socket.close();
    try { server.close(1000, "FICS connection closed"); } catch { /* already closed */ }
  };
  server.addEventListener("message", event => {
    if (typeof event.data !== "string") {
      close();
      return;
    }
    let command = event.data;
    if (command.startsWith('{')) {
      let credential;
      try { credential = JSON.parse(command); } catch { close(); return; }
      if (credential?.type === 'command' && sessionReady &&
          typeof credential.value === 'string' && /^[\x20-\x7e]{1,256}$/.test(credential.value)) {
        command = credential.value;
      } else {
        if (!registeredLoginNameSent || registeredPasswordSent || !awaitingPassword ||
            credential?.type !== 'password' || typeof credential.value !== 'string' ||
            !/^[\x21-\x7e]{1,128}$/.test(credential.value)) {
          close();
          return;
        }
        registeredPasswordSent = true;
        awaitingPassword = false;
        command = credential.value;
      }
    } else if (command !== '' && !FICS_COMMAND.test(command)) {
      if (registeredLoginNameSent || !/^[A-Za-z]{3,17}$/.test(command)) {
        close();
        return;
      }
      registeredLoginNameSent = true;
    }
    pending = pending.then(() => writer.write(encoder.encode(command + "\n"))).catch(close);
  });
  server.addEventListener("close", close);
  server.addEventListener("error", close);
  (async () => {
    try {
      const reader = socket.readable.getReader();
      const decoder = new TextDecoder();
      while (!closed) {
        const { value, done } = await reader.read();
        if (done) break;
        const data = decoder.decode(value, { stream: true });
        if (data) {
          serverPromptTail = (serverPromptTail + data.replace(/\x1b\[[0-9;]*[A-Za-z]/g, '').replace(/\r/g, '')).slice(-256);
          if (/password:\s*$/i.test(serverPromptTail)) awaitingPassword = true;
          if (/fics%\s*$/i.test(serverPromptTail)) sessionReady = true;
          server.send(data);
        }
      }
    } catch (error) {
      if (!closed) server.send(`\nConnection error: ${String(error)}\n`);
    } finally {
      close();
    }
  })();
  return new Response(null, { status: 101, webSocket: client });
}
