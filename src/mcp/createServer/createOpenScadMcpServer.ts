import { HeadlessGeometryService } from '../geometryService'
import { CACHE_FIVE_MINUTES, SERVER_INSTRUCTIONS } from '../createServer'
import type { CreateOpenScadMcpServerOptions } from '../createServer'
import { IndependentArtifactCache } from '../independentArtifactCache'
import { McpServer, SUPPORTED_PROTOCOL_VERSIONS } from '@modelcontextprotocol/server'
import { OfficialArtifactCache } from '../officialArtifactCache'
import { OfficialOpenScadRuntimeSupervisor } from '../officialOpenScadRuntimeService'
import { assertGeometryManifestArchive } from '../engineManifest'

import { registerCatalogTools } from './register/registerCatalogTools'
import { registerEngineTools } from './register/registerEngineTools'
import { registerOfficialTools } from './register/registerOfficialTools'
import { registerIndependentTools } from './register/registerIndependentTools'
import { registerBuildTools } from './register/registerBuildTools'
import { registerPrompts } from './register/registerPrompts'
import { registerStaticResources } from './register/registerStaticResources'
import { registerTemplateResources } from './register/registerTemplateResources'
import { installRequestLifecycle } from './register/installRequestLifecycle'
import type { McpServerContext } from './register/context'

export function createOpenScadMcpServer(options: CreateOpenScadMcpServerOptions): McpServer {
  assertGeometryManifestArchive()
  const { store } = options
  const geometry = options.geometry ?? new HeadlessGeometryService()
  const officialRuntime = options.officialRuntime ?? new OfficialOpenScadRuntimeSupervisor()
  const officialArtifacts = new OfficialArtifactCache()
  const independentArtifacts = new IndependentArtifactCache()
  const server = new McpServer(
    {
      name: 'open-scad-viewer',
      version: '0.1.0',
      description: 'Bounded local OpenSCAD analysis, customization, revision storage, and STL/OBJ export.',
    },
    {
      supportedProtocolVersions: ['2026-07-28', ...SUPPORTED_PROTOCOL_VERSIONS],
      instructions: [options.additionalInstructions, SERVER_INSTRUCTIONS].filter(Boolean).join('\n\n'),
      cacheHints: {
        'server/discover': { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
        'tools/list': { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
        'prompts/list': { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
        'resources/templates/list': { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
      },
    },
  )
  const notifyResourceListChanged = () => {
    void server.server.sendResourceListChanged().catch(error => {
      console.error('Could not send MCP resource-list notification:', error)
    })
  }

  const ctx: McpServerContext = { options, store, geometry, officialRuntime, officialArtifacts, independentArtifacts, server, notifyResourceListChanged }
  registerCatalogTools(ctx)
  registerEngineTools(ctx)
  registerOfficialTools(ctx)
  registerIndependentTools(ctx)
  registerBuildTools(ctx)
  registerPrompts(ctx)
  registerStaticResources(ctx)
  registerTemplateResources(ctx)
  installRequestLifecycle(ctx)

  return server
}
