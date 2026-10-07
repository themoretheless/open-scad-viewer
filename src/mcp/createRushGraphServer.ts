import { RUSH_GRAPH_INSTRUCTIONS } from './rushGraphInstructions'
import { createOpenScadMcpServer, type CreateOpenScadMcpServerOptions } from './createServer'
import { HeadlessGeometryService } from './geometryService'
import { registerRushGraphTools } from './rushGraphTools'

/** Extend the frozen MCP factory without changing its qualification evidence. */
export function createRushGraphMcpServer(options: CreateOpenScadMcpServerOptions) {
  const geometry = options.geometry ?? new HeadlessGeometryService()
  const server = createOpenScadMcpServer({ ...options, geometry, additionalInstructions: [RUSH_GRAPH_INSTRUCTIONS, options.additionalInstructions].filter(Boolean).join('\n\n') })
  registerRushGraphTools(server, geometry)
  return server
}
