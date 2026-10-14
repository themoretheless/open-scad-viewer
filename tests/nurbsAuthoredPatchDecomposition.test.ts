import {expect,it} from 'vitest'
import {evaluateNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {previewProgressiveNurbsProfiles,type AuthoredProgressiveSweepOptions} from '../src/services/nurbsConstructors'

it('carries original-profile and retained decomposition error through the packaged Rust API',()=>{
 const profile:NurbsCurve={degree:1,knots:[0,...Array.from({length:33},(_,i)=>i),32],
  controlPoints:Array.from({length:33},(_,i)=>[1+i/32,(i%2)/64,0]),
  weights:Array.from({length:33},(_,i)=>i%2===0?1:2),periodic:false}
 const path:NurbsCurve={degree:1,knots:[2,2,5,5],controlPoints:[[0,0,0],[0,0,10]],weights:[1,1],periodic:false}
 const scale={degree:1,knots:[7,7,9,9],values:[1,1],weights:[1,1]}
 const twist={...scale,values:[.25*180/Math.PI,.25*180/Math.PI]}
 const options:AuthoredProgressiveSweepOptions={normal:[1,0,0],orientation:'authored',initialSections:3,maxSections:3,maxDeviation:.01,
  frameAxis:{...scale,values:[[0,0,1],[0,0,1]]},frameNormal:{...scale,values:[[1,0,0],[1,0,0]]}}
 const before=structuredClone({profile,path,scale,twist,options})
 const level=previewProgressiveNurbsProfiles([profile],path,scale,twist,options,3)
 expect(level.report).toMatchObject({accepted:true,continuousBound:true,roundingCertified:true,decompositionProducts:384,
  continuousErrorScope:'retained-patches-relative-to-original-profile-transport'})
 expect(level.patches).toHaveLength(32)
 const upper=level.report.continuousErrorUpper!
 expect(upper).toBeGreaterThan(0)
 expect(upper).toBeLessThan(1e-9)
 // Falsification samples supplement the Rust interval proof, not replace it.
 for(const [span,patch] of level.patches.entries()){
  const a=patch.knotsU[patch.degreeU]!,b=patch.knotsU[patch.controlPoints.length]!
  for(const f of [0,.375,1]){
   const p=evaluateNurbsCurve(profile,span+f).point
   for(const t of [0,.375,1]){
    const got=evaluateNurbsSurface(patch,a+(b-a)*f,t).point
    const expected=[p[0]*Math.cos(.25)-p[1]*Math.sin(.25),p[0]*Math.sin(.25)+p[1]*Math.cos(.25),10*t]
    expect(Math.hypot(...got.map((v,k)=>v-expected[k]!))).toBeLessThanOrEqual(upper)
   }
  }
 }
 expect({profile,path,scale,twist,options}).toEqual(before)
})
