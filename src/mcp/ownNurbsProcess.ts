import { buildOwnNurbs, type OwnNurbsRequest } from '../services/modelGraphNurbsKernel';
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
        process.stdout.write(JSON.stringify(result));
    }
    catch (error) {
        process.stdout.write(JSON.stringify({ ok: false, error: { code: 'NURBS_OPERATION_FAILED', message: error instanceof Error ? error.message.slice(0, 1500) : 'NURBS operation failed.' } }));
    }
});
