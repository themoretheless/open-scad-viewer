import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { afterEach, describe, expect, it } from 'vitest'

interface JsonRpcResponse {
  jsonrpc: '2.0'
  id: number
  result?: Record<string, unknown>
  error?: { code: number; message: string }
}

interface QualificationEvent {
  event: string
  [key: string]: unknown
}

const serverPath = fileURLToPath(
  new URL('./fixtures/mcp-shadow-stdio-server.ts', import.meta.url),
)
const repositoryRoot = fileURLToPath(new URL('../', import.meta.url))
const liveChildren = new Set<ChildProcessWithoutNullStreams>()

class StdioQualificationClient {
  readonly child: ChildProcessWithoutNullStreams
  readonly responses: JsonRpcResponse[] = []
  readonly qualificationEvents: QualificationEvent[] = []
  readonly invalidStdoutLines: string[] = []
  readonly stderrLines: string[] = []
  private readonly responseWaiters = new Map<number, (response: JsonRpcResponse) => void>()
  private readonly eventWaiters = new Set<{
    predicate: (event: QualificationEvent) => boolean
    resolve: (event: QualificationEvent) => void
  }>()
  private stdoutBuffer = ''
  private stderrBuffer = ''
  private nextId = 1

  constructor() {
    this.child = spawn(process.execPath, ['--import', 'tsx', serverPath], {
      cwd: repositoryRoot,
      env: process.env,
      stdio: ['pipe', 'pipe', 'pipe'],
    })
    liveChildren.add(this.child)
    this.child.once('close', () => liveChildren.delete(this.child))
    this.child.stdout.setEncoding('utf8')
    this.child.stderr.setEncoding('utf8')
    this.child.stdout.on('data', chunk => this.consumeStdout(chunk))
    this.child.stderr.on('data', chunk => this.consumeStderr(chunk))
  }

  private consumeStdout(chunk: string): void {
    this.stdoutBuffer += chunk
    let newline = this.stdoutBuffer.indexOf('\n')
    while (newline >= 0) {
      const line = this.stdoutBuffer.slice(0, newline).trim()
      this.stdoutBuffer = this.stdoutBuffer.slice(newline + 1)
      newline = this.stdoutBuffer.indexOf('\n')
      if (!line) continue
      try {
        const message = JSON.parse(line) as JsonRpcResponse
        if (message.jsonrpc !== '2.0') throw new TypeError('not JSON-RPC')
        if (typeof message.id === 'number' && ('result' in message || 'error' in message)) {
          this.responses.push(message)
          this.responseWaiters.get(message.id)?.(message)
          this.responseWaiters.delete(message.id)
        }
      } catch {
        this.invalidStdoutLines.push(line)
      }
    }
  }

  private consumeStderr(chunk: string): void {
    this.stderrBuffer += chunk
    let newline = this.stderrBuffer.indexOf('\n')
    while (newline >= 0) {
      const line = this.stderrBuffer.slice(0, newline).trim()
      this.stderrBuffer = this.stderrBuffer.slice(newline + 1)
      newline = this.stderrBuffer.indexOf('\n')
      if (!line) continue
      this.stderrLines.push(line)
      const prefix = 'QUALIFICATION_EVENT '
      if (!line.startsWith(prefix)) continue
      try {
        const event = JSON.parse(line.slice(prefix.length)) as QualificationEvent
        this.qualificationEvents.push(event)
        for (const waiter of [...this.eventWaiters]) {
          if (!waiter.predicate(event)) continue
          this.eventWaiters.delete(waiter)
          waiter.resolve(event)
        }
      } catch {
        // Non-JSON stderr remains diagnostic output and cannot corrupt stdio.
      }
    }
  }

  waitForEvent(
    predicate: (event: QualificationEvent) => boolean,
    timeoutMs = 10_000,
  ): Promise<QualificationEvent> {
    const existing = this.qualificationEvents.find(predicate)
    if (existing) return Promise.resolve(existing)
    return new Promise((resolve, reject) => {
      const waiter = {
        predicate,
        resolve: (event: QualificationEvent) => {
          clearTimeout(timeout)
          resolve(event)
        },
      }
      const timeout = setTimeout(() => {
        this.eventWaiters.delete(waiter)
        reject(new Error(`Timed out waiting for qualification event\nstderr:\n${this.stderrLines.join('\n')}`))
      }, timeoutMs)
      this.eventWaiters.add(waiter)
    })
  }

  async request(method: string, params: Record<string, unknown> = {}): Promise<JsonRpcResponse> {
    const id = this.nextId++
    const response = new Promise<JsonRpcResponse>((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.responseWaiters.delete(id)
        reject(new Error(
          `Timed out waiting for ${method}\nstdout invalid:\n${this.invalidStdoutLines.join('\n')}`,
        ))
      }, 10_000)
      this.responseWaiters.set(id, value => {
        clearTimeout(timeout)
        resolve(value)
      })
    })
    await new Promise<void>((resolve, reject) => {
      this.child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`, error => {
        if (error) reject(error)
        else resolve()
      })
    })
    return await response
  }

  async notify(method: string, params: Record<string, unknown> = {}): Promise<void> {
    await new Promise<void>((resolve, reject) => {
      this.child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', method, params })}\n`, error => {
        if (error) reject(error)
        else resolve()
      })
    })
  }

  async close(): Promise<{ code: number | null; signal: NodeJS.Signals | null }> {
    if (this.child.exitCode !== null || this.child.signalCode !== null) {
      return { code: this.child.exitCode, signal: this.child.signalCode }
    }
    const closed = new Promise<{ code: number | null; signal: NodeJS.Signals | null }>((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.child.kill()
        reject(new Error(`Timed out closing qualification MCP server\nstderr:\n${this.stderrLines.join('\n')}`))
      }, 10_000)
      this.child.once('close', (code, signal) => {
        clearTimeout(timeout)
        resolve({ code, signal })
      })
    })
    this.child.stdin.end()
    return await closed
  }
}

afterEach(async () => {
  await Promise.all([...liveChildren].map(async child => {
    if (child.exitCode === null && child.signalCode === null) child.kill()
    await new Promise<void>(resolve => child.once('close', () => resolve()))
  }))
})

function toolContent(response: JsonRpcResponse): Record<string, unknown> {
  if (response.error) throw new Error(`${response.error.code}: ${response.error.message}`)
  return response.result?.structuredContent as Record<string, unknown>
}

describe('qualification-only Manifold shadow over actual MCP stdio', () => {
  it('keeps stdio/admission/store primary-only while a non-cooperative shadow is hard-killed', async () => {
    const client = new StdioQualificationClient()
    await client.waitForEvent(event => event.event === 'server-ready')
    const initialized = await client.request('initialize', {
      protocolVersion: '2025-11-25',
      capabilities: {},
      clientInfo: { name: 'qualification-stdio-test', version: '1.0.0' },
    })
    expect(initialized.error).toBeUndefined()
    await client.notify('notifications/initialized')

    const firstAnalyze = await client.request('tools/call', {
      name: 'openscad_analyze',
      arguments: { source: 'cube(2);', quality: 'full' },
    })
    expect(toolContent(firstAnalyze)).toMatchObject({
      build: { status: 'succeeded', execution: { semantic_program_version: 'legacy-direct-evaluator-v1' } },
      analysis: { volume: 8, surface_area: 24 },
    })

    await client.waitForEvent(event => event.event === 'shadow-stdout-captured')
    await client.waitForEvent(event => event.event === 'shadow-entered-noncooperative-loop')
    expect(client.qualificationEvents.some(event => event.event === 'shadow-settled')).toBe(false)

    const [ping, tools] = await Promise.all([
      client.request('ping'),
      client.request('tools/list'),
    ])
    expect(ping.error).toBeUndefined()
    expect((tools.result?.tools as Array<{ name: string }>).map(tool => tool.name))
      .toContain('openscad_analyze')
    expect(client.qualificationEvents.some(event => event.event === 'shadow-settled')).toBe(false)

    const shadowSettled = await client.waitForEvent(event => event.event === 'shadow-settled')
    expect(shadowSettled).toMatchObject({
      outcome: 'E_MCP_MANIFOLD_PLAN_DEADLINE',
      settlements: 1,
      snapshot: {
        activeWorkerEpoch: null,
        lastJoinedWorkerEpoch: 1,
        workersStarted: 1,
        workersJoined: 1,
        quarantined: false,
      },
    })

    // A joined child has no channel left for a late result. Give queued parent
    // work a turn, then verify the only durable record is the primary build.
    await new Promise(resolve => setTimeout(resolve, 100))
    const firstHistory = toolContent(await client.request('tools/call', {
      name: 'openscad_build_history',
      arguments: { limit: 10 },
    })) as { builds: Array<{ status: string; execution: { semantic_program_version: string } }> }
    expect(firstHistory.builds).toEqual([
      expect.objectContaining({
        status: 'succeeded',
        execution: expect.objectContaining({ semantic_program_version: 'legacy-direct-evaluator-v1' }),
      }),
    ])

    const secondAnalyze = await client.request('tools/call', {
      name: 'openscad_analyze',
      arguments: { source: 'cube(3);', quality: 'full' },
    })
    expect(toolContent(secondAnalyze)).toMatchObject({
      build: { status: 'succeeded' },
      analysis: { volume: 27, surface_area: 54 },
    })
    const finalHistory = toolContent(await client.request('tools/call', {
      name: 'openscad_build_history',
      arguments: { limit: 10 },
    })) as { builds: Array<{ status: string }> }
    expect(finalHistory.builds).toHaveLength(2)
    expect(finalHistory.builds.every(build => build.status === 'succeeded')).toBe(true)

    await new Promise(resolve => setTimeout(resolve, 50))
    expect(client.qualificationEvents.filter(event => event.event === 'shadow-settled')).toHaveLength(1)
    expect(client.responses.filter(response => response.id === firstAnalyze.id)).toHaveLength(1)
    expect(client.responses.filter(response => response.id === secondAnalyze.id)).toHaveLength(1)
    expect(client.invalidStdoutLines).toEqual([])

    await expect(client.close()).resolves.toEqual({ code: 0, signal: null })
  }, 30_000)
})
