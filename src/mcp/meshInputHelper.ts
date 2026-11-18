import { z } from 'zod/v4'
import { buildBinaryStl } from '../exchange/meshExport'
import { MESH_IMPORT_FORMATS, MESH_IMPORT_MAX_BYTES } from '../exchange/meshFormats'
import { importMeshFile, MeshImportError, type ImportedMesh } from '../exchange/meshImport'
import { toPolygonIndices, toPolygonPositions, type PolygonMesh } from '../kernel/geometry/polygon'
import type { McpGeometryService } from './geometryService'
import { InvalidToolInputError } from './publicError'
import { wellFormedText } from './createServer/schemas/commonSchemas'

export const MAX_MESH_UPLOAD_BASE64_LENGTH = Math.ceil(MESH_IMPORT_MAX_BYTES / 3) * 4

export const meshUploadFields = {
  file_name: z.string().min(1).max(255).describe('Mesh file name (extension selects format unless overridden).'),
  data_base64: z.string().min(1).max(MAX_MESH_UPLOAD_BASE64_LENGTH).describe('Standard base64 encoded file bytes.'),
  source_format: z.enum(MESH_IMPORT_FORMATS).optional().describe('Override format inferred from file_name.'),
}

export const meshUploadSchema = z.object(meshUploadFields).strict()

export const meshSourceOrUploadInputSchema = z.union([
  z
    .object({
      source: wellFormedText(262_144, 'Source').refine(s => s.trim().length > 0, 'Source must not be empty'),
      file_name: z.never().optional(),
      data_base64: z.never().optional(),
      source_format: z.never().optional(),
    })
    .strict(),
  z
    .object({
      source: z.never().optional(),
      ...meshUploadFields,
    })
    .strict(),
])

export type MeshSourceOrUploadInput = z.infer<typeof meshSourceOrUploadInputSchema>

export function decodeBase64Bytes(value: string, label = 'data_base64'): Uint8Array {
  const normalized = value.replace(/\s+/gu, '')
  if (!normalized || !/^[A-Za-z0-9+/]*={0,2}$/u.test(normalized) || normalized.length % 4 !== 0) {
    throw new InvalidToolInputError(`${label} must be non-empty standard base64.`)
  }
  return Uint8Array.from(Buffer.from(normalized, 'base64'))
}

export async function decodeUploadedPolygonMesh(input: {
  file_name: string
  data_base64: string
  source_format?: (typeof MESH_IMPORT_FORMATS)[number] | undefined
}): Promise<ImportedMesh> {
  try {
    const bytes = decodeBase64Bytes(input.data_base64)
    return await importMeshFile(input.file_name, bytes, {
      ...(input.source_format !== undefined ? { format: input.source_format } : {}),
      repair: 'none',
    })
  } catch (error) {
    if (error instanceof MeshImportError) {
      throw new InvalidToolInputError(error.message)
    }
    throw error
  }
}

export async function resolveMeshFromSourceOrUpload(
  input: MeshSourceOrUploadInput,
  geometry: McpGeometryService,
  signal?: AbortSignal,
): Promise<{ mesh: PolygonMesh; sourceLabel: string }> {
  if ('source' in input && input.source !== undefined) {
    const compiled = await geometry.compile(input.source, 'full', signal, 'analysis')
    if (compiled.meshes.length === 0) {
      throw new InvalidToolInputError('OpenSCAD source produced no 3D meshes.', { code: 'invalid_geometry' })
    }
    const stlBytes = buildBinaryStl(compiled.meshes, 'mcp')
    const imported = await importMeshFile('compiled.stl', stlBytes, {
      format: 'stl',
      repair: 'none',
      weld: 0,
    })
    return {
      mesh: {
        positions: toPolygonPositions(imported.positions),
        indices: toPolygonIndices(imported.indices),
      },
      sourceLabel: 'openscad_source',
    }
  }
  const imported = await decodeUploadedPolygonMesh({
    file_name: input.file_name,
    data_base64: input.data_base64,
    ...(input.source_format !== undefined ? { source_format: input.source_format } : {}),
  })
  return {
    mesh: {
      positions: toPolygonPositions(imported.positions),
      indices: toPolygonIndices(imported.indices),
    },
    sourceLabel: input.file_name,
  }
}
