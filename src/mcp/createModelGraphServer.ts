import { createOpenScadMcpServer, type CreateOpenScadMcpServerOptions } from './createServer'
import { HeadlessGeometryService } from './geometryService'
import { registerModelGraphTools } from './modelGraphTools'

/** Extend the frozen MCP factory without changing its qualification evidence. */
export function createModelGraphMcpServer(options: CreateOpenScadMcpServerOptions) {
  const geometry = options.geometry ?? new HeadlessGeometryService()
  const server = createOpenScadMcpServer({ ...options, geometry })
  registerModelGraphTools(server, geometry)
  return server
}
