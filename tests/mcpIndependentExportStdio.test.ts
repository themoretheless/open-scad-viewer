import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { createHash } from 'node:crypto'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { afterEach, describe, expect, it } from 'vitest'

interface JsonRpcResponse {
  jsonrpc: '2.0'
  id: number
  result?: Record<string, unknown>
  error?: { code: number; message: string; data?: unknown }
}

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const serverPath = join(repositoryRoot, 'src', 'mcp', 'server.ts')
const liveChildren = new Set<ChildProcessWithoutNullStreams>()

class IndependentExportStdioClient {
  readonly child: ChildProcessWithoutNullStreams
  readonly invalidStdoutLines: string[] = []
  readonly stderrLines: string[] = []
  private readonly waiters = new Map<number, (response: JsonRpcResponse) => void>()
  private stdoutBuffer = ''
  private stderrBuffer = ''
  private nextId = 1

  constructor() {
    this.child = spawn(process.execPath, ['--import', 'tsx', serverPath, '--memory'], {
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
          this.waiters.get(message.id)?.(message)
          this.waiters.delete(message.id)
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
      if (line) this.stderrLines.push(line)
    }
  }

  async request(method: string, params: Record<string, unknown> = {}): Promise<JsonRpcResponse> {
    const id = this.nextId++
    const response = new Promise<JsonRpcResponse>((resolveResponse, rejectResponse) => {
      const timeout = setTimeout(() => {
        this.waiters.delete(id)
        rejectResponse(new Error(
          `Timed out waiting for ${method}\nstderr:\n${this.stderrLines.join('\n')}`,
        ))
      }, 30_000)
      this.waiters.set(id, value => {
        clearTimeout(timeout)
        resolveResponse(value)
      })
    })
    await new Promise<void>((resolveWrite, rejectWrite) => {
      this.child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`, error => {
        if (error) rejectWrite(error)
        else resolveWrite()
      })
    })
    return await response
  }

  async notify(method: string): Promise<void> {
    await new Promise<void>((resolveWrite, rejectWrite) => {
      this.child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', method })}\n`, error => {
        if (error) rejectWrite(error)
        else resolveWrite()
      })
    })
  }

  async close(): Promise<{ code: number | null; signal: NodeJS.Signals | null }> {
    if (this.child.exitCode !== null || this.child.signalCode !== null) {
      return { code: this.child.exitCode, signal: this.child.signalCode }
    }
    const closed = new Promise<{ code: number | null; signal: NodeJS.Signals | null }>((resolveClose, rejectClose) => {
      const timeout = setTimeout(() => {
        this.child.kill()
        rejectClose(new Error(`Timed out closing MCP server\nstderr:\n${this.stderrLines.join('\n')}`))
      }, 15_000)
      this.child.once('close', (code, signal) => {
        clearTimeout(timeout)
        resolveClose({ code, signal })
      })
    })
    this.child.stdin.end()
    return await closed
  }
}

afterEach(async () => {
  await Promise.all([...liveChildren].map(async child => {
    if (child.exitCode === null && child.signalCode === null) child.kill()
    if (child.exitCode === null && child.signalCode === null) {
      await new Promise<void>(resolveClose => child.once('close', () => resolveClose()))
    }
  }))
})

function result(response: JsonRpcResponse): Record<string, unknown> {
  if (response.error) throw new Error(`${response.error.code}: ${response.error.message}`)
  return response.result ?? {}
}

function toolContent(response: JsonRpcResponse): Record<string, unknown> {
  const envelope = result(response) as { isError?: boolean; structuredContent?: Record<string, unknown> }
  if (envelope.isError) throw new Error(`Independent export failed: ${JSON.stringify(envelope.structuredContent)}`)
  if (!envelope.structuredContent) throw new Error('Independent export omitted structuredContent')
  return envelope.structuredContent
}

describe('independent OpenSCAD export over actual MCP stdio', () => {
  it('lists the tool and serves an integrity-matched session resource', async () => {
    const client = new IndependentExportStdioClient()
    expect((await client.request('initialize', {
      protocolVersion: '2025-11-25',
      capabilities: {},
      clientInfo: { name: 'independent-export-stdio', version: '1.0.0' },
    })).error).toBeUndefined()
    await client.notify('notifications/initialized')

    const tools = result(await client.request('tools/list')) as { tools: Array<{ name: string }> }
    expect(tools.tools).toContainEqual(expect.objectContaining({ name: 'openscad_independent_export' }))

    const exported = toolContent(await client.request('tools/call', {
      name: 'openscad_independent_export',
      arguments: {
        source: 'cube([1, 2, 3]);',
        format: 'stl',
        file_name: 'stdio-cube',
        max_bytes: 4096,
      },
    })) as {
      export: { engine: { upstream_runtime_used: boolean }; quality: string; metrics: { volume: number } }
      artifact: { resource_uri: string; id: string; sha256: string; byte_length: number }
    }
    expect(exported.export).toMatchObject({
      engine: { upstream_runtime_used: false },
      quality: 'full',
      metrics: { volume: 6 },
    })
    expect(exported.artifact.resource_uri).toBe(`openscad://independent-artifacts/${exported.artifact.id}`)

    const resource = result(await client.request('resources/read', {
      uri: exported.artifact.resource_uri,
    })) as { contents: Array<{ blob: string; mimeType: string }> }
    const bytes = Buffer.from(resource.contents[0].blob, 'base64')
    const digest = createHash('sha256').update(bytes).digest('hex')
    expect(resource.contents[0].mimeType).toBe('model/stl')
    expect(bytes.byteLength).toBe(exported.artifact.byte_length)
    expect(digest).toBe(exported.artifact.sha256)
    expect(digest).toBe(exported.artifact.id)

    const resources = result(await client.request('resources/list')) as { resources: Array<{ uri: string }> }
    expect(resources.resources).toContainEqual(expect.objectContaining({ uri: exported.artifact.resource_uri }))
    const templates = result(await client.request('resources/templates/list')) as {
      resourceTemplates: Array<{ uriTemplate: string }>
    }
    expect(templates.resourceTemplates).toContainEqual(expect.objectContaining({
      uriTemplate: 'openscad://independent-artifacts/{sha256}',
    }))
    expect(client.invalidStdoutLines).toEqual([])
    expect(await client.close()).toEqual({ code: 0, signal: null })
  }, 45_000)
})
