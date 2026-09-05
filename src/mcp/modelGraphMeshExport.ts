import { z } from 'zod/v4';
import type { McpServer } from '@modelcontextprotocol/server';
import type { McpGeometryService } from './geometryService';
import { compileModelGraph, modelGraphSchema } from '../services/modelGraph';
import { exportMeshFormat, MESH_EXPORT_FORMATS, meshExportBase64 } from '../services/meshExportFormats';
import { flattenExportMeshes } from '../services/meshExportAdapter';
export function registerModelGraphMeshExport(server: McpServer, geometry: McpGeometryService) {
    server.registerTool('modelgraph_export', { description: 'Export ModelGraph mesh as STL (ASCII or binary), 3MF, OBJ, PLY, OFF or AMF. Printing formats require closed consistently oriented geometry; maximum 4 MiB. Exports geometry, without slicer settings, textures or assembly identities.', inputSchema: z.object({ document: modelGraphSchema, format: z.enum(MESH_EXPORT_FORMATS) }), annotations: { readOnlyHint: true, openWorldHint: false } }, async (input, context) => {
        try {
            const compiled = compileModelGraph(input.document), built = await geometry.compile(compiled.source, 'full', context.mcpReq.signal);
            const mesh = flattenExportMeshes(built.meshes);
            const artifact = exportMeshFormat(mesh, input.format);
            return { content: [{ type: 'resource' as const, resource: { uri: `modelgraph://export/${compiled.document_sha256}.${artifact.extension}`, mimeType: artifact.mimeType, blob: meshExportBase64(artifact.data) } }] };
        }
        catch (error) {
            return { isError: true, content: [{ type: 'text' as const, text: error instanceof Error ? error.message : 'Mesh export failed.' }] };
        }
    });
}
