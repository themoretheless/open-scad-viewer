import { createServer } from 'node:http'
import { pathToFileURL } from 'node:url'
import { McpServer, WebStandardStreamableHTTPServerTransport } from '@modelcontextprotocol/server'
import { defaultGeometryBuildEngine } from '../kernel/geometryBuildEngine'
import { DirectGeometrySupervisor } from './directGeometrySupervisor'
import { HeadlessGeometryService } from './geometryService'
import { registerStatelessDomainTools } from './registerStatelessDomainTools'
import { RUSH_GRAPH_INSTRUCTIONS } from './rushGraphInstructions'
import { hasProfile, parseMcpToolsetProfiles, type McpToolsetProfile } from './toolsetProfiles'

export interface RushGraphHttpOptions {
  readonly toolsetProfiles?: ReadonlySet<McpToolsetProfile>
  readonly maxBodyBytes?: number
}

/** Stateless, catalog-free endpoint. Put behind HTTPS and access control for remote use. */
export function createRushGraphHttpServer(options: RushGraphHttpOptions = {}) {
  const maxBodyBytes = options.maxBodyBytes ?? 262_144
  const runtime = new DirectGeometrySupervisor()
  const geometry = new HeadlessGeometryService(defaultGeometryBuildEngine, runtime)
  let active = 0

  const httpServer = createServer(async (req, res) => {
    if (req.url !== '/mcp') {
      res.writeHead(404).end()
      return
    }
    if (req.headers.origin) {
      res.writeHead(403).end('Browser-origin requests are not enabled')
      return
    }
    if (req.method !== 'POST') {
      res.writeHead(405, { Allow: 'POST' }).end()
      return
    }
    if (active >= 2) {
      res.writeHead(503, { 'Retry-After': '2' }).end()
      return
    }
    active++
    const controller = new AbortController()
    const timer = setTimeout(() => {
      controller.abort()
      if (!res.headersSent) res.writeHead(504)
      res.end()
      if (!req.complete) req.destroy()
    }, 30_000)
    res.on('close', () => controller.abort())
    let server: McpServer | undefined
    try {
      const chunks: Buffer[] = []
      let bytes = 0
      for await (const chunk of req) {
        bytes += chunk.length
        if (bytes > maxBodyBytes) {
          res.writeHead(413).end()
          return
        }
        chunks.push(Buffer.from(chunk))
      }
      const instructions = [
        hasProfile(options.toolsetProfiles, 'rush') ? RUSH_GRAPH_INSTRUCTIONS : undefined,
        'HTTP mode has no persistent catalog.',
      ]
        .filter(Boolean)
        .join('\n')
      server = new McpServer({ name: 'rush', version: '0.1.0' }, { instructions })
      registerStatelessDomainTools(server, geometry, options.toolsetProfiles)
      const transport = new WebStandardStreamableHTTPServerTransport({
        sessionIdGenerator: undefined,
        enableJsonResponse: true,
      })
      await server.connect(transport)
      const headers = new Headers()
      for (const [key, value] of Object.entries(req.headers)) {
        if (value) headers.set(key, Array.isArray(value) ? value.join(',') : value)
      }
      const response = await transport.handleRequest(
        new Request('http://localhost/mcp', {
          method: 'POST',
          headers,
          body: Buffer.concat(chunks),
          signal: controller.signal,
        }),
      )
      res.writeHead(response.status, Object.fromEntries(response.headers))
      res.end(Buffer.from(await response.arrayBuffer()))
    } catch {
      if (!res.headersSent) res.writeHead(400)
      res.end()
    } finally {
      clearTimeout(timer)
      await Promise.allSettled([server?.close()])
      active--
    }
  })

  httpServer.once('close', () => {
    void runtime.close()
  })
  return httpServer
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const port = Number(process.env.RUSH_GRAPH_PORT ?? 7433)
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error('Invalid RUSH_GRAPH_PORT')
  const rawMaxBody = process.env.RUSH_GRAPH_MAX_BODY_BYTES
  const maxBodyBytes = rawMaxBody === undefined ? 24 * 1024 * 1024 : Number(rawMaxBody)
  if (!Number.isFinite(maxBodyBytes) || maxBodyBytes <= 0) {
    throw new Error('Invalid RUSH_GRAPH_MAX_BODY_BYTES')
  }
  const server = createRushGraphHttpServer({
    maxBodyBytes,
    ...(process.env.OPENSCAD_VIEWER_MCP_PROFILES !== undefined
      ? { toolsetProfiles: parseMcpToolsetProfiles(process.env.OPENSCAD_VIEWER_MCP_PROFILES) }
      : {}),
  })
  server.requestTimeout = 30_000
  server.headersTimeout = 10_000
  server.listen(port, '127.0.0.1', () =>
    console.error(
      `RushGraph HTTP: http://127.0.0.1:${port}/mcp (loopback; use an HTTPS proxy/tunnel for remote clients)`,
    ),
  )
  for (const signal of ['SIGINT', 'SIGTERM'] as const) {
    process.on(signal, () => {
      server.close()
      server.closeAllConnections()
    })
  }
}
