import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {checkedProfileSweepNurbsSurface,lineNurbsCurve,bezierNurbsCurve,circleNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'

it('matches an independent rational profile and scale equation on a straight path',()=>{
 const profile=bezierNurbsCurve([[2,0,0],[2,2,1],[0,2,2]],[1,2,1])
 const sweep=checkedProfileSweepNurbsSurface(profile,lineNurbsCurve([0,0,0],[0,0,10]),{degree:1,knots:[2,2,6,6],values:[1,2],weights:[1,1]},[1,0,0],8,1e-8)
 expect(sweep.report).toMatchObject({accepted:true,continuousBound:false,closedPath:false})
 for(let i=0;i<=40;i++)for(const v of [0,.13,.5,.87,1]){
  const u=i/40,a=(1-u)**2,b=4*u*(1-u),c=u*u,d=a+b+c
  const q=[(2*a+2*b)/d,(2*b+2*c)/d,(b+2*c)/d]
  const p=evaluateNurbsSurface(sweep.surface!,u,v).point
  q.forEach((x,k)=>expect(p[k]).toBeCloseTo((1+v)*x+(k===2?10*v:0),10))
 }
 expect(sweep.surface!.weights.map(row=>row[0])).toEqual(profile.weights)
})

it('retains curved construction diagnostics and refuses unmet budgets',()=>{
 const source=readFileSync('examples/rush/profile-sweep.r','utf8')
 const graph=compileRushFrontend(source)
 const sweep=graph.document.nodes.find(n=>n.op==='profile_sweep')!
 const result=buildOwnNurbs(graph.document,{action:'build'})
 expect(result.report.construction?.[sweep.id]).toMatchObject({accepted:true,continuousBound:false,sections:32})
 expect(result.report.error_bound_certified).toBe(false)
 expect(()=>buildOwnNurbs(compileRushFrontend(source.replace('sections: 32','sections: 3')).document,{action:'build'})).toThrow(/Profile sweep sampled refinement/)
 for(const invalid of [source.replace('values: [1,2]','values: [1,2mm]'),source.replace('sections: 32','sections: 32mm'),source.replace('normal: [0,0,1]','normal: [0,0,1mm]'),source.replace('weights: [1,1]','weights: [1,1],typo: 2')])expect(()=>compileRushFrontend(invalid)).toThrow()
})

it('retains a cyclic seam and rejects incompatible closed scale endpoints',()=>{
 const source=readFileSync('examples/rush/closed-profile-sweep.r','utf8')
 const graph=compileRushFrontend(source),sweep=graph.document.nodes.find(n=>n.op==='profile_sweep')!
 const result=buildOwnNurbs(graph.document,{action:'build'})
 expect(result.report.construction?.[sweep.id]).toMatchObject({accepted:true,closedPath:true,seamContinuity:'C0',continuousBound:false})
 const profile=lineNurbsCurve([5,0,-1],[5,0,1]),path=circleNurbsCurve([0,0,0],[0,0,1],5)
 expect(()=>checkedProfileSweepNurbsSurface(profile,path,{degree:1,knots:[0,0,1,1],values:[1,2],weights:[1,1]},[0,0,1],32,.2)).toThrow(/Closed section scale/)
 expect(()=>checkedProfileSweepNurbsSurface(profile,path,{degree:1,knots:[0,0,1,1],values:[0,1],weights:[1,1]},[0,0,1],32,.2)).toThrow(/positive/)
})
