import { spawn } from 'node:child_process'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { InMemoryTransport } from '@modelcontextprotocol/server'
import { describe, expect, it, vi } from 'vitest'
import { BoundedTransport } from '../src/mcp/boundedTransport'
import { DuckDbModelStore } from '../src/mcp/duckdbModelStore'
import {
  parseMcpCliOptions,
  runMcpServer,
  type RunMcpServerDependencies,
} from '../src/mcp/server'

const testDirectory = dirname(fileURLToPath(import.meta.url))
const repositoryRoot = resolve(testDirectory, '..')
const serverPath = join(repositoryRoot, 'src', 'mcp', 'server.ts')

interface StdioRun {
  code: number | null
  signal: NodeJS.Signals | null
  stdout: string
  stderr: string
}

interface StdioBurstRun extends StdioRun {
  responseIds: number[]
  busyResponses: number
}

function runStdioLifecycle(): Promise<StdioRun> {
  const child = spawn(process.execPath, ['--import', 'tsx', serverPath, '--memory'], {
    cwd: repositoryRoot,
    env: process.env,
    stdio: ['pipe', 'pipe', 'pipe'],
  })
  child.stdout.setEncoding('utf8')
  child.stderr.setEncoding('utf8')

  let stdout = ''
  let stderr = ''
  let initialized = false
  let toolsRequested = false
  let analyzeRequested = false
  let analyzeSeen = false
  let stdinEnded = false

  return new Promise((resolveRun, rejectRun) => {
    let settled = false
    const timer = setTimeout(() => {
      if (settled) return
      settled = true
      child.kill()
      rejectRun(new Error(`Timed out waiting for MCP stdio shutdown\nstdout:\n${stdout}\nstderr:\n${stderr}`))
    }, 10_000)

    const reject = (error: Error) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      if (child.exitCode === null && child.signalCode === null) child.kill()
      rejectRun(error)
    }

    child.stdout.on('data', chunk => {
      stdout += chunk
      for (const line of stdout.split(/\r?\n/).filter(Boolean)) {
        try {
          const message = JSON.parse(line) as { id?: unknown; result?: unknown; error?: unknown }
          if (message.id === 1 && ('result' in message || 'error' in message) && !toolsRequested) {
            initialized = true
            toolsRequested = true
            child.stdin.write(`${JSON.stringify({
              jsonrpc: '2.0',
              method: 'notifications/initialized',
            })}\n${JSON.stringify({
              jsonrpc: '2.0',
              id: 2,
              method: 'tools/list',
              params: {},
            })}\n`)
          } else if (message.id === 2 && ('result' in message || 'error' in message) && !analyzeRequested) {
            analyzeRequested = true
            child.stdin.write(`${JSON.stringify({
              jsonrpc: '2.0',
              id: 3,
              method: 'tools/call',
              params: {
                name: 'openscad_analyze',
                arguments: { source: 'cube(2);', quality: 'full' },
              },
            })}\n`)
          } else if (message.id === 3 && ('result' in message || 'error' in message)) {
            analyzeSeen = true
          }
        } catch {
          // The final assertion reports any non-JSON stdout with the complete output.
        }
      }
      if (analyzeSeen && !stdinEnded) {
        stdinEnded = true
        child.stdin.end()
      }
    })
    child.stderr.on('data', chunk => {
      stderr += chunk
    })
    child.once('error', error => reject(error))
    child.once('close', (code, signal) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      if (!initialized || !analyzeSeen) {
        rejectRun(new Error(`MCP process exited before the stdio smoke completed\nstdout:\n${stdout}\nstderr:\n${stderr}`))
        return
      }
      resolveRun({ code, signal, stdout, stderr })
    })

    child.stdin.write(`${JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: {
        protocolVersion: '2025-11-25',
        capabilities: {},
        clientInfo: { name: 'open-scad-viewer-cli-test', version: '1.0.0' },
      },
    })}\n`, error => {
      if (error) reject(error)
    })
  })
}

function runStdioBurst(requestCount = 24): Promise<StdioBurstRun> {
  const child = spawn(process.execPath, ['--import', 'tsx', serverPath, '--memory'], {
    cwd: repositoryRoot,
    env: process.env,
    stdio: ['pipe', 'pipe', 'pipe'],
  })
  child.stdout.setEncoding('utf8')
  child.stderr.setEncoding('utf8')

  let stdout = ''
  let stderr = ''
  let lineBuffer = ''
  let burstSent = false
  let stdinEnded = false
  let busyResponses = 0
  const responseIds = new Set<number>()

  return new Promise((resolveRun, rejectRun) => {
    let settled = false
    const timer = setTimeout(() => {
      if (settled) return
      settled = true
      child.kill()
      rejectRun(new Error(`Timed out waiting for MCP burst responses\nstdout:\n${stdout}\nstderr:\n${stderr}`))
    }, 10_000)

    const reject = (error: Error) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      if (child.exitCode === null && child.signalCode === null) child.kill()
      rejectRun(error)
    }

    child.stdout.on('data', chunk => {
      stdout += chunk
      lineBuffer += chunk
      let newline = lineBuffer.indexOf('\n')
      while (newline >= 0) {
        const line = lineBuffer.slice(0, newline).trim()
        lineBuffer = lineBuffer.slice(newline + 1)
        newline = lineBuffer.indexOf('\n')
        if (!line) continue
        try {
          const message = JSON.parse(line) as {
            id?: unknown
            result?: unknown
            error?: { data?: { code?: unknown } }
          }
          if (message.id === 1 && ('result' in message || 'error' in message) && !burstSent) {
            burstSent = true
            const messages = [
              { jsonrpc: '2.0', method: 'notifications/initialized' },
              ...Array.from({ length: requestCount }, (_, index) => ({
                jsonrpc: '2.0',
                id: 100 + index,
                method: 'tools/list',
                params: {},
              })),
            ]
            child.stdin.write(`${messages.map(message => JSON.stringify(message)).join('\n')}\n`, error => {
              if (error) reject(error)
            })
          } else if (typeof message.id === 'number'
            && message.id >= 100 && message.id < 100 + requestCount) {
            if (!responseIds.has(message.id) && message.error?.data?.code === 'server_busy') {
              busyResponses++
            }
            responseIds.add(message.id)
          }
        } catch {
          // The final assertion reports any non-JSON stdout with the complete output.
        }
      }
      if (responseIds.size === requestCount && !stdinEnded) {
        stdinEnded = true
        child.stdin.end()
      }
    })
    child.stderr.on('data', chunk => {
      stderr += chunk
    })
    child.once('error', error => reject(error))
    child.once('close', (code, signal) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      if (responseIds.size !== requestCount) {
        rejectRun(new Error(`MCP process exited before completing the burst\nstdout:\n${stdout}\nstderr:\n${stderr}`))
        return
      }
      resolveRun({
        code,
        signal,
        stdout,
        stderr,
        responseIds: [...responseIds].sort((left, right) => left - right),
        busyResponses,
      })
    })

    child.stdin.write(`${JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: {
        protocolVersion: '2025-11-25',
        capabilities: {},
        clientInfo: { name: 'open-scad-viewer-burst-test', version: '1.0.0' },
      },
    })}\n`, error => {
      if (error) reject(error)
    })
  })
}

describe('MCP CLI', () => {
  it('parses default, environment, explicit, in-memory, and help options', () => {
    expect(parseMcpCliOptions([], {}, repositoryRoot)).toEqual({
      databasePath: join(repositoryRoot, '.open-scad-viewer.duckdb'),
      showHelp: false,
    })
    expect(parseMcpCliOptions([], {
      OPENSCAD_VIEWER_DUCKDB: 'state/catalog.duckdb',
    }, repositoryRoot).databasePath).toBe(join(repositoryRoot, 'state', 'catalog.duckdb'))
    expect(parseMcpCliOptions(['--db', 'data/catalog.duckdb'], {}, repositoryRoot).databasePath)
      .toBe(join(repositoryRoot, 'data', 'catalog.duckdb'))
    expect(parseMcpCliOptions(['--db', ':memory:'], {}, repositoryRoot).databasePath).toBe(':memory:')
    expect(parseMcpCliOptions(['--memory'], {
      OPENSCAD_VIEWER_DUCKDB: 'persistent.duckdb',
    }, repositoryRoot).databasePath).toBe(':memory:')
    expect(parseMcpCliOptions(['--help'], {}, repositoryRoot).showHelp).toBe(true)
    expect(() => parseMcpCliOptions(['--db'], {}, repositoryRoot)).toThrow('--db requires a file path')
    expect(() => parseMcpCliOptions(['--unknown'], {}, repositoryRoot)).toThrow('Unknown argument')
  })

  it('keeps stdout JSON-RPC-clean and shuts down after stdin EOF', async () => {
    const run = await runStdioLifecycle()
    const lines = run.stdout.split(/\r?\n/).filter(Boolean)
    const messages = lines.map(line => JSON.parse(line) as {
      id?: unknown
      method?: string
      result?: { serverInfo?: { name?: string } }
    })
    const responses = messages.filter(message => typeof message.id === 'number')

    expect(run.code).toBe(0)
    expect(run.signal).toBeNull()
    expect(responses).toHaveLength(3)
    expect(responses[0]).toMatchObject({
      id: 1,
      result: { serverInfo: { name: 'open-scad-viewer' } },
    })
    expect(responses[1]).toMatchObject({ id: 2 })
    expect(responses[2]).toMatchObject({
      id: 3,
      result: {
        structuredContent: {
          build: { status: 'succeeded' },
          analysis: { triangle_count: 12, volume: 8 },
        },
      },
    })
    expect(messages).toContainEqual(expect.objectContaining({
      method: 'notifications/resources/list_changed',
    }))
    expect(run.stderr).toContain('OpenSCAD Viewer MCP server ready (DuckDB: :memory:)')
  }, 15_000)

  it('survives a 24-request stdio burst and replies to every admitted or rejected id', async () => {
    const run = await runStdioBurst()

    expect(run.code).toBe(0)
    expect(run.signal).toBeNull()
    expect(run.responseIds).toEqual(Array.from({ length: 24 }, (_, index) => 100 + index))
    expect(run.busyResponses).toBeGreaterThan(0)
    expect(run.stderr).not.toContain('MCP transport error')
  }, 15_000)

  it('closes the stdio handle and geometry runtime before the store, exactly once', async () => {
    const events: string[] = []
    const store = await DuckDbModelStore.open(':memory:')
    const originalStoreClose = store.close.bind(store)
    const storeClose = vi.spyOn(store, 'close').mockImplementation(async () => {
      events.push('store')
      await originalStoreClose()
    })
    const runtime: ReturnType<RunMcpServerDependencies['createGeometryRuntime']> = {
      capabilities: vi.fn(async () => { throw new Error('not used') }),
      build: vi.fn(async () => { throw new Error('not used') }),
      close: vi.fn(async () => { events.push('runtime') }),
    }
    const officialRuntime = {
      capabilities: vi.fn(async () => { throw new Error('not used') }),
      check: vi.fn(async () => { throw new Error('not used') }),
      export: vi.fn(async () => { throw new Error('not used') }),
      close: vi.fn(async () => { events.push('official-runtime') }),
    } satisfies ReturnType<RunMcpServerDependencies['createOfficialRuntime']>
    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()
    const transport = new BoundedTransport(serverTransport)
    const handleClose = vi.fn(async () => { events.push('handle') })
    const startServer = vi.fn(() => ({ close: handleClose })) as unknown as
      RunMcpServerDependencies['startServer']

    const running = await runMcpServer({ databasePath: ':memory:', showHelp: false }, {
      openStore: async () => store,
      createGeometryRuntime: () => runtime,
      createOfficialRuntime: () => officialRuntime,
      createTransport: () => transport,
      startServer,
    })
    await Promise.all([running.close(), running.close()])
    await clientTransport.close()

    expect(handleClose).toHaveBeenCalledTimes(1)
    expect(runtime.close).toHaveBeenCalledTimes(1)
    expect(officialRuntime.close).toHaveBeenCalledTimes(1)
    expect(storeClose).toHaveBeenCalledTimes(1)
    expect(events.slice(0, 3).sort()).toEqual(['handle', 'official-runtime', 'runtime'])
    expect(events[3]).toBe('store')
  })

  it('closes transport, geometry runtime, and store when bootstrap fails', async () => {
    const store = await DuckDbModelStore.open(':memory:')
    const originalStoreClose = store.close.bind(store)
    const storeClose = vi.spyOn(store, 'close').mockImplementation(originalStoreClose)
    const runtime: ReturnType<RunMcpServerDependencies['createGeometryRuntime']> = {
      capabilities: vi.fn(async () => { throw new Error('not used') }),
      build: vi.fn(async () => { throw new Error('not used') }),
      close: vi.fn(async () => undefined),
    }
    const [, serverTransport] = InMemoryTransport.createLinkedPair()
    const transport = new BoundedTransport(serverTransport)
    const transportClose = vi.spyOn(transport, 'close')
    const startupError = new Error('synthetic bootstrap failure')
    const startServer = vi.fn(() => { throw startupError }) as unknown as
      RunMcpServerDependencies['startServer']

    await expect(runMcpServer({ databasePath: ':memory:', showHelp: false }, {
      openStore: async () => store,
      createGeometryRuntime: () => runtime,
      createTransport: () => transport,
      startServer,
    })).rejects.toBe(startupError)

    expect(transportClose).toHaveBeenCalledTimes(1)
    expect(runtime.close).toHaveBeenCalledTimes(1)
    expect(storeClose).toHaveBeenCalledTimes(1)
  })

  it('closes the store when geometry runtime construction fails', async () => {
    const store = await DuckDbModelStore.open(':memory:')
    const originalStoreClose = store.close.bind(store)
    const storeClose = vi.spyOn(store, 'close').mockImplementation(originalStoreClose)
    const startupError = new Error('synthetic geometry runtime construction failure')

    await expect(runMcpServer({ databasePath: ':memory:', showHelp: false }, {
      openStore: async () => store,
      createGeometryRuntime: () => { throw startupError },
    })).rejects.toBe(startupError)

    expect(storeClose).toHaveBeenCalledTimes(1)
  })

  it('closes the geometry runtime and store when official runtime construction fails', async () => {
    const store = await DuckDbModelStore.open(':memory:')
    const originalStoreClose = store.close.bind(store)
    const storeClose = vi.spyOn(store, 'close').mockImplementation(originalStoreClose)
    const runtime: ReturnType<RunMcpServerDependencies['createGeometryRuntime']> = {
      capabilities: vi.fn(async () => { throw new Error('not used') }),
      build: vi.fn(async () => { throw new Error('not used') }),
      close: vi.fn(async () => undefined),
    }
    const startupError = new Error('synthetic official runtime construction failure')

    await expect(runMcpServer({ databasePath: ':memory:', showHelp: false }, {
      openStore: async () => store,
      createGeometryRuntime: () => runtime,
      createOfficialRuntime: () => { throw startupError },
    })).rejects.toBe(startupError)

    expect(runtime.close).toHaveBeenCalledTimes(1)
    expect(storeClose).toHaveBeenCalledTimes(1)
  })

  it('still closes the store when both host shutdown operations fail', async () => {
    const store = await DuckDbModelStore.open(':memory:')
    const originalStoreClose = store.close.bind(store)
    const storeClose = vi.spyOn(store, 'close').mockImplementation(originalStoreClose)
    const runtimeError = new Error('synthetic runtime close failure')
    const handleError = new Error('synthetic handle close failure')
    const runtime: ReturnType<RunMcpServerDependencies['createGeometryRuntime']> = {
      capabilities: vi.fn(async () => { throw new Error('not used') }),
      build: vi.fn(async () => { throw new Error('not used') }),
      close: vi.fn(async () => { throw runtimeError }),
    }
    const [, serverTransport] = InMemoryTransport.createLinkedPair()
    const transport = new BoundedTransport(serverTransport)
    const handleClose = vi.fn(async () => { throw handleError })
    const startServer = vi.fn(() => ({ close: handleClose })) as unknown as
      RunMcpServerDependencies['startServer']
    const running = await runMcpServer({ databasePath: ':memory:', showHelp: false }, {
      openStore: async () => store,
      createGeometryRuntime: () => runtime,
      createTransport: () => transport,
      startServer,
    })

    const error = await running.close().catch(reason => reason) as AggregateError

    expect(error).toBeInstanceOf(AggregateError)
    expect([...error.errors]).toEqual([handleError, runtimeError])
    expect(handleClose).toHaveBeenCalledTimes(1)
    expect(runtime.close).toHaveBeenCalledTimes(1)
    expect(storeClose).toHaveBeenCalledTimes(1)
  })
})
