import { describe, expect, it, vi } from 'vitest'
import { MeshDrawBundle, type BundleMesh } from '../src/services/meshDrawBundle'

function harness() {
  const commands: unknown[][] = []
  const encoder = () => ({
    setPipeline: (p: unknown) => commands.push(['pipeline', p]),
    setBindGroup: (n: number, p: unknown) => commands.push(['bind', n, p]),
    setVertexBuffer: (n: number, p: unknown) => commands.push(['vertex', n, p]),
    setIndexBuffer: (p: unknown) => commands.push(['index', p]),
    drawIndexed: (n: number) => commands.push(['draw', n]),
    finish: () => ({ commands: commands.splice(0) }),
  })
  const create = vi.fn(encoder)
  const execute = vi.fn()
  const pass = { ...encoder(), executeBundles: execute } as unknown as GPURenderPassEncoder
  const device = { createRenderBundleEncoder: create } as unknown as GPUDevice
  const pipeline = {} as GPURenderPipeline, scene = {} as GPUBindGroup
  const meshes = Array.from({ length: 32 }, () => ({ vb: {}, ib: {}, ic: 3, bg: {}, edgeIB: {}, edgeIC: 6 } as BundleMesh))
  const cache = new MeshDrawBundle()
  const draw = (list = meshes, edges = false) => cache.draw(pass, device, 'bgra8unorm', pipeline, scene, list, edges)
  return { cache, draw, meshes, create, execute, commands }
}

describe('retained draw commands', () => {
  it('replays native commands without encoding the same draw calls again', () => {
    const h = harness()
    h.draw()
    const bundle = h.execute.mock.calls[0][0][0]
    expect(bundle.commands.filter((c: unknown[]) => c[0] === 'draw')).toHaveLength(32)
    expect(bundle.commands.filter((c: unknown[]) => c[0] === 'bind' && c[1] === 1).map((c: unknown[]) => c[2])).toEqual(h.meshes.map(m => m.bg))
    h.draw()
    expect(h.create).toHaveBeenCalledTimes(1)
    expect(h.execute.mock.calls[1][0][0]).toBe(bundle)
  })

  it('rebuilds for reordered, hidden, replaced, or warmed edge geometry', () => {
    const h = harness()
    h.draw(h.meshes, true)
    h.meshes.reverse(); h.draw(h.meshes, true)
    h.meshes.pop(); h.draw(h.meshes, true)
    h.meshes[0] = { ...h.meshes[0], bg: {} as GPUBindGroup }; h.draw(h.meshes, true)
    h.meshes[0].edgeIB = {} as GPUBuffer; h.draw(h.meshes, true)
    h.meshes[0].edgeIC = 12; h.draw(h.meshes, true)
    expect(h.create).toHaveBeenCalledTimes(6)
    const last = h.execute.mock.calls.at(-1)![0][0]
    expect(last.commands.find((c: unknown[]) => c[0] === 'draw')).toEqual(['draw', 12])
  })

  it('clears old references on empty lists and after scene/device teardown', () => {
    const h = harness()
    h.draw(); h.draw([])
    expect(h.execute).toHaveBeenCalledTimes(1)
    h.draw(); h.cache.clear(); h.draw()
    expect(h.create).toHaveBeenCalledTimes(3)
  })

  it('encodes small selections directly, including edge index counts', () => {
    const h = harness()
    h.draw(h.meshes.slice(0, 1), true)
    expect(h.create).not.toHaveBeenCalled()
    expect(h.execute).not.toHaveBeenCalled()
    expect(h.commands.at(-1)).toEqual(['draw', 6])
  })
})
