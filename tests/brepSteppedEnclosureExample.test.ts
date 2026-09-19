import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {EXAMPLES} from '../src/data/examples'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
import {analyzeNurbsBrep,inspectNurbsBrep,type NurbsBrep} from '../src/services/geometry/brep'

function circularLensArea(r:number,R:number,d:number):number {
  return r*r*Math.acos((d*d+r*r-R*R)/(2*d*r))
    +R*R*Math.acos((d*d+R*R-r*r)/(2*d*R))
    -.5*Math.sqrt((-d+r+R)*(d+r-R)*(d-r+R)*(d+r+R))
}

it('builds and exports the public stepped enclosure with retained curved B-rep authority',()=>{
  const source=readFileSync(new URL('../examples/brep/stepped-enclosure.mg',import.meta.url),'utf8')
  expect(EXAMPLES['brep-enclosure']).toBe(source)
  const compiled=compileModelGraphText(source)
  expect(compiled.execution_target).toBe('own-nurbs')
  if(compiled.execution_target!=='own-nurbs')throw new Error('Example must use the native NURBS backend')
  const result=buildOwnNurbs(compiled.document,{action:'build',display:{segments:6,subdivisionLevels:0}})
  expect(result).toMatchObject({ok:true,automatic_fallback:false,report:{geometry_authority:'rational control data',error_bound_certified:false}})
  if(!('nativeGeometry' in result))throw new Error('Native geometry must survive display tessellation')
  expect(result.nativeGeometry.kind).toBe('brep')
  const model=JSON.parse(result.nativeGeometry.geometryJson).geometry as NurbsBrep
  expect(inspectNurbsBrep(model)).toMatchObject({topologyValid:true,solidGeometryStatus:'not_certified'})
  expect(model.bodies).toHaveLength(1)
  expect(model.shells).toHaveLength(1)
  expect(model.edges.some(edge=>edge.curve.degree===2)).toBe(true)
  // The pocket floor is a retained horizontal face, two millimetres above the base.
  expect(model.faces.some(face=>face.surface.controlPoints.flat().every(p=>Math.abs(p[2]-2)<1e-9))).toBe(true)
  // Compute union and pocket volumes independently using circle lenses and heights.
  const expectedVolume=2634*Math.PI-4*circularLensArea(18,8,16)-8*circularLensArea(11,8,16)
  expect(analyzeNurbsBrep(model).signedVolumeMm3).toBeCloseTo(expectedVolume,6)
  expect(result.report.bounds).toEqual({min:[-18,-18,0],max:[24,18,16]})
  expect(result.report.mesh).toMatchObject({closed:true,boundaryEdges:0,nonManifoldEdges:0,orientationConflicts:0,degenerateTriangles:0})
  expect(result.report.mesh!.triangleCount).toBeLessThan(5000)
  const json=buildOwnNurbs(compiled.document,{action:'export',format:'json'})
  if(!('artifact' in json)||!('text' in json.artifact))throw new Error('JSON export is required')
  expect(JSON.parse(json.artifact.text)).toEqual(compiled.document)
  const stl=buildOwnNurbs(compiled.document,{action:'export',format:'stl'})
  if(!('artifact' in stl)||!('text' in stl.artifact))throw new Error('STL export is required')
  expect(stl.artifact.text).toContain('facet normal')
  expect(stl.artifact.text.match(/facet normal/g)).toHaveLength(result.report.mesh!.triangleCount)
},15000)
