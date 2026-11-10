import { engineRegistrySchema } from '../../createServer'
import { engineRegistryToWire, toolOutputSchema } from '../officialToolInputSchema'
import { errorResult, textResult } from '../persistedFailureStatus'
import { z } from 'zod/v4'
import type { McpServerContext } from './context'

/** Engine registry and B-rep capability discovery. */
export function registerEngineTools(ctx: McpServerContext): void {
  const { geometry, server } = ctx
  server.registerTool('openscad_list_engines', {
    title: 'List geometry engines',
    description: 'Return both permanent geometry engine classes, their source-language routes, runtime availability, manifests, and the no-fallback policy.',
    inputSchema: z.object({}),
    outputSchema: toolOutputSchema(z.object({ geometry_engines: engineRegistrySchema })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async () => {
    try {
      return textResult({
        geometry_engines: engineRegistryToWire(await geometry.capabilities()),
      })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_brep_capabilities', {
    title: 'List B-rep capability maturity',
    description: 'Return the honest B-rep/NURBS capability maturity matrix. Unavailable and ResearchOnly entries never claim product readiness. B-rep failure never falls back to Manifold.',
    inputSchema: z.object({}).strict(),
    outputSchema: toolOutputSchema(z.object({
      cross_engine_fallback: z.literal(false),
      capabilities: z.array(z.object({
        id: z.string(),
        maturity: z.enum(['Unavailable', 'ResearchOnly', 'NumericUncertified', 'AnalyticComplete', 'Qualified']),
        permits_topology_change: z.boolean(),
        notes: z.string(),
      })),
    }).strict()),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async () => {
    try {
      const { BREP_CAPABILITY_MATRIX } = await import('../../../services/geometry/brepCapability')
      return textResult({
        cross_engine_fallback: false as const,
        capabilities: BREP_CAPABILITY_MATRIX.map(entry => ({
          id: entry.id,
          maturity: entry.maturity,
          permits_topology_change: entry.permitsTopologyChange,
          notes: entry.notes,
        })),
      })
    } catch (error) {
      return errorResult(error)
    }
  })
}
