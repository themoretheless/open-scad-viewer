import { describe, expect, it } from 'vitest'
import { emptyDirectDocument, parseDirectDocument } from '../src/services/directModeling'
import { solidDocumentToMeshDocument } from '../src/services/solidBridge'
import {
  createSolidNurbsCurve,
  createSolidNurbsSurface,
  importModelGraphNurbs,
  nurbsCurveToSketch,
  sampleSolidNurbsCurve,
} from '../src/services/solidNurbs'

describe('Solid native NURBS bridge', () => {
  it('retains rational definitions in the Solid document and validates edits', () => {
    const document = emptyDirectDocument()
    document.curves!.push(createSolidNurbsCurve('curve'))
    document.surfaces!.push(createSolidNurbsSurface('surface'))
    const parsed = parseDirectDocument(JSON.stringify(document))
    expect(parsed.curves?.[0].curve.degree).toBe(3)
    expect(parsed.surfaces?.[0].surface.controlPoints[1][1][2]).toBe(10)

    parsed.curves![0].curve.weights[0] = 0
    expect(() => parseDirectDocument(JSON.stringify(parsed))).toThrow()
  })

  it('samples curves only on explicit sketch conversion', () => {
    const native = createSolidNurbsCurve('curve')
    const sampled = sampleSolidNurbsCurve(native.curve, 12)
    const sketch = nurbsCurveToSketch(native)
    expect(sampled).toHaveLength(13)
    expect(sketch.points.length).toBeGreaterThan(12)
    expect(native.curve.controlPoints).toHaveLength(4)
  })

  it('tessellates native surfaces only at the Solid to Mesh boundary', () => {
    const document = emptyDirectDocument()
    document.surfaces!.push(createSolidNurbsSurface('surface'))
    const mesh = solidDocumentToMeshDocument(document)
    expect(mesh.objects).toHaveLength(1)
    expect(mesh.objects[0].name).toContain('tessellated')
    expect(mesh.objects[0].mesh.indices.length).toBeGreaterThan(100)
    expect(document.bodies).toHaveLength(0)
  })

  it('imports reachable native definitions from ModelGraph/NURBS', () => {
    const imported = importModelGraphNurbs({
      language: 'modelgraph/nurbs-1',
      units: 'mm',
      parameters: [],
      nodes: [{
        id: 'path', op: 'curve', degree: 2,
        knots: [0, 0, 0, 1, 1, 1],
        control_points: [[10, 0, 0], [10, 10, 0], [0, 10, 0]],
        weights: [1, Math.SQRT1_2, 1], periodic: false,
      }, {
        id: 'skin', op: 'surface_extrude', input: 'path', vector: [0, 0, 5],
      }],
      root: 'skin',
    })
    expect(imported.curves.map(item => item.name)).toEqual(['path'])
    expect(imported.surfaces.map(item => item.name)).toEqual(['skin'])
  })
})
