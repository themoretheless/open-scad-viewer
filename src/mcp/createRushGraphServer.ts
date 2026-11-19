import { RUSH_GRAPH_INSTRUCTIONS } from './rushGraphInstructions'
import { createOpenScadMcpServer, type CreateOpenScadMcpServerOptions } from './createServer'
import { HeadlessGeometryService } from './geometryService'
import { registerStatelessDomainTools } from './registerStatelessDomainTools'
import { hasProfile } from './toolsetProfiles'

/** Extend the frozen MCP factory without changing its qualification evidence. */
export function createRushGraphMcpServer(options: CreateOpenScadMcpServerOptions) {
  const geometry = options.geometry ?? new HeadlessGeometryService()
  const includeRushInstructions = hasProfile(options.toolsetProfiles, 'rush')
  const server = createOpenScadMcpServer({
    ...options,
    geometry,
    additionalInstructions: [
      includeRushInstructions ? RUSH_GRAPH_INSTRUCTIONS : undefined,
      options.additionalInstructions,
    ]
      .filter(Boolean)
      .join('\n\n'),
  })
  registerStatelessDomainTools(server, geometry, options.toolsetProfiles)
  return server
}
