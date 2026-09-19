import { EventEmitter } from 'node:events'
import { PassThrough } from 'node:stream'
import { setImmediate } from 'node:timers/promises'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'

const { spawn } = vi.hoisted(() => ({ spawn: vi.fn() }))
vi.mock('node:child_process', () => ({ spawn }))
vi.mock('../src/services/modelGraphNurbs', () => ({ compileModelGraphNurbs: vi.fn() }))
import { runOwnNurbs } from '../src/mcp/modelGraphNurbsRuntime'

class Child extends EventEmitter {
  stdin = new PassThrough()
  stdout = new PassThrough()
  stderr = new PassThrough()
  exitCode: number | null = null
  signalCode: NodeJS.Signals | null = null
  kill = vi.fn(() => true)

  close(code: number | null = null, signal: NodeJS.Signals | null = 'SIGKILL') {
    this.exitCode = code
    this.signalCode = signal
    this.emit('close', code, signal)
  }
}

let children: Child[]
beforeEach(() => {
  children = []
  spawn.mockImplementation(() => { const child = new Child(); children.push(child); return child })
})
afterEach(() => { vi.useRealTimers(); spawn.mockReset() })
const run = (signal?: AbortSignal) => runOwnNurbs({}, { action: 'build' }, signal)
const ok = { ok: true }

it('waits for EOF before termination and for close before releasing an admission slot', async () => {
  const first = run(), second = run()
  const [a, b] = children
  await expect(run()).rejects.toThrow('Two NURBS jobs')
  a!.stdout.write(JSON.stringify(ok))
  await setImmediate()
  expect(a!.kill).not.toHaveBeenCalled()
  a!.stdout.end()
  await setImmediate()
  expect(a!.kill).toHaveBeenCalledWith('SIGKILL')
  await expect(run()).rejects.toThrow('Two NURBS jobs')
  let settled = false
  void first.then(() => { settled = true })
  await setImmediate()
  expect(settled).toBe(false)
  a!.close()
  await expect(first).resolves.toEqual(ok)
  b!.stdout.end(JSON.stringify(ok))
  await setImmediate()
  b!.close()
  await expect(second).resolves.toEqual(ok)
})

it('collects split UTF-8 chunks and accepts a completed natural exit', async () => {
  const pending = run()
  const child = children[0]!
  const response = { ok: false, error: { code: 'TEST', message: '\u041e\u0448\u0438\u0431\u043a\u0430' } }
  const bytes = Buffer.from(JSON.stringify(response))
  for (const byte of bytes) child.stdout.write(Buffer.from([byte]))
  child.exitCode = 0
  child.stdout.end()
  await setImmediate()
  expect(child.kill).not.toHaveBeenCalled()
  child.close(0, null)
  await expect(pending).resolves.toEqual(response)
})

it.each(['', '{"ok":', '{"ok":"yes"}', '{"ok":true} trailing'])('refuses malformed or incomplete EOF: %j', async response => {
  const pending = run()
  const rejected = expect(pending).rejects.toThrow('Invalid NURBS process response')
  const child = children[0]!
  child.stdout.end(response)
  await setImmediate()
  expect(child.kill).toHaveBeenCalledWith('SIGKILL')
  child.close()
  await rejected
})

it('does not treat an unrelated nonzero exit as successful response termination', async () => {
  const pending = run()
  const rejected = expect(pending).rejects.toThrow('NURBS process failed')
  const child = children[0]!
  child.stdout.end(JSON.stringify(ok))
  await setImmediate()
  child.close(1, null)
  await rejected
})

it('retains cancellation precedence after EOF while waiting for join', async () => {
  const controller = new AbortController()
  const pending = run(controller.signal)
  const rejected = expect(pending).rejects.toThrow('cancelled')
  const child = children[0]!
  child.stdout.end(JSON.stringify(ok))
  await setImmediate()
  controller.abort()
  child.close()
  await rejected
})

it('keeps the deadline active until close even after a complete response', async () => {
  vi.useFakeTimers()
  const pending = run()
  const rejected = expect(pending).rejects.toThrow('exceeded 30 seconds')
  const child = children[0]!
  child.stdout.end(JSON.stringify(ok))
  await setImmediate()
  await vi.advanceTimersByTimeAsync(30_000)
  child.close()
  await rejected
})

it('rejects output overflow and stream errors after joining', async () => {
  for (const kind of ['overflow', 'read-error']) {
    const pending = run()
    const rejected = expect(pending).rejects.toThrow(kind === 'overflow' ? 'exceeds 12 MiB' : 'could not be read')
    const child = children.at(-1)!
    if (kind === 'overflow') child.stdout.end('x'.repeat(12 * 1024 * 1024 + 1))
    else child.stdout.emit('error', new Error('pipe failed'))
    await setImmediate()
    expect(child.kill).toHaveBeenCalledWith('SIGKILL')
    child.close()
    await rejected
  }
})

it('does not accept SIGKILL if the requested completion termination was not sent', async () => {
  const pending = run()
  const rejected = expect(pending).rejects.toThrow('NURBS process failed')
  const child = children[0]!
  child.kill.mockReturnValue(false)
  child.stdout.end(JSON.stringify(ok))
  await setImmediate()
  child.close()
  await rejected
})

it('rejects exit without EOF even when the buffered JSON is valid', async () => {
  const pending = run()
  const rejected = expect(pending).rejects.toThrow('Invalid NURBS process response')
  const child = children[0]!
  child.stdout.write(JSON.stringify(ok))
  child.close(0, null)
  await rejected
})

it('reports process startup failures and releases the admission slot on close', async () => {
  const pending = run()
  const rejected = expect(pending).rejects.toThrow('could not start')
  const child = children[0]!
  child.emit('error', new Error('spawn failed'))
  child.close(-2, null)
  await rejected
})

it('does not start pre-cancelled or oversized requests', async () => {
  await expect(run(AbortSignal.abort())).rejects.toThrow('cancelled')
  await expect(runOwnNurbs({ data: 'x'.repeat(262144) }, { action: 'build' })).rejects.toThrow('exceeds 256 KiB')
  expect(spawn).not.toHaveBeenCalled()
})
