import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import { MESH_EXPORT_FORMATS, exportMeshFormatCompressed, meshExportBase64 } from '../exchange/meshExportFormats'
import { warmGeometryKernel } from '../kernel/geometry/kernel'
import {
  checkManifoldInKernel,
  diagnoseManifoldInKernel,
  metricsManifoldInKernel,
  repairManifoldInKernel,
} from '../kernel/geometry/manifoldAnalysis'
import { inspectNurbsMesh } from '../kernel/geometry/tessellation'
import type { McpGeometryService } from './geometryService'
import { meshSourceOrUploadInputSchema, resolveMeshFromSourceOrUpload } from './meshInputHelper'
import { toolOutputSchema } from './createServer/officialToolInputSchema'
import { errorResult, textResult } from './createServer/persistedFailureStatus'
import { InvalidToolInputError } from './publicError'

const annotations = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false }

function registerMeshInspectTool(server: McpServer, geometry: McpGeometryService): void {
  server.registerTool(
    'mesh_inspect',
    {
      title: 'Inspect manifoldness, watertightness, volume, wall thickness, and self-intersections',
      description:
        'Run Rust manifold-core topology, metric, wall-thickness, clearance, and self-intersection diagnostics on an OpenSCAD source or uploaded mesh file (STL/OBJ/PLY/OFF/AMF/3MF).',
      inputSchema: z
        .object({
          target: meshSourceOrUploadInputSchema,
          sliver_area: z.number().nonnegative().max(100).optional(),
          min_thickness_mm: z.number().positive().max(100).optional(),
          clearance_threshold_mm: z.number().positive().max(100).optional(),
          density_g_cm3: z.number().positive().max(30).optional(),
        })
        .strict(),
      outputSchema: toolOutputSchema(
        z.object({
          source_label: z.string(),
          vertex_count: z.number().int().nonnegative(),
          triangle_count: z.number().int().nonnegative(),
          edge_count: z.number().int().nonnegative(),
          component_count: z.number().int().nonnegative(),
          manifold_component_count: z.number().int().nonnegative(),
          non_orientable_components: z.number().int().nonnegative(),
          is_manifold: z.boolean(),
          is_manifold_with_boundary: z.boolean(),
          watertight: z.boolean(),
          euler_characteristic: z.number().int(),
          signed_volume_mm3: z.number(),
          surface_area_mm2: z.number(),
          defects: z.object({
            boundary_edges: z.number().int().nonnegative(),
            non_manifold_edges: z.number().int().nonnegative(),
            orientation_conflict_edges: z.number().int().nonnegative(),
            degenerate_triangles: z.number().int().nonnegative(),
            non_manifold_vertices: z.number().int().nonnegative(),
            isolated_vertices: z.number().int().nonnegative(),
            zero_area_triangles: z.number().int().nonnegative(),
            sliver_triangles: z.number().int().nonnegative(),
          }),
          diagnostics: z.object({
            self_intersection_pairs: z.number().int().nonnegative(),
            self_intersection_truncated: z.boolean(),
            min_thickness_mm: z.number().nullable(),
            max_thickness_mm: z.number().nullable(),
            thin_triangles: z.number().int().nonnegative(),
            min_gap_mm: z.number().nullable(),
            clearance_pairs: z.number().int().nonnegative(),
            mass: z
              .object({
                volume_mm3: z.number(),
                mass_g: z.number(),
                center_of_mass_mm: z.tuple([z.number(), z.number(), z.number()]),
                inertia_tensor: z.array(z.number()),
              })
              .nullable(),
          }),
        }),
      ),
      annotations,
    },
    async (input, context) => {
      try {
        await warmGeometryKernel()
        const { mesh, sourceLabel } = await resolveMeshFromSourceOrUpload(input.target, geometry, context.mcpReq.signal)
        const check = checkManifoldInKernel(mesh.positions, mesh.indices, input.sliver_area ?? 0)
        const metrics = metricsManifoldInKernel(mesh.positions, mesh.indices)
        const diag = diagnoseManifoldInKernel(mesh.positions, mesh.indices, {
          ...(input.density_g_cm3 !== undefined ? { density: input.density_g_cm3 } : {}),
          ...(input.min_thickness_mm !== undefined ? { minThickness: input.min_thickness_mm } : {}),
          ...(input.clearance_threshold_mm !== undefined ? { clearanceThreshold: input.clearance_threshold_mm } : {}),
        })
        return textResult({
          source_label: sourceLabel,
          vertex_count: check.vertexCount,
          triangle_count: check.triangleCount,
          edge_count: metrics.edgeCount,
          component_count: check.componentCount,
          manifold_component_count: check.manifoldComponentCount,
          non_orientable_components: check.nonOrientableComponents,
          is_manifold: check.isManifold,
          is_manifold_with_boundary: check.isManifoldWithBoundary,
          watertight: metrics.watertight,
          euler_characteristic: metrics.eulerCharacteristic,
          signed_volume_mm3: metrics.signedVolume,
          surface_area_mm2: metrics.surfaceArea,
          defects: {
            boundary_edges: check.boundaryEdges.length / 2,
            non_manifold_edges: check.nonManifoldEdges.length / 2,
            orientation_conflict_edges: check.orientationEdges.length / 2,
            degenerate_triangles: check.degenerateTriangles.length,
            non_manifold_vertices: check.nonManifoldVertices.length,
            isolated_vertices: check.isolatedVertices.length,
            zero_area_triangles: check.zeroAreaTriangles.length,
            sliver_triangles: check.sliverTriangles.length,
          },
          diagnostics: {
            self_intersection_pairs: diag.selfIntersectionPairs.length / 2,
            self_intersection_truncated: diag.selfIntersectionTruncated,
            min_thickness_mm: diag.minThickness,
            max_thickness_mm: diag.maxThickness,
            thin_triangles: diag.thinTriangles.length,
            min_gap_mm: diag.minGap,
            clearance_pairs: diag.clearancePairs.length,
            mass: diag.mass
              ? {
                  volume_mm3: diag.mass.volume,
                  mass_g: diag.mass.mass,
                  center_of_mass_mm: diag.mass.centerOfMass,
                  inertia_tensor: diag.mass.inertia,
                }
              : null,
          },
        })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

function registerMeshRepairTool(server: McpServer, geometry: McpGeometryService): void {
  server.registerTool(
    'mesh_repair',
    {
      title: 'Repair mesh manifold defects and optionally re-export',
      description:
        'Repair welded vertices, degenerate triangles, flipped winding, non-manifold fans, and simple holes via Rust manifold-core. Optionally exports the repaired mesh as an embedded base64 resource.',
      inputSchema: z
        .object({
          target: meshSourceOrUploadInputSchema,
          epsilon_mm: z.number().nonnegative().max(10).optional(),
          mode: z.enum(['conservative', 'full']).optional(),
          remove_isolated_vertices: z.boolean().optional(),
          export_format: z.enum(MESH_EXPORT_FORMATS).optional(),
        })
        .strict(),
      outputSchema: toolOutputSchema(
        z.object({
          mode: z.enum(['conservative', 'full']),
          is_manifold: z.boolean(),
          is_manifold_with_boundary: z.boolean(),
          watertight: z.boolean(),
          vertex_count: z.number().int().nonnegative(),
          triangle_count: z.number().int().nonnegative(),
          signed_volume_mm3: z.number(),
          surface_area_mm2: z.number(),
          welded_vertices: z.number().int().nonnegative(),
          removed_degenerate_triangles: z.number().int().nonnegative(),
          flipped_triangles: z.number().int().nonnegative(),
          split_vertices: z.number().int().nonnegative(),
          filled_holes: z.number().int().nonnegative(),
          filled_triangles: z.number().int().nonnegative(),
          removed_isolated_vertices: z.number().int().nonnegative(),
          exported_byte_length: z.number().int().nonnegative().nullable(),
        }),
      ),
      annotations,
    },
    async (input, context) => {
      try {
        await warmGeometryKernel()
        const mode = input.mode ?? 'full'
        const { mesh } = await resolveMeshFromSourceOrUpload(input.target, geometry, context.mcpReq.signal)
        const repaired = repairManifoldInKernel(
          mesh.positions,
          mesh.indices,
          input.epsilon_mm ?? 1e-5,
          mode === 'full',
          input.remove_isolated_vertices ?? true,
        )
        const metrics = metricsManifoldInKernel(repaired.positions, repaired.indices)
        let exportedByteLength: number | null = null
        let resourceBlock: {
          type: 'resource'
          resource: { uri: string; mimeType: string; blob: string }
        } | null = null
        if (input.export_format !== undefined && repaired.indices.length > 0) {
          try {
            const artifact = await exportMeshFormatCompressed(
              {
                positions: repaired.positions,
                indices: repaired.indices,
                report: inspectNurbsMesh(repaired.positions, repaired.indices),
              },
              input.export_format,
            )
            exportedByteLength = artifact.data.byteLength
            resourceBlock = {
              type: 'resource',
              resource: {
                uri: `mesh://repaired/repaired.${artifact.extension}`,
                mimeType: artifact.mimeType,
                blob: meshExportBase64(artifact.data),
              },
            }
          } catch (exportError) {
            throw new InvalidToolInputError(
              exportError instanceof Error ? exportError.message : 'Could not export repaired mesh.',
              { code: 'invalid_geometry' },
            )
          }
        }
        const structured = {
          mode,
          is_manifold: repaired.isManifold,
          is_manifold_with_boundary: repaired.isManifoldWithBoundary,
          watertight: metrics.watertight,
          vertex_count: metrics.vertexCount,
          triangle_count: metrics.triangleCount,
          signed_volume_mm3: metrics.signedVolume,
          surface_area_mm2: metrics.surfaceArea,
          welded_vertices: repaired.weldedVertices,
          removed_degenerate_triangles: repaired.removedDegenerateTriangles,
          flipped_triangles: repaired.flippedTriangles,
          split_vertices: repaired.splitVertices,
          filled_holes: repaired.filledHoles,
          filled_triangles: repaired.filledTriangles,
          removed_isolated_vertices: repaired.removedIsolatedVertices,
          exported_byte_length: exportedByteLength,
        }
        return resourceBlock
          ? {
              structuredContent: structured,
              content: [{ type: 'text' as const, text: JSON.stringify(structured) }, resourceBlock],
            }
          : textResult(structured)
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

export function registerMeshManifoldTools(server: McpServer, geometry: McpGeometryService): void {
  registerMeshInspectTool(server, geometry)
  registerMeshRepairTool(server, geometry)
}
