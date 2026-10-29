import { createServer, type Server } from 'node:http';
import { FORGE_STUB_PORT } from './env.js';

/**
 * A loopback stand-in for api.github.com that answers every merge request with the 403 a token without
 * `pull_requests:write` gets, and keeps what it was asked. The app reaches it through `[github].api_base`
 * (lib/env.ts); a real 403 needs the owner's read-only token, which no process a worker starts may read.
 */
export interface ForgeStub {
  /** `METHOD path` of every request, in order. */
  requests: string[];
  close(): Promise<void>;
}

export function startForgeStub(): Promise<ForgeStub> {
  const requests: string[] = [];
  const server: Server = createServer((request, response) => {
    requests.push(`${request.method} ${request.url}`);
    const merge = request.method === 'PUT' && /\/pulls\/\d+\/merge$/.test(request.url ?? '');
    response.writeHead(merge ? 403 : 404, {
      'content-type': 'application/json',
      'x-ratelimit-remaining': '4999',
    });
    response.end(JSON.stringify({ message: merge ? 'Resource not accessible by personal access token' : 'Not Found' }));
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(FORGE_STUB_PORT, '127.0.0.1', () =>
      resolve({
        requests,
        close: () => new Promise<void>((done) => server.close(() => done())),
      }),
    );
  });
}

/**
 * The live walk's forge: reads go to api.github.com with the token the app sent, the merge request gets the
 * 403 a read-only token gets. The owner's read-only token stays in the keychain no worker may open, so this
 * stands in for it at the one request that differs; `gh pr merge` by the owner is the real merge.
 */
export function startForgeProxy(): Promise<ForgeStub> {
  const requests: string[] = [];
  const server: Server = createServer((request, response) => {
    const path = request.url ?? '/';
    requests.push(`${request.method} ${path}`);
    if (request.method === 'PUT' && /\/pulls\/\d+\/merge$/.test(path)) {
      response.writeHead(403, { 'content-type': 'application/json', 'x-ratelimit-remaining': '4999' });
      response.end(JSON.stringify({ message: 'Resource not accessible by personal access token' }));
      return;
    }
    const headers: Record<string, string> = { 'user-agent': 'droid-observatory-live-walk', accept: 'application/vnd.github+json' };
    if (request.headers.authorization) headers.authorization = request.headers.authorization;
    fetch(`https://api.github.com${path}`, { headers })
      .then(async (upstream) => {
        const out: Record<string, string> = { 'content-type': upstream.headers.get('content-type') ?? 'application/json' };
        for (const name of ['x-ratelimit-remaining', 'x-ratelimit-reset', 'link']) {
          const value = upstream.headers.get(name);
          if (value) out[name] = value;
        }
        response.writeHead(upstream.status, out);
        response.end(Buffer.from(await upstream.arrayBuffer()));
      })
      .catch(() => {
        response.writeHead(502);
        response.end();
      });
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(FORGE_STUB_PORT, '127.0.0.1', () =>
      resolve({
        requests,
        close: () => new Promise<void>((done) => server.close(() => done())),
      }),
    );
  });
}
