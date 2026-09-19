import { describe, expect, it } from 'vitest'
import { nurbsToPolygonMesh, polygonBoundaryNurbsCurves } from '../src/services/geometry/polygonBridge'
import { booleanPolygonMeshes, polygonBoundaryLoops, thickenPolygonMesh, transformPolygonMesh, inspectPolygonMesh, exportPolygonStl } from '../src/services/geometry/polygon'
import { evaluateNurbsCurve } from '../src/services/nurbsCurve'
import { extrudeNurbsCurve } from '../src/services/nurbsConstructors'
import type { NurbsSurface } from '../src/services/nurbsSurface'
const plane: NurbsSurface = { degreeU: 1, degreeV: 1, knotsU: [0,0,1,1], knotsV: [0,0,1,1], controlPoints: [[[0,0,0],[0,10,0]],[[10,0,0],[10,10,0]]], weights: [[1,1],[1,1]] }
describe('two independent Rust libraries through the WASM bridge', () => {
  it('passes a NURBS mesh into polygon operations and its boundary back into NURBS', () => {
    const before = JSON.stringify(plane)
    const mesh = nurbsToPolygonMesh(plane, {segmentsU:4,segmentsV:3})
    const loops = polygonBoundaryLoops(mesh), curves = polygonBoundaryNurbsCurves(mesh)
    expect(curves).toHaveLength(1)
    expect(curves[0]).toMatchObject({degree:1,periodic:false})
    loops[0].forEach((vertex,index) => expect(evaluateNurbsCurve(curves[0],index).point).toEqual(mesh.positions.slice(3*vertex,3*vertex+3)))
    const solid = thickenPolygonMesh(mesh,[0,0,2])
    expect(solid.report.closed).toBe(true)
    expect(solid.report.signedVolumeMm3).toBeCloseTo(200,9)
    expect(solid.uv).toBeUndefined()
    expect(exportPolygonStl(solid)).toContain('facet normal')
    const walls = extrudeNurbsCurve(curves[0],[0,0,3])
    const wallMesh = nurbsToPolygonMesh(walls,{segmentsU:14,segmentsV:2})
    expect(wallMesh.report.parameterSeamsWelded?.u).toBe(true)
    expect(JSON.stringify(plane)).toBe(before)
  })
  it('works on a plain imported mesh with no NURBS input', () => {
    const mesh = {positions:[0,0,0,2,0,0,2,3,0,0,3,0],indices:[0,1,2,0,2,3]}
    expect(inspectPolygonMesh(mesh).boundaryEdges).toBe(4)
    const solid = thickenPolygonMesh(mesh,[0,0,4])
    const reflected = transformPolygonMesh(solid,[[-2,0,0,100],[0,1,0,200],[0,0,1,300],[0,0,0,1]])
    expect(reflected.report.closed).toBe(true)
    expect(reflected.report.signedVolumeMm3).toBeCloseTo(48,10)
    expect(() => inspectPolygonMesh({...mesh,indices:[0,1,99]})).toThrow()
  })
})

it('performs own Rust CSG on tessellated NURBS solids through WASM', () => {
  const a = thickenPolygonMesh(nurbsToPolygonMesh(plane,{segmentsU:3,segmentsV:4}),[0,0,2])
  const b = transformPolygonMesh(a,[[1,0,0,5],[0,1,0,5],[0,0,1,1],[0,0,0,1]])
  for (const [operation,volume] of [['union',375],['intersection',25],['difference',175]] as const) {
    const result = booleanPolygonMeshes(a,b,operation)
    expect(result.report).toMatchObject({closed:true,construction:'boolean',selfIntersectionStatus:'checked_with_tolerance',boolean:{operation}})
    expect(result.report.signedVolumeMm3).toBeCloseTo(volume,7)
    expect(exportPolygonStl(result)).toContain('facet normal')
  }
  expect(booleanPolygonMeshes(a,a,'difference').indices).toEqual([])
  expect(() => booleanPolygonMeshes(a,b,'union',{maxWork:1})).toThrow(/budget|work|limit/i)
})
it('clips a curved NURBS-derived solid and conserves its volume', () => {
  const curved: NurbsSurface = {degreeU:2,degreeV:2,knotsU:[0,0,0,1,1,1],knotsV:[0,0,0,1,1,1],controlPoints:[[[0,0,0],[0,5,0],[0,10,0]],[[5,0,0],[5,5,3],[5,10,0]],[[10,0,0],[10,5,0],[10,10,0]]],weights:[[1,1,1],[1,1,1],[1,1,1]]}
  const a=thickenPolygonMesh(nurbsToPolygonMesh(curved,{segmentsU:6,segmentsV:6}),[0,0,2])
  const b=thickenPolygonMesh({positions:[5,-1,-1,11,-1,-1,11,11,-1,5,11,-1],indices:[0,1,2,0,2,3]},[0,0,5])
  const inside=booleanPolygonMeshes(a,b,'intersection'),outside=booleanPolygonMeshes(a,b,'difference')
  expect(inside.report.closed).toBe(true)
  expect(outside.report.closed).toBe(true)
  expect(inside.report.signedVolumeMm3).toBeCloseTo(100,7)
  expect(inside.report.signedVolumeMm3+outside.report.signedVolumeMm3).toBeCloseTo(a.report.signedVolumeMm3,7)
})

it('preserves exact indexed inspection diagnostics without welding coordinates', () => {
  const positions = [0,0,0, 2,0,0, 2,3,0, 0,3,0, 0,0,0]
  for (const [indices, expected] of [
    [[], {boundaryEdges:0,nonManifoldEdges:0,orientationConflicts:0,degenerateTriangles:0}],
    [[0,1,2,0,2,3], {boundaryEdges:4,nonManifoldEdges:0,orientationConflicts:0,degenerateTriangles:0}],
    [[0,1,2,0,3,2], {boundaryEdges:4,nonManifoldEdges:0,orientationConflicts:1,degenerateTriangles:0}],
    [[0,1,2,0,3,2,0,2,1], {boundaryEdges:2,nonManifoldEdges:1,orientationConflicts:0,degenerateTriangles:0}],
    [[0,1,2,4,2,3], {boundaryEdges:6,nonManifoldEdges:0,orientationConflicts:0,degenerateTriangles:0}],
    [[0,0,1], {boundaryEdges:1,nonManifoldEdges:0,orientationConflicts:0,degenerateTriangles:1}],
  ] as const) {
    const report = inspectPolygonMesh({positions,indices:[...indices]})
    expect(report).toMatchObject({...expected,closed:false,signedVolumeMm3:0,construction:'triangle_mesh',selfIntersectionStatus:'not_checked',errorBoundCertified:false})
  }
  const ambiguous = {positions,indices:[0,1,2,4,2,3]}
  expect(() => polygonBoundaryLoops(ambiguous)).toThrow(/branch/)
  expect(() => thickenPolygonMesh({positions,indices:[0,1,2,0,3,2]},[0,0,1])).toThrow(/oriented/)
})

it('keeps canonical boundary and thickening output order through WASM', () => {
  const mesh = {positions:[0,0,0,2,0,0,2,3,0,0,3,0],indices:[0,1,2,0,2,3]}
  expect(polygonBoundaryLoops(mesh)).toEqual([[0,1,2,3,0]])
  expect(polygonBoundaryLoops({...mesh,indices:[0,2,3,0,1,2]})).toEqual([[0,1,2,3,0]])
  expect(polygonBoundaryLoops({...mesh,indices:[2,1,0,3,2,0]})).toEqual([[0,3,2,1,0]])
  expect(thickenPolygonMesh(mesh,[0,0,4]).indices).toEqual([
    0,2,1,4,5,6,0,3,2,4,6,7,
    0,1,5,0,5,4,3,0,4,3,4,7,
    1,2,6,1,6,5,2,3,7,2,7,6,
  ])
  expect(() => inspectPolygonMesh({...mesh,uv:[0,0]})).toThrow(/Malformed/)
  expect(() => inspectPolygonMesh({...mesh,indices:[0,1,99]})).toThrow(/Malformed/)
})
