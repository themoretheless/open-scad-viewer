import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateCatenoidNurbsPatches,approximateCatenoidNurbsSurface} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
it('matches independent catenoid radius and preserves full circle seam controls',()=>{
 const a=approximateCatenoidNurbsSurface([3,4,5],2,-2,3,1e-4)
 for(let i=0;i<=100;i++){
  const u=i/100,z=-2+5*u,radius=2*Math.cosh(z/2)
  for(const v of [0,.3,1,2.7,4]){
   const p=evaluateNurbsSurface(a.surface,u,v).point
   expect(Math.abs(Math.hypot(p[0]!-3,p[1]!-4)-radius)).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
   expect(p[2]).toBeCloseTo(5+z,10)
  }
 }
 a.surface.controlPoints.forEach(row=>expect(row.at(-1)).toEqual(row[0]))
 expect(()=>approximateCatenoidNurbsSurface([0,0,0],1,-5,5,1e-10)).toThrow()
})
it('checks catenoid scale and local Z length dimensions through packaged Rush',()=>{
 const source=readFileSync('examples/rush/catenoid-surface.r','utf8')
 expect(compileRushFrontend(source).document.nodes.find(n=>n.op==='catenoid_surface')).toMatchObject({scale:2,start_z:-2,end_z:3})
 for(const field of ['scale: 2mm','end_z: 3mm'])expect(()=>compileRushFrontend(source.replace(field,field.replace('mm','deg')))).toThrow()
 expect(()=>compileRushFrontend(source.replace(',scale: 2mm',''))).toThrow()
})
it('preserves approximate profile report despite a surface foundation certificate',()=>{
 const graph=compileRushFrontend(readFileSync('examples/rush/catenoid-surface.r','utf8'))
 const surface=graph.document.nodes.find(n=>n.op==='catenoid_surface')!
 const built=buildOwnNurbs({...graph.document,nodes:[surface],root:surface.id},{action:'build'})
 expect(built.report.foundation_certificate).not.toBeNull()
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[surface.id]).toMatchObject({continuousBound:false,roundingCertified:false,method:'catenary-profile-Hermite-rational-revolution'})
})

it('retains dense catenoid domains, radial accuracy and adjacent patch boundaries',()=>{
 const dense=approximateCatenoidNurbsSurface([3,4,5],1,-5,5,1e-4)
 for(let i=0;i<=100;i++)for(const v of [0,.3,1,2.7,4]){
  const u=i/100,z=-5+10*u,p=evaluateNurbsSurface(dense.surface,u,v).point
  expect(Math.abs(Math.hypot(p[0]!-3,p[1]!-4)-Math.cosh(z))).toBeLessThanOrEqual(dense.report.realArithmeticErrorEstimate+1e-10)
  expect(p[2]).toBeCloseTo(5+z,10)
 }
 const fit=approximateCatenoidNurbsPatches([3,4,5],1,-5,5,1e-4)
 expect(fit.patches).toHaveLength(fit.report.spans)
 expect(fit.patches.length).toBeGreaterThan(10)
 for(let i=0;i<fit.patches.length;i++){
  const patch=fit.patches[i]!,lo=i/fit.patches.length,hi=(i+1)/fit.patches.length
  expect(patch.knotsU[0]).toBe(lo);expect(patch.knotsU.at(-1)).toBe(hi)
  for(const t of [0,.13,.5,.87,1]){
   const u=lo+(hi-lo)*t,z=-5+10*u
   for(const v of [0,.3,1,2.7,4]){
    const p=evaluateNurbsSurface(patch,u,v).point
    expect(Math.abs(Math.hypot(p[0]!-3,p[1]!-4)-Math.cosh(z))).toBeLessThanOrEqual(fit.report.realArithmeticErrorEstimate+1e-10)
    expect(p[2]).toBeCloseTo(5+z,10)
    if(i&&t===0){
     const a=evaluateNurbsSurface(fit.patches[i-1]!,u,v).point
     expect(Math.hypot(...a.map((x,d)=>x-p[d]!))).toBeLessThan(1e-10)
    }
   }
  }
  patch.controlPoints.forEach(row=>expect(row.at(-1)).toEqual(row[0]))
 }
 expect(()=>approximateCatenoidNurbsPatches([0,0,0],1,-5,5,1e-10)).toThrow()
})
it('checks dense catenoid Rush dimensions and preserves approximation report',()=>{
 const source=readFileSync('examples/rush/catenoid-patches.r','utf8')
 const graph=compileRushFrontend(source)
 const node=graph.document.nodes.find(n=>n.op==='catenoid_patches')!
 expect(node).toMatchObject({scale:1,start_z:-5,end_z:5,max_deviation:1e-4})
 for(const field of ['scale: 1mm','end_z: 5mm'])expect(()=>compileRushFrontend(source.replace(field,field.replace('mm','deg')))).toThrow()
 const built=buildOwnNurbs(graph.document,{action:'build'})
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[node.id]).toMatchObject({budget:1e-4,continuousBound:false,roundingCertified:false,method:'catenary-profile-Hermite-rational-revolution'})
})
