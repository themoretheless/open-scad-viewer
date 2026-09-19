import {afterEach, expect, it, vi} from 'vitest'
import {HeadlessGeometryService} from '../src/mcp/geometryService'
import {GeometryBuildEngine, geometryExecutionForError} from '../src/services/geometryBuildEngine'
import {GEOMETRY_MANIFEST_ARCHIVE} from '../src/core/geometryExecution'

function fixture(warm: () => Promise<void>) {
  const manifest = GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1']
  const build = vi.fn(async (_source: string) => ({meshes: [], warnings: [], volume: 0, surfaceArea: 0,
    quality: 'full' as const, reduced: false,
    timings: {parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0}}))
  const provider = {engineClass: 'mesh' as const, engineKey: manifest.engineKey,
    kernelFingerprint: manifest.kernelFingerprint, capabilityManifestVersion: manifest.capabilityManifestVersion,
    warm: vi.fn(warm), build}
  const engine = new GeometryBuildEngine([provider])
  return {provider, engine, service: new HeadlessGeometryService(engine)}
}

afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers() })

it('initializes the injected provider before its normal readiness check', async () => {
  vi.useFakeTimers()
  const {provider, service} = fixture(() => new Promise(resolve => setTimeout(resolve, 350)))
  const result = service.compile('cube(1);')
  const assertion = expect(result).resolves.toMatchObject({analysis: {execution: {evidence: 'runtime'}}})
  await Promise.all([assertion, vi.advanceTimersByTimeAsync(351)])
  expect(provider.warm).toHaveBeenCalledTimes(1)
  expect(provider.build).toHaveBeenCalledTimes(1)
  expect(vi.getTimerCount()).toBe(0)
})

it('waits for the same warmup after a short capabilities probe times out', async () => {
  vi.useFakeTimers()
  const {engine, provider, service} = fixture(() => new Promise(resolve => setTimeout(resolve, 400)))
  const probe = engine.capabilities()
  await vi.advanceTimersByTimeAsync(251)
  expect((await probe).engines[0].availability).toBe('unavailable')
  const result = service.compile('cube(1);')
  await Promise.all([expect(result).resolves.toBeDefined(), vi.advanceTimersByTimeAsync(150)])
  expect(provider.warm).toHaveBeenCalledTimes(1)
  expect(provider.build).toHaveBeenCalledTimes(1)
  expect(vi.getTimerCount()).toBe(0)
})

it('cancels one startup waiter without cancelling shared warmup or building its source', async () => {
  vi.useFakeTimers()
  const {provider, service} = fixture(() => new Promise(resolve => setTimeout(resolve, 350)))
  const controller = new AbortController()
  const remove = vi.spyOn(controller.signal, 'removeEventListener')
  const cancelled = service.compile('cube(1);', 'full', controller.signal).catch(error => error)
  const other = service.compile('cube(2);')
  await vi.advanceTimersByTimeAsync(50)
  controller.abort()
  const error = await cancelled
  expect(error.name).toBe('AbortError')
  expect(geometryExecutionForError(error)?.evidence).toBe('planned')
  expect(provider.build).not.toHaveBeenCalled()
  expect(remove).toHaveBeenCalledWith('abort', expect.any(Function))
  await Promise.all([expect(other).resolves.toBeDefined(), vi.advanceTimersByTimeAsync(301)])
  expect(provider.warm).toHaveBeenCalledTimes(1)
  expect(provider.build).toHaveBeenCalledTimes(1)
  expect(provider.build.mock.calls[0]?.[0]).toBe('cube(2);')
  expect(vi.getTimerCount()).toBe(0)
})

it('bounds hung initialization and does not restart the deadline on subsequent requests', async () => {
  vi.useFakeTimers()
  let rejectWarm!: (error: Error) => void
  const {provider, service} = fixture(() => new Promise((_resolve, reject) => { rejectWarm = reject }))
  const result = service.compile('cube(1);').catch(error => error)
  await vi.advanceTimersByTimeAsync(4999)
  expect(provider.build).not.toHaveBeenCalled()
  await vi.advanceTimersByTimeAsync(1)
  expect((await result).reason).toContain('initialization check exceeded 5000 ms')
  await expect(service.compile('cube(2);')).rejects.toMatchObject({availabilityCause: 'readiness-timeout'})
  expect(provider.warm).toHaveBeenCalledTimes(1)
  expect(vi.getTimerCount()).toBe(0)
  rejectWarm(new Error('late failure'))
  await vi.advanceTimersByTimeAsync(0)
  expect(provider.build).not.toHaveBeenCalled()
  expect(vi.getTimerCount()).toBe(0)
})

it('rejects revoked providers before initialization', async () => {
  const {provider} = fixture(async () => undefined)
  const engine = new GeometryBuildEngine([provider], {revokedManifestDigests: [GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'].manifestDigest]})
  await expect(new HeadlessGeometryService(engine).compile('cube(1);')).rejects.toMatchObject({availabilityCause: 'revoked'})
  expect(provider.warm).not.toHaveBeenCalled()
  expect(provider.build).not.toHaveBeenCalled()
})

it('does not let a concurrent short probe erase the initialization timeout', async () => {
  vi.useFakeTimers()
  const {engine, provider, service} = fixture(() => new Promise(() => undefined))
  const initialization = service.compile('cube(1);').catch(error => error)
  await vi.advanceTimersByTimeAsync(4750)
  const probe = engine.capabilities()
  await vi.advanceTimersByTimeAsync(250)
  expect((await initialization).reason).toContain('initialization check exceeded 5000 ms')
  expect((await probe).engines[0].unavailableReason).toContain('initialization check exceeded 5000 ms')
  let repeatError: unknown
  void service.compile('cube(2);').catch(error => { repeatError = error })
  await vi.advanceTimersByTimeAsync(0)
  expect(repeatError).toMatchObject({availabilityCause: 'readiness-timeout'})
  expect(provider.warm).toHaveBeenCalledTimes(1)
  expect(vi.getTimerCount()).toBe(0)
})
