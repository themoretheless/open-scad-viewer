import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import type { DirectDocument } from '../app/directModeling'
import { warmGeometryKernel } from '../kernel/geometry/kernel'
import { solveRushGraphSketch, type SketchConstraint } from '../language/rushGraphSketch'
import { ensureRawSourceLoader } from './ensureRawSourceLoader'
import { toolOutputSchema } from './createServer/officialToolInputSchema'
import { errorResult, textResult } from './createServer/persistedFailureStatus'
import { InvalidToolInputError } from './publicError'

const UNIT_VALUES = ['mm', 'cm', 'm', 'in'] as const

const DIRECT_SCHEMA_GUIDE = {
  schema: 'openscad-viewer/direct-document-schema-v1',
  document: {
    version: 1,
    units: UNIT_VALUES,
    collections: ['sketches', 'bodies', 'curves', 'surfaces', 'groups'],
    optionalMetadata: ['pivots', 'cplane', 'interchange', 'blenderProjectId'],
    minimalDocument: {
      version: 1,
      units: 'mm',
      sketches: [],
      bodies: [],
      curves: [],
      surfaces: [],
      groups: [],
    },
  },
  actionScript: {
    version: 1,
    limits: { minActions: 1, maxActions: 80, maxScriptBytes: 16 * 1024 * 1024 },
    actionKinds: [
      {
        kind: 'primitive',
        options: { primitive: 'box|cylinder|sphere|cone', id: 'string', name: 'string', size: '[x,y,z] (for box)' },
      },
      { kind: 'extrusion', options: { operation: 'new|union|difference', targetId: 'string', height: 'number' } },
      { kind: 'sceneEdit', options: { op: 'move|rotate|scale|fillet|chamfer|shell|delete' } },
      { kind: 'pattern', sourceId: 'string', pattern: 'LiveBodyPattern|null' },
      { kind: 'sketchCleanup', sourceId: 'string', createdId: 'string', tolerance: 'number' },
      { kind: 'outlineStroke', sourceId: 'string', createdId: 'string', width: 'number' },
      { kind: 'documentEdit', edits: 'DirectDocumentEdit[]' },
      { kind: 'checkpoint', document: 'serialized DirectDocument JSON' },
    ],
  },
}

const directBodySummarySchema = z.object({
  id: z.string(),
  name: z.string(),
  has_brep: z.boolean(),
  vertex_count: z.number().int().nonnegative(),
  triangle_count: z.number().int().nonnegative(),
  face_count: z.number().int().nonnegative().nullable(),
  edge_count: z.number().int().nonnegative().nullable(),
})

const directSketchSummarySchema = z.object({
  id: z.string(),
  name: z.string(),
  point_count: z.number().int().nonnegative(),
  closed: z.boolean(),
  has_retained_profile: z.boolean(),
})

function registerDirectSchemaResource(server: McpServer): void {
  server.registerResource(
    'direct-schema',
    'direct://schema',
    {
      title: 'DirectDocument and ActionScript schema guide',
      description: 'Machine-readable schema reference for openscad_direct_transaction and direct_create_document.',
      mimeType: 'application/json',
    },
    () => ({
      contents: [
        {
          uri: 'direct://schema',
          mimeType: 'application/json',
          text: JSON.stringify(DIRECT_SCHEMA_GUIDE, null, 2),
        },
      ],
    }),
  )
}

function registerDirectCreateTool(server: McpServer): void {
  server.registerTool(
    'direct_create_document',
    {
      title: 'Create empty DirectDocument',
      description: 'Create an empty version 1 DirectDocument JSON string for openscad_direct_transaction.',
      inputSchema: z.object({ units: z.enum(UNIT_VALUES).optional() }).strict(),
      outputSchema: toolOutputSchema(z.object({ units: z.enum(UNIT_VALUES), document: z.string() })),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
    },
    async input => {
      try {
        ensureRawSourceLoader()
        const { emptyDirectDocument, serializeDirectDocument } = await import('../app/directModeling')
        const doc: DirectDocument = { ...emptyDirectDocument(), ...(input.units ? { units: input.units } : {}) }
        return textResult({ units: doc.units ?? 'mm', document: serializeDirectDocument(doc) })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

function registerDirectInspectTool(server: McpServer): void {
  server.registerTool(
    'direct_inspect_document',
    {
      title: 'Inspect a DirectDocument JSON payload',
      description:
        'Validate a serialized DirectDocument and return structured body, B-rep, sketch, curve, surface, and group summaries.',
      inputSchema: z
        .object({
          document: z
            .string()
            .min(2)
            .max(16 * 1024 * 1024),
        })
        .strict(),
      outputSchema: toolOutputSchema(
        z.object({
          units: z.enum(UNIT_VALUES),
          body_count: z.number().int().nonnegative(),
          sketch_count: z.number().int().nonnegative(),
          curve_count: z.number().int().nonnegative(),
          surface_count: z.number().int().nonnegative(),
          group_count: z.number().int().nonnegative(),
          bodies: z.array(directBodySummarySchema),
          sketches: z.array(directSketchSummarySchema),
          markdown_report: z.string(),
        }),
      ),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
    },
    async input => {
      try {
        ensureRawSourceLoader()
        await warmGeometryKernel()
        const { parseDirectDocument } = await import('../app/directModeling')
        let parsed: DirectDocument
        try {
          parsed = parseDirectDocument(input.document)
        } catch (error) {
          throw new InvalidToolInputError(error instanceof Error ? error.message : 'Invalid DirectDocument JSON.')
        }
        const bodies = parsed.bodies.map(body => ({
          id: body.id,
          name: body.name,
          has_brep: Boolean(body.brep),
          vertex_count: Math.floor(body.mesh.positions.length / 3),
          triangle_count: Math.floor(body.mesh.indices.length / 3),
          face_count: body.brep ? body.brep.faces.length : null,
          edge_count: body.brep ? body.brep.edges.length : null,
        }))
        const sketches = parsed.sketches.map(sketch => ({
          id: sketch.id,
          name: sketch.name,
          point_count: sketch.points.length,
          closed: sketch.closed,
          has_retained_profile: Boolean(sketch.retainedProfile),
        }))
        const markdown_report = [
          `# Direct Document (${parsed.units ?? 'mm'})`,
          `- Bodies: ${bodies.length}, Sketches: ${sketches.length}, Curves: ${(parsed.curves ?? []).length}, Surfaces: ${(parsed.surfaces ?? []).length}, Groups: ${(parsed.groups ?? []).length}`,
          ...bodies.map(
            b =>
              `- Body \`${b.id}\` (${b.name}): ${b.triangle_count} triangles, B-rep=${b.has_brep ? `${b.face_count} faces/${b.edge_count} edges` : 'no'}`,
          ),
        ].join('\n')
        return textResult({
          units: parsed.units ?? 'mm',
          body_count: bodies.length,
          sketch_count: sketches.length,
          curve_count: (parsed.curves ?? []).length,
          surface_count: (parsed.surfaces ?? []).length,
          group_count: (parsed.groups ?? []).length,
          bodies,
          sketches,
          markdown_report,
        })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

function registerSketchSolveTool(server: McpServer): void {
  const sketchOutputSchema = toolOutputSchema(
    z.object({
      status: z.enum(['solved', 'underconstrained', 'not_converged', 'inconsistent']),
      degrees_of_freedom: z.number().int().nonnegative(),
      redundant_equations: z.number().int().nonnegative(),
      iterations: z.number().int().nonnegative(),
      tolerance_mm: z.number().positive(),
      maximum_residual_mm: z.number().nonnegative(),
      degenerate_constraints: z.array(z.string()),
      points: z.array(z.object({ id: z.string(), position: z.tuple([z.number(), z.number()]) })),
      circles: z
        .array(z.object({ id: z.string(), center: z.tuple([z.number(), z.number()]), radius: z.number().positive() }))
        .optional(),
      constraints: z.array(z.object({ id: z.string(), residual_mm: z.number(), satisfied: z.boolean() })),
      markdown_report: z.string(),
    }),
  )
  server.registerTool(
    'sketch_solve',
    {
      title: 'Solve a constrained 2D sketch',
      description:
        'Run the Rust 2D geometric constraint solver on named points (3..16), optional circles (0..8), and up to 48 constraints, returning solved coordinates, DOF, and per-constraint residuals.',
      inputSchema: z
        .object({
          points: z
            .array(
              z
                .object({
                  id: z.string().min(1).max(64),
                  position: z.tuple([z.number().finite(), z.number().finite()]),
                })
                .strict(),
            )
            .min(3)
            .max(16),
          circles: z
            .array(
              z
                .object({
                  id: z.string().min(1).max(64),
                  center: z.string().min(1).max(64),
                  radius: z.number().positive(),
                })
                .strict(),
            )
            .max(8)
            .optional(),
          constraints: z.array(z.record(z.string(), z.unknown())).max(48),
          tolerance: z.number().min(1e-8).max(0.1).optional(),
        })
        .strict(),
      outputSchema: sketchOutputSchema,
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
    },
    async input => {
      try {
        await warmGeometryKernel()
        let solved: ReturnType<typeof solveRushGraphSketch>
        try {
          solved = solveRushGraphSketch(
            input.points,
            input.constraints as SketchConstraint[],
            input.tolerance ?? 1e-6,
            input.circles ?? [],
          )
        } catch (error) {
          throw new InvalidToolInputError(error instanceof Error ? error.message : 'Sketch constraint solve failed.')
        }
        const markdown_report = [
          `# Sketch Solve: \`${solved.status}\``,
          `- DOF: ${solved.degrees_of_freedom}, Redundant: ${solved.redundant_equations}, Iterations: ${solved.iterations}`,
          `- Max residual: ${solved.maximum_residual_mm.toExponential(3)} mm (tolerance ${solved.tolerance_mm} mm)`,
        ].join('\n')
        return textResult({ ...solved, markdown_report })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

export function registerDirectModelingTools(server: McpServer): void {
  registerDirectSchemaResource(server)
  registerDirectCreateTool(server)
  registerDirectInspectTool(server)
  registerSketchSolveTool(server)
}
