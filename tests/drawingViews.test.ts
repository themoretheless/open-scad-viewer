import { describe, expect, it } from 'vitest'
import { vectorDrawingPdf } from '../src/app/drawingSheets'
import { createDrawingViews, drawingViewsSheet, type DrawingMesh } from '../src/kernel/drawingViews'

type P = [number, number, number]

class Builder {
  positions: number[] = []
  indices: number[] = []
  private ids = new Map<string, number>()
  point(p: P): number {
    const key = p.join(',')
    let id = this.ids.get(key)
    if (id === undefined) {
      id = this.positions.length / 3
      this.positions.push(...p)
      this.ids.set(key, id)
    }
    return id
  }
  quad(a: P, b: P, c: P, d: P) {
    this.indices.push(this.point(a), this.point(b), this.point(c), this.point(a), this.point(c), this.point(d))
  }
  cuboid(lo: P, hi: P) {
    const p = (x: number, y: number, z: number): P => [x ? hi[0] : lo[0], y ? hi[1] : lo[1], z ? hi[2] : lo[2]]
    this.quad(p(0, 0, 0), p(0, 1, 0), p(1, 1, 0), p(1, 0, 0))
    this.quad(p(0, 0, 1), p(1, 0, 1), p(1, 1, 1), p(0, 1, 1))
    this.quad(p(0, 0, 0), p(1, 0, 0), p(1, 0, 1), p(0, 0, 1))
    this.quad(p(0, 1, 0), p(0, 1, 1), p(1, 1, 1), p(1, 1, 0))
    this.quad(p(0, 0, 0), p(0, 0, 1), p(0, 1, 1), p(0, 1, 0))
    this.quad(p(1, 0, 0), p(1, 1, 0), p(1, 1, 1), p(1, 0, 1))
  }
  mesh(): DrawingMesh {
    return { positions: this.positions, indices: this.indices }
  }
}

const N = 16
const snap = (x: number) => Math.round(x * 1e9) / 1e9

/** 20 mm cube with a 16-gon through hole (radius 5) along Z. */
function cubeWithHole(): DrawingMesh {
  const m = new Builder()
  const ring = (k: number, r: number | null, z: number): P => {
    const a = (2 * Math.PI * (k % N)) / N
    const c = Math.cos(a)
    const s = Math.sin(a)
    const t = r ?? 10 / Math.max(Math.abs(c), Math.abs(s))
    return [snap(10 + t * c), snap(10 + t * s), z]
  }
  for (let k = 0; k < N; k++) {
    const o0 = (z: number) => ring(k, null, z)
    const o1 = (z: number) => ring(k + 1, null, z)
    const i0 = (z: number) => ring(k, 5, z)
    const i1 = (z: number) => ring(k + 1, 5, z)
    m.quad(o0(20), o1(20), i1(20), i0(20))
    m.quad(o0(0), i0(0), i1(0), o1(0))
    m.quad(o0(0), o1(0), o1(20), o0(20))
    m.quad(i0(0), i0(20), i1(20), i1(0))
  }
  return m.mesh()
}

function cylinder(n: number): DrawingMesh {
  const m = new Builder()
  const p = (k: number, z: number): P => {
    const a = (2 * Math.PI * (k % n)) / n
    return [snap(5 * Math.cos(a)), snap(5 * Math.sin(a)), z]
  }
  for (let k = 0; k < n; k++) {
    m.quad(p(k, 0), p(k + 1, 0), p(k + 1, 10), p(k, 10))
    m.indices.push(m.point([0, 0, 10]), m.point(p(k, 10)), m.point(p(k + 1, 10)))
    m.indices.push(m.point([0, 0, 0]), m.point(p(k + 1, 0)), m.point(p(k, 0)))
  }
  return m.mesh()
}

const count = (segments: { hidden: boolean }[]) => [
  segments.filter(s => !s.hidden).length,
  segments.filter(s => s.hidden).length,
]

describe('drawing views with hidden lines', () => {
  it('draws the bore of a cube with a hole as hidden dashed lines', () => {
    const r = createDrawingViews(cubeWithHole(), { section: { view: 'top', offset: 10, spacing: 1 } })
    const byView = Object.fromEntries(r.views.map(v => [v.view, v.segments]))
    expect(count(byView.front)).toEqual([4, 2])
    expect(count(byView.right)).toEqual([4, 2])
    expect(count(byView.top)).toEqual([4 + N, 0])
    expect(byView.front.filter(s => s.hidden).every(s => s.silhouette && s.a[0] === s.b[0])).toBe(true)
    expect(count(byView.iso)[1]).toBeGreaterThan(0)
    expect(r.svg).toContain('class="hidden"')
    expect(r.svg).toContain('stroke-dasharray')
    expect(r.svg).toContain('data-view="ISO"')
    expect(r.section?.rings).toHaveLength(2)
    const polygon = 0.5 * N * 25 * Math.sin((2 * Math.PI) / N)
    expect(r.section?.area).toBeCloseTo(400 - polygon, 6)
    expect(r.svg).toContain('fill-rule="evenodd"')
    expect(r.svg).toContain('class="hatch"')
    expect(createDrawingViews(cubeWithHole(), { section: { view: 'top', offset: 10, spacing: 1 } }).svg).toBe(r.svg)
  })

  it('draws cylinder silhouettes as visible edges', () => {
    const r = createDrawingViews(cylinder(32), { views: ['front', 'top'] })
    const front = r.views[0].segments
    expect(count(front)).toEqual([4, 0])
    expect(
      front
        .filter(s => s.silhouette)
        .map(s => s.a[0])
        .sort((a, b) => a - b),
    ).toEqual([-5, 5])
    expect(count(r.views[1].segments)).toEqual([32, 0])
    expect(r.svg).not.toContain('class="hidden"')
  })
})

describe('sheet dimensions and export', () => {
  it('dimensions every view including ISO and SECTION', () => {
    const r = createDrawingViews(cubeWithHole(), { section: { view: 'top', offset: 10, spacing: 1 } })
    const dims = [
      ...r.svg.matchAll(/<g data-view="([^"]+)">.*?<g class="dimension" data-width="([\d.]+)" data-height="([\d.]+)"/g),
    ]
    const byView = Object.fromEntries(dims.map(d => [d[1], [Number(d[2]), Number(d[3])]]))
    expect(Object.keys(byView)).toEqual(['FRONT', 'TOP', 'RIGHT', 'ISO', 'SECTION TOP'])
    expect(byView.FRONT).toEqual([20, 20])
    expect(byView['SECTION TOP']).toEqual([20, 20])
    expect(byView.ISO[0]).toBeGreaterThan(20)
    expect(r.svg).toMatch(/scale [\d.]+:1/)
  })

  it('converts to a drawing sheet and a vector PDF', () => {
    const r = createDrawingViews(cubeWithHole(), { views: ['front', 'top'] })
    const sheet = drawingViewsSheet(r, 'Cube')
    expect(sheet).toMatchObject({ title: 'Cube', kind: 'drawing', width: 190, height: 150 })
    const pdf = new TextDecoder().decode(vectorDrawingPdf([sheet]))
    expect(pdf.startsWith('%PDF-')).toBe(true)
    expect(pdf).toContain('%%EOF')
    expect(() => drawingViewsSheet({ ...r, svg: '<svg/>' })).toThrow()
  })
})

describe('print checks in the drawing', () => {
  it('reports wall thickness against the threshold', () => {
    const ok = createDrawingViews(cubeWithHole(), { views: ['top'], minWallMm: 1 }).printChecks
    expect(ok.wallOk).toBe(true)
    expect(ok.minWallMm).toBeGreaterThan(4)
    const thin = createDrawingViews(cubeWithHole(), { views: ['top'], minWallMm: 6 })
    expect(thin.printChecks.wallOk).toBe(false)
    expect(thin.printChecks.thinTriangles).toBeGreaterThan(0)
    expect(thin.svg).toMatch(/Min wall: [\d.]+ mm · thin triangles \d+ · FAIL/)
  })

  it('reports clearance between parts and overhangs', () => {
    const m = new Builder()
    m.cuboid([0, 0, 0], [2, 2, 5])
    m.cuboid([0, 0, 5.1], [10, 10, 6])
    const r = createDrawingViews(m.mesh(), { views: ['front'], minClearanceMm: 0.2, minWallMm: 0.5 })
    expect(r.printChecks.clearanceOk).toBe(false)
    expect(r.printChecks.minClearanceMm).toBeCloseTo(0.1, 6)
    expect(r.printChecks.overhangOk).toBe(false)
    expect(r.printChecks.overhangAreaMm2).toBeCloseTo(100, 6)
    expect(r.svg).toContain('Overhang &gt; 45°: 100 mm²')
    const single = createDrawingViews(cubeWithHole(), { views: ['front'] }).printChecks
    expect(single.clearanceOk).toBe(true)
    expect(single.overhangOk).toBe(true)
  })

  it('rejects invalid requests', () => {
    expect(() => createDrawingViews(cubeWithHole(), { section: { view: 'top', offset: Number.NaN } })).toThrow()
    expect(() => createDrawingViews(cubeWithHole(), { minClearanceMm: 0 })).toThrow()
  })
})
