import {expect,it} from 'vitest'
import {checkedProfileSweepNurbsSurface,bezierNurbsCurve} from '../src/services/nurbsConstructors'
import type {NurbsCurve} from '../src/services/nurbsCurve'
// Authored inputs only: frame premises and all certificates execute in Rust.
const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
it('delivers an exact tilted-plane Bishop premise and a bounded work refusal through WASM',()=>{
 const profile=bezierNurbsCurve([[1,-1,0],[2,-2,0]])
 const path=bezierNurbsCurve([[0,0,0],[.125,.125,2],[0,0,4]])
 const result=checkedProfileSweepNurbsSurface(profile,path,law,[1,-1,0],9,.01)
 expect(result.report.continuousCertificate).toMatchObject({method:'interval-planar-bishop-frame',withinBudget:true})
 const limited=checkedProfileSweepNurbsSurface(profile,path,law,[1,-1,0],9,.01,1)
 expect(limited.surface).toBeNull()
 expect(limited.report).toMatchObject({accepted:false,continuousBound:false})
})
it('proves authored oblique multi-span tangents and refuses a changed source through WASM',()=>{
 const profile=bezierNurbsCurve([[1,0,0],[2,0,0]])
 const path:NurbsCurve={degree:2,knots:[0,0,0,.5,.5,1,1,1],controlPoints:[[0,0,0],[0,.015625,1],[0,.03125,2],[0,.046875,3],[0,0,4]],weights:[1,1,1,1,1]}
 const result=checkedProfileSweepNurbsSurface(profile,path,law,[1,0,0],9,.01)
 expect(result.report.continuousCertificate).toMatchObject({method:'interval-planar-bishop-frame',withinBudget:true})
 const changed=structuredClone(path);changed.controlPoints[3]![1]!+=Number.EPSILON
 const refused=checkedProfileSweepNurbsSurface(profile,changed,law,[1,0,0],9,.01)
 expect(refused.surface).toBeNull()
 expect(refused.report.continuousCertificate).toMatchObject({withinBudget:false,reason:'continuous-path-tangent-unproved'})
})
