import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {wallSearchCandidates,wholeWallSearchCandidates} from '../src/services/solidWallSearch'
import {inspectMaterialWall,materialWallExpectation,validMaterialWall} from '../src/services/solidMaterialWall'
import {transformNurbsBrep} from '../src/services/geometry/brep'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
const fixtures=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-03/material-wall/contract.json',import.meta.url),'utf8')).cases
it('finds original material walls after rigid rotation and translation without editing the source',()=>{
 for(const name of ['cube-wall']){
  const c=fixtures.find((c:any)=>c.name===name),before=JSON.stringify(c.request.model)
  const model=transformNurbsBrep(c.request.model,[[0,-1,0,123],[0,0,-1,-45],[1,0,0,67],[0,0,0,1]])
  const rotated=JSON.stringify(model),lines=wallSearchCandidates(model,c.request.faceGroups,undefined,8)
  expect(lines.length).toBeGreaterThan(0)
  expect(lines.length).toBeLessThanOrEqual(8)
  let found=false
  for(const line of lines){
   const options={...c.request,model,origin:line.origin,direction:line.direction}
   const r=inspectMaterialWall(options)
   expect(validMaterialWall(materialWallExpectation(options),r)).toBe(true)
   if(r.converged){
    const expected=name==='cube-wall'?10:15
    expect(r.intervalMm![0]).toBeLessThanOrEqual(expected)
    expect(r.intervalMm![1]).toBeGreaterThanOrEqual(expected)
    found=true;break
   }
  }
  expect(found,name).toBe(true)
  expect(JSON.stringify(model)).toBe(rotated)
  expect(JSON.stringify(c.request.model)).toBe(before)
 }
},30000)
it('keeps annular candidate geometry covariant under rigid placement without claiming volume qualification',()=>{
 const c=fixtures.find((c:any)=>c.name==='annular-wall'),m=[[0,-1,0,123],[0,0,-1,-45],[1,0,0,67],[0,0,0,1]]
 const original=wallSearchCandidates(c.request.model,c.request.faceGroups,undefined,8)
 const placed=wallSearchCandidates(transformNurbsBrep(c.request.model,m),c.request.faceGroups,undefined,8)
 expect(placed).toHaveLength(original.length)
 for(let i=0;i<original.length;i++)for(let k=0;k<3;k++){
  const a=original[i],b=placed[i]
  expect(b.face).toBe(a.face)
  expect(b.uv).toEqual(a.uv)
  expect(b.origin[k]).toBeCloseTo(m[k][3]+a.origin.reduce((sum,x,j)=>sum+m[k][j]*x,0),9)
  expect(b.direction[k]).toBeCloseTo(a.direction.reduce((sum,x,j)=>sum+m[k][j]*x,0),9)
 }
})
it('refuses invalid groups and skips singular normal samples',()=>{
 const c=fixtures[0],model=c.request.model
 expect(()=>wallSearchCandidates(model,[[0],[0]])).toThrow('groups')
 expect(()=>wallSearchCandidates(model,c.request.faceGroups,undefined,0)).toThrow('budget')
 const singular=(s:any,u:number,v:number)=>({...evaluateNurbsSurface(s,u,v),normal:null})
 expect(wallSearchCandidates(model,c.request.faceGroups,singular)).toEqual([])
})

it('bounds whole-model sample work and skips singular regions without marking them covered',()=>{
 const source=fixtures[0].request.model,model=structuredClone(source)
 model.faces=Array.from({length:10000},()=>structuredClone(source.faces[0]))
 let evaluations=0
 const singular=(s:any,u:number,v:number)=>{evaluations++;return {point:[0,0,0],normal:null} as any}
 expect(wholeWallSearchCandidates(model,singular,8)).toEqual([])
 expect(evaluations).toBe(8+4*8)
 expect(()=>wholeWallSearchCandidates(model,singular,257)).toThrow('budget')
})
