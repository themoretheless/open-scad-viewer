import assert from 'node:assert/strict'
import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {scaledSweepNurbsCurve,checkedProfileSweepNurbsSurface,bezierNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface,isoNurbsCurve} from '../src/services/nurbsSurface'
import {createCappedBrepLoftSurfaces} from '../src/services/sweep/construction/brep'
import {exportDirectStepV5,importDirectStepV5} from '../src/services/cadNurbsStep'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
const root=resolve(process.argv[2]??'/tmp/scalar-sweep-step');mkdirSync(root,{recursive:true})
const ring=[[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0]],profiles=ring.map((p,i)=>bezierNurbsCurve([p,ring[(i+1)%4]!]))
const path=bezierNurbsCurve([[0,0,0],[0,0,5]]),law={degree:1,knots:[10,10,14,14],values:[1,2],weights:[3,1]}
const cases=['scaled_sweep','profile_sweep'].map(op=>{
 const sides=profiles.map(profile=>op==='scaled_sweep'
  ?scaledSweepNurbsCurve(profile,{...path,knots:[2,2,7,7],weights:[1,2]},law,[0,0,0])
  :checkedProfileSweepNurbsSurface(profile,path,{...law,weights:[1,1]},[1,0,0],5,.001).surface!)
 assert(sides.every(Boolean));const starts=sides.map(s=>isoNurbsCurve(s,'v',0)),ends=sides.map(s=>isoNurbsCurve(s,'v',1))
 const model=createCappedBrepLoftSurfaces([starts],[ends],[sides]),text=exportDirectStepV5(model).text
 const imported=importDirectStepV5(text).model
 assert.equal(imported.faces.length,6);assert.equal(imported.bodies.length,1)
 const samples=sides.map((s,i)=>[0,.17,.5,.83,1].flatMap(u=>[0,.13,.5,.87,1].map(v=>{
  const r=op==='scaled_sweep'?(3-v)/(3-2*v):1+v,z=op==='scaled_sweep'?10*v/(1+v):5*v
  const a=ring[i]!,b=ring[(i+1)%4]!,expected=[r*(a[0]!+(b[0]!-a[0]!)*u),r*(a[1]!+(b[1]!-a[1]!)*u),z]
  const p=evaluateNurbsSurface(s,u,v).point;assert(Math.hypot(...p.map((x,j)=>x-expected[j]!))<1e-10)
  const q=evaluateNurbsSurface(imported.faces[i]!.surface,u,v).point;assert(Math.hypot(...p.map((x,j)=>x-q[j]!))<1e-9)
  return {u,v,point:p}
 })))
 const file=op+'.step';writeFileSync(resolve(root,file),text)
 return {op,file,sha256:createHash('sha256').update(text).digest('hex'),faces:6,solids:1,wallSamples:samples,nativeRoundTrip:true}
})
writeFileSync(resolve(root,'manifest.json'),JSON.stringify({schema:'scalar-sweep-step/1',artifactProvenance:sweepStepArtifactProvenance(),cases},null,2)+'\n')
console.log('Two scalar sweep native STEP round-trips passed:',root)
