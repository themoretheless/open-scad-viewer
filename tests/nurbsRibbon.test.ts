import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {checkedRibbonNurbsSurface,lineNurbsCurve,circleNurbsArc} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
it('matches straight ribbon boundaries and projected orientation between stations',()=>{
 const path=lineNurbsCurve([3,4,5],[3,4,15]),width=lineNurbsCurve([2,0,0],[4,0,0])
 for(const sign of [-1,1]){
  const fit=checkedRibbonNurbsSurface(path,width,[sign,0,2],8,1e-8)
  expect(fit.report).toMatchObject({accepted:true,continuousBound:false})
  for(let i=0;i<=100;i++)for(const u of [0,.13,.5,.87,1]){
   const v=i/100,p=evaluateNurbsSurface(fit.surface!,u,v).point
   expect(p[0]).toBeCloseTo(3+sign*(u-.5)*(2+2*v),10)
   expect(p[1]).toBeCloseTo(4,10);expect(p[2]).toBeCloseTo(5+10*v,10)
  }
 }
})
it('rejects coarse budgets, parallel initial directions and nonpositive widths',()=>{
 const path=circleNurbsArc([0,0,0],[0,0,1],5,0,90),width=lineNurbsCurve([1,0,0],[2,0,0])
 const coarse=checkedRibbonNurbsSurface(path,width,[0,0,1],3,.01)
 expect(coarse.report.accepted).toBe(false);expect(coarse.surface).toBeNull()
 expect(checkedRibbonNurbsSurface(path,width,[0,0,1],32,.01).report.accepted).toBe(true)
 expect(()=>checkedRibbonNurbsSurface(path,width,[0,1,0],32,.01)).toThrow()
 expect(()=>checkedRibbonNurbsSurface(path,lineNurbsCurve([0,0,0],[1,0,0]),[0,0,1],32,.01)).toThrow()
})
it('checks ribbon dimensions and required fields through packaged Rush',()=>{
 const source=readFileSync('examples/rush/ribbon-surface.r','utf8')
 expect(compileModelGraphText(source).document.nodes.find(n=>n.op==='ribbon_surface')).toMatchObject({sections:32,max_deviation:.01})
 for(const [from,to] of [['sections: 32','sections: 32mm'],['max_deviation: 0.01mm','max_deviation: 0.01deg'],[',max_deviation: 0.01mm',''],['width_law: width,','']])expect(()=>compileModelGraphText(source.replace(from!,to!))).toThrow()
})
it('keeps open and weighted closed reports uncertified through graph construction',()=>{
 for(const name of ['ribbon-surface','closed-ribbon-surface']){
  const graph=compileModelGraphText(readFileSync(`examples/rush/${name}.r`,'utf8')),n=graph.document.nodes.find(n=>n.op==='ribbon_surface')!
  const built=buildOwnNurbs(graph.document,{action:'build'})
  expect(built.report.error_bound_certified).toBe(false)
  expect(built.report.construction?.[n.id]).toMatchObject({accepted:true,continuousBound:false,closedPath:name.startsWith('closed'),method:'double-reflection-fourfold-section-refinement'})
 }
 const coarse=compileModelGraphText(readFileSync('examples/rush/ribbon-surface.r','utf8').replace('sections: 32','sections: 3'))
 expect(()=>buildOwnNurbs(coarse.document,{action:'build'})).toThrow(/Ribbon sampled refinement/)
})
