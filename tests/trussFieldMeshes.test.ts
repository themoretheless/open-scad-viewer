import {expect, it} from 'vitest'
import {trussFieldMeshes, TRUSS_FORCE_COLORS} from '../src/services/trussFieldMeshes'
import type {TrussModel, TrussResponse} from '../src/services/trussAnalysis'

const model: TrussModel={nodesMm:[[0,0,0],[10,0,0],[10,10,0]],members:[
  {nodes:[0,1],youngMpa:1,areaMm2:1},{nodes:[1,2],youngMpa:1,areaMm2:1},{nodes:[2,0],youngMpa:1,areaMm2:1}],
  restrained:[],forcesN:[]}
const result:TrussResponse={axialForcesN:[-5,0,7],axialStressesMpa:[],displacementsMm:[],reactionsN:[],freeDofs:0,maxDeflectionMm:0,maxRelativeResidual:0}
it('groups markers by actual force sign and centers them on members',()=>{
  const meshes=trussFieldMeshes(model,result,2)
  expect(meshes.map(mesh=>mesh.color)).toEqual(Object.values(TRUSS_FORCE_COLORS))
  expect(meshes.map(mesh=>mesh.indices.length)).toEqual([24,24,24])
  const xs=Array.from(meshes[0].vertices).filter((_,i)=>i%6===0)
  expect(Math.min(...xs)).toBe(4);expect(Math.max(...xs)).toBe(6)
  expect(trussFieldMeshes(model,{...result,axialForcesN:[1,2,3]},2)).toHaveLength(1)
})
it('refuses invalid fields and marker sizes without mutating the model',()=>{
  const before=structuredClone(model)
  for(const size of [0,-1,NaN,Infinity])expect(()=>trussFieldMeshes(model,result,size)).toThrow()
  expect(()=>trussFieldMeshes(model,{...result,axialForcesN:[NaN,0,1]},1)).toThrow()
  expect(()=>trussFieldMeshes(model,{...result,axialForcesN:[1]},1)).toThrow()
  expect(()=>trussFieldMeshes({...model,members:[{nodes:[0,3],youngMpa:1,areaMm2:1}]},{...result,axialForcesN:[1]},1)).toThrow()
  expect(model).toEqual(before)
})

it('bounds a 400-member field to three meshes and 3200 triangles',()=>{
  const members=Array.from({length:400},(_,i)=>model.members[i%3])
  const axialForcesN=members.map((_,i)=>i%3-1)
  const meshes=trussFieldMeshes({...model,members},{...result,axialForcesN},1)
  expect(meshes).toHaveLength(3)
  expect(meshes.reduce((sum,mesh)=>sum+mesh.indices.length/3,0)).toBe(3200)
  expect(()=>trussFieldMeshes({...model,members:[...members,members[0]]},{...result,axialForcesN:[...axialForcesN,0]},1)).toThrow()
})

it('refuses sparse force fields rather than coloring missing values as zero',()=>{
  const forces=[-1,0,1];delete forces[1]
  expect(()=>trussFieldMeshes(model,{...result,axialForcesN:forces},1)).toThrow()
})

it('refuses partial fields when distant markers collapse at display precision',()=>{
  const far:TrussModel={...model,nodesMm:[[0,0,0],[2,0,0],[1e6-2,0,0],[1e6,0,0]],members:[
    {nodes:[0,1],youngMpa:1,areaMm2:1},{nodes:[2,3],youngMpa:1,areaMm2:1}]}
  expect(()=>trussFieldMeshes(far,{...result,axialForcesN:[1,1]},0.001)).toThrow()
  const large=trussFieldMeshes(far,{...result,axialForcesN:[1,1]},1)
  expect(large[0].indices.length/3).toBe(16)
})
