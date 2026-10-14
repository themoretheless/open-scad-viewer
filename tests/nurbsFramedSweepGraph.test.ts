import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
import {evaluateNurbsSurface,type NurbsSurface} from '../src/services/nurbsSurface'
import {readFileSync} from 'node:fs'
const source=()=>readFileSync('examples/rush/framed-sweep.r','utf8')

it('retains a sampled construction report and never certifies continuous frame error',()=>{
 const graph=compileRushFrontend(source())
 const built=buildOwnNurbs(graph.document,{action:'build'})
 expect(built.report.construction?.[graph.document.nodes.find(n=>n.op==='framed_sweep')!.id]).toMatchObject({accepted:true,continuousBound:false,sections:16,stations:61,closedPath:false,seamContinuity:'open'})
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.definitions[graph.document.nodes.find(n=>n.op==='framed_sweep')!.id]).toMatchObject({kind:'surface'})
})

it('closes the represented circular frame sweep with a C0 seam and preserves section radii',()=>{
 const circle=`// @rush/1\nprofile=bezier_curve(points:[[12mm,0mm,0mm],[15mm,0mm,0mm]])\npath=circle_curve(center:[0mm,0mm,0mm],normal:[0,0,1],radius:12mm)\nshow framed_sweep(profile,path,normal:[1,0,0],sections:16,max_deviation:0.6mm)`
 const graph=compileRushFrontend(circle)
 const built=buildOwnNurbs(graph.document,{action:'build'})
 expect(built.report.construction?.[graph.document.nodes.find(n=>n.op==='framed_sweep')!.id]).toMatchObject({closedPath:true,seamContinuity:'C0',continuousBound:false})
 const s=built.report.definitions[graph.document.nodes.find(n=>n.op==='framed_sweep')!.id] as unknown as NurbsSurface
 expect(s.periodicV).toBe(true)
 for(const u of [0,.17,.5,.83,1]){
  const first=evaluateNurbsSurface(s,u,0).point,last=evaluateNurbsSurface(s,u,1).point
  for(let k=0;k<3;k++)expect(last[k]).toBeCloseTo(first[k]!,11)
  for(let j=0;j<16;j++){
   const p=evaluateNurbsSurface(s,u,j/15).point
   expect(Math.hypot(p[0],p[1])).toBeCloseTo(12+3*u,10)
  }
 }
 expect(built.report.error_bound_certified).toBe(false)
})

it('refuses insufficient refinement, an invalid frame and nonsmooth path tangents',()=>{
 expect(()=>buildOwnNurbs(compileRushFrontend(source().replace('0.25mm','0mm')).document,{action:'build'})).toThrow('sampled refinement')
 expect(()=>buildOwnNurbs(compileRushFrontend(source().replace('[1,0,0]','[0,0,0]')).document,{action:'build'})).toThrow()
 const kink=source().replace('bezier_curve(points: [[0mm,0mm,0mm],[0mm,12mm,8mm],[12mm,20mm,16mm]])','polyline_curve(points: [[0mm,0mm,0mm],[0mm,8mm,0mm],[8mm,8mm,0mm]])')
 expect(()=>buildOwnNurbs(compileRushFrontend(kink).document,{action:'build'})).toThrow('tangent discontinuities')
})

it('checks named frame controls, section count, physical units and piped profile syntax',()=>{
 const graph=compileRushFrontend(source())
 expect(graph.document.nodes.find(n=>n.op==='framed_sweep')).toMatchObject({inputs:graph.document.nodes.slice(0,2).map(n=>n.id),normal:[1,0,0],sections:16,max_deviation:.25})
 expect(()=>compileRushFrontend(source().replace('normal: [1,0,0],',''))).toThrow()
 expect(()=>compileRushFrontend(source().replace('sections: 16','sections: 33'))).toThrow()
 expect(()=>compileRushFrontend(source().replace('0.25mm','0.25deg'))).toThrow()
 const piped=compileRushFrontend(source().replace('framed_sweep(profile,path,','profile.framed_sweep(path,'))
 expect(piped.document.nodes.find(n=>n.op==='framed_sweep')).toMatchObject({inputs:piped.document.nodes.slice(0,2).map(n=>n.id)})
})
