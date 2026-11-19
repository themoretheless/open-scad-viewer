import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import type { VolumeValidityLimits } from '../kernel/solidDistance'
import type { FaceContactLimits } from '../kernel/solidFaceContacts'
import { brepSourceInputSchema, resolveBrepFromInput } from './brepInputHelper'
import { ensureRawSourceLoader } from './ensureRawSourceLoader'
import { toolOutputSchema } from './createServer/officialToolInputSchema'
import { errorResult, textResult } from './createServer/persistedFailureStatus'
import { InvalidToolInputError } from './publicError'

const DEFAULT_VOLUME_VALIDITY_LIMITS: VolumeValidityLimits = {
  exactWork: 1_000_000,
  faceCells: 100_000,
  faceCellsPerPair: 1_000,
  faceDomainCells: 1_000_000,
  faceDomainCellsPerPair: 10_000,
  facePairs: 10_000,
  nestingCells: 100_000,
  nestingDomainCells: 1_000_000,
  nestingPairs: 100,
  orientationCells: 100_000,
  orientationDomainCells: 1_000_000,
  orientationSpans: 100,
  spans: 1_000,
  trimCells: 100_000,
  trimDomainCells: 1_000_000,
  trimPairs: 10_000,
}

function registerBrepInspectContactsTool(server: McpServer): void {
  server.registerTool(
    'brep_inspect_contacts',
    {
      title: 'Inspect exact B-rep topology, face contacts, and self-intersections',
      description:
        'Audit an exact NURBS B-rep model for topological validity, whole-face injectivity, pairwise face contacts, and absence of self-intersections.',
      inputSchema: z
        .object({
          model: brepSourceInputSchema,
          tolerance_uv: z.number().min(1e-10).max(1e-2).optional(),
          max_cells: z.number().int().min(16).max(50_000).optional(),
        })
        .strict(),
      outputSchema: toolOutputSchema(
        z.object({
          valid: z.boolean(),
          face_count: z.number().int().nonnegative(),
          edge_count: z.number().int().nonnegative(),
          vertex_count: z.number().int().nonnegative(),
          all_faces_injective: z.boolean(),
          absence_proven: z.boolean(),
          all_pairs_classified: z.boolean(),
          pair_counts: z.object({
            total: z.number().int().nonnegative(),
            shared_boundary: z.number().int().nonnegative(),
            pair_disjoint: z.number().int().nonnegative(),
            interior_contact: z.number().int().nonnegative(),
            unresolved: z.number().int().nonnegative(),
          }),
          markdown_report: z.string(),
        }),
      ),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
    },
    async input => {
      try {
        ensureRawSourceLoader()
        const [{ inspectNurbsBrep }, { inspectFaceContacts }, { inspectSelfIntersection }] = await Promise.all([
          import('../kernel/geometry/brep'),
          import('../kernel/solidFaceContacts'),
          import('../kernel/solidSelfIntersection'),
        ])
        const brep = await resolveBrepFromInput(input.model)
        const toleranceUv = input.tolerance_uv ?? 1e-6
        const cellsPerPair = input.max_cells ?? 2_000
        const limits: FaceContactLimits = {
          maxPairs: 10_000,
          maxCells: Math.min(cellsPerPair * 20, 200_000),
          maxDomainCells: 1_000_000,
          cellsPerPair,
          domainCellsPerPair: 10_000,
          maxBoxes: 1_000,
        }
        let report: ReturnType<typeof inspectNurbsBrep>
        let contacts: ReturnType<typeof inspectFaceContacts>
        let selfIntersection: ReturnType<typeof inspectSelfIntersection>
        try {
          report = inspectNurbsBrep(brep)
          contacts = inspectFaceContacts(brep, toleranceUv, limits)
          selfIntersection = inspectSelfIntersection(brep, toleranceUv, limits, 1_000)
        } catch (error) {
          throw new InvalidToolInputError(error instanceof Error ? error.message : 'B-rep inspection failed.', {
            code: 'invalid_geometry',
          })
        }
        const markdown_report = [
          `# B-rep Contact & Self-Intersection Audit`,
          `- Topology: faces=${brep.faces.length}, edges=${brep.edges.length}, vertices=${brep.vertices.length}, shell_count=${brep.shells.length}`,
          `- Injective faces: ${selfIntersection.allFacesInjective}, Self-intersection absence proven: ${selfIntersection.absenceProven}`,
          `- Face pairs (${contacts.totalPairs}): shared_boundary=${contacts.sharedBoundaryPairCount}, disjoint=${contacts.disjointPairCount}, interior_contact=${contacts.contactPairCount}, unresolved=${contacts.unresolvedPairCount}`,
        ].join('\n')
        return textResult({
          valid: report.topologyValid && brep.shells.length > 0 && brep.faces.length > 0,
          face_count: brep.faces.length,
          edge_count: brep.edges.length,
          vertex_count: brep.vertices.length,
          all_faces_injective: selfIntersection.allFacesInjective,
          absence_proven: selfIntersection.absenceProven,
          all_pairs_classified: contacts.allPairsClassified,
          pair_counts: {
            total: contacts.totalPairs,
            shared_boundary: contacts.sharedBoundaryPairCount,
            pair_disjoint: contacts.disjointPairCount,
            interior_contact: contacts.contactPairCount,
            unresolved: contacts.unresolvedPairCount,
          },
          markdown_report,
        })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

function registerBrepSolidDistanceTool(server: McpServer): void {
  server.registerTool(
    'brep_solid_distance',
    {
      title: 'Measure certified distance between two exact B-rep solids',
      description:
        'Run interval-certified solid-to-solid B-rep distance and material containment verification between two NURBS B-rep models.',
      inputSchema: z
        .object({
          a: brepSourceInputSchema,
          b: brepSourceInputSchema,
          tolerance_mm: z.number().min(1e-6).max(10).optional(),
        })
        .strict(),
      outputSchema: toolOutputSchema(
        z.object({
          distance_lower_mm: z.number().nullable(),
          distance_upper_mm: z.number().nullable(),
          certified: z.boolean(),
          material_overlap: z.boolean().nullable(),
          relation: z.enum([
            'volume-validity-unproven',
            'shell-pairs-incomplete',
            'containment-unproven',
            'material-containment',
            'separated-volumes',
            'boundary-distance-unresolved',
            'certified-boundary-contact',
          ]),
          markdown_report: z.string(),
        }),
      ),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
    },
    async input => {
      try {
        ensureRawSourceLoader()
        const [{ measureSolidDistance }, [brepA, brepB]] = await Promise.all([
          import('../kernel/solidDistance'),
          Promise.all([resolveBrepFromInput(input.a), resolveBrepFromInput(input.b)]),
        ])
        let result: ReturnType<typeof measureSolidDistance>
        try {
          result = measureSolidDistance({
            a: brepA,
            b: brepB,
            toleranceMm: input.tolerance_mm ?? 1e-3,
            toleranceUv: 1e-8,
            maxPairs: 100,
            maxContactPairs: 1_000,
            maxCells: 100_000,
            maxDomainCells: 1_000_000,
            validityLimits: DEFAULT_VOLUME_VALIDITY_LIMITS,
          })
        } catch (error) {
          throw new InvalidToolInputError(
            error instanceof Error ? error.message : 'Solid distance measurement failed.',
            {
              code: 'invalid_geometry',
            },
          )
        }
        const lower = result.distanceIntervalMm?.[0] ?? null
        const upper = result.distanceIntervalMm?.[1] ?? null
        const certified = result.converged && result.validity.every(v => v.proven)
        const markdown_report = [
          `# Exact B-rep Solid Distance`,
          `- Relation: \`${result.reason}\` (certified=${certified}, overlap=${String(result.materialOverlap)})`,
          `- Interval: ${lower === null || upper === null ? 'unavailable' : `[${lower}, ${upper}] mm`}`,
        ].join('\n')
        return textResult({
          distance_lower_mm: lower,
          distance_upper_mm: upper,
          certified,
          material_overlap: result.materialOverlap,
          relation: result.reason,
          markdown_report,
        })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

export function registerBrepDiagnosticTools(server: McpServer): void {
  registerBrepInspectContactsTool(server)
  registerBrepSolidDistanceTool(server)
}
