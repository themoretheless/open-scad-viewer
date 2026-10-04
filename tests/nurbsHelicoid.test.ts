import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateHelicoidNurbsPatches,approximateHelicoidNurbsSurface} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
it('matches independent helicoid equation for axis and annular signed surfaces',()=>{
 for(const [inner,turns,height] of [[0,.25,3],[1,-.5,-4]]){
  const a=approximateHelicoidNurbsSurface([3,4,5],inner!,2,height!,turns!,31,1e-4)
  for(let i=0;i<=100;i++){
   const v=i/100,theta=31*Math.PI/180+2*Math.PI*turns!*v
   for(const u of [0,.13,.5,.87,1]){
    const r=inner!+(2-inner!)*u,p=evaluateNurbsSurface(a.surface,u,v).point
    expect(Math.hypot(p[0]!-3-r*Math.cos(theta),p[1]!-4-r*Math.sin(theta))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
    expect(p[2]).toBeCloseTo(5+height!*v,10)
   }
  }
 }
 expect(()=>approximateHelicoidNurbsSurface([0,0,0],0,2,3,2,0,1e-10)).toThrow()
 expect(()=>approximateHelicoidNurbsSurface([0,0,0],0,2,0,.25,0,1e-4)).toThrow()
})
it('checks helicoid length, angle and scalar dimensions through packaged Rush',()=>{
 const source=readFileSync('examples/rush/helicoid-surface.r','utf8')
 expect(compileModelGraphText(source).document.nodes.find(n=>n.op==='helicoid_surface')).toMatchObject({inner_radius:0,outer_radius:2,height:3,turns:.25,phase_degrees:31})
 for(const field of ['outer_radius: 2mm','height: 3mm'])expect(()=>compileModelGraphText(source.replace(field,field.replace('mm','deg')))).toThrow()
 expect(()=>compileModelGraphText(source.replace('turns: 0.25','turns: 0.25mm'))).toThrow()
 expect(()=>compileModelGraphText(source.replace('phase_degrees: 31deg','phase_degrees: 31mm'))).toThrow()
 expect(()=>compileModelGraphText(source.replace(',outer_radius: 2mm',''))).toThrow()
})
it('retains helicoid approximation report without an error certificate',()=>{
 const graph=compileModelGraphText(readFileSync('examples/rush/helicoid-surface.r','utf8'))
 const surface=graph.document.nodes.find(n=>n.op==='helicoid_surface')!
 const built=buildOwnNurbs({...graph.document,nodes:[surface],root:surface.id},{action:'build'})
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[surface.id]).toMatchObject({continuousBound:false,roundingCertified:false,method:'radial-ruled-helix-Hermite'})
})

it('retains dense helicoid patch domains and independent equation without refitting',()=>{
 const fit=approximateHelicoidNurbsPatches([3,4,5],1,2,-8,-2,31,1e-4)
 expect(fit.patches.length).toBe(fit.report.spans);expect(fit.patches.length).toBeGreaterThan(10)
 for(let i=0;i<fit.patches.length;i++){
  const patch=fit.patches[i]!,lo=i/fit.report.spans,hi=(i+1)/fit.report.spans
  expect(patch.knotsV[0]).toBe(lo);expect(patch.knotsV.at(-1)).toBe(hi)
  for(const t of [0,.13,.5,.87,1])for(const u of [0,.3,1]){
   const v=lo+(hi-lo)*t,theta=31*Math.PI/180-4*Math.PI*v,r=1+u
   const p=evaluateNurbsSurface(patch,u,v).point
   expect(Math.hypot(p[0]!-3-r*Math.cos(theta),p[1]!-4-r*Math.sin(theta))).toBeLessThanOrEqual(fit.report.realArithmeticErrorEstimate+1e-11)
   expect(p[2]).toBeCloseTo(5-8*v,10)
   if(i>0&&t===0){const a=evaluateNurbsSurface(fit.patches[i-1]!,u,v).point;expect(Math.hypot(...p.map((x,k)=>x-a[k]!))).toBeLessThan(1e-10)}
  }
 }
 expect(()=>approximateHelicoidNurbsPatches([0,0,0],1,2,8,2,0,1e-12)).toThrow()
})
it('retains dense helicoid reports through packaged Rush',()=>{
 const graph=compileModelGraphText(readFileSync('examples/rush/helicoid-patches.r','utf8'))
 const node=graph.document.nodes.find(n=>n.op==='helicoid_patches')!
 const built=buildOwnNurbs(graph.document,{action:'build'})
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[node.id]).toMatchObject({continuousBound:false,roundingCertified:false,method:'radial-ruled-helix-Hermite'})
 expect(()=>compileModelGraphText(readFileSync('examples/rush/helicoid-patches.r','utf8').replace('turns: 2','turns: 2mm'))).toThrow()
})
