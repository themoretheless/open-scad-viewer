import { existsSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  InMemoryTransport,
  type JSONRPCMessage,
} from '@modelcontextprotocol/server'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import {
  OPENSCAD_2021_01_CONTRACT,
  OPENSCAD_2021_01_BUILTIN_FUNCTIONS,
  OPENSCAD_2021_01_BUILTIN_MODULES,
  OPENSCAD_2021_01_SMOKE_FIXTURES,
  type OpenScadSmokeCase,
} from '../src/core/openScad2021Contract'
import {
  OfficialOpenScadRuntimeSupervisor,
  type OfficialOpenScadProjectFileInput,
} from '../src/mcp/officialOpenScadRuntimeService'
import {
  OFFICIAL_OPENSCAD_EXPORT_FORMATS,
  officialOpenScadMimeType,
  type OfficialOpenScadExportFormat,
} from '../src/mcp/officialOpenScadRuntimeProtocol'
import { createOpenScadMcpServer } from '../src/mcp/createServer'
import { DuckDbModelStore } from '../src/mcp/duckdbModelStore'

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const runtimeManifest = join(repositoryRoot, '.open-scad-runtime', 'runtime-manifest.json')
const realRuntimeInstalled = existsSync(runtimeManifest)
const describeWithRuntime = realRuntimeInstalled ? describe : describe.skip

const fixturesById = new Map(OPENSCAD_2021_01_SMOKE_FIXTURES.map(fixture => [fixture.id, fixture]))

function filesForSmoke(smoke: OpenScadSmokeCase): OfficialOpenScadProjectFileInput[] {
  return smoke.fixtureIds.flatMap(id => {
    const fixture = fixturesById.get(id)
    if (!fixture) throw new Error(`Unknown OpenSCAD smoke fixture ${id}`)
    if (fixture.provisioning === 'runtime-cache-required') return []
    if (fixture.provisioning !== 'inline-text' || fixture.text === undefined) {
      throw new Error(`Smoke fixture ${id} must be explicitly provisioned by this test`)
    }
    return [{ path: fixture.path, data: fixture.text }]
  })
}

function binaryTetrahedronStl(): Uint8Array {
  const triangles = [
    [[0, 0, 0], [0, 1, 0], [1, 0, 0]],
    [[0, 0, 0], [1, 0, 0], [0, 0, 1]],
    [[0, 0, 0], [0, 0, 1], [0, 1, 0]],
    [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
  ] as const
  const buffer = new ArrayBuffer(84 + triangles.length * 50)
  const view = new DataView(buffer)
  view.setUint32(80, triangles.length, true)
  let offset = 84
  for (const triangle of triangles) {
    // A zero normal is legal; OpenSCAD recomputes it from consistently wound vertices.
    offset += 12
    for (const point of triangle) {
      for (const coordinate of point) {
        view.setFloat32(offset, coordinate, true)
        offset += 4
      }
    }
    view.setUint16(offset, 0, true)
    offset += 2
  }
  return new Uint8Array(buffer)
}

async function connectedOfficialMcp(runtime: OfficialOpenScadRuntimeSupervisor) {
  const store = await DuckDbModelStore.open(':memory:')
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()
  const server = createOpenScadMcpServer({ store, officialRuntime: runtime })
  const pending = new Map<number, (value: Record<string, unknown>) => void>()
  let nextId = 1
  clientTransport.onmessage = message => {
    if (!('id' in message) || typeof message.id !== 'number'
      || (!('result' in message) && !('error' in message))) return
    const resolveResponse = pending.get(message.id)
    if (!resolveResponse) return
    pending.delete(message.id)
    resolveResponse(message as Record<string, unknown>)
  }
  await clientTransport.start()
  await server.connect(serverTransport)

  const request = async (method: string, params: Record<string, unknown> = {}) => {
    const id = nextId++
    const response = new Promise<Record<string, unknown>>((resolveResponse, rejectResponse) => {
      const timeout = setTimeout(() => {
        pending.delete(id)
        rejectResponse(new Error(`Timed out waiting for ${method}`))
      }, 130_000)
      pending.set(id, value => {
        clearTimeout(timeout)
        resolveResponse(value)
      })
    })
    await clientTransport.send({ jsonrpc: '2.0', id, method, params } as JSONRPCMessage)
    const envelope = await response
    if (envelope.error) throw new Error(`MCP ${method} failed: ${JSON.stringify(envelope.error)}`)
    return (envelope.result ?? {}) as Record<string, unknown>
  }

  await request('initialize', {
    protocolVersion: '2025-11-25',
    capabilities: {},
    clientInfo: { name: 'official-conformance', version: '1.0.0' },
  })
  await clientTransport.send({
    jsonrpc: '2.0',
    method: 'notifications/initialized',
  } as JSONRPCMessage)

  return {
    request,
    close: async () => {
      await server.close()
      await clientTransport.close()
      await store.close()
    },
  }
}

function projectFilesToWire(files: readonly OfficialOpenScadProjectFileInput[]) {
  return files.map(file => typeof file.data === 'string'
    ? { path: file.path, text: file.data }
    : { path: file.path, data_base64: Buffer.from(file.data).toString('base64') })
}

describeWithRuntime('official OpenSCAD 2021.01 runtime conformance', () => {
  const runtime = new OfficialOpenScadRuntimeSupervisor({ cacheRoot: join(repositoryRoot, '.open-scad-runtime') })
  let mcp: Awaited<ReturnType<typeof connectedOfficialMcp>>

  beforeAll(async () => {
    await expect(runtime.capabilities()).resolves.toMatchObject({
      available: true,
      expectedRuntimeVersion: '2026.09.01',
      runtimeVersion: '2026.09.01',
    })
    mcp = await connectedOfficialMcp(runtime)
  })

  afterAll(async () => {
    await mcp?.close()
    await runtime.close()
  })

  const exportThroughMcp = async (
    source: string,
    files: readonly OfficialOpenScadProjectFileInput[] = [],
    format: OfficialOpenScadExportFormat = 'stl',
  ) => {
    const called = await mcp.request('tools/call', {
      name: 'openscad_official_export',
      arguments: {
        source,
        files: projectFilesToWire(files),
        format,
        hard_warnings: true,
        timeout_ms: 120_000,
      },
    }) as {
      isError?: boolean
      structuredContent?: {
        export: {
          language_contract: { id: string; resource_uri: string }
          runtime_version: string
          format: OfficialOpenScadExportFormat
          source_sha256: string
          logs: { stdout: string[]; stderr: string[]; truncated: boolean }
        }
        artifact: {
          resource_uri: string
          byte_length: number
          sha256: string
          mime_type: string
          file_name: string
        }
      }
    }
    expect(called.isError, JSON.stringify(called.structuredContent)).not.toBe(true)
    expect(called.structuredContent).toBeDefined()
    return called.structuredContent!
  }

  it('renders every one of the 38 stable built-in function smoke programs', async () => {
    expect(OPENSCAD_2021_01_BUILTIN_FUNCTIONS).toHaveLength(38)
    for (const entry of OPENSCAD_2021_01_BUILTIN_FUNCTIONS) {
      const result = await exportThroughMcp(entry.smoke.source, filesForSmoke(entry.smoke))
      expect(result.export.language_contract).toMatchObject({
        id: OPENSCAD_2021_01_CONTRACT.id,
        resource_uri: 'openscad://language/openscad-2021.01',
      })
      expect(result.export.runtime_version, entry.name).toBe('2026.09.01')
      expect(result.artifact.byte_length, entry.name).toBeGreaterThan(84)
      expect([...result.export.logs.stdout, ...result.export.logs.stderr].some(line => line.startsWith('ECHO:')), entry.name)
        .toBe(true)
      expect([...result.export.logs.stdout, ...result.export.logs.stderr].some(line => /^(?:WARNING|ERROR):/.test(line)), entry.name)
        .toBe(false)
    }
  }, 600_000)

  it('renders every one of the 35 stable built-in module smoke programs', async () => {
    expect(OPENSCAD_2021_01_BUILTIN_MODULES).toHaveLength(35)
    for (const entry of OPENSCAD_2021_01_BUILTIN_MODULES) {
      const result = await exportThroughMcp(entry.smoke.source, filesForSmoke(entry.smoke))
      expect(result.export.language_contract.id, entry.name).toBe(OPENSCAD_2021_01_CONTRACT.id)
      expect(result.export.runtime_version, entry.name).toBe('2026.09.01')
      expect(result.artifact.byte_length, entry.name).toBeGreaterThan(84)
      expect([...result.export.logs.stdout, ...result.export.logs.stderr].some(line => /^(?:WARNING|ERROR):/.test(line)), entry.name)
        .toBe(false)
    }
  }, 600_000)

  it('evaluates multi-file language constructs and binary imports inside MEMFS', async () => {
    const source = `
include <lib/values.scad>
use <lib/shapes.scad>
function factorial(n, acc = 1) = n <= 1 ? acc : factorial(n - 1, acc * n);
values = [for (i = [0:2]) each [i, i + 10]];
checked = assert(len(values) == 6) echo(values, factorial(5), included_value) values;
assert(checked[5] == 12);
shape(checked[1]);
translate([3, 0, 0]) import("assets/tetra.stl");
`
    const result = await exportThroughMcp(source, [
      { path: 'lib/values.scad', data: 'included_value = 7;\n' },
      { path: 'lib/shapes.scad', data: 'module shape(size) { resize([size + 1, 2, 1]) cube(1); }\n' },
      { path: 'assets/tetra.stl', data: binaryTetrahedronStl() },
    ])

    expect(result.artifact.byte_length).toBeGreaterThan(84)
    expect([...result.export.logs.stdout, ...result.export.logs.stderr]).toContainEqual(expect.stringContaining('ECHO:'))
    expect([...result.export.logs.stdout, ...result.export.logs.stderr].some(line => /^(?:WARNING|ERROR):/.test(line))).toBe(false)

    const resource = await mcp.request('resources/read', { uri: result.artifact.resource_uri }) as {
      contents?: Array<{ blob?: string; mimeType?: string }>
    }
    expect(resource.contents?.[0]).toMatchObject({ mimeType: 'model/stl', blob: expect.any(String) })
    expect(Buffer.from(resource.contents![0].blob!, 'base64').byteLength).toBe(result.artifact.byte_length)
  }, 120_000)

  it('exports and serves every advertised official format with canonical identity', async () => {
    for (const format of OFFICIAL_OPENSCAD_EXPORT_FORMATS) {
      const source = format === 'dxf' || format === 'svg' ? 'square([2, 3]);' : 'cube([2, 3, 4]);'
      const result = await exportThroughMcp(source, [], format)
      expect(result.export.format).toBe(format)
      expect(result.artifact).toMatchObject({
        byte_length: expect.any(Number),
        mime_type: officialOpenScadMimeType(format),
        file_name: `model.${format}`,
      })
      expect(result.artifact.byte_length, format).toBeGreaterThan(0)

      const resource = await mcp.request('resources/read', { uri: result.artifact.resource_uri }) as {
        contents?: Array<{ blob?: string; mimeType?: string }>
      }
      expect(resource.contents?.[0]?.mimeType).toBe(officialOpenScadMimeType(format))
      const bytes = Buffer.from(resource.contents![0].blob!, 'base64')
      expect(bytes.byteLength).toBe(result.artifact.byte_length)
      expect(createHash('sha256').update(bytes).digest('hex')).toBe(result.artifact.sha256)
    }
  }, 180_000)
})
