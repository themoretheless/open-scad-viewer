import { describe, expect, it } from 'vitest'
import { bodyPoints, DirectHistory, directBodiesScad, emptyDirectDocument, extrudeDirectSketch, parseDirectDocument, transformDirectPoints } from '../src/services/directModeling'
import { inspectPolygonMesh } from '../src/services/polygonKernel'
import { parseOpenSCAD } from '../src/services/openscadParser'
const sketch = () => ({ id: 's', name: 'L', closed: true, points: [[0,0],[20,0],[20,10],[10,10],[10,20],[0,20]] as [number,number][] })
describe('direct modeling', () => {
  it('creates a closed concave solid, independent of edits and deletion of its sketch', () => {
    const d = emptyDirectDocument(); d.sketches.push(sketch()); d.bodies.push(extrudeDirectSketch(d.sketches[0], 5, 'body'))
    const mesh = JSON.stringify(d.bodies[0].mesh)
    d.sketches[0].points[0][0] = -100; d.sketches = []
    expect(JSON.stringify(d.bodies[0].mesh)).toBe(mesh)
    const report = inspectPolygonMesh(d.bodies[0].mesh)
    expect(report.closed).toBe(true); expect(report.signedVolumeMm3).toBeCloseTo(1500)
  })
  it('undoes and redoes complete snapshots, drops redo after a new edit and isolates callers', () => {
    const h = new DirectHistory(), d = h.document; d.sketches.push(sketch()); h.commit(d)
    d.sketches[0].points[0][0] = 99
    expect(h.document.sketches[0].points[0][0]).toBe(0)
    const e = h.document; e.bodies.push(extrudeDirectSketch(e.sketches[0], 5, 'b')); h.commit(e)
    expect(h.undo().bodies).toHaveLength(0); expect(h.redo().bodies).toHaveLength(1)
    h.undo(); const f = h.document; f.sketches[0].name = 'New'; h.commit(f)
    expect(h.canRedo).toBe(false); expect(h.document.bodies).toHaveLength(0)
  })
  it('transforms body vertices without modifying the 2D source', () => {
    const s = sketch(), b = extrudeDirectSketch(s, 5, 'b'), before = JSON.stringify(s)
    b.mesh.positions = transformDirectPoints(bodyPoints(b), [10,20,30], 90, 2).flat()
    expect(inspectPolygonMesh(b.mesh).signedVolumeMm3).toBeCloseTo(12000)
    expect(JSON.stringify(s)).toBe(before)
    expect(Math.min(...bodyPoints(b).map(p=>p[2]))).toBeCloseTo(27.5)
  })
  it('round trips JSON and rejects invalid geometry atomically', () => {
    const d = emptyDirectDocument(); d.sketches.push(sketch()); const h = new DirectHistory(d)
    expect(parseDirectDocument(JSON.stringify(d))).toEqual(d)
    const invalid = h.document; invalid.sketches[0].points[0][0] = NaN
    expect(()=>h.commit(invalid)).toThrow(); expect(h.canUndo).toBe(false)
    expect(()=>extrudeDirectSketch({...sketch(), closed:false},5,'b')).toThrow()
    expect(()=>extrudeDirectSketch(sketch(),-1,'b')).toThrow()
    expect(()=>transformDirectPoints([[0,0]], [0,0], 0, 0)).toThrow()
  })
  it('exports baked solids that compile into the main scene with correct volume and topology', async () => {
    const d = emptyDirectDocument(); d.bodies.push(extrudeDirectSketch(sketch(),5,'b'))
    const result = await parseOpenSCAD(directBodiesScad(d))
    expect(result.volume).toBeCloseTo(1500)
    expect(result.meshes.length).toBeGreaterThan(0)
    for (const m of result.meshes) { expect(m.topology.boundary).toBe(0); expect(m.topology.nonManifold).toBe(0) }
  })
})
