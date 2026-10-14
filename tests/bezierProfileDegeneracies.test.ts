import {expect,it} from 'vitest'
import type {NurbsCurve} from '../src/services/nurbsCurve'
import {reverseNurbsCurve,trimNurbsCurve} from '../src/services/nurbsCurve'
import {authorBrepProfile,booleanBrepProfiles,normalizeBrepProfile,validateBrepProfile} from '../src/services/geometry/brepProfile'
import {pathFromCircle} from '../src/services/geometry/path2d'
const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
const cubic=(points:number[][]):NurbsCurve=>({degree:3,knots:[0,0,0,0,1,1,1,1],controlPoints:points,weights:[1,1,1,1],periodic:false})
it('admits monotone cubic carriers whose control polygons reverse',()=>{
 const c=cubic([[0,0],[1,1.1],[.9,.8],[1.9,2.1]])
 const region=(curve:NurbsCurve)=>{const a=curve.controlPoints[0],b=curve.controlPoints.at(-1)!;return [line([a[0],-1],[b[0],-1]),line([b[0],-1],b),reverseNurbsCurve(curve),line(a,[a[0],-1])]}
 const a=validateBrepProfile([region(c)]),b=validateBrepProfile([region(trimNurbsCurve(c,.25,.75))]),before=JSON.stringify([a,b])
 expect(a.areaMm2).toBeCloseTo(3.811,10)
 expect(normalizeBrepProfile(a.loops).areaMm2).toBeCloseTo(3.811,10)
 for(const [operation,area] of [['union',a.areaMm2],['intersection',b.areaMm2],['difference',a.areaMm2-b.areaMm2]] as const){
  const result=booleanBrepProfiles(a,b,operation)
  expect(result.areaMm2).toBeCloseTo(area,5);expect(result.loops.flat().some(c=>c.degree===3)).toBe(true)
 }
 expect(JSON.stringify([a,b])).toBe(before)
})
it('keeps curved partial coincidences through all three profile operations',()=>{
 const top=cubic([[0,0],[1,2],[2,1],[3,3]])
 const region=(c:NurbsCurve)=>{const a=c.controlPoints[0],b=c.controlPoints.at(-1)!;return [line([a[0],-2],[b[0],-2]),line([b[0],-2],b),reverseNurbsCurve(c),line(a,[a[0],-2])]}
 const a=validateBrepProfile([region(top)]),b=validateBrepProfile([region(trimNurbsCurve(top,.25,.75))]),before=JSON.stringify([a,b])
 for(const [op,area] of [['union',a.areaMm2],['intersection',b.areaMm2],['difference',a.areaMm2-b.areaMm2]] as const){
  const result=booleanBrepProfiles(a,b,op);expect(result.areaMm2).toBeCloseTo(area,5);expect(result.loops.flat().some(c=>c.degree===3)).toBe(true)
 }
 const normalized=normalizeBrepProfile([a.loops[0],b.loops[0]])
 expect(normalized.loops).toHaveLength(2);expect(normalized.areaMm2).toBeCloseTo(a.areaMm2-b.areaMm2,5)
 expect(JSON.stringify([a,b])).toBe(before)
})
it('cancels repeated curved passes and an exactly retraced spur with even-odd fill',()=>{
 const circle=authorBrepProfile({kind:'bezier',path:pathFromCircle([0,0],2)}),wire=circle.loops[0],before=JSON.stringify(circle)
 expect(normalizeBrepProfile([[...wire,...wire]]).loops).toHaveLength(0)
 const triple=normalizeBrepProfile([[...wire,...wire,...wire]])
 expect(triple.loops).toHaveLength(1);expect(triple.areaMm2).toBeCloseTo(circle.areaMm2,5)
 const spur=line(wire[0].controlPoints[0],[3,1])
 expect(normalizeBrepProfile([[...wire,spur,reverseNurbsCurve(spur)]]).areaMm2).toBeCloseTo(circle.areaMm2,5)
 expect(JSON.stringify(circle)).toBe(before)
})
it('normalizes interior cusps at dyadic and non-dyadic parameters as retained cubics',()=>{
 for(const [points,area] of [
  [[[3,-1],[-1,1],[-1,-1],[3,1]],2.4],
  [[[1,-1],[-1,2],[0,-4],[4,8]],2.7],
 ] as [number[][],number][]){
  const c=cubic(points),wire=[c,line(points[3],points[0])],before=JSON.stringify(wire)
  const result=normalizeBrepProfile([wire]);expect(result.loops).toHaveLength(1);expect(result.areaMm2).toBeCloseTo(area,5)
  expect(result.loops[0].filter(c=>c.degree===3)).toHaveLength(2);expect(JSON.stringify(wire)).toBe(before)
 }
})
it('orders self-touching cusp branches with zero first jets',()=>{
 const lobe=(sign:number)=>[cubic([[0,0],[0,0],[sign,0],[sign*2,4]]),line([sign*2,4],[sign*2,-4]),cubic([[sign*2,-4],[sign,0],[0,0],[0,0]])]
 for(const rotated of [false,true]){
  const wire=[...lobe(1),...lobe(-1)]
  if(rotated)for(const c of wire)for(const p of c.controlPoints){const [x,y]=p;p[0]=17+x-y;p[1]=-9+x+y}
  const before=JSON.stringify(wire),result=normalizeBrepProfile([wire])
  expect(result.loops).toHaveLength(2);expect(result.areaMm2).toBeCloseTo(rotated?22.4:11.2,5)
  // Rotation adds coordinate extrema, so the original carriers may be split further.
  expect(result.loops.flat().filter(c=>c.degree===3).length).toBeGreaterThanOrEqual(4)
  expect(JSON.stringify(wire)).toBe(before)
 }
})

it('cancels a straight cubic with nonlinear speed against the same line',()=>{
 const a=[cubic([[0,0],[.5,0],[2.5,0],[3,0]]),line([3,0],[3,2]),line([3,2],[0,2]),line([0,2],[0,0])]
 const b=[line([0,0],[3,0]),...a.slice(1)]
 expect(normalizeBrepProfile([a,b]).loops).toHaveLength(0)
 expect(normalizeBrepProfile([a,b,b]).areaMm2).toBeCloseTo(6,5)
})
it('retains a shared curved boundary when the two source spans use different injective axes',()=>{
 const c=cubic([[0,0],[6,-2],[11,-1],[7,11]]),ac=trimNurbsCurve(c,0,.625),bc=trimNurbsCurve(c,.375,1)
 const ap=ac.controlPoints[0],az=ac.controlPoints.at(-1)!,bp=bc.controlPoints[0],bz=bc.controlPoints.at(-1)!
 const a=validateBrepProfile([[line([ap[0],-3],[az[0],-3]),line([az[0],-3],az),reverseNurbsCurve(ac),line(ap,[ap[0],-3])]])
 const b=validateBrepProfile([[line([-2,bp[1]],bp),bc,line(bz,[-2,bz[1]]),line([-2,bz[1]],[-2,bp[1]])]])
 const union=booleanBrepProfiles(a,b,'union'),intersection=booleanBrepProfiles(a,b,'intersection'),difference=booleanBrepProfiles(a,b,'difference')
 expect(union.areaMm2+intersection.areaMm2).toBeCloseTo(a.areaMm2+b.areaMm2,5)
 expect(difference.areaMm2+intersection.areaMm2).toBeCloseTo(a.areaMm2,5)
 expect(union.loops).toHaveLength(2);expect(union.loops.every(wire=>wire.length>=3)).toBe(true)
 expect(union.loops.flat().some(c=>c.degree===3)).toBe(true)
})
