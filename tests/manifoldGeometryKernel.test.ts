import type { ManifoldToplevel } from '../src/services/ownGeometryModule'
import { describe, expect, it, vi } from 'vitest'
import { ManifoldGeometryKernel } from '../src/services/manifoldGeometryKernel'
import { getWasm, parseOpenSCAD } from '../src/services/openscadParser'

function deleted(value: object): boolean {
  return (value as { isDeleted(): boolean }).isDeleted()
}

describe('ManifoldGeometryKernel lifecycle', () => {
  it('warms once and disposes each serialized lease exactly once', async () => {
    const module = {} as ManifoldToplevel
    const load = vi.fn(async () => module)
    const instrument = vi.fn((value: ManifoldToplevel) => value)
    const cleanup = vi.fn()
    const kernel = new ManifoldGeometryKernel({ load, instrument, cleanup })

    const [first, second] = await Promise.all([kernel.openSession(), kernel.openSession()])
    expect(first.module).toBe(module)
    expect(second.module).toBe(module)
    expect(load).toHaveBeenCalledTimes(1)
    expect(instrument).toHaveBeenCalledTimes(1)
    first.dispose()
    first.dispose()
    second.dispose()
    expect(cleanup).toHaveBeenCalledTimes(2)
  })

  it('does not cache a rejected warm-up attempt', async () => {
    const module = {} as ManifoldToplevel
    const load = vi.fn()
      .mockRejectedValueOnce(new Error('temporary load failure'))
      .mockResolvedValueOnce(module)
    const kernel = new ManifoldGeometryKernel({ load, instrument: value => value, cleanup: vi.fn() })

    await expect(kernel.warm()).rejects.toThrow('temporary load failure')
    await expect(kernel.warm()).resolves.toBe(module)
    expect(load).toHaveBeenCalledTimes(2)
  })

  it('disposes real constructor, member, cross-dimension and array results without double deletion', async () => {
    const kernel = new ManifoldGeometryKernel()
    const session = await kernel.openSession()
    const wasm = session.module
    const cube = wasm.Manifold.cube([4, 4, 4], true)
    // Own runtimes have isolated prototypes rather than shared global wrappers.
    expect(Object.getPrototypeOf(cube)).toBe(wasm.Manifold.prototype)
    const translated = cube.translate([2, 0, 0])
    const normals = translated.calculateNormals(0, 52.5)
    const constructed = new wasm.Manifold(cube.getMesh())
    const pieces = cube.splitByPlane([1, 0, 0], 0)
    const contours: [number, number][][] = [[[0, 0], [2, 0], [2, 2], [0, 2]]]
    const section = new wasm.CrossSection(contours)
    const movedSection = section.translate([3, 0])
    const extruded = movedSection.extrude(3)
    const revolved = movedSection.revolve(16)
    const projected = normals.project()
    const children = section.decompose()
    const resources = [cube, translated, normals, constructed, ...pieces, section, movedSection, extruded, revolved, projected, ...children]
    expect(normals.volume()).toBeCloseTo(64)
    expect(extruded.volume()).toBeCloseTo(12)
    expect(resources.every(value => !deleted(value))).toBe(true)
    translated.delete()
    expect(() => session.dispose()).not.toThrow()
    expect(resources.every(deleted)).toBe(true)
    expect(() => session.dispose()).not.toThrow()
  })

  it('keeps ownership independent across two real WASM modules', async () => {
    const first = await new ManifoldGeometryKernel().openSession()
    const second = await new ManifoldGeometryKernel().openSession()
    const a = first.module.Manifold.cube(2).translate([1, 0, 0])
    const b = second.module.Manifold.cube(3).calculateNormals()
    first.dispose()
    expect(deleted(a)).toBe(true)
    expect(deleted(b)).toBe(false)
    expect(b.volume()).toBeCloseTo(27)
    second.dispose()
    expect(deleted(b)).toBe(true)
  })

  it('owns centered extrusion without an intermediate native receiver', async () => {
    const session = await new ManifoldGeometryKernel().openSession()
    const probe = session.module.Manifold.cube(1)
    const prototype = Object.getPrototypeOf(probe) as typeof probe
    const original = prototype.translate
    const receivers: object[] = []
    const translate = vi.spyOn(prototype, 'translate').mockImplementation(function (this: typeof probe, ...args) {
      receivers.push(this)
      return Reflect.apply(original, this, args)
    })
    try {
      const section = session.module.CrossSection.square([2, 2])
      const centered = section.extrude(3, 0, 0, [1, 1], true)
      expect(centered.volume()).toBeCloseTo(12)
      expect(centered.boundingBox().min[2]).toBeCloseTo(-1.5)
      session.dispose()
      expect(receivers.every(deleted)).toBe(true)
      expect(deleted(centered)).toBe(true)
    } finally { translate.mockRestore(); session.dispose() }
  })

  it('deletes a rejected native mesh constructor before propagating its error', async () => {
    const session = await new ManifoldGeometryKernel().openSession()
    const probe = session.module.Manifold.cube(1)
    const prototype = Object.getPrototypeOf(probe) as typeof probe
    const dispose = vi.spyOn(prototype, 'delete')
    try {
      const invalid = new session.module.Mesh({
        numProp: 3,
        vertProperties: new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]),
        triVerts: new Uint32Array([0, 1, 2]),
      })
      expect(() => session.module.Manifold.ofMesh(invalid)).toThrow('Not manifold')
      expect(dispose).toHaveBeenCalledTimes(1)
      const rejected = dispose.mock.contexts[0]
      expect(deleted(rejected)).toBe(true)
      session.dispose()
      expect(dispose).toHaveBeenCalledTimes(2)
    } finally { dispose.mockRestore(); session.dispose() }
  })

  it.each(['success', 'failure', 'cancel'] as const)('releases actual parser member results after %s', async outcome => {
    const wasm = await getWasm()
    const probe = wasm.Manifold.cube(1)
    const prototype = Object.getPrototypeOf(probe) as typeof probe
    probe.delete()
    const created: object[] = []
    const originalTranslate = prototype.translate
    const originalNormals = prototype.calculateNormals
    const translate = vi.spyOn(prototype, 'translate').mockImplementation(function (this: typeof probe, ...args) {
      const result = Reflect.apply(originalTranslate, this, args)
      created.push(result)
      return result
    })
    const normals = vi.spyOn(prototype, 'calculateNormals').mockImplementation(function (this: typeof probe, ...args) {
      const result = Reflect.apply(originalNormals, this, args)
      created.push(result)
      return result
    })
    try {
      const source = 'translate([1,0,0]) cube(2);'
      if (outcome === 'success') {
        const result = await parseOpenSCAD(source)
        expect(result.volume).toBeCloseTo(8)
        expect(result.meshes[0].vertices.every(Number.isFinite)).toBe(true)
        expect(normals).toHaveBeenCalled()
      } else if (outcome === 'failure') {
        await expect(parseOpenSCAD(`${source} assert(false, "stop");`)).rejects.toThrow('stop')
      } else {
        let cancelled = false
        await expect(parseOpenSCAD(source, {
          shouldAbort: () => cancelled,
          yieldControl: async () => { cancelled = true },
        })).rejects.toThrow('aborted')
      }
      expect(created.length).toBeGreaterThan(0)
      expect(created.every(deleted)).toBe(true)
    } finally {
      translate.mockRestore()
      normals.mockRestore()
    }
  })
})
