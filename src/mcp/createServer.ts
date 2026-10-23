import {
  McpServer,
  SUPPORTED_PROTOCOL_VERSIONS,
  type RequestId,
} from '@modelcontextprotocol/server'
import {
  HeadlessGeometryService,
  type McpGeometryService,
} from './geometryService'
import { IndependentArtifactCache } from './independentArtifactCache'
import { assertGeometryManifestArchive } from './engineManifest'
import type { ModelStore } from './modelStore'
import { OfficialArtifactCache } from './officialArtifactCache'
import {
  OfficialOpenScadRuntimeSupervisor,
  type OfficialOpenScadRuntimeService,
} from './officialOpenScadRuntimeService'
import {
  CACHE_FIVE_MINUTES,
  MCP_GEOMETRY_HOST_CONTRACT,
  SERVER_INSTRUCTIONS,
} from './server/contracts'
import {
  persistedFailureStatus,
  registerMcpCatalogAndEngineTools,
} from './server/registerTools'
import { registerMcpPromptsAndResources } from './server/registerResourcesAndPrompts'

export { MCP_GEOMETRY_HOST_CONTRACT }
export { persistedFailureStatus }

export interface CreateOpenScadMcpServerOptions {
  store: ModelStore
  additionalInstructions?: string
  geometry?: McpGeometryService
  /** Optional upstream oracle used only by the explicitly named official tools. */
  officialRuntime?: OfficialOpenScadRuntimeService
  /** Releases transport admission only after an SDK request handler settles. */
  onRequestSettled?: (requestId: RequestId) => void
  /** Mirrors the SDK handler AbortSignal after protocol validation. */
  onRequestCancelled?: (requestId: RequestId) => void
}

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

  const deps = {
    server,
    store,
    geometry,
    officialRuntime,
    officialArtifacts,
    independentArtifacts,
    notifyResourceListChanged,
  }

  registerMcpCatalogAndEngineTools(deps)
  registerMcpPromptsAndResources(deps)

  if (options.onRequestSettled) {
    type DispatchHandler = (...args: unknown[]) => unknown
    const protocolServer = server.server as unknown as {
      setRequestHandler: (...args: unknown[]) => void
      _requestHandlers: Map<string, DispatchHandler>
    }
    const wrappedHandlers = new WeakSet<DispatchHandler>()
    const wrapDispatchHandler = (method: string) => {
      const handler = protocolServer._requestHandlers.get(method)
      if (!handler || wrappedHandlers.has(handler)) return
      const wrapped: DispatchHandler = async (...args: unknown[]) => {
        const context = args.at(-1) as {
          mcpReq?: { id?: RequestId; signal?: AbortSignal }
        } | undefined
        const requestId = context?.mcpReq?.id
        const signal = context?.mcpReq?.signal
        if (requestId !== undefined && signal) {
          const cancelled = () => options.onRequestCancelled?.(requestId)
          if (signal.aborted) cancelled()
          else signal.addEventListener('abort', cancelled, { once: true })
        }
        try {
          return await handler(...args)
        } finally {
          if (requestId !== undefined) options.onRequestSettled?.(requestId)
        }
      }
      wrappedHandlers.add(wrapped)
      protocolServer._requestHandlers.set(method, wrapped)
    }

    // MCP SDK 2.0 has no public outer-dispatch middleware. Wrap finalized
    // registry entries so lifecycle covers SDK validation and application
    // callbacks. serveStdio installs a fresh modern discover handler after the
    // factory returns, so future registrations must pass through the same seam.
    const originalSetRequestHandler = protocolServer.setRequestHandler.bind(protocolServer)
    protocolServer.setRequestHandler = (...registration: unknown[]) => {
      originalSetRequestHandler(...registration)
      if (typeof registration[0] === 'string') wrapDispatchHandler(registration[0])
    }
    for (const method of protocolServer._requestHandlers.keys()) wrapDispatchHandler(method)
  }

  return server
}
