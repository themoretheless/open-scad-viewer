import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {checkedPipeNurbsSurface,lineNurbsCurve,circleNurbsArc,circleNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
it('matches the cylinder equation between stations on a straight path',()=>{
 const fit=checkedPipeNurbsSurface(lineNurbsCurve([3,4,5],[3,4,12]),2,[1,0,0],8,1e-5)
 expect(fit.report.accepted).toBe(true)
 const surface=fit.surface!,lo=surface.knotsU[surface.degreeU]!,hi=surface.knotsU[surface.controlPoints.length]!
 for(let i=0;i<=100;i++)for(const v of [0,.13,.5,.87,1]){
  const p=evaluateNurbsSurface(surface,lo+(hi-lo)*i/100,v).point
  expect(Math.hypot(p[0]!-3,p[1]!-4)).toBeCloseTo(2,10)
  expect(p[2]).toBeCloseTo(5+7*v,10)
 }
})
it('keeps sampled refinement refusals and the closed cyclic seam',()=>{
 const path=circleNurbsArc([0,0,0],[0,0,1],5,0,90)
 const coarse=checkedPipeNurbsSurface(path,1,[0,0,1],3,.005)
 expect(coarse.report.accepted).toBe(false);expect(coarse.surface).toBeNull()
 const fine=checkedPipeNurbsSurface(path,1,[0,0,1],32,.005)
 expect(fine.report.accepted).toBe(true)
 expect(fine.report.sampledControlDeviation).toBeLessThan(coarse.report.sampledControlDeviation)
 const closed=checkedPipeNurbsSurface(circleNurbsCurve([0,0,0],[0,0,1],5),1,[0,0,1],32,.1)
 expect(closed.report).toMatchObject({accepted:true,closedPath:true,continuousBound:false,seamContinuity:'C0'})
 const s=closed.surface!,lo=s.knotsU[s.degreeU]!,hi=s.knotsU[s.controlPoints.length]!
 for(let i=0;i<=40;i++){
  const u=lo+(hi-lo)*i/40,a=evaluateNurbsSurface(s,u,0).point,b=evaluateNurbsSurface(s,u,1).point
  expect(Math.hypot(...a.map((x,d)=>x-b[d]!))).toBeLessThan(1e-11)
 }
})
it('checks pipe radius and budget dimensions in packaged Rush',()=>{
 const source=readFileSync('examples/rush/pipe-surface.r','utf8')
 expect(compileRushFrontend(source).document.nodes.find(n=>n.op==='pipe_surface')).toMatchObject({radius:1,sections:32,max_deviation:.005})
 expect(()=>compileRushFrontend(source.replace('radius: 1mm','radius: 1deg'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('max_deviation: 0.005mm','max_deviation: 0.005deg'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('sections: 32','sections: 32mm'))).toThrow()
 expect(()=>compileRushFrontend(source.replace(',max_deviation: 0.005mm',''))).toThrow()
})
it('keeps the sampled pipe report uncertified and rejects failed graph budgets',()=>{
 for(const name of ['pipe-surface','closed-pipe-surface']){
  const source=readFileSync(`examples/rush/${name}.r`,'utf8')
  const graph=compileRushFrontend(source),node=graph.document.nodes.find(n=>n.op==='pipe_surface')!
  const built=buildOwnNurbs(graph.document,{action:'build'})
  expect(built.report.error_bound_certified).toBe(false)
  expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,continuousBound:false,method:'double-reflection-fourfold-section-refinement'})
 }
 const graph=compileRushFrontend(readFileSync('examples/rush/pipe-surface.r','utf8').replace('sections: 32','sections: 3'))
 expect(()=>buildOwnNurbs(graph.document,{action:'build'})).toThrow(/Pipe sampled refinement/)
})
