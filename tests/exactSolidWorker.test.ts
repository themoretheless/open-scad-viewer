import { afterEach, describe, expect, it, vi } from 'vitest'
import { buildExactSolidsInWorker, type ExactSolidWorkerPort } from '../src/services/solid/exactSolidClient'
import { runExactSolidRequest } from '../src/services/solid/exactSolidRuntime'
import { evaluateExactSolids } from '../src/services/geometryBuildEngine'
import { buildExactSolidBodies } from '../src/services/solid/brepBuild'
import { EXACT_SOLID_TIMEOUT_MS, isExactSolidRequest, isExactSolidResponse, type ExactSolidRequest } from '../src/services/solid/exactSolidProtocol'
import * as directDocument from '../src/services/directModeling'

class FakeWorker extends EventTarget implements ExactSolidWorkerPort {
  terminate = vi.fn()
  postMessage = vi.fn<(request: ExactSolidRequest) => void>()
  reply(data: unknown) { this.dispatchEvent(new MessageEvent('message', { data })) }
}

afterEach(() => { vi.useRealTimers(); vi.restoreAllMocks() })

describe('exact-solid worker boundary', () => {
  it.each([
    'cube([10,20,30], center=true);',
    'cube(2); translate([8,0,0]) cube(3);',
    '// @modelgraph-text/1\npart = box(10mm, 10mm, 10mm)\nshow part',
  ])('preserves exact geometry for %s', async source => {
    const plan = (await evaluateExactSolids(source)).exactSolids!
    const expected = buildExactSolidBodies(plan.nodes, plan.roots).map(({ id, ...body }) => body)
    const worker = new FakeWorker()
    const pending = buildExactSolidsInWorker(source, undefined, () => worker)
    worker.reply(await runExactSolidRequest(worker.postMessage.mock.calls[0][0]))
    const actual = (await pending).map(({ id, ...body }) => body)
    expect(actual).toEqual(expected)
    expect(worker.terminate).toHaveBeenCalledOnce()
  })

  it('preserves named inexact refusal without publishing partial bodies', async () => {
    const response = await runExactSolidRequest({ kind: 'exact-solid', version: 1, source: 'cube(2); hull(){cube(2);translate([6,0,0])cube(2);}' })
    expect(response).toMatchObject({ ok: false, error: { name: 'InexactSolidError', message: expect.stringContaining('hull') } })
    expect(isExactSolidResponse(response)).toBe(true)
  })

  it('rejects oversize source before starting a worker', async () => {
    const factory = vi.fn(() => new FakeWorker())
    await expect(buildExactSolidsInWorker(' '.repeat(100001), undefined, factory)).rejects.toThrow('100000')
    expect(factory).not.toHaveBeenCalled()
    expect(isExactSolidRequest({ kind: 'exact-solid', version: 1, source: '', extra: 1 })).toBe(false)
    expect((await runExactSolidRequest({ kind: 'exact-solid', version: 2, source: '' })).ok).toBe(false)
  })

  it('aborts without starting, or terminates active computation and ignores late publication', async () => {
    const controller = new AbortController()
    const worker = new FakeWorker()
    const pending = buildExactSolidsInWorker('cube(2);', controller.signal, () => worker)
    controller.abort()
    await expect(pending).rejects.toMatchObject({ name: 'AbortError' })
    worker.reply({ kind: 'exact-solid', version: 1, ok: true, document: '{}' })
    expect(worker.terminate).toHaveBeenCalledOnce()
    const factory = vi.fn(() => worker)
    await expect(buildExactSolidsInWorker('', controller.signal, factory)).rejects.toMatchObject({ name: 'AbortError' })
    expect(factory).not.toHaveBeenCalled()
  })

  it.each(['error', 'messageerror'])('terminates on %s', async type => {
    const worker = new FakeWorker()
    const pending = buildExactSolidsInWorker('', undefined, () => worker)
    worker.dispatchEvent(new Event(type))
    await expect(pending).rejects.toThrow('unexpectedly')
    expect(worker.terminate).toHaveBeenCalledOnce()
  })

  it('propagates cancellation into receiving-realm validation after a worker reply', async () => {
    let validationSignal: AbortSignal | undefined
    const validate = vi.spyOn(directDocument, 'parseDirectDocumentAsync').mockImplementation((_text, options) => {
      validationSignal = options?.signal
      return new Promise((_resolve, reject) => validationSignal?.addEventListener('abort', () => {
        reject(new DOMException('Operation cancelled', 'AbortError'))
      }, { once: true }))
    })
    const worker = new FakeWorker()
    const controller = new AbortController()
    const pending = buildExactSolidsInWorker('', controller.signal, () => worker)
    worker.reply({ kind: 'exact-solid', version: 1, ok: true, document: '{"version":1,"sketches":[],"bodies":[]}' })
    await vi.waitFor(() => expect(validate).toHaveBeenCalledOnce())
    controller.abort()
    await expect(pending).rejects.toMatchObject({ name: 'AbortError' })
    expect(validationSignal?.aborted).toBe(true)
    expect(worker.terminate).toHaveBeenCalledOnce()
  })

  it('terminates on deadline, transport failure, and malformed response', async () => {
    vi.useFakeTimers()
    const worker = new FakeWorker()
    const pending = buildExactSolidsInWorker('', undefined, () => worker)
    const rejected = expect(pending).rejects.toThrow('120 second')
    await vi.advanceTimersByTimeAsync(EXACT_SOLID_TIMEOUT_MS)
    await rejected
    expect(worker.terminate).toHaveBeenCalledOnce()
    worker.postMessage.mockImplementation(() => { throw new Error('transport') })
    await expect(buildExactSolidsInWorker('', undefined, () => worker)).rejects.toThrow('transport')
    const malformed = new FakeWorker()
    const next = buildExactSolidsInWorker('', undefined, () => malformed)
    malformed.reply({ kind: 'exact-solid', version: 1, ok: true, document: '{}' })
    await expect(next).rejects.toThrow('Invalid direct modeling document')
    expect(malformed.terminate).toHaveBeenCalledOnce()
    expect(vi.getTimerCount()).toBe(0)
  })
})
