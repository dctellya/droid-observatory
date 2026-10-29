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
