import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { compileModelGraphNurbs } from '../services/modelGraphNurbs';
import type { OwnNurbsRequest, buildOwnNurbs } from '../services/modelGraphNurbsKernel';
export type { OwnNurbsRequest };
export type OwnNurbsResult = ReturnType<typeof buildOwnNurbs> | {
    ok: false;
    error: {
        code: string;
        message: string;
    };
};
const root = fileURLToPath(new URL('../../', import.meta.url)), entry = fileURLToPath(new URL('./ownNurbsProcess.ts', import.meta.url));
let active = 0;
/** Disposable own-kernel process: cancellation and CPU deadline never select another engine. */
export async function runOwnNurbs(document: unknown, request: OwnNurbsRequest, signal?: AbortSignal): Promise<OwnNurbsResult> {
    compileModelGraphNurbs(document);
    if (signal?.aborted)
        throw new Error('NURBS request cancelled.');
    if (active >= 2)
        throw new Error('Two NURBS jobs are already running; retry after completion.');
    const data = JSON.stringify({ document, request });
    if (Buffer.byteLength(data) > 262144)
        throw new Error('NURBS request exceeds 256 KiB.');
    active++;
    try {
        return await new Promise((resolve, reject) => {
            const child = spawn(process.execPath, ['--import', 'tsx', entry], { cwd: root, stdio: ['pipe', 'pipe', 'pipe'], shell: false, env: { PATH: process.env.PATH ?? '', SYSTEMROOT: process.env.SYSTEMROOT ?? '' } });
            let text = '', bytes = 0, failure: Error | undefined;
            const kill = (error: Error) => { failure ??= error; child.kill('SIGKILL'); }, abort = () => kill(new Error('NURBS request cancelled.'));
            const timer = setTimeout(() => kill(new Error('NURBS request exceeded 30 seconds.')), 30000);
            signal?.addEventListener('abort', abort, { once: true });
            if (signal?.aborted)
                abort();
            child.stdout.setEncoding('utf8');
            child.stdout.on('data', (s: string) => { bytes += Buffer.byteLength(s); if (bytes > 12 * 1024 * 1024)
                kill(new Error('NURBS response exceeds 12 MiB.'));
            else
                text += s; });
            child.stderr.on('data', () => { });
            child.stdin.on('error', () => { });
            child.on('error', () => { failure ??= new Error('NURBS process could not start.'); });
            child.on('close', code => { clearTimeout(timer); signal?.removeEventListener('abort', abort); if (failure) {
                reject(failure);
                return;
            } if (code !== 0) {
                reject(new Error('NURBS process failed; no other kernel was used.'));
                return;
            } try {
                const result = JSON.parse(text);
                if (typeof result?.ok !== 'boolean')
                    throw new Error('Invalid response');
                resolve(result);
            }
            catch {
                reject(new Error('Invalid NURBS process response.'));
            } });
            child.stdin.end(data);
        });
    }
    finally {
        active--;
    }
}
