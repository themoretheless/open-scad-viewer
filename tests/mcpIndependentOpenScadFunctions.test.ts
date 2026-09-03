import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest'
import { InMemoryTransport, type JSONRPCMessage } from '@modelcontextprotocol/server'
import { OPENSCAD_2021_01_BUILTIN_FUNCTIONS } from '../src/core/openScad2021Contract'
import { createOpenScadMcpServer } from '../src/mcp/createServer'
import { DuckDbModelStore } from '../src/mcp/duckdbModelStore'
import type { OfficialOpenScadRuntimeService } from '../src/mcp/officialOpenScadRuntimeService'

describe('independent OpenSCAD function execution through MCP', () => {
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
      clientInfo: { name: 'independent-function-conformance', version: '1.0.0' },
    })
    await clientTransport.send({ jsonrpc: '2.0', method: 'notifications/initialized' } as JSONRPCMessage)
    close = async () => {
      await server.close()
      await clientTransport.close()
      await store.close()
    }
  })

  afterAll(async () => close?.())

  it('executes the canonical 38-function inventory without calling the upstream oracle', async () => {
    for (const entry of OPENSCAD_2021_01_BUILTIN_FUNCTIONS) {
      const called = await request('tools/call', {
        name: 'openscad_independent_check',
        arguments: { source: entry.smoke.source, quality: 'full' },
      }) as {
        isError?: boolean
        structuredContent?: {
          check: {
            engine: {
              id: string
              upstream_runtime_used: boolean
              complete_language_claim: boolean
              stable_function_inventory: number
              stable_module_inventory: number
            }
            mesh_count: number
            warnings: string[]
          }
        }
      }
      expect(called.isError, entry.name).not.toBe(true)
      expect(called.structuredContent?.check, entry.name).toMatchObject({
        engine: {
          id: 'open-scad-viewer/independent-2021.01-dev.1',
          upstream_runtime_used: false,
          complete_language_claim: false,
          stable_function_inventory: 38,
          stable_module_inventory: 35,
        },
        mesh_count: 1,
      })
      expect(called.structuredContent!.check.warnings.some(line => line.startsWith('ECHO:')), entry.name)
        .toBe(true)
    }

    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('executes stable wrappers, comprehensions and multi-iterator controls without upstream', async () => {
    const called = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        quality: 'full',
        source: `
          function f(a, b, c) = [a, b, c];
          echo(
            let(a = 1, b = a + 1) b,
            [for (i = [0:2]) if (i > 0) each [i, i * 10]],
            f(b = 2, 1, 3),
            pow(y = 3, 2),
            abs("x"),
            [3:1]
          );
          for (i = [0:1], j = [0:1])
            translate([i * 2, j * 2, 0]) cube(1);
          intersection_for (i = [0:1], j = [0:1])
            translate([i * 0.25, j * 0.25, 0]) cube(1);
        `,
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
      warnings: [
        'abs() argument 1 must be a number',
        'begin is greater than the end, but step is positive',
        'ECHO: 2, [1, 10, 2, 20], [1, 2, 3], 9, undef, [3 : 1 : 1]',
      ],
    })
    expect(called.structuredContent!.check.volume).toBeCloseTo(4, 6)
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('preserves stable lexical and dynamic scopes without upstream', async () => {
    const called = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        quality: 'full',
        source: `
          x = 2;
          function f() = [x, $foo];
          module m() {
            echo(f(), $children, $parent_modules, parent_module(0));
            children();
          }
          x = 3;
          let(x = 99, $foo = 7) m() cube(1);
        `,
      },
    }) as {
      isError?: boolean
      structuredContent?: {
        check: {
          engine: { upstream_runtime_used: boolean }
          mesh_count: number
          warnings: string[]
        }
      }
    }

    expect(called.isError).not.toBe(true)
    expect(called.structuredContent?.check).toMatchObject({
      engine: { upstream_runtime_used: false },
      mesh_count: 1,
      warnings: ['ECHO: [3, 7], 1, 1, "m"'],
    })
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('injects bounded animation, preview, and 2021 camera variables through MCP', async () => {
    const called = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        quality: 'preview',
        time: 0.625,
        source: 'echo($t, $preview, $vpt, $vpr, $vpd, $vpf); cube(1);',
      },
    }) as {
      isError?: boolean
      structuredContent?: {
        check: {
          engine: { upstream_runtime_used: boolean }
          warnings: string[]
        }
      }
    }

    expect(called.isError).not.toBe(true)
    expect(called.structuredContent?.check).toMatchObject({
      engine: { upstream_runtime_used: false },
      warnings: ['ECHO: 0.625, true, [0, 0, 0], [55, 0, 25], 140, 22.5'],
    })
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('executes the stable grammar, operators, recursion, and viewport modifiers through MCP', async () => {
    const called = await request('tools/call', {
      name: 'openscad_independent_check',
      arguments: {
        quality: 'full',
        source: `
          // line comment
          /* block comment */
          ;
          function factorial(n) = n <= 1 ? 1 : n * factorial(n - 1);
          callback = function(x) x + 1;
          module model(size = 1) { { cube([size, size, size]); } }
          values = [for (i = [0:2]) if (i != 1) each [i, -i]];
          cstyle = [for (i = 0; i < 3; i = i + 1) i];
          echo(
            factorial(4), callback(2), values, cstyle,
            let(a = 2) assert(a == 2) echo("nested") a ^ 3,
            [10, 20, 30][1], [7, 8, 9].z,
            !false && true || (3 % 2 == 1),
            8 / 2 + 3 - 1, 1 < 2, 2 >= 2
          );
          !union() {
            #model(size = 1,);
            *cube(99);
            %translate([5, 0, 0]) cube(1);
          }
        `,
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
      volume: 1,
    })
    expect(called.structuredContent!.check.warnings).toContain('ECHO: "nested"')
    expect(called.structuredContent!.check.warnings.some(line => line.startsWith('ECHO: 24, 3,')))
      .toBe(true)
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })
})
