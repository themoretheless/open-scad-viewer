import type { GeometryEvaluationResult, GeometryQuality } from '../core/build'
import {
  OPENSCAD_2021_01_CONTRACT,
  OPENSCAD_2021_01_INDEPENDENT_ENGINE,
} from '../core/openScad2021Contract'
import {
  OpenScadProject,
  type OpenScadProjectFileInput,
} from '../services/openScadProject'
import { parseOpenSCAD, parseOpenScadProject } from '../services/openscadParser'

export type IndependentOpenScadProjectFile = Readonly<{
  path: string
  text: string
  data_base64?: never
} | {
  path: string
  text?: never
  data_base64: string
}>

export interface IndependentOpenScadRunInput {
  readonly source: string
  readonly files: readonly IndependentOpenScadProjectFile[]
  readonly quality: GeometryQuality
  readonly time: number
  readonly signal?: AbortSignal
}

export interface IndependentOpenScadRun {
  readonly result: GeometryEvaluationResult
  readonly durationMs: number
}

export const INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION = Object.freeze({
  id: OPENSCAD_2021_01_INDEPENDENT_ENGINE.engineId,
  stage: 'development' as const,
  language_contract: OPENSCAD_2021_01_CONTRACT.id,
  upstream_runtime_used: false as const,
  production_authoritative: false as const,
  complete_language_claim: false as const,
  stable_function_inventory: OPENSCAD_2021_01_CONTRACT.builtins.functions.length,
  stable_module_inventory: OPENSCAD_2021_01_CONTRACT.builtins.modules.length,
})

function throwIfAborted(signal: AbortSignal | undefined): void {
  if (signal?.aborted) throw new DOMException('OpenSCAD evaluation was cancelled', 'AbortError')
}

function projectFile(file: IndependentOpenScadProjectFile): OpenScadProjectFileInput {
  return file.text !== undefined
    ? { kind: 'source', path: file.path, source: file.text }
    : {
        kind: 'blob',
        path: file.path,
        data: new Uint8Array(Buffer.from(file.data_base64, 'base64')),
      }
}

/** Execute only the repository-owned stable frontend and geometry evaluator. */
export async function executeIndependentOpenScad(
  input: IndependentOpenScadRunInput,
): Promise<IndependentOpenScadRun> {
  throwIfAborted(input.signal)
  const startedAt = performance.now()
  const parseOptions = {
    quality: input.quality,
    animationTime: input.time,
    shouldAbort: () => input.signal?.aborted ?? false,
  } as const
  const result = input.files.length === 0
    ? await parseOpenSCAD(input.source, {
        ...parseOptions,
        languageProfile: 'openscad/stable-2021.01',
      })
    : await parseOpenScadProject(new OpenScadProject({
        entrypoint: 'main.scad',
        files: [
          { kind: 'source', path: 'main.scad', source: input.source },
          ...input.files.map(projectFile),
        ],
      }), parseOptions)
  throwIfAborted(input.signal)
  return {
    result,
    durationMs: Math.max(0, performance.now() - startedAt),
  }
}

export function independentOpenScadMetrics(result: GeometryEvaluationResult) {
  return {
    mesh_count: result.meshes.length,
    vertex_count: result.meshes.reduce(
      (total, mesh) => total + Math.floor(mesh.vertices.length / 6),
      0,
    ),
    triangle_count: result.meshes.reduce(
      (total, mesh) => total + Math.floor(mesh.indices.length / 3),
      0,
    ),
    volume: result.volume,
    surface_area: result.surfaceArea,
  }
}
