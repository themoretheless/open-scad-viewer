import {expect,it} from 'vitest'
import {inspectBuildSurfaces} from '../src/services/buildSurfaceInspection'
import {extrudeDirectSketch} from '../src/services/directModeling'

const body=()=>extrudeDirectSketch({id:'s',name:'Box',closed:true,points:[[0,0],[2,0],[2,3],[0,3]]},4,'0')
const options={buildDirection:[0,0,1] as [number,number,number],coneDegrees:45,planeOffsetMm:0,planeToleranceMm:0}
it('executes actual WASM and preserves explicit geometric scope',()=>{
  const mesh=body().mesh,before=structuredClone(mesh)
  expect(inspectBuildSurfaces(mesh,options)).toEqual({modelKind:'signed-triangle-build-surfaces-v1',totalAreaMm2:52,downwardAreaMm2:0,downwardTriangles:0,contactAreaMm2:6,belowPlaneTriangles:0})
  expect(inspectBuildSurfaces(mesh,{...options,planeOffsetMm:-1}).downwardAreaMm2).toBe(6)
  expect(inspectBuildSurfaces(mesh,{...options,buildDirection:[1,0,1],coneDegrees:60,planeOffsetMm:-1}).downwardAreaMm2).toBe(18)
  expect(mesh).toEqual(before)
})
it('propagates native refusal instead of inferring omitted or unsupported settings',()=>{
  const mesh=body().mesh
  expect(()=>inspectBuildSurfaces(mesh,{...options,buildDirection:[0,0,0]})).toThrow()
  expect(()=>inspectBuildSurfaces(mesh,{...options,planeToleranceMm:-1})).toThrow()
  expect(()=>inspectBuildSurfaces(mesh,{...options,coneDegrees:91})).toThrow()
  expect(()=>inspectBuildSurfaces(mesh,{...options,supports:true} as typeof options)).toThrow()
})
