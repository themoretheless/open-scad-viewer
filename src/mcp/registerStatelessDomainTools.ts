import type { McpServer } from '@modelcontextprotocol/server'
import { registerBrepDiagnosticTools } from './brepDiagnosticTools'
import { registerCadExchangeTools } from './cadExchangeTools'
import { registerCaeTools } from './caeTools'
import { registerCamTools } from './camTools'
import { registerCadDomainPrompts } from './createServer/register/registerPrompts'
import { registerDirectModelingTools } from './directModelingTools'
import { registerDrawingTools } from './drawingTools'
import type { McpGeometryService } from './geometryService'
import { registerMeshConvertTool } from './meshConvertTool'
import { registerMeshManifoldTools } from './meshManifoldTools'
import { registerDirectTransactionTool } from './registerDirectTransactionTool'
import { registerRushGraphTools } from './rushGraphTools'
import { createStaticToolRegistration } from './staticToolSchemas'
import { hasProfile, type McpToolsetProfile } from './toolsetProfiles'

const staticToolRegistration = createStaticToolRegistration()

/** Register stateless, catalog-free CAD/CAM/CAE domain tools filtered by active profiles. */
export function registerStatelessDomainTools(
  server: McpServer,
  geometry: McpGeometryService,
  profiles?: ReadonlySet<McpToolsetProfile>,
): void {
  const staticServer = staticToolRegistration(server)
  if (hasProfile(profiles, 'rush')) {
    registerRushGraphTools(server, geometry)
  }
  if (hasProfile(profiles, 'direct')) {
    registerDirectTransactionTool(staticServer)
    registerDirectModelingTools(staticServer)
    registerBrepDiagnosticTools(staticServer)
  }
  if (hasProfile(profiles, 'exchange')) {
    if (!hasProfile(profiles, 'rush')) {
      registerMeshConvertTool(staticServer)
    }
    registerCadExchangeTools(staticServer)
    registerMeshManifoldTools(staticServer, geometry)
  }
  if (hasProfile(profiles, 'drawing')) {
    registerDrawingTools(staticServer, geometry)
  }
  if (hasProfile(profiles, 'cam')) {
    registerCamTools(staticServer, geometry)
  }
  if (hasProfile(profiles, 'cae')) {
    registerCaeTools(staticServer)
  }
  registerCadDomainPrompts(server, profiles)
}
