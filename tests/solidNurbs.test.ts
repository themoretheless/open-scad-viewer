import { describe, expect, it } from 'vitest'
import { emptyDirectDocument, parseDirectDocument } from '../src/services/directModeling'
import { solidDocumentToMeshDocument } from '../src/services/solidBridge'
import {
  createSolidNurbsCurve,
  createSolidNurbsSurface,
  importModelGraphNurbs,
  nurbsCurveToSketch,
  sampleSolidNurbsCurve,
  updateSolidNurbsControlPoint,
} from '../src/services/solidNurbs'
import { trimNurbsSurface } from '../src/services/nurbsSurface'

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

  it('edits curve and surface CVs without baking either definition', () => {
    const document = emptyDirectDocument()
    document.curves!.push(createSolidNurbsCurve('curve'))
    document.surfaces!.push(createSolidNurbsSurface('surface'))
    const curveEdit = updateSolidNurbsControlPoint(document, 'curve', 1, 0, [-7, 15, 4], 1.5)
    const surfaceEdit = updateSolidNurbsControlPoint(curveEdit, 'surface', 1, 1, [0, 0, 14], 2)
    expect(surfaceEdit.curves![0].curve.controlPoints[1]).toEqual([-7, 15, 4])
    expect(surfaceEdit.curves![0].curve.weights[1]).toBe(1.5)
    expect(surfaceEdit.surfaces![0].surface.controlPoints[1][1]).toEqual([0, 0, 14])
    expect(surfaceEdit.surfaces![0].surface.weights[1][1]).toBe(2)
    expect(surfaceEdit.bodies).toEqual([])
    expect(document.curves![0].curve.controlPoints[1]).toEqual([-8, 18, 0])
  })

  it('trims a native surface to an exact rectangular UV subdomain', () => {
    const source = createSolidNurbsSurface('surface').surface
    const trimmed = trimNurbsSurface(source, [.2, .8, .1, .9])
    expect(trimmed.knotsU[trimmed.degreeU]).toBeCloseTo(.2)
    expect(trimmed.knotsU[trimmed.controlPoints.length]).toBeCloseTo(.8)
    expect(trimmed.knotsV[trimmed.degreeV]).toBeCloseTo(.1)
    expect(trimmed.knotsV[trimmed.controlPoints[0].length]).toBeCloseTo(.9)
  })
})
