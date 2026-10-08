import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {checkedVariablePipeNurbsSurface,lineNurbsCurve,bezierNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
it('preserves the linear cone radius between authored stations',()=>{
 const result=checkedVariablePipeNurbsSurface(lineNurbsCurve([0,0,0],[0,0,10]),lineNurbsCurve([1,0,0],[3,0,0]),[1,0,0],8,1e-8)
 expect(result.report).toMatchObject({accepted:true,continuousBound:false})
 const s=result.surface!,lo=s.knotsU[s.degreeU]!,hi=s.knotsU[s.controlPoints.length]!
 for(let i=0;i<=100;i++)for(let j=0;j<=40;j++){
  const v=i/100,p=evaluateNurbsSurface(s,lo+(hi-lo)*j/40,v).point
  expect(Math.hypot(p[0]!,p[1]!)).toBeCloseTo(1+2*v,10)
  expect(p[2]).toBeCloseTo(10*v,10)
 }
})
it('retains sampled diagnostics and rejects coarse variable pipe graphs',()=>{
 const source=readFileSync('examples/rush/variable-pipe-surface.r','utf8')
 const graph=compileRushFrontend(source),n=graph.document.nodes.find(n=>n.op==='variable_pipe_surface')!
 const result=buildOwnNurbs(graph.document,{action:'build'})
 expect(result.report.error_bound_certified).toBe(false)
 expect(result.report.construction?.[n.id]).toMatchObject({accepted:true,continuousBound:false})
 expect(()=>buildOwnNurbs(compileRushFrontend(source.replace('sections: 32','sections: 3')).document,{action:'build'})).toThrow(/Variable pipe sampled refinement/)
 expect(()=>compileRushFrontend(source.replace('sections: 32','sections: 32mm'))).toThrow()
 expect(()=>compileRushFrontend(source.replace(',max_deviation: 0.02mm',''))).toThrow()
})

it('retains the weighted closed radius law and cyclic seam',()=>{
 const graph=compileRushFrontend(readFileSync('examples/rush/closed-variable-pipe-surface.r','utf8'))
 const n=graph.document.nodes.find(n=>n.op==='variable_pipe_surface')!
 const result=buildOwnNurbs(graph.document,{action:'build'})
 expect(result.report.construction?.[n.id]).toMatchObject({accepted:true,closedPath:true,continuousBound:false,seamContinuity:'C0'})
})

it('matches an independent weighted radius equation at section stations',()=>{
 const law=bezierNurbsCurve([[1,0,0],[2,0,0],[1,0,0]],[1,2,1])
 const result=checkedVariablePipeNurbsSurface(lineNurbsCurve([0,0,0],[0,0,10]),law,[1,0,0],32,.01)
 expect(result.report.accepted).toBe(true)
 const s=result.surface!,lo=s.knotsU[s.degreeU]!,hi=s.knotsU[s.controlPoints.length]!
 for(let i=0;i<32;i++)for(let j=0;j<=40;j++){
  const v=i/31,q=v*(1-v),radius=(1+6*q)/(1+2*q)
  const p=evaluateNurbsSurface(s,lo+(hi-lo)*j/40,v).point
  expect(Math.hypot(p[0]!,p[1]!)).toBeCloseTo(radius,10)
 }
})
