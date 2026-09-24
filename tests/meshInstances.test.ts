import { describe, expect, it, vi, afterEach } from 'vitest'
import { MeshInstances, type InstanceMesh } from '../src/services/meshInstances'
import { identity } from '../src/services/math3d'

function harness() {
  vi.stubGlobal('GPUBufferUsage', { STORAGE: 1, COPY_DST: 2, VERTEX: 4 })
  const buffers: { destroy: ReturnType<typeof vi.fn> }[] = []
  const writes: Float32Array[] = []
  let failWrite = false
  const device = {
    limits: { maxStorageBufferBindingSize: 1 << 24 },
    createBuffer: vi.fn(() => { const buffer = { destroy: vi.fn() }; buffers.push(buffer); return buffer }),
    createBindGroup: vi.fn(() => ({})),
    queue: { writeBuffer: vi.fn((_buffer, _offset, data: Float32Array, start: number, count: number) => {
      if (failWrite) { failWrite = false; throw new Error('write failed') }
      writes.push(data.slice(start, start + count))
    }) },
  }
  const pass = { setPipeline: vi.fn(), setBindGroup: vi.fn(), setVertexBuffer: vi.fn(), setIndexBuffer: vi.fn(), drawIndexed: vi.fn() }
  const geometry = { vb: {} as GPUBuffer, ib: {} as GPUBuffer, edgeIB: {} as GPUBuffer }
  const meshes: InstanceMesh[] = Array.from({ length: 32 }, (_, index) => {
    const transform = identity(); transform[3] = index * 3
    const inverseTransform = identity(); inverseTransform[3] = -index * 3
    return { ...geometry, ic: 3, edgeIC: 6, bg: {} as GPUBindGroup, transform, inverseTransform,
      color: [index / 32, 0.2, 0.3, 1], styleAlpha: 1, styleSelected: 0, styleEdge: 0.7, styleHovered: 0 }
  })
  const cache = new MeshInstances()
  const draw = (list = meshes, edges = false) => cache.draw(pass as unknown as GPURenderPassEncoder, device as unknown as GPUDevice, {} as GPUBindGroupLayout, {} as GPURenderPipeline, {} as GPUBindGroup, list, edges)
  return { cache, draw, meshes, pass, device, buffers, writes, failNextWrite: () => { failWrite = true } }
}

afterEach(() => vi.unstubAllGlobals())

describe('instanced geometry commands', () => {
  it('merges shared geometry and retains exact per-object matrix and style bytes', () => {
    const h = harness()
    expect(h.draw()).toBe(true)
    expect(h.pass.drawIndexed).toHaveBeenCalledExactlyOnceWith(3, 32, 0, 0, 0)
    expect(h.writes[0]).toHaveLength(32 * 44)
    const second = h.writes[0].slice(44, 88)
    expect(second[12]).toBe(3)
    expect(second[16 + 3]).toBe(-3)
    expect([...second.slice(32, 40)]).toEqual([...new Float32Array([1 / 32, 0.2, 0.3, 1, 1, 0, 0.7, 0])])
    // The morph weight stays at rest for instances.
    expect([...second.slice(40, 44)]).toEqual([0, 0, 0, 0])
    // The instanced pipeline declares a second vertex buffer for morph sources.
    expect(h.pass.setVertexBuffer.mock.calls).toContainEqual([1, h.buffers[1]])
    h.draw()
    expect(h.writes).toHaveLength(1)
    expect(h.device.createBuffer).toHaveBeenCalledTimes(2)
  })

  it('preserves consecutive geometry groups and sorted object order', () => {
    const h = harness()
    const other = {} as GPUBuffer
    const list = h.meshes.map((m, i) => i >= 8 && i < 16 ? { ...m, vb: other } : m).reverse()
    h.draw(list)
    expect(h.pass.drawIndexed.mock.calls).toEqual([[3, 16, 0, 0, 0], [3, 8, 0, 0, 16], [3, 8, 0, 0, 24]])
    expect(h.writes[0][12]).toBe(31 * 3)
    expect(h.writes[0][31 * 44 + 12]).toBe(0)
  })

  it('refreshes hover/selection uniforms and retries a failed upload', () => {
    const h = harness(); h.draw()
    Object.assign(h.meshes[3], { styleAlpha: 0.24, styleSelected: 1, styleHovered: 1 })
    h.failNextWrite()
    expect(() => h.draw()).toThrow('write failed')
    expect(h.pass.drawIndexed).toHaveBeenCalledTimes(1)
    h.draw()
    expect(h.writes).toHaveLength(2)
    expect([...h.writes[1].slice(3 * 44 + 36, 3 * 44 + 40)]).toEqual([...new Float32Array([0.24, 1, 0.7, 1])])
    h.draw(); expect(h.writes).toHaveLength(2)
  })

  it('detects warmed edge-buffer changes even in the middle of a group', () => {
    const h = harness(); h.draw(h.meshes, true)
    expect(h.pass.drawIndexed.mock.calls).toEqual([[6, 32, 0, 0, 0]])
    h.meshes[7].edgeIB = {} as GPUBuffer
    h.draw(h.meshes, true)
    expect(h.pass.drawIndexed.mock.calls.slice(1)).toEqual([[6, 7, 0, 0, 0], [6, 1, 0, 0, 7], [6, 24, 0, 0, 8]])
  })

  it('falls back for unique geometry, small lists and device limits; releases buffers', () => {
    const h = harness()
    expect(h.draw(h.meshes.map(m => ({ ...m, vb: {} as GPUBuffer })))).toBe(false)
    expect(h.device.createBuffer).not.toHaveBeenCalled()
    h.draw(); h.cache.clear()
    expect(h.buffers[0].destroy).toHaveBeenCalledTimes(1)
    expect(h.draw(h.meshes.slice(0, 1))).toBe(false)
    h.device.limits.maxStorageBufferBindingSize = 160
    expect(h.draw()).toBe(false)
  })

  it('releases a newly allocated buffer if bind-group creation fails', () => {
    const h = harness()
    h.device.createBindGroup.mockImplementationOnce(() => { throw new Error('bind failed') })
    expect(() => h.draw()).toThrow('bind failed')
    expect(h.buffers[0].destroy).toHaveBeenCalledTimes(1)
    expect(h.draw()).toBe(true)
  })
})
