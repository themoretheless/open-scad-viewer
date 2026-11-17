import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import {
  MESH_EXPORT_FORMATS,
  exportMeshFormatCompressed,
  meshExportBase64,
  type MeshExportFormat,
} from '../exchange/meshExportFormats'
import { warmGeometryKernel } from '../kernel/geometry/kernel'
import { inspectNurbsMesh, type NurbsMesh } from '../kernel/geometry/tessellation'
import { brepSourceInputSchema, resolveBrepFromInput } from './brepInputHelper'
import { ensureRawSourceLoader } from './ensureRawSourceLoader'
import { toolOutputSchema } from './createServer/officialToolInputSchema'
import { errorResult, textResult } from './createServer/persistedFailureStatus'
import { InvalidToolInputError } from './publicError'

const annotations = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false }

const certificateSchema = z.object({
  capability: z.string(),
  complete: z.boolean(),
  notes: z.array(z.string()),
})

const identitySchema = z.object({
  preserved: z.boolean(),
  source: z.string(),
  preserved_count: z.number().int().nonnegative(),
  created_count: z.number().int().nonnegative(),
  lost_count: z.number().int().nonnegative(),
})

const topologySummarySchema = z.object({
  valid: z.boolean(),
  body_count: z.number().int().nonnegative(),
  shell_count: z.number().int().nonnegative(),
  face_count: z.number().int().nonnegative(),
  edge_count: z.number().int().nonnegative(),
  vertex_count: z.number().int().nonnegative(),
})

const meshPreviewSummarySchema = z.object({
  vertex_count: z.number().int().nonnegative(),
  triangle_count: z.number().int().nonnegative(),
})

const importMeshOptionsSchema = {
  include_mesh_preview: z.boolean().optional(),
  tessellation_segments: z.number().int().min(2).max(32).optional(),
  export_mesh_format: z.enum(MESH_EXPORT_FORMATS).optional(),
}

type EmbeddedResourceBlock = { type: 'resource'; resource: { uri: string; mimeType: string; blob: string } }

function toWireIdentity(identity: {
  preserved: boolean
  source: string
  preservedCount: number
  createdCount: number
  lostCount: number
}) {
  return {
    preserved: identity.preserved,
    source: identity.source,
    preserved_count: identity.preservedCount,
    created_count: identity.createdCount,
    lost_count: identity.lostCount,
  }
}

function withOptionalResource(structured: Record<string, unknown>, resourceBlock: EmbeddedResourceBlock | null) {
  return resourceBlock
    ? {
        structuredContent: structured,
        content: [{ type: 'text' as const, text: JSON.stringify(structured) }, resourceBlock],
      }
    : textResult(structured)
}

async function buildMeshPreviewAndResource(
  positions: Float64Array,
  indices: Uint32Array,
  exportFormat: MeshExportFormat | undefined,
  uriPrefix: string,
) {
  const preview = {
    vertex_count: Math.floor(positions.length / 3),
    triangle_count: Math.floor(indices.length / 3),
  }
  if (exportFormat === undefined) {
    return { preview, exported_byte_length: null as number | null, resourceBlock: null as EmbeddedResourceBlock | null }
  }
  try {
    const nurbsMesh: NurbsMesh = {
      positions,
      indices,
      report: inspectNurbsMesh(positions, indices),
    }
    const artifact = await exportMeshFormatCompressed(nurbsMesh, exportFormat)
    return {
      preview,
      exported_byte_length: artifact.data.byteLength,
      resourceBlock: {
        type: 'resource' as const,
        resource: {
          uri: `${uriPrefix}/mesh.${artifact.extension}`,
          mimeType: artifact.mimeType,
          blob: meshExportBase64(artifact.data),
        },
      },
    }
  } catch (error) {
    throw new InvalidToolInputError(error instanceof Error ? error.message : 'Failed to export imported mesh.', {
      code: 'invalid_geometry',
    })
  }
}

async function importDirectBrepStepResult(
  stepText: string,
  options: { wantMesh: boolean; segments: number; exportFormat?: MeshExportFormat },
) {
  ensureRawSourceLoader()
  const [{ importDirectStepScene }, { inspectNurbsBrep, tessellateNurbsBrep }] = await Promise.all([
    import('../kernel/cadNurbsStep'),
    import('../kernel/geometry/brep'),
  ])
  const scene = importDirectStepScene(stepText)
  const brepReport = inspectNurbsBrep(scene.model)
  const rawIdentity = (scene.report as { identity?: Parameters<typeof toWireIdentity>[0] }).identity ?? scene.report
  const meshOutcome = options.wantMesh
    ? await (() => {
        const tess = tessellateNurbsBrep(scene.model, options.segments)
        return buildMeshPreviewAndResource(tess.positions, tess.indices, options.exportFormat, 'cad://step-import')
      })()
    : { preview: null, exported_byte_length: null, resourceBlock: null }
  return withOptionalResource(
    {
      mode: 'direct-brep-step' as const,
      certificate: scene.certificate,
      identity: toWireIdentity(rawIdentity),
      instance_count: scene.report.instanceCount,
      reachable_count: scene.report.reachableCount,
      ignored_entities: scene.report.ignoredEntities,
      metadata_loss: scene.report.metadataLoss,
      product_count: scene.sceneTree.products.length,
      products: scene.sceneTree.products.slice(0, 64).map(p => ({
        id: p.id,
        name: p.name,
        material: p.material,
        body_count: p.bodies.length,
      })),
      topology: {
        valid: brepReport.topologyValid,
        body_count: scene.model.bodies.length,
        shell_count: scene.model.shells.length,
        face_count: scene.model.faces.length,
        edge_count: scene.model.edges.length,
        vertex_count: scene.model.vertices.length,
      },
      mesh_preview: meshOutcome.preview,
      exported_byte_length: meshOutcome.exported_byte_length,
    },
    meshOutcome.resourceBlock,
  )
}

async function importTessellatedStepResult(
  stepText: string,
  options: { wantMesh: boolean; exportFormat?: MeshExportFormat },
) {
  ensureRawSourceLoader()
  const { importTessellatedStepV10 } = await import('../kernel/cadStep')
  const tess = importTessellatedStepV10(stepText)
  const first = tess.bodies[0]
  const totalVertices = tess.bodies.reduce((sum, body) => sum + Math.floor(body.mesh.positions.length / 3), 0)
  const totalTriangles = tess.bodies.reduce((sum, body) => sum + Math.floor(body.mesh.indices.length / 3), 0)
  const meshOutcome =
    options.exportFormat !== undefined && first
      ? await buildMeshPreviewAndResource(
          first.mesh.positions,
          first.mesh.indices,
          options.exportFormat,
          'cad://step-import',
        )
      : {
          preview: options.wantMesh ? { vertex_count: totalVertices, triangle_count: totalTriangles } : null,
          exported_byte_length: null,
          resourceBlock: null,
        }
  return withOptionalResource(
    {
      mode: 'tessellated-step' as const,
      certificate: { capability: 'step-interchange/10-tessellated', complete: true, notes: tess.metadataLoss },
      identity: null,
      instance_count: tess.bodies.length,
      reachable_count: tess.bodies.length,
      ignored_entities: [] as string[],
      metadata_loss: tess.metadataLoss,
      product_count: tess.bodies.length,
      products: tess.bodies.slice(0, 64).map(body => ({
        id: body.id,
        name: body.name,
        material: null,
        body_count: 1,
      })),
      topology: {
        valid: tess.bodies.length > 0,
        body_count: tess.bodies.length,
        shell_count: tess.bodies.length,
        face_count: 0,
        edge_count: 0,
        vertex_count: totalVertices,
      },
      mesh_preview: meshOutcome.preview,
      exported_byte_length: meshOutcome.exported_byte_length,
    },
    meshOutcome.resourceBlock,
  )
}

function registerCadImportStepTool(server: McpServer): void {
  server.registerTool(
    'cad_import_step',
    {
      title: 'Import STEP AP214/AP242 file',
      description:
        'Import an ISO 10303-21 STEP file (exact NURBS B-rep scene preferred, with AP242 tessellated fallback). Returns topology counts, product metadata, optional tessellation summary, and optional exported mesh resource.',
      inputSchema: z
        .object({
          step_text: z
            .string()
            .min(10)
            .max(8 * 1024 * 1024),
          ...importMeshOptionsSchema,
        })
        .strict(),
      outputSchema: toolOutputSchema(
        z.object({
          mode: z.enum(['direct-brep-step', 'tessellated-step']),
          certificate: certificateSchema,
          identity: identitySchema.nullable(),
          instance_count: z.number().int().nonnegative(),
          reachable_count: z.number().int().nonnegative(),
          ignored_entities: z.array(z.string()),
          metadata_loss: z.array(z.string()),
          product_count: z.number().int().nonnegative(),
          products: z.array(
            z.object({
              id: z.string(),
              name: z.string(),
              material: z.string().nullable(),
              body_count: z.number().int().nonnegative(),
            }),
          ),
          topology: topologySummarySchema,
          mesh_preview: meshPreviewSummarySchema.nullable(),
          exported_byte_length: z.number().int().nonnegative().nullable(),
        }),
      ),
      annotations,
    },
    async input => {
      try {
        await warmGeometryKernel()
        const wantMesh = input.include_mesh_preview === true || input.export_mesh_format !== undefined
        const segments = input.tessellation_segments ?? 4
        const exportFormat = input.export_mesh_format
        try {
          return await importDirectBrepStepResult(input.step_text, {
            wantMesh,
            segments,
            ...(exportFormat !== undefined ? { exportFormat } : {}),
          })
        } catch (primaryError) {
          if (!/\b(TESSELLATED_SHAPE_REPRESENTATION|TRIANGULATED_FACE_SET)\b/i.test(input.step_text)) {
            throw primaryError
          }
          return await importTessellatedStepResult(input.step_text, {
            wantMesh,
            ...(exportFormat !== undefined ? { exportFormat } : {}),
          })
        }
      } catch (error) {
        return errorResult(
          error instanceof InvalidToolInputError
            ? error
            : new InvalidToolInputError(error instanceof Error ? error.message : 'STEP import failed.', {
                code: 'invalid_geometry',
              }),
        )
      }
    },
  )
}

function registerCadImportIgesTool(server: McpServer): void {
  server.registerTool(
    'cad_import_iges',
    {
      title: 'Import IGES B-rep file',
      description:
        'Import an 80-column IGES B-rep exchange file (entities 126/128/142/144/186). Returns header metadata, topology identity report, optional tessellation summary, and optional exported mesh resource.',
      inputSchema: z
        .object({
          iges_text: z
            .string()
            .min(10)
            .max(8 * 1024 * 1024),
          ...importMeshOptionsSchema,
        })
        .strict(),
      outputSchema: toolOutputSchema(
        z.object({
          header: z.object({
            product_id: z.string().nullable(),
            file_name: z.string().nullable(),
            units_name: z.string().nullable(),
            directory_entry_count: z.number().int().nonnegative(),
            has_nurbs_surface_128: z.boolean(),
            has_trimmed_surface_144: z.boolean(),
          }),
          certificate: certificateSchema,
          identity: identitySchema,
          entity_count: z.number().int().nonnegative(),
          topology_count: z.number().int().nonnegative(),
          ignored_metadata: z.array(z.string()),
          topology: topologySummarySchema,
          mesh_preview: meshPreviewSummarySchema.nullable(),
          exported_byte_length: z.number().int().nonnegative().nullable(),
        }),
      ),
      annotations,
    },
    async input => {
      try {
        ensureRawSourceLoader()
        await warmGeometryKernel()
        const [{ importDirectIgesV2, inspectIgesHeaderMetadata }, { inspectNurbsBrep, tessellateNurbsBrep }] =
          await Promise.all([import('../kernel/cadIges'), import('../kernel/geometry/brep')])
        const header = inspectIgesHeaderMetadata(input.iges_text)
        const imported = importDirectIgesV2(input.iges_text)
        const brepReport = inspectNurbsBrep(imported.model)
        const wantMesh = input.include_mesh_preview === true || input.export_mesh_format !== undefined
        const meshOutcome = wantMesh
          ? await (() => {
              const tess = tessellateNurbsBrep(imported.model, input.tessellation_segments ?? 4)
              return buildMeshPreviewAndResource(
                tess.positions,
                tess.indices,
                input.export_mesh_format,
                'cad://iges-import',
              )
            })()
          : { preview: null, exported_byte_length: null, resourceBlock: null }
        return withOptionalResource(
          {
            header: {
              product_id: header.productId,
              file_name: header.fileName,
              units_name: header.unitsName,
              directory_entry_count: header.directoryEntryCount,
              has_nurbs_surface_128: header.hasNurbsSurface128,
              has_trimmed_surface_144: header.hasTrimmedSurface144,
            },
            certificate: imported.certificate,
            identity: toWireIdentity(imported.identity),
            entity_count: imported.entityCount,
            topology_count: imported.topologyCount,
            ignored_metadata: imported.ignoredMetadata,
            topology: {
              valid: brepReport.topologyValid,
              body_count: imported.model.bodies.length,
              shell_count: imported.model.shells.length,
              face_count: imported.model.faces.length,
              edge_count: imported.model.edges.length,
              vertex_count: imported.model.vertices.length,
            },
            mesh_preview: meshOutcome.preview,
            exported_byte_length: meshOutcome.exported_byte_length,
          },
          meshOutcome.resourceBlock,
        )
      } catch (error) {
        return errorResult(
          error instanceof InvalidToolInputError
            ? error
            : new InvalidToolInputError(error instanceof Error ? error.message : 'IGES import failed.', {
                code: 'invalid_geometry',
              }),
        )
      }
    },
  )
}

function registerCadExportIgesTool(server: McpServer): void {
  server.registerTool(
    'cad_export_iges',
    {
      title: 'Export exact B-rep solid to IGES',
      description:
        'Export an exact NURBS B-rep solid (`rush/nurbs-1` document/source, STEP, or IGES) as an 80-column IGES exchange file.',
      inputSchema: z.object({ model: brepSourceInputSchema }).strict(),
      outputSchema: toolOutputSchema(
        z.object({
          certificate: certificateSchema,
          identity: identitySchema,
          entity_count: z.number().int().nonnegative(),
          topology_count: z.number().int().nonnegative(),
          ignored_metadata: z.array(z.string()),
          byte_length: z.number().int().nonnegative(),
          iges_text: z.string(),
        }),
      ),
      annotations,
    },
    async input => {
      try {
        ensureRawSourceLoader()
        const [{ exportDirectIgesV2 }, brep] = await Promise.all([
          import('../kernel/cadIges'),
          resolveBrepFromInput(input.model),
        ])
        const exported = exportDirectIgesV2(brep)
        return textResult({
          certificate: exported.certificate,
          identity: toWireIdentity(exported.identity),
          entity_count: exported.entityCount,
          topology_count: exported.topologyCount,
          ignored_metadata: exported.ignoredMetadata,
          byte_length: Buffer.byteLength(exported.text, 'utf8'),
          iges_text: exported.text,
        })
      } catch (error) {
        return errorResult(
          error instanceof InvalidToolInputError
            ? error
            : new InvalidToolInputError(error instanceof Error ? error.message : 'IGES export failed.', {
                code: 'invalid_geometry',
              }),
        )
      }
    },
  )
}

export function registerCadExchangeTools(server: McpServer): void {
  registerCadImportStepTool(server)
  registerCadImportIgesTool(server)
  registerCadExportIgesTool(server)
}
