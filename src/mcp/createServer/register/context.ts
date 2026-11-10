import type { McpServer } from '@modelcontextprotocol/server'
import type { CreateOpenScadMcpServerOptions } from '../../createServer'
import type { HeadlessGeometryService } from '../../geometryService'
import type { IndependentArtifactCache } from '../../independentArtifactCache'
import type { OfficialArtifactCache } from '../../officialArtifactCache'
import type { OfficialOpenScadRuntimeSupervisor } from '../../officialOpenScadRuntimeService'

/** Everything the registration modules share; built once by `createOpenScadMcpServer`. */
export interface McpServerContext {
  readonly options: CreateOpenScadMcpServerOptions
  readonly store: CreateOpenScadMcpServerOptions['store']
  readonly geometry: NonNullable<CreateOpenScadMcpServerOptions['geometry']> | HeadlessGeometryService
  readonly officialRuntime: NonNullable<CreateOpenScadMcpServerOptions['officialRuntime']> | OfficialOpenScadRuntimeSupervisor
  readonly officialArtifacts: OfficialArtifactCache
  readonly independentArtifacts: IndependentArtifactCache
  readonly server: McpServer
  readonly notifyResourceListChanged: () => void
}
