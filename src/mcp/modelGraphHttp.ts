import { MODELGRAPH_INSTRUCTIONS } from './modelGraphInstructions'
import { createServer } from 'node:http'
import { pathToFileURL } from 'node:url'
import { McpServer, WebStandardStreamableHTTPServerTransport } from '@modelcontextprotocol/server'
import { registerModelGraphTools } from './modelGraphTools'
import { HeadlessGeometryService } from './geometryService'
import { DirectGeometrySupervisor } from './directGeometrySupervisor'
import { defaultGeometryBuildEngine } from '../services/geometryBuildEngine'

/** Stateless, catalog-free endpoint. Put behind HTTPS and access control for remote use. */
export function createModelGraphHttpServer() {
  let active = 0
  return createServer(async (req, res) => {
    if (req.url !== '/mcp') { res.writeHead(404).end(); return }
    if (req.headers.origin) { res.writeHead(403).end('Browser-origin requests are not enabled'); return }
    if (req.method !== 'POST') { res.writeHead(405, { Allow: 'POST' }).end(); return }
    if (active >= 2) { res.writeHead(503, { 'Retry-After': '2' }).end(); return }
    active++
    const controller = new AbortController()
    const timer = setTimeout(() => { controller.abort(); if (!res.headersSent) res.writeHead(504); res.end(); if (!req.complete) req.destroy() }, 30_000)
    res.on('close', () => controller.abort())
    const runtime = new DirectGeometrySupervisor()
    let server: McpServer | undefined
    try {
      const chunks: Buffer[] = []; let bytes = 0
      for await (const chunk of req) {
        bytes += chunk.length
        if (bytes > 262144) { res.writeHead(413).end(); return }
        chunks.push(Buffer.from(chunk))
      }
      const geometry = new HeadlessGeometryService(defaultGeometryBuildEngine, runtime)
      server = new McpServer({ name: 'modelgraph', version: '0.1.0' }, { instructions: MODELGRAPH_INSTRUCTIONS + '\nHTTP mode has no persistent catalog.' })
      registerModelGraphTools(server, geometry)
      const transport = new WebStandardStreamableHTTPServerTransport({ sessionIdGenerator: undefined, enableJsonResponse: true })
      await server.connect(transport)
      const headers = new Headers()
      for (const [key, value] of Object.entries(req.headers)) if (value) headers.set(key, Array.isArray(value) ? value.join(',') : value)
      const response = await transport.handleRequest(new Request('http://localhost/mcp', { method: 'POST', headers, body: Buffer.concat(chunks), signal: controller.signal }))
      res.writeHead(response.status, Object.fromEntries(response.headers))
      res.end(Buffer.from(await response.arrayBuffer()))
    } catch { if (!res.headersSent) res.writeHead(400); res.end() }
    finally { clearTimeout(timer); await Promise.allSettled([server?.close(), runtime.close()]); active-- }
  })
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const port = Number(process.env.MODELGRAPH_PORT ?? 7433)
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error('Invalid MODELGRAPH_PORT')
  const server = createModelGraphHttpServer()
  server.requestTimeout = 30_000
  server.headersTimeout = 10_000
  server.listen(port, '127.0.0.1', () => console.error(`ModelGraph HTTP: http://127.0.0.1:${port}/mcp (loopback; use an HTTPS proxy/tunnel for remote clients)`))
  for (const signal of ['SIGINT', 'SIGTERM'] as const) process.on(signal, () => { server.close(); server.closeAllConnections() })
}
