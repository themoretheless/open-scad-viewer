import { MAX_NATIVE_GEOMETRY_CHARACTERS } from '../core/nativeGeometry'
import { warmGeometryKernel } from '../kernel/geometry/kernel'
import { compileRushGraphNurbs } from './rushGraphNurbsCompiler'
import { transformNurbsSurfacePatches } from '../kernel/nurbsSurface'
import type { ProgressiveSweepPreview } from '../kernel/nurbsConstructors'
/** Publish an own-kernel mesh directly to the viewer, without a SCAD/Manifold pass. */
import { buildOwnNurbsAsync, collectSdfJobs, type OwnNurbsBuildControl } from './rushGraphNurbsKernel'
import { prepareSdfGpuBatch, primeSdfGpuBatch } from '../kernel/geometry/sdf'
import { runSdfSweepBatch } from '../render/sdfGpu'
import type { GeometryEvaluationResult, GeometryQuality } from '../core/build'
import { geometryAssetId } from '../core/scene'
import { buildMeshBvh } from '../kernel/meshBvh'
import { extractSemanticEdges } from '../kernel/meshTopology'
import { metricsManifoldInKernel } from '../kernel/geometry/manifoldAnalysis'
import { expandFlatTrianglesInKernel, fillVertexNormalsInKernel } from '../kernel/geometry/meshAnalysis'
export async function buildTextNurbsScene(
  document: unknown,
  quality: GeometryQuality = 'full',
  control: OwnNurbsBuildControl = {},
): Promise<GeometryEvaluationResult> {
  const start = performance.now()
  if (typeof window !== 'undefined') await warmGeometryKernel()
  const scenePreview = control.onSweepPreview ? progressiveScenePreviewMapper(document) : null
  // GPU prefetch: eligible SDF grids are sampled by WebGPU before the
  // synchronous build — all grids in one session (one pipeline, one submit);
  // failures silently leave the CPU reference path.
  if (typeof navigator !== 'undefined' && navigator.gpu) {
    const jobs = collectSdfJobs(document)
    if (jobs.length) {
      try {
        const batch = prepareSdfGpuBatch(jobs)
        if (batch) primeSdfGpuBatch(jobs, batch.ids, await runSdfSweepBatch(batch.payload))
      } catch {
        /* CPU path at tessellation time */
      }
    }
  }
  const exportQ = control.exportQuality
  const displaySegments = quality === 'preview' ? 8 : exportQ === 'ultra' ? 32 : exportQ === 'high' ? 28 : 24
  const displaySubdiv = quality === 'preview' ? 1 : exportQ === 'ultra' || exportQ === 'high' ? 3 : 2
  const result = await buildOwnNurbsAsync(
    document,
    { action: 'build', display: { segments: displaySegments, subdivisionLevels: displaySubdiv } },
    {
      ...control,
      onSweepPreview: async (id, preview) => {
        const placed = scenePreview?.(id, preview)
        if (placed) await control.onSweepPreview?.(id, placed)
      },
    },
  )
  if (!('mesh' in result) || !result.mesh)
    throw new Error(
      'This geometry needs an explicit display conversion (SDF requires grid bounds; curves/profiles do not yet have a line renderer)',
    )
  const mesh = result.mesh,
    evaluated = performance.now()
  const nativeGeometry = 'nativeGeometry' in result ? result.nativeGeometry : undefined
  const entityId = `entity:native/${nativeGeometry?.nodeId ?? result.report.root}` as const
  const { mesh: output, surfaceArea } = ownNurbsDisplayMesh(mesh, entityId, nativeGeometry)
  let reportCharacters = 0
  const measurements = Object.entries(result.report.construction ?? {}).flatMap(([node, entry]) => {
    if (!('measurement' in entry)) return []
    const { operation, report } = entry.measurement as { operation: string; report: object }
    const {step,iges,...summary} = report as {step?:unknown;iges?:unknown}
    const assemblyFile = (operation === 'planet_occurrences_step' || operation === 'planetary_gear_set_step' || ["make_step_assembly", "step_part_tolerances", "set_scaled_step_placement", "step_occurrence_bounds", "step_proximity_pairs", "step_common_material", "step_interferences", "step_material_volume", "step_material_centroid", "step_material_inertia", "step_material_area", "step_principal_inertia", "step_mass_properties","move_step_group","copy_step_group","replace_step_variant","open_step_file","read_shared_step", "replace_step_part", "replace_step_instance", "place_step_instance", "remove_step_instance", "add_step_instance", "insert_step_solid", "append_step_assembly", "deduplicate_step_parts", "extract_step_instances", "edited_gear_step", "compact_body_step", "compact_product_step", "name_step_parts", "read_step_names", "step_parts_list", "select_step_parts", "name_step_variant", "append_named_parts", "consolidate_named_parts", "compare_parts_lists", "group_step_parts", "style_step_occurrences", "read_occurrence_styles"].includes(operation)) && typeof step === 'string' ? step : undefined
    const igesFile = operation === 'export_iges_body' && typeof iges === 'string' ? iges : undefined
    const serialized = JSON.stringify(assemblyFile || igesFile ? summary : report)
    reportCharacters += (assemblyFile?.length ?? 0) + (igesFile?.length ?? 0)
    reportCharacters += serialized.length
    if (reportCharacters > MAX_NATIVE_GEOMETRY_CHARACTERS) {
      throw new Error('Measurement reports exceed the editor output limit. Reduce the work budget or parameter range.')
    }
    return [{ node, operation, report: serialized, ...(assemblyFile && {step:assemblyFile}), ...(igesFile && {iges:igesFile}) }]
  })
  return {
    meshes: mesh.indices.length ? [output] : [],
    ...(measurements.length ? { measurements } : {}),
    warnings: mesh.report.closed || !mesh.indices.length ? [] : ['Open surface: enclosed volume is undefined.'],
    volume: mesh.report.closed ? Math.abs(mesh.report.signedVolumeMm3) : 0,
    surfaceArea,
    quality,
    reduced: false,
    timings: {
      parseMs: 0,
      bindMs: 0,
      initializeMs: 0,
      evaluateMs: evaluated - start,
      analyzeMs: performance.now() - evaluated,
    },
  }
}

function indexedPatchDisplay(positions: Float64Array, indices: Uint32Array) {
  const vertices = new Float32Array(positions.length * 2)
  for (let source = 0, target = 0; source < positions.length; source += 3, target += 6) {
    vertices[target] = positions[source]!
    vertices[target + 1] = positions[source + 1]!
    vertices[target + 2] = positions[source + 2]!
  }
  fillVertexNormalsInKernel(vertices, indices)
  return {vertices, indices, surfaceArea:metricsManifoldInKernel(positions, indices).surfaceArea}
}

/** Display-only adapter shared by final scenes and intermediate sweep previews. */
export function ownNurbsDisplayMesh(
  mesh: ReturnType<typeof import('../kernel/geometry/tessellation').tessellateNurbsSurface>,
  entityId: import('../core/mesh').SceneEntityId,
  nativeGeometry?: import('../core/nativeGeometry').NativeGeometryArtifact,
) {
  const posArray = mesh.positions instanceof Float64Array ? mesh.positions : Float64Array.from(mesh.positions)
  const idxArray = mesh.indices instanceof Uint32Array ? mesh.indices : Uint32Array.from(mesh.indices)
  const { vertices, indices, surfaceArea } = mesh.sharedPatchVertices
    ? indexedPatchDisplay(posArray, idxArray)
    : expandFlatTrianglesInKernel(posArray, idxArray)
  const edges = extractSemanticEdges(vertices, indices)
  const output = {
    vertices,
    indices,
    geometryAssetId: geometryAssetId(vertices, indices),
    entityId,
    ...(nativeGeometry ? { nativeGeometry } : {}),
    bvh: buildMeshBvh(vertices, indices),
    edgeIndices: edges.indices,
    topology: edges.diagnostics,
    color: [0.85, 0.67, 0.3, 1] as [number, number, number, number],
    transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
    faceIds: mesh.faceIds ? new Uint32Array(mesh.faceIds) : new Uint32Array(indices.length / 3),
    faceIdsAuthoritative: !!mesh.faceIds || mesh.report.construction === 'sampled_surface',
    provenance: [{ triangleStart: 0, triangleEnd: indices.length / 3, source: null, backside: false }],
  }
  return { mesh: output, surfaceArea }
}

/** Preserves exact downstream affine placement before the display conversion.
 * Operations that change geometry require their own preview evaluator.
 */
export function progressiveScenePreviewMapper(document: unknown) {
  const compiled = compileRushGraphNurbs(document)
  const nodes = new Map(compiled.resolved_document.nodes.map(node => [node.id, node]))
  return (id: string, preview: ProgressiveSweepPreview): ProgressiveSweepPreview | null => {
    let key = compiled.document.root
    const matrices: number[][][] = []
    const seen = new Set<string>()
    while (key !== id) {
      if (seen.has(key)) return null
      seen.add(key)
      const node = nodes.get(key)
      if (!node) return null
      if (node.op === 'transform') {
        matrices.push(node.matrix)
        key = node.input
      } else if (node.op === 'nurbs_patches_tessellate' || node.op === 'brep_tessellate') key = node.input
      else return null
    }
    let patches = preview.patches
    for (const matrix of matrices.reverse()) patches = transformNurbsSurfacePatches(patches, matrix)
    return { ...preview, patches }
  }
}
