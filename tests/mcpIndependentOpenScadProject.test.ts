import { readFileSync } from 'node:fs'
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest'
import { InMemoryTransport, type JSONRPCMessage } from '@modelcontextprotocol/server'
import { OPENSCAD_2021_01_SMOKE_FIXTURES } from '../src/core/openScad2021Contract'
import { createOpenScadMcpServer } from '../src/mcp/createServer'
import { DuckDbModelStore } from '../src/mcp/duckdbModelStore'
import type { OfficialOpenScadRuntimeService } from '../src/mcp/officialOpenScadRuntimeService'

const RGBA_PNG_2X2 = 'iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAGUlEQVR4nGNgAIL///8LMjQ0NCg6ODgYAgAyLgWhbk/ABgAAAABJRU5ErkJggg=='
const IMPORT_SVG_FIXTURE = OPENSCAD_2021_01_SMOKE_FIXTURES.find(
  fixture => fixture.id === 'import-square-svg',
)!
const LEGACY_TETRA_STL = readFileSync(new URL('./fixtures/legacy-tetra.stl', import.meta.url), 'utf8')
const LEGACY_TETRA_OFF = readFileSync(new URL('./fixtures/legacy-tetra.off', import.meta.url), 'utf8')
const LEGACY_SQUARE_DXF = readFileSync(new URL('./fixtures/legacy-square.dxf', import.meta.url), 'utf8')
const LEGACY_DIMENSIONED_DXF = readFileSync(
  new URL('./fixtures/legacy-dimensioned.dxf', import.meta.url),
  'utf8',
)

describe('independent OpenSCAD project execution through MCP', () => {
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
      clientInfo: { name: 'independent-project-conformance', version: '1.0.0' },
    })
    await clientTransport.send({ jsonrpc: '2.0', method: 'notifications/initialized' } as JSONRPCMessage)
    close = async () => {
      await server.close()
      await clientTransport.close()
      await store.close()
    }
  })

  afterAll(async () => close?.())

  it('executes include/use files without any upstream runtime call', async () => {
    const called = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        source: 'include <config.scad> use <lib/part.scad> make_part(edge);',
        files: [
          { path: 'config.scad', text: 'edge = 2;' },
          {
            path: 'lib/part.scad',
            text: 'sphere(100); module make_part(size) { cube(size); }',
          },
          { path: 'assets/unused.bin', data_base64: 'AAECAw==' },
        ],
        quality: 'full',
      },
    }) as {
      isError?: boolean
      structuredContent?: {
        check: {
          engine: {
            stage: string
            upstream_runtime_used: boolean
            complete_language_claim: boolean
          }
          mesh_count: number
          volume: number
          surface_area: number
        }
      }
    }

    expect(called.isError).not.toBe(true)
    expect(called.structuredContent?.check).toMatchObject({
      engine: {
        stage: 'development',
        upstream_runtime_used: false,
        complete_language_claim: false,
      },
      mesh_count: 1,
      volume: 8,
      surface_area: 24,
    })
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('accepts entrypoint source above the legacy model limit but within the advertised project limit', async () => {
    const called = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        source: `${' '.repeat(250_000)}cube(1);`,
        quality: 'full',
      },
    }) as {
      isError?: boolean
      structuredContent?: { check: { volume: number } }
    }

    expect(called.isError).not.toBe(true)
    expect(called.structuredContent?.check.volume).toBeCloseTo(1, 6)
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('loads module-relative DAT and PNG surface assets without any upstream runtime call', async () => {
    const dat = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        source: 'use <lib/terrain.scad> terrain();',
        files: [
          {
            path: 'lib/terrain.scad',
            text: 'module terrain() { surface(file = "../assets/map.dat", center = true, convexity = 4); }',
          },
          { path: 'assets/map.dat', text: '0 1\n2 3\n' },
        ],
        quality: 'full',
      },
    }) as {
      isError?: boolean
      structuredContent?: { check: { mesh_count: number; volume: number; triangle_count: number } }
    }
    expect(dat.isError).not.toBe(true)
    expect(dat.structuredContent?.check.mesh_count).toBe(1)
    expect(dat.structuredContent?.check.volume).toBeCloseTo(2.5, 5)
    expect(dat.structuredContent?.check.triangle_count).toBeGreaterThan(0)

    const png = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        source: 'surface(file = "assets/map.png", center = true, invert = true);',
        files: [{ path: 'assets/map.png', data_base64: RGBA_PNG_2X2 }],
        quality: 'full',
      },
    }) as {
      isError?: boolean
      structuredContent?: { check: { mesh_count: number; volume: number; triangle_count: number } }
    }
    expect(png.isError).not.toBe(true)
    expect(png.structuredContent?.check.mesh_count).toBe(1)
    expect(png.structuredContent?.check.volume).toBeGreaterThan(0)
    expect(png.structuredContent?.check.triangle_count).toBeGreaterThan(0)

    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('loads a module-relative SVG import without any upstream runtime call', async () => {
    if (IMPORT_SVG_FIXTURE.provisioning !== 'inline-text') throw new Error('Expected inline SVG fixture')
    const called = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        source: 'use <lib/part.scad> imported_part();',
        files: [
          {
            path: 'lib/part.scad',
            text: 'module imported_part() { linear_extrude(height = 2) import(file = "../assets/part.svg"); }',
          },
          { path: 'assets/part.svg', text: IMPORT_SVG_FIXTURE.text },
        ],
        quality: 'full',
      },
    }) as {
      isError?: boolean
      structuredContent?: { check: { mesh_count: number; volume: number; triangle_count: number } }
    }

    expect(called.isError).not.toBe(true)
    expect(called.structuredContent?.check.mesh_count).toBe(1)
    expect(called.structuredContent?.check.volume).toBeGreaterThan(0)
    expect(called.structuredContent?.check.triangle_count).toBeGreaterThan(0)
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('returns typed defining-source diagnostics for missing import assets', async () => {
    const called = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        source: 'use <lib/part.scad> imported_part();',
        files: [{
          path: 'lib/part.scad',
          text: 'module imported_part() {\n  import(file = "../assets/missing.svg");\n}',
        }],
        quality: 'full',
      },
    }) as {
      isError?: boolean
      structuredContent?: {
        error: {
          code: string
          line: number
          column: number
          details: Record<string, unknown>
        }
      }
    }

    expect(called).toMatchObject({
      isError: true,
      structuredContent: {
        error: {
          code: 'source_syntax_error',
          line: 2,
          column: 3,
          details: {
            diagnostic_code: 'E_IMPORT_FILE_MISSING',
            source_path: 'lib/part.scad',
            specifier: '../assets/missing.svg',
            asset_path: 'assets/missing.svg',
            format: 'svg',
          },
        },
      },
    })
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('returns typed defining-source diagnostics for missing text fonts', async () => {
    const called = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        source: 'use <lib/label.scad> label();',
        files: [{
          path: 'lib/label.scad',
          text: 'module label() {\n  text("A", font = "../fonts/missing.ttf");\n}',
        }],
        quality: 'full',
      },
    }) as {
      isError?: boolean
      structuredContent?: {
        error: {
          code: string
          line: number
          column: number
          details: Record<string, unknown>
        }
      }
    }

    expect(called).toMatchObject({
      isError: true,
      structuredContent: {
        error: {
          code: 'source_syntax_error',
          line: 2,
          column: 3,
          details: {
            diagnostic_code: 'E_TEXT_FONT_NOT_FOUND',
            source_path: 'lib/label.scad',
            font: '../fonts/missing.ttf',
            font_path: 'fonts/missing.ttf',
          },
        },
      },
    })
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('returns typed defining-source diagnostics for missing surface assets', async () => {
    const called = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        source: 'use <lib/terrain.scad> terrain();',
        files: [{
          path: 'lib/terrain.scad',
          text: 'module terrain() {\n  surface(file = "../assets/missing.dat");\n}',
        }],
        quality: 'full',
      },
    }) as {
      isError?: boolean
      structuredContent?: {
        error: {
          code: string
          line: number
          column: number
          details: Record<string, unknown>
        }
      }
    }

    expect(called).toMatchObject({
      isError: true,
      structuredContent: {
        error: {
          code: 'source_syntax_error',
          line: 2,
          column: 3,
          details: {
            diagnostic_code: 'E_SURFACE_FILE_MISSING',
            source_path: 'lib/terrain.scad',
            specifier: '../assets/missing.dat',
            asset_path: 'assets/missing.dat',
          },
        },
      },
    })
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('runs all nine compatibility symbols with dynamic forced-format VFS paths and no upstream call', async () => {
    const called = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        source: `
          stl_path = "assets/tetra.mesh";
          off_path = "assets/tetra.model";
          profile_path = "assets/profile.shape";
          query_path = "assets/dimensions.data";
          echo(
            dxf_dim(file = query_path, layer = "dims", name = "contract_dimension"),
            dxf_cross(file = query_path, layer = "contract_cross")
          );
          assign(edge = 1) translate([20, 0, 0]) cube(edge);
          module legacy_child() { child(0); }
          legacy_child() translate([22, 0, 0]) cube(1);
          union() {
            import_stl(file = stl_path);
            translate([3, 0, 0]) import_off(file = off_path);
            translate([6, 0, 0]) linear_extrude(height = 1)
              import_dxf(file = profile_path, layer = "profile");
            translate([10, 0, 0])
              dxf_linear_extrude(file = profile_path, layer = "profile", height = 1);
            translate([14, 0, 0])
              dxf_rotate_extrude(file = profile_path, layer = "profile", angle = 90, $fn = 12);
          }
        `,
        files: [
          { path: 'assets/tetra.mesh', text: LEGACY_TETRA_STL },
          { path: 'assets/tetra.model', text: LEGACY_TETRA_OFF },
          { path: 'assets/profile.shape', text: LEGACY_SQUARE_DXF },
          { path: 'assets/dimensions.data', text: LEGACY_DIMENSIONED_DXF },
        ],
        quality: 'full',
      },
    }) as {
      isError?: boolean
      structuredContent?: {
        check: {
          engine: { upstream_runtime_used: boolean }
          mesh_count: number
          volume: number
          warnings: string[]
        }
      }
    }

    expect(called.isError).not.toBe(true)
    expect(called.structuredContent?.check).toMatchObject({
      engine: { upstream_runtime_used: false },
      mesh_count: 1,
      warnings: expect.arrayContaining([
        'ECHO: 6, [3, 1]',
        'child() will be removed in future releases. Use children() instead.',
        'The import_stl() module will be removed in future releases. Use import() instead.',
        'The import_off() module will be removed in future releases. Use import() instead.',
        'The import_dxf() module will be removed in future releases. Use import() instead.',
        'The dxf_linear_extrude() module will be removed in future releases. Use linear_extrude() instead.',
        'The dxf_rotate_extrude() module will be removed in future releases. Use rotate_extrude() instead.',
      ]),
    })
    expect(called.structuredContent!.check.volume).toBeGreaterThan(6)
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })
})
