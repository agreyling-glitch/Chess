const ISOLATION_HEADERS = {
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Embedder-Policy": "require-corp",
  "Cross-Origin-Resource-Policy": "same-origin",
};

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
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
    if (
      (url.hostname === "127.0.0.1" || url.hostname === "localhost") &&
      !url.pathname.startsWith("/engine/")
    ) {
      headers.set("Cache-Control", "no-store");
    }
    if (url.pathname.startsWith("/engine/")) {
      headers.set("Cache-Control", "public, max-age=31536000, immutable");
    }
    if (url.pathname.endsWith(".wasm")) headers.set("Content-Type", "application/wasm");
    if (url.pathname.startsWith("/engine/") || url.pathname.startsWith("/pkg/") || url.pathname === "/engine-worker.js") {
      headers.set("X-Robots-Tag", "noindex, nofollow, noarchive");
    }
    return new Response(response.body, { status: response.status, headers });
  },
};
