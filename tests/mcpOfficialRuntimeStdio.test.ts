import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { createHash } from 'node:crypto'
import { existsSync } from 'node:fs'
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
const runtimeManifest = join(repositoryRoot, '.open-scad-runtime', 'runtime-manifest.json')
const describeWithRuntime = existsSync(runtimeManifest) ? describe : describe.skip
const liveChildren = new Set<ChildProcessWithoutNullStreams>()

class OfficialStdioClient {
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
      }, 130_000)
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
        rejectClose(new Error(`Timed out closing official MCP server\nstderr:\n${this.stderrLines.join('\n')}`))
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
  if (envelope.isError) throw new Error(`Official MCP tool failed: ${JSON.stringify(envelope.structuredContent)}`)
  if (!envelope.structuredContent) throw new Error('Official MCP tool omitted structuredContent')
  return envelope.structuredContent
}

describeWithRuntime('official OpenSCAD over actual MCP stdio', () => {
  it('checks a multi-file program, exports text, serves the artifact, and joins on EOF', async () => {
    const client = new OfficialStdioClient()
    expect((await client.request('initialize', {
      protocolVersion: '2025-11-25',
      capabilities: {},
      clientInfo: { name: 'official-stdio-conformance', version: '1.0.0' },
    })).error).toBeUndefined()
    await client.notify('notifications/initialized')

    const status = toolContent(await client.request('tools/call', {
      name: 'openscad_official_status',
      arguments: {},
    })) as { official_runtime: { provider_role: string; available: boolean; runtime_version: string } }
    expect(status.official_runtime).toMatchObject({
      provider_role: 'qualification-oracle',
      available: true,
      runtime_version: '2026.09.01',
    })

    const checked = toolContent(await client.request('tools/call', {
      name: 'openscad_official_check',
      arguments: {
        source: 'include <lib/value.scad>\nfunction squared(x) = x * x; echo(squared(value)); cube(squared(value));',
        files: [{ path: 'lib/value.scad', text: 'value = 2;\n' }],
        hard_warnings: true,
      },
    })) as {
      check: {
        provider_role: string
        language_contract: { id: string }
        runtime_version: string
        csg_bytes: number
        logs: { stdout: string[]; stderr: string[] }
      }
    }
    expect(checked.check.provider_role).toBe('qualification-oracle')
    expect(checked.check.language_contract.id).toBe('openscad/stable-2021.01')
    expect(checked.check.runtime_version).toBe('2026.09.01')
    expect(checked.check.csg_bytes).toBeGreaterThan(0)
    expect([...checked.check.logs.stdout, ...checked.check.logs.stderr]).toContainEqual(expect.stringContaining('ECHO: 4'))

    const exported = toolContent(await client.request('tools/call', {
      name: 'openscad_official_export',
      arguments: {
        source: 'linear_extrude(height = 1) text("MCP", size = 10, font = "Basic:style=Regular");',
        files: [],
        format: 'stl',
        hard_warnings: true,
      },
    })) as { artifact: { resource_uri: string; byte_length: number; sha256: string } }
    expect(exported.artifact.byte_length).toBeGreaterThan(84)
    expect(exported.artifact.sha256).toMatch(/^[a-f0-9]{64}$/)

    const resource = result(await client.request('resources/read', {
      uri: exported.artifact.resource_uri,
    })) as { contents?: Array<{ blob?: string; mimeType?: string }> }
    expect(resource.contents?.[0]).toMatchObject({ mimeType: 'model/stl', blob: expect.any(String) })
    const artifactBytes = Buffer.from(resource.contents![0].blob!, 'base64')
    expect(artifactBytes.byteLength).toBe(exported.artifact.byte_length)
    expect(createHash('sha256').update(artifactBytes).digest('hex')).toBe(exported.artifact.sha256)
    expect(client.invalidStdoutLines).toEqual([])
    expect(await client.close()).toEqual({ code: 0, signal: null })
  }, 180_000)
})
