import { MESH_EXPORT_FORMATS } from '../services/meshExportFormats';
import { z } from 'zod/v4';
import type { McpServer } from '@modelcontextprotocol/server';
import { modelGraphNurbsSchema, compileModelGraphNurbs, MODELGRAPH_NURBS_GUIDE, MODELGRAPH_NURBS_EXAMPLE, MODELGRAPH_NURBS_SURFACE_EXAMPLE } from '../services/modelGraphNurbs';
import { runOwnNurbs } from './modelGraphNurbsRuntime';
import { renderModelGraphPreviews } from './modelGraphPreview';
const annotations = { readOnlyHint: true, openWorldHint: false };
const text = (value: object, isError = false) => ({ isError, structuredContent: value as Record<string, unknown>, content: [{ type: 'text' as const, text: JSON.stringify(value) }] });
const failure = (error: unknown) => text({ error: { code: 'NURBS_REQUEST_FAILED', message: error instanceof Error ? error.message : 'NURBS request failed.' } }, true);
export const ownNurbsLanguage = () => ({ language: 'modelgraph/nurbs-1', guide: MODELGRAPH_NURBS_GUIDE, schema: z.toJSONSchema(modelGraphNurbsSchema), example: MODELGRAPH_NURBS_EXAMPLE, surface_example: MODELGRAPH_NURBS_SURFACE_EXAMPLE });
export function registerModelGraphNurbsTools(server: McpServer) {
    server.registerResource('modelgraph-nurbs-language', 'openscad://language/modelgraph-nurbs-1', { description: 'Own rational NURBS language, mathematical operations, tessellation and limitations.', mimeType: 'application/json' }, async (uri) => ({ contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify(ownNurbsLanguage()) }] }));
    server.registerTool('modelgraph_nurbs_language', { description: 'Read the own NURBS language schema and examples before modeling. No third-party NURBS kernel, B-rep or STEP provider.', inputSchema: z.object({}), annotations }, async () => text(ownNurbsLanguage()));
    server.registerTool('modelgraph_nurbs_compile', { description: 'Validate the NURBS document graph and resolve parameters. Numeric spline and mesh validation runs in build/evaluate.', inputSchema: z.object({ document: modelGraphNurbsSchema }), annotations }, async (input) => { try {
        return text(compileModelGraphNurbs(input.document));
    }
    catch (error) {
        return failure(error);
    } });
    server.registerTool('modelgraph_nurbs_evaluate', { description: 'Evaluate own rational curve/surface coordinates and first/second derivatives. Inspect derivative_status and null normals/curvatures at singularities.', inputSchema: z.object({ document: modelGraphNurbsSchema, evaluations: z.array(z.object({ node: z.string().max(32), u: z.number().finite(), v: z.number().finite().optional() }).strict()).min(1).max(64) }), annotations }, async (input, context) => { try {
        const r = await runOwnNurbs(input.document, { action: 'evaluate', evaluations: input.evaluations }, context.mcpReq.signal);
        return text(r, !r.ok);
    }
    catch (error) {
        return failure(error);
    } });
    server.registerTool('modelgraph_nurbs_build', { description: 'Build with our own spline kernel. Return authoritative rational definitions, mesh topology and three PNG views for tessellated models. Mesh deviation is sampled, not a certified bound; self-intersection and printability remain unknown.', inputSchema: z.object({ document: modelGraphNurbsSchema }), annotations }, async (input, context) => {
        try {
            const result = await runOwnNurbs(input.document, { action: 'build' }, context.mcpReq.signal);
            if (!result.ok)
                return text(result, true);
            let images: ReturnType<typeof renderModelGraphPreviews> = [], imageError: string | null = null;
            if ('mesh' in result && result.mesh) {
                try {
                    const m = result.mesh, vertices = new Float32Array(m.positions.length * 2);
                    for (let i = 0; i < m.positions.length / 3; i++)
                        vertices.set(m.positions.slice(i * 3, i * 3 + 3), i * 6);
                    images = renderModelGraphPreviews([{ vertices, indices: new Uint32Array(m.indices), transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]) }]);
                }
                catch (error) {
                    imageError = error instanceof Error ? error.message : 'Preview unavailable';
                }
            }
            const { mesh: _mesh, ...report } = result as typeof result & {
                mesh?: unknown;
            };
            const structured = { ...report, images_status: images.length ? 'rendered' : 'unavailable', image_error: imageError, views: images.map(i => i.view) };
            return { ...text(structured), content: [{ type: 'text' as const, text: JSON.stringify(structured) }, ...images.map(i => ({ type: 'image' as const, mimeType: 'image/png', data: i.png.toString('base64') }))] };
        }
        catch (error) {
            return failure(error);
        }
    });
    server.registerTool('modelgraph_nurbs_export', { description: 'Export NURBS JSON or mesh STL (ASCII/binary), 3MF, OBJ, PLY, OFF or AMF. Printing formats require a closed oriented mesh. No STEP export. Mesh topology checks do not certify absence of self-intersections or printability.', inputSchema: z.object({ document: modelGraphNurbsSchema, format: z.enum(['json', ...MESH_EXPORT_FORMATS]) }), annotations }, async (input, context) => {
        try {
            const result = await runOwnNurbs(input.document, { action: 'export', format: input.format }, context.mcpReq.signal);
            if (!result.ok)
                return text(result, true);
            if (!('artifact' in result) || !result.artifact)
                throw new Error('Export artifact missing.');
            const { artifact, ...report } = result;
            return { ...text(report), content: [{ type: 'text' as const, text: JSON.stringify(report) }, { type: 'resource' as const, resource: { uri: `modelgraph://nurbs-export/${result.document_sha256}.${input.format === 'stl_binary' ? 'stl' : input.format}`, mimeType: artifact.mime_type, blob: typeof artifact.base64 === 'string' ? artifact.base64 : Buffer.from(artifact.text!).toString('base64') } }] };
        }
        catch (error) {
            return failure(error);
        }
    });
}
