import { z } from 'zod/v4'
import type { McpServer } from '@modelcontextprotocol/server'
import { convertMeshFile, MESH_EXPORT_FORMATS, MESH_IMPORT_FORMATS } from '../services/meshConvert'
import { MESH_IMPORT_MAX_BYTES, MeshImportError } from '../services/meshImport'
import { meshExportBase64 } from '../services/meshExportFormats'

/** Base64 of the 20 MB import cap, rounded up to whole quads. */
const MAX_BASE64_LENGTH = Math.ceil(MESH_IMPORT_MAX_BYTES / 3) * 4

function decodeBase64(value: string): Uint8Array {
  const normalized = value.replace(/\s+/gu, '')
  if (!/^[A-Za-z0-9+/]*={0,2}$/u.test(normalized) || normalized.length % 4 !== 0) {
    throw new MeshImportError('invalid-data', 'data_base64 must be standard base64.')
  }
  return Uint8Array.from(Buffer.from(normalized, 'base64'))
}

export function registerMeshConvertTool(server: McpServer) {
  server.registerTool('mesh_convert', {
    title: 'Convert a mesh file between formats',
    description: `Convert a mesh file (${MESH_IMPORT_FORMATS.map(f => f.toUpperCase()).join(', ')}; ASCII or binary where applicable) to ${MESH_EXPORT_FORMATS.map(f => f.toUpperCase()).join(', ')}. Geometry only: colours, textures, units metadata and object names are dropped; vertices are welded exactly and degenerate triangles removed. 3MF/AMF/STL writers require a closed, consistently oriented mesh. Input is capped at 20 MB and 250,000 triangles; output at 4 MiB and 100,000 triangles. Returns the artifact as an embedded base64 resource plus source statistics.`,
    inputSchema: z.object({
      file_name: z.string().min(1).max(255).describe('Input file name; the extension selects the decoder unless source_format is set.'),
      data_base64: z.string().min(1).max(MAX_BASE64_LENGTH).describe('Standard base64 of the file bytes.'),
      format: z.enum(MESH_EXPORT_FORMATS).describe('Target format.'),
      source_format: z.enum(MESH_IMPORT_FORMATS).optional().describe('Override the decoder chosen from the file extension.'),
      weld: z.union([z.number().nonnegative().max(1), z.literal(false)]).optional()
        .describe('Absolute vertex welding tolerance in model units; 0 (default) merges only identical positions, false keeps file indexing.'),
      compressed: z.boolean().optional().describe('3MF only: DEFLATE the package (default true).'),
    }).strict(),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async input => {
    try {
      const result = await convertMeshFile(input.file_name, decodeBase64(input.data_base64), input.format, {
        format: input.source_format,
        weld: input.weld,
        compressed: input.compressed,
      })
      const structured = {
        format: result.format,
        file_name: result.fileName,
        mime_type: result.mimeType,
        byte_length: result.data.byteLength,
        source: {
          format: result.source.format,
          byte_length: result.source.byteLength,
          triangle_count: result.source.triangleCount,
          vertex_count: result.source.vertexCount,
          source_vertex_count: result.source.sourceVertexCount,
          degenerate_triangles: result.source.degenerateTriangles,
        },
      }
      return {
        structuredContent: structured,
        content: [
          { type: 'text' as const, text: JSON.stringify(structured) },
          {
            type: 'resource' as const,
            resource: {
              uri: `mesh://convert/${encodeURIComponent(result.fileName)}`,
              mimeType: result.mimeType,
              blob: meshExportBase64(result.data),
            },
          },
        ],
      }
    } catch (error) {
      const code = error instanceof MeshImportError ? error.code : 'conversion_failed'
      const message = error instanceof Error ? error.message : 'Mesh conversion failed.'
      return { isError: true, structuredContent: { error: { code, message } }, content: [{ type: 'text' as const, text: message }] }
    }
  })
}
