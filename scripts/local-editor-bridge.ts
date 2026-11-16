import { randomUUID } from 'node:crypto'
import type { IncomingMessage, ServerResponse } from 'node:http'
import type { Plugin } from 'vite'
import {
  LOCAL_EDITOR_HEADER, LOCAL_EDITOR_MAX_BYTES, LOCAL_EDITOR_PATH, LOCAL_EDITOR_TIMEOUT_MS,
} from '../src/core/localEditorProtocol'

function respond(res: ServerResponse, status: number, value?: unknown) {
  res.writeHead(status, {
    'Cache-Control': 'no-store', 'Content-Type': 'application/json', [LOCAL_EDITOR_HEADER]: 'bridge',
  })
  res.end(value === undefined ? undefined : JSON.stringify(value))
}

/** Only the local Vite server brokers commands. The browser must acknowledge each one. */
export function localEditorBridge(): Plugin {
  let pending: {
    id: string
    bundle: unknown
    claimed: boolean
    finish: (status: number) => void
  } | undefined
  let lastEditorPoll = 0

  async function middleware(req: IncomingMessage, res: ServerResponse, next: () => void) {
    if (req.url !== LOCAL_EDITOR_PATH) return next()
    const host = req.headers.host ?? ''
    const localHost = /^(?:127\.0\.0\.1|localhost|\[::1\])(?::\d{1,5})?$/.test(host)
    const localPeer = ['127.0.0.1', '::1', '::ffff:127.0.0.1'].includes(req.socket.remoteAddress ?? '')
    const origin = req.headers.origin
    if (!localHost || !localPeer || (origin && origin !== `http://${host}` && origin !== `https://${host}`)) {
      return respond(res, 403)
    }

    const command = req.headers[LOCAL_EDITOR_HEADER]
    const fetchSite = req.headers['sec-fetch-site']
    if (req.method === 'GET' && command === 'poll' && fetchSite === 'same-origin') {
      lastEditorPoll = Date.now()
      if (!pending || pending.claimed) return respond(res, 204)
      pending.claimed = true
      return respond(res, 200, { id: pending.id, bundle: pending.bundle })
    }
    if (req.method === 'POST' && fetchSite === 'same-origin') {
      const [result, id] = String(command).split(':')
      if (!pending || !pending.claimed || id !== pending.id || !['opened', 'blocked'].includes(result)) {
        return respond(res, 409)
      }
      pending.finish(result === 'opened' ? 200 : 422)
      return respond(res, 204)
    }
    if (req.method !== 'POST' || command !== 'open' || origin || fetchSite) return respond(res, 403)
    if (Date.now() - lastEditorPoll > 4_000) return respond(res, 412)
    if (pending) return respond(res, 409)

    let bytes = 0
    const chunks: Buffer[] = []
    try {
      // Leaving the stream intact lets the HTTP client receive a typed size error.
      for await (const chunk of req.iterator({ destroyOnReturn: false })) {
        bytes += chunk.length
        if (bytes > LOCAL_EDITOR_MAX_BYTES) return respond(res, 413)
        chunks.push(chunk)
      }
      const bundle: unknown = JSON.parse(Buffer.concat(chunks).toString('utf8'))
      if (pending || res.destroyed) return respond(res, 409)
      const finish = (status: number) => {
        clearTimeout(timer)
        pending = undefined
        if (!res.destroyed) respond(res, status)
      }
      const timer = setTimeout(() => finish(504), LOCAL_EDITOR_TIMEOUT_MS)
      pending = { id: randomUUID(), bundle, claimed: false, finish }
      res.once('close', () => { if (pending?.finish === finish) finish(499) })
    } catch {
      if (!res.destroyed) respond(res, 400)
    }
  }

  return {
    name: 'openscada-local-mcp-editor',
    configureServer(server) { server.middlewares.use(middleware) },
    configurePreviewServer(server) { server.middlewares.use(middleware) },
  }
}
