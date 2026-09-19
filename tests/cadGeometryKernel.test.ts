import type { CadToplevel } from '../src/services/geometry/module'
import { describe, expect, it, vi } from 'vitest'
import { CadGeometryKernel } from '../src/services/cadGeometryKernel'
import { defaultGeometryKernel } from '../src/services/cadGeometryKernel'
import { parseOpenSCAD } from '../src/services/openscadParser'

function deleted(value: object): boolean {
  return (value as { isDeleted(): boolean }).isDeleted()
}

describe('kernel-built display meshes', () => {
  it('splits vertex normals at creases inside the kernel and reports merge pairs', async () => {
    const kernel = new CadGeometryKernel()
    const session = await kernel.openSession()
    try {
      const cube = session.module.CadSolid.cube([2, 2, 2], true)
      const mesh = cube.calculateNormals(0, 52.5).getMesh()
      expect(mesh.numProp).toBe(6)
      expect(mesh.numTri).toBe(12)
      // Every corner meets three faces at 90°, so it becomes three property vertices.
      expect(mesh.numVert).toBe(24)
      expect(mesh.mergeFromVert.length).toBe(24 - 8)
      expect(mesh.mergeToVert.length).toBe(mesh.mergeFromVert.length)
      for (let vertex = 0; vertex < mesh.numVert; vertex++) {
        const [x, y, z, nx, ny, nz] = mesh.vertProperties.subarray(vertex * 6, vertex * 6 + 6)
        expect(Math.hypot(nx, ny, nz)).toBeCloseTo(1, 6)
        // On an axis-aligned cube the normal points along exactly one axis, outward.
        expect([Math.abs(nx), Math.abs(ny), Math.abs(nz)].sort()).toEqual([0, 0, 1])
        expect(nx * x + ny * y + nz * z).toBeCloseTo(1, 6)
      }
      for (let pair = 0; pair < mesh.mergeFromVert.length; pair++) {
        const from = mesh.mergeFromVert[pair] * 6
        const to = mesh.mergeToVert[pair] * 6
        expect([...mesh.vertProperties.subarray(from, from + 3)]).toEqual([...mesh.vertProperties.subarray(to, to + 3)])
        expect(mesh.mergeToVert[pair]).toBeLessThan(mesh.mergeFromVert[pair])
      }
      expect(mesh.faceID).toHaveLength(12)
      expect(new Set(mesh.faceID).size).toBe(6)
      // The arrays are owned copies: the kernel may be called again freely.
      const again = cube.calculateNormals(0, 52.5).getMesh()
      expect(again.vertProperties).toEqual(mesh.vertProperties)
      expect(again.vertProperties).not.toBe(mesh.vertProperties)
      // A crease angle wider than the dihedral angle smooths the corners instead.
      const smooth = cube.calculateNormals(0, 120).getMesh()
      expect(smooth.numVert).toBe(8)
      expect(smooth.mergeFromVert).toHaveLength(0)
    } finally {
      session.dispose()
    }
  })
})

describe('CadGeometryKernel lifecycle', () => {
  it('warms once and disposes each serialized lease exactly once', async () => {
    const module = {} as CadToplevel
    const load = vi.fn(async () => module)
    const instrument = vi.fn((value: CadToplevel) => value)
    const cleanup = vi.fn()
    const kernel = new CadGeometryKernel({ load, instrument, cleanup })

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
    const module = {} as CadToplevel
    const load = vi.fn()
      .mockRejectedValueOnce(new Error('temporary load failure'))
      .mockResolvedValueOnce(module)
    const kernel = new CadGeometryKernel({ load, instrument: value => value, cleanup: vi.fn() })

    await expect(kernel.warm()).rejects.toThrow('temporary load failure')
    await expect(kernel.warm()).resolves.toBe(module)
    expect(load).toHaveBeenCalledTimes(2)
  })

  it('disposes real constructor, member, cross-dimension and array results without double deletion', async () => {
    const kernel = new CadGeometryKernel()
    const session = await kernel.openSession()
    const wasm = session.module
    const cube = wasm.CadSolid.cube([4, 4, 4], true)
    // Own runtimes have isolated prototypes rather than shared global wrappers.
    expect(Object.getPrototypeOf(cube)).toBe(wasm.CadSolid.prototype)
    const translated = cube.translate([2, 0, 0])
    const normals = translated.calculateNormals(0, 52.5)
    const constructed = new wasm.CadSolid(cube.getMesh())
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
    const first = await new CadGeometryKernel().openSession()
    const second = await new CadGeometryKernel().openSession()
    const a = first.module.CadSolid.cube(2).translate([1, 0, 0])
    const b = second.module.CadSolid.cube(3).calculateNormals()
    first.dispose()
    expect(deleted(a)).toBe(true)
    expect(deleted(b)).toBe(false)
    expect(b.volume()).toBeCloseTo(27)
    second.dispose()
    expect(deleted(b)).toBe(true)
  })

  it('owns centered extrusion without an intermediate native receiver', async () => {
    const session = await new CadGeometryKernel().openSession()
    const probe = session.module.CadSolid.cube(1)
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
    const session = await new CadGeometryKernel().openSession()
    const probe = session.module.CadSolid.cube(1)
    const prototype = Object.getPrototypeOf(probe) as typeof probe
    const dispose = vi.spyOn(prototype, 'delete')
    try {
      const invalid = new session.module.Mesh({
        numProp: 3,
        vertProperties: new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]),
        triVerts: new Uint32Array([0, 1, 2]),
      })
      expect(() => session.module.CadSolid.ofMesh(invalid)).toThrow('Not manifold')
      expect(dispose).toHaveBeenCalledTimes(1)
      const rejected = dispose.mock.contexts[0]
      expect(deleted(rejected)).toBe(true)
      session.dispose()
      expect(dispose).toHaveBeenCalledTimes(2)
    } finally { dispose.mockRestore(); session.dispose() }
  })

  it.each(['success', 'failure', 'cancel'] as const)('releases actual parser member results after %s', async outcome => {
    const wasm = await defaultGeometryKernel.warm()
    const probe = wasm.CadSolid.cube(1)
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
        expect(normals).not.toHaveBeenCalled()
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
