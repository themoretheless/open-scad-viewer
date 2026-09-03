import { readFileSync } from 'node:fs'
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest'
import { InMemoryTransport, type JSONRPCMessage } from '@modelcontextprotocol/server'
import {
  OPENSCAD_2021_01_BUILTIN_MODULES,
  OPENSCAD_2021_01_SMOKE_FIXTURES,
} from '../src/core/openScad2021Contract'
import { createOpenScadMcpServer } from '../src/mcp/createServer'
import { DuckDbModelStore } from '../src/mcp/duckdbModelStore'
import type { OfficialOpenScadRuntimeService } from '../src/mcp/officialOpenScadRuntimeService'

const KNOWN_FILE_MODULE_GAPS = new Set<string>()
const IMPORT_SVG_FIXTURE = OPENSCAD_2021_01_SMOKE_FIXTURES.find(
  fixture => fixture.id === 'import-square-svg',
)!
const SURFACE_DAT_FIXTURE = OPENSCAD_2021_01_SMOKE_FIXTURES.find(
  fixture => fixture.id === 'heightmap-dat',
)!
const TEXT_FONT_FIXTURE = OPENSCAD_2021_01_SMOKE_FIXTURES.find(
  fixture => fixture.id === 'fontconfig-default-font',
)!
const BASIC_TTF_BASE64 = readFileSync(
  new URL('./fixtures/Basic-Regular.ttf', import.meta.url),
).toString('base64')

describe('independent OpenSCAD module execution through MCP', () => {
  let close: () => Promise<void>
  let request: (method: string, params?: Record<string, unknown>) => Promise<Record<string, unknown>>
  const upstream = {
    capabilities: vi.fn(async () => { throw new Error('upstream oracle must not be called') }),
    check: vi.fn(async () => { throw new Error('upstream oracle must not be called') }),
    export: vi.fn(async () => { throw new Error('upstream oracle must not be called') }),
    close: vi.fn(async () => undefined),
  } as unknown as OfficialOpenScadRuntimeService

  beforeAll(async () => {
    const store = await DuckDbModelStore.open(':memory:')
    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()
    const server = createOpenScadMcpServer({ store, officialRuntime: upstream })
    const pending = new Map<number, (value: Record<string, unknown>) => void>()
    let nextId = 1
    clientTransport.onmessage = message => {
      if (!('id' in message) || typeof message.id !== 'number'
        || (!('result' in message) && !('error' in message))) return
      pending.get(message.id)?.(message as Record<string, unknown>)
      pending.delete(message.id)
    }
    await clientTransport.start()
    await server.connect(serverTransport)
    request = async (method, params = {}) => {
      const id = nextId++
      const response = new Promise<Record<string, unknown>>((resolve, reject) => {
        const timeout = setTimeout(() => {
          pending.delete(id)
          reject(new Error(`Timed out waiting for ${method}`))
        }, 10_000)
        pending.set(id, value => {
          clearTimeout(timeout)
          resolve(value)
        })
      })
      await clientTransport.send({ jsonrpc: '2.0', id, method, params } as JSONRPCMessage)
      const envelope = await response
      if (envelope.error) throw new Error(JSON.stringify(envelope.error))
      return (envelope.result ?? {}) as Record<string, unknown>
    }
    await request('initialize', {
      protocolVersion: '2025-11-25',
      capabilities: {},
      clientInfo: { name: 'independent-module-conformance', version: '1.0.0' },
    })
    await clientTransport.send({ jsonrpc: '2.0', method: 'notifications/initialized' } as JSONRPCMessage)
    close = async () => {
      await server.close()
      await clientTransport.close()
      await store.close()
    }
  })

  afterAll(async () => close?.())

  it('executes every currently implemented stable module without calling upstream', async () => {
    expect([...KNOWN_FILE_MODULE_GAPS].sort()).toEqual([])

    for (const entry of OPENSCAD_2021_01_BUILTIN_MODULES) {
      if (KNOWN_FILE_MODULE_GAPS.has(entry.name)) continue
      const called = await request('tools/call', {
        name: 'openscad_independent_check',
        arguments: {
          source: entry.smoke.source,
          quality: 'full',
          ...(entry.name === 'surface'
            ? {
              files: [{
                path: SURFACE_DAT_FIXTURE.path,
                text: SURFACE_DAT_FIXTURE.provisioning === 'inline-text'
                  ? SURFACE_DAT_FIXTURE.text
                  : '',
              }],
            }
            : entry.name === 'import'
              ? {
                files: [{
                  path: IMPORT_SVG_FIXTURE.path,
                  text: IMPORT_SVG_FIXTURE.provisioning === 'inline-text'
                    ? IMPORT_SVG_FIXTURE.text
                    : '',
                }],
              }
              : entry.name === 'text'
                ? {
                  files: [{
                    path: TEXT_FONT_FIXTURE.path,
                    data_base64: BASIC_TTF_BASE64,
                  }],
                }
                : {}),
        },
      }) as {
        isError?: boolean
        structuredContent?: {
          check: {
            engine: {
              upstream_runtime_used: boolean
              complete_language_claim: boolean
            }
            mesh_count: number
            volume: number
          }
        }
      }

      expect(called.isError, entry.name).not.toBe(true)
      expect(called.structuredContent?.check.engine, entry.name).toMatchObject({
        upstream_runtime_used: false,
        complete_language_claim: false,
      })
      expect(called.structuredContent!.check.mesh_count, entry.name).toBeGreaterThan(0)
      expect(called.structuredContent!.check.volume, entry.name).toBeGreaterThanOrEqual(0)
    }

    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('keeps stable primitive and transform semantics through the independent MCP route', async () => {
    const called = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        source: `
          intersection() {
            rotate(a=90, [1,0,0]) {
              cube([2,"bad",4]);
              translate([3,0,0]) cube(1);
            }
            translate([0,-2,-1]) cube([5,2,3]);
          }
        `,
        quality: 'full',
      },
    }) as {
      isError?: boolean
      structuredContent?: {
        check: {
          engine: { upstream_runtime_used: boolean; complete_language_claim: boolean }
          mesh_count: number
          volume: number
          surface_area: number
          warnings: string[]
        }
      }
    }

    expect(called.isError).not.toBe(true)
    expect(called.structuredContent?.check).toMatchObject({
      engine: { upstream_runtime_used: false, complete_language_claim: false },
      mesh_count: 1,
      volume: 3,
      surface_area: 16,
      warnings: ['cube size was not a scalar or exact 3-component numeric vector; unit size is used'],
    })
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })
})
