import { connect } from "cloudflare:sockets";

const ISOLATION_HEADERS = {
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Embedder-Policy": "require-corp",
  "Cross-Origin-Resource-Policy": "same-origin",
};

const FICS_COMMAND = /^(?:guest|set style 12|set seek 0|seek (?:[1-9]|[1-5]\d|60) (?:[0-9]|[1-5]\d|60) unrated|unseek|sought|play \d{1,4}|match [A-Za-z][A-Za-z0-9_-]{0,19} unrated (?:[1-9]|[1-5]\d|60) (?:[0-9]|[1-5]\d|60)|accept|decline|draw|resign|quit|[a-h][1-8][a-h][1-8][qrbn]?)$/;

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
    } else if (url.pathname.startsWith("/engine/")) {
      if (!env.ENGINE_ORIGIN) {
        return new Response("The full-NNUE engine origin is not configured.", { status: 503 });
      }
      const upstream = new URL(url.pathname.replace("/engine/", "/"), env.ENGINE_ORIGIN);
      response = await fetch(new Request(upstream, request));
    } else {
      response = await env.ASSETS.fetch(request);
    }

    const headers = new Headers(response.headers);
    for (const [name, value] of Object.entries(ISOLATION_HEADERS)) headers.set(name, value);
    if (url.hostname === "127.0.0.1" || url.hostname === "localhost") {
      headers.set("Cache-Control", "no-store");
    }
    if (url.pathname.startsWith("/engine/") && url.hostname !== "127.0.0.1" && url.hostname !== "localhost") {
      headers.set("Cache-Control", "public, max-age=31536000, immutable");
    }
    if (url.pathname === "/app-version.json" || url.pathname === "/service-worker.js") {
      headers.set("Cache-Control", "no-store, max-age=0");
    }
    if (url.pathname.endsWith(".wasm")) headers.set("Content-Type", "application/wasm");
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
