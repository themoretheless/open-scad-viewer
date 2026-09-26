import { buildOwnNurbs, type OwnNurbsRequest } from '../services/modelGraphNurbsKernel';
import { stringifyMeshJson } from '../services/meshJson';
let input = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', (chunk: string) => { input += chunk; if (Buffer.byteLength(input) > 262144)
    process.exit(2); });
process.stdin.on('end', () => {
    try {
        const request = JSON.parse(input) as {
            document: unknown;
            request: OwnNurbsRequest;
        };
        const result = buildOwnNurbs(request.document, request.request);
        // EOF publishes the complete response; the parent owns termination and join.
        process.stdout.end(stringifyMeshJson(result));
    }
    catch (error) {
        process.stdout.end(JSON.stringify({ ok: false, error: { code: 'NURBS_OPERATION_FAILED', message: error instanceof Error ? error.message.slice(0, 1500) : 'NURBS operation failed.' } }));
    }
});
