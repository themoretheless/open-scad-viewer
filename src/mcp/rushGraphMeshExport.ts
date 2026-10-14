import { evaluateRushGraphGeometry, requireRushGraphChecks, RushGraphCheckError } from '../services/rushGraphChecks'
import { z } from 'zod/v4';
import type { McpServer } from '@modelcontextprotocol/server';
import type { McpGeometryService } from './geometryService';
import { compileRushGraph, rushGraphSchema } from '../services/rushGraph';
import { exportMeshFormatCompressed, MESH_EXPORT_FORMATS, meshExportBase64 } from '../services/meshExportFormats';
import { flattenExportMeshes } from '../services/meshExportAdapter';
export function registerRushGraphMeshExport(server: McpServer, geometry: McpGeometryService) {
    server.registerTool('rush_export', { description: 'Export RushGraph mesh as STL (ASCII or binary), 3MF, OBJ, PLY, OFF or AMF. Printing formats require closed consistently oriented geometry; maximum 4 MiB. 3MF preserves scene meshes as separate objects in world coordinates and uses lossless ZIP compression. Exports geometry, without slicer settings, textures or assembly identities.', inputSchema: z.object({ document: rushGraphSchema, format: z.enum(MESH_EXPORT_FORMATS) }), annotations: { readOnlyHint: true, openWorldHint: false } }, async (input, context) => {
        try {
            const compiled = compileRushGraph(input.document), built = await geometry.compile(compiled.source, 'full', context.mcpReq.signal);
            requireRushGraphChecks(await evaluateRushGraphGeometry(compiled.geometry_assertions, built.meshes, async source => (await geometry.compile(source, 'full', context.mcpReq.signal)).meshes));
            const mesh = flattenExportMeshes(built.meshes);
            const artifact = await exportMeshFormatCompressed(mesh, input.format);
            return { content: [{ type: 'resource' as const, resource: { uri: `rush://export/${compiled.document_sha256}.${artifact.extension}`, mimeType: artifact.mimeType, blob: meshExportBase64(artifact.data) } }] };
        }
        catch (error) {
            if (error instanceof RushGraphCheckError) return { isError: true, structuredContent: {checks: error.checks}, content: [{type: 'text' as const, text: error.message}] };
            return { isError: true, content: [{ type: 'text' as const, text: error instanceof Error ? error.message : 'Mesh export failed.' }] };
        }
    });
}
