import {it,expect} from 'vitest'
import {splitSolid} from '../src/services/directSolidTools'
import {createBrepBox,createBrepSphere,tessellateNurbsBrep,analyzeNurbsBrep} from '../src/services/geometry/brep'
import {bounds} from '../src/services/cadWorkbench'
import {extrudePolygonProfile,inspectPolygonMesh} from '../src/services/geometry/polygon'
const body=()=>{const brep=createBrepBox([0,0,0],[10,10,10]);return {id:'stock',name:'Stock',brep,mesh:tessellateNurbsBrep(brep,1)}}
it('splits retained B-rep into oriented positive and negative halves with matching display meshes',()=>{
 const input=body(),before=JSON.stringify(input)
 const [positive,negative]=splitSolid(input,[0,0,2],4)
 expect(positive.id).toBe('stock');expect(negative.id).toBe('stock-split')
 expect(positive.name).toBe('Stock · +');expect(negative.name).toBe('Stock · −')
 for(const [part,min,max] of [[positive,[0,0,4],[10,10,10]],[negative,[0,0,0],[10,10,4]]] as const){const b=bounds([part]);b.min.forEach((x,k)=>expect(x).toBeCloseTo(min[k],10));b.max.forEach((x,k)=>expect(x).toBeCloseTo(max[k],10))}
 for(const [part,volume] of [[positive,600],[negative,400]] as const){
  expect(analyzeNurbsBrep(part.brep!).signedVolumeMm3).toBeCloseTo(volume,8)
  expect(inspectPolygonMesh(part.mesh).signedVolumeMm3).toBeCloseTo(volume,8)
  expect(bounds([{...part,mesh:{positions:part.brep!.vertices.flatMap(v=>v.point),indices:[]}}])).toEqual(bounds([part]))
 }
 expect(JSON.stringify(input)).toBe(before)
 const again=splitSolid(positive,[0,0,1],7)
 again.forEach(p=>expect(analyzeNurbsBrep(p.brep!).signedVolumeMm3).toBeCloseTo(300,8))
})
it('preserves signed-distance semantics for oblique axes and admits equivalent extreme scales',()=>{
 const input=body(),parts=splitSolid(input,[1,1,1],8)
 expect(parts.reduce((s,p)=>s+analyzeNurbsBrep(p.brep!).signedVolumeMm3,0)).toBeCloseTo(1000,7)
 for(const [i,part] of parts.entries())for(const v of part.brep!.vertices){const d=v.point.reduce((a,b)=>a+b,0)/Math.sqrt(3)-8;expect(i===0?d>=-1e-8:d<=1e-8).toBe(true)}
 const standard=splitSolid(input,[0,0,1],4)
 for(const z of [Number.MIN_VALUE,Number.MAX_VALUE])expect(splitSolid(input,[0,0,z],4)).toEqual(standard)
})
it('refuses nonintersecting, malformed and unsupported curved splits without mesh fallback',()=>{
 const input=body(),before=JSON.stringify(input)
 for(const offset of [-1,0,10,11])expect(()=>splitSolid(input,[0,0,1],offset)).toThrow()
 expect(()=>splitSolid(input,[0,0,0],4)).toThrow('Zero direction')
 expect(JSON.stringify(input)).toBe(before)
 const brep=createBrepSphere(2),sphere={id:'sphere',name:'Sphere',brep,mesh:tessellateNurbsBrep(brep,4)},snapshot=JSON.stringify(sphere)
 expect(()=>splitSolid(sphere,[0,0,1],0)).toThrow()
 expect(JSON.stringify(sphere)).toBe(snapshot)
})
it('splits mesh bodies natively with signed-distance planes, conserved volume and atomic refusal',()=>{
 const {brep:_,...input}=body(),before=JSON.stringify(input)
 for(const [normal,offset,volumes] of [
  [[0,0,1],4,[600,400]],[[1,0,0],3,[700,300]],[[1,1,0],Math.SQRT2*5,[500,500]],
 ] as const){
  const parts=splitSolid(input,[...normal],offset)
  parts.forEach((part,i)=>{
   expect(part.brep).toBeUndefined()
   const report=inspectPolygonMesh(part.mesh)
   expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeCloseTo(volumes[i],6)
   for(let j=0;j<part.mesh.positions.length;j+=3){
    const d=normal.reduce((sum,x,k)=>sum+x*part.mesh.positions[j+k],0)/Math.hypot(...normal)-offset
    expect(i===0?d>=-1e-6:d<=1e-6).toBe(true)
   }
  })
 }
 const standard=splitSolid(input,[0,0,1],4)
 for(const z of [Number.MIN_VALUE,Number.MAX_VALUE])expect(splitSolid(input,[0,0,z],4)).toEqual(standard)
 for(const offset of [-1,0,10,11,Infinity])expect(()=>splitSolid(input,[0,0,1],offset)).toThrow()
 expect(()=>splitSolid(input,[0,0,0],4)).toThrow()
 expect(()=>splitSolid({...input,mesh:{positions:[],indices:[]}},[0,0,1],4)).toThrow()
 expect(JSON.stringify(input)).toBe(before)
 expect(splitSolid(input,[0,0,1],4)).toEqual(standard)
})

it('retains concave mesh splitting with independently known half volumes',()=>{
 const mesh=extrudePolygonProfile({outer:[[0,0],[20,0],[20,10],[10,10],[10,20],[0,20]]},[0,0,5])
 const input={id:'concave',name:'L',mesh},before=JSON.stringify(input)
 const parts=splitSolid(input,[1,0,0],15)
 ;[250,1250].forEach((volume,i)=>{
  const report=inspectPolygonMesh(parts[i].mesh)
  expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeCloseTo(volume,6)
 })
 expect(JSON.stringify(input)).toBe(before)
})
