import { describe, expect, it } from 'vitest'
import {
  PARSER_ISOLATION_BOUNDARY,
  UntrustedParserWorkerClient,
  type UntrustedParserPort,
  type UntrustedParserRequest,
} from '../src/services/untrustedParserWorker'
import { executeUntrustedParserJob } from '../src/services/untrustedParserWorkerRuntime'

class FakePort implements UntrustedParserPort {
  onmessage: UntrustedParserPort['onmessage'] = null
  onerror: UntrustedParserPort['onerror'] = null
  onmessageerror: UntrustedParserPort['onmessageerror'] = null
  terminated = false
  request: UntrustedParserRequest | null = null
  constructor(private readonly mode: 'success' | 'silent' | 'crash') {}
  postMessage(request: UntrustedParserRequest): void {
    this.request = request
    if (this.mode === 'success') {
      const result = executeUntrustedParserJob(request.job)
      queueMicrotask(() => this.onmessage?.({ data: { version: 1, id: request.id, ok: true, result } } as MessageEvent))
    } else if (this.mode === 'crash') queueMicrotask(() => this.onerror?.({ message: 'parser trap' } as ErrorEvent))
  }
  terminate(): void { this.terminated = true }
}

describe('untrusted parser worker boundary', () => {
  it('states API/worker isolation without claiming an OS sandbox', () => {
    expect(PARSER_ISOLATION_BOUNDARY).toMatchObject({ kind: 'worker-api-isolation', sandbox: false, terminatesOnTimeout: true })
    expect(PARSER_ISOLATION_BOUNDARY.note).not.toMatch(/security sandbox\./i && /is a security sandbox/i)
  })

  it('returns the raw pass result across the same typed request lifecycle', async () => {
    const port = new FakePort('success')
    const client = new UntrustedParserWorkerClient(() => port)
    const raw = await client.run({ kind: 'step-raw', text: '#1=A(#2);' })
    expect(raw.entities[0]).toMatchObject({ id: 1, references: [2] })
    expect(port.terminated).toBe(false)
    client.dispose()
  })

  it('terminates a non-cooperative parser on timeout', async () => {
    const port = new FakePort('silent')
    const client = new UntrustedParserWorkerClient(() => port)
    await expect(client.run({ kind: 'step-raw', text: '#1=A();' }, { timeoutMs: 5 }))
      .rejects.toMatchObject({ code: 'PARSER_TIMEOUT' })
    expect(port.terminated).toBe(true)
  })

  it('terminates on abort and discards a crashed realm', async () => {
    const silent = new FakePort('silent'), controller = new AbortController()
    const client = new UntrustedParserWorkerClient(() => silent)
    const pending = client.run({ kind: 'step-raw', text: '#1=A();' }, { signal: controller.signal })
    controller.abort()
    await expect(pending).rejects.toMatchObject({ code: 'PARSER_CANCELLED', name: 'AbortError' })
    expect(silent.terminated).toBe(true)

    const crashed = new FakePort('crash')
    await expect(new UntrustedParserWorkerClient(() => crashed).run({ kind: 'step-raw', text: '#1=A();' }))
      .rejects.toMatchObject({ code: 'PARSER_CRASH' })
    expect(crashed.terminated).toBe(true)
  })
})
