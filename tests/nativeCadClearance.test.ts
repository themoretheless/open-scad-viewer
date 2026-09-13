import {it,expect} from 'vitest'
import {inspectCadPairs} from '../src/services/cadInspection'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
const box=(name:string,min:number[],max:number[])=>{const brep=createBrepBox(min,max);return {id:name,name,brep,mesh:tessellateNurbsBrep(brep,1)}}
it('measures separated, touching, overlapping and contained solid meshes',()=>{
 const a=box('A',[0,0,0],[10,10,10])
 for(const [b,gap,volume] of [[box('gap',[13,0,0],[23,10,10]),3,0],[box('touch',[10,0,0],[20,10,10]),0,0],[box('overlap',[5,0,0],[15,10,10]),0,500],[box('inside',[1,1,1],[2,2,2]),0,1]] as const){
  const report=inspectCadPairs([a,b])[0]
  expect(report.a).toBe('A');expect(report.b).toBe(b.name)
  expect(report.gapMm).toBeCloseTo(gap,10);expect(report.overlapMm3).toBeCloseTo(volume,8)
  const reverse=inspectCadPairs([b,a])[0];expect(reverse.gapMm).toBeCloseTo(report.gapMm,10);expect(reverse.overlapMm3).toBeCloseTo(report.overlapMm3,8)
 }
})
it('finds diagonal corner minima and returns ordered pair reports without changing inputs',()=>{
 const bodies=[box('a',[0,0,0],[1,1,1]),box('b',[2,2,2],[3,3,3]),box('c',[4,4,4],[5,5,5])],before=JSON.stringify(bodies)
 const reports=inspectCadPairs(bodies)
 expect(reports.map(r=>[r.a,r.b])).toEqual([['a','b'],['a','c'],['b','c']])
 expect(reports[0].gapMm).toBeCloseTo(Math.sqrt(3),12);expect(reports[1].gapMm).toBeCloseTo(3*Math.sqrt(3),12)
 expect(JSON.stringify(bodies)).toBe(before)
})
it('refuses over-budget or malformed requests and recovers',()=>{
 const a=box('a',[0,0,0],[1,1,1]),b=box('b',[2,2,2],[3,3,3])
 const expanded={...a,mesh:{...a.mesh,indices:Array.from({length:1500},()=>[0,1,2]).flat()}}
 expect(()=>inspectCadPairs([expanded,expanded])).toThrow('two million')
 expect(()=>inspectCadPairs([a])).toThrow('at least two')
 expect(()=>inspectCadPairs([a,{...b,mesh:{positions:[0,0],indices:[]}}])).toThrow()
 expect(inspectCadPairs([a,b])[0].gapMm).toBeCloseTo(Math.sqrt(3),12)
})
