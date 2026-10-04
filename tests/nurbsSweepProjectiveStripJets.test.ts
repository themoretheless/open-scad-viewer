import {expect,it} from 'vitest'
import {inspectSweepExactStripJets,inspectSweepProjectiveStripJets,inspectSweepProjectiveSeams,type SweepSeamDeclaration} from '../src/services/nurbsSweepAudit'
import type {NurbsSurface} from '../src/services/nurbsSurface'
const quarter=(turn:number):NurbsSurface=>({degreeU:1,degreeV:2,knotsU:[0,0,1,1],knotsV:[0,0,0,1,1,1],
 controlPoints:[0,1].map(z=>[[1,0],[1,1],[0,1]].map(([a,b])=>{let x=a!,y=b!;for(let i=0;i<turn;i++)[x,y]=[-y,x];return [x,y,z]})),
 weights:Array.from({length:2},()=>[1,Math.SQRT1_2,1]),periodicU:false,periodicV:false})
it('certifies all four rational G2 seams including closure, preserving strict homogeneous semantics',()=>{
 const patches=[0,1,2,3].map(quarter),before=structuredClone(patches)
 for(let i=0;i<4;i++){
  const a=patches[i]!,b=patches[(i+1)%4]!
  expect(inspectSweepExactStripJets(a,b,'vMax','vMin',2,1,1000000).certified).toBe(false)
  const report=inspectSweepProjectiveStripJets(a,b,'vMax','vMin',2,1,1000000)
  expect(report).toMatchObject({certified:true,exactIdentity:true,regularityCertified:true,certifiedOrder:2,method:'constant-projective-strip-jets'})
  expect(inspectSweepProjectiveStripJets(a,b,'vMax','vMin',2,1,report.work-1).certified).toBe(false)
 }
 expect(patches).toEqual(before)
})
it('allows proportional weights and refuses changed curvature, singularity and exhausted work',()=>{
 const a=quarter(0),b=quarter(1)
 const audit=(b:NurbsSurface,order:1|2=2,work=1000000)=>inspectSweepProjectiveStripJets(a,b,'vMax','vMin',order,1,work)
 const scaled=structuredClone(b);for(const row of scaled.weights)for(let i=0;i<row.length;i++)row[i]!*=2
 expect(audit(scaled).certified).toBe(true)
 const changed=structuredClone(b);for(const row of changed.controlPoints)row[2]![1]=.125
 expect(audit(changed,1).certified).toBe(true)
 expect(audit(changed).certified).toBe(false)
 expect(audit(b,2,0)).toMatchObject({certified:false,certifiedOrder:null,work:0})
 const singular=structuredClone([a,b]);for(const s of singular)s.controlPoints[1]=structuredClone(s.controlPoints[0]!)
 expect(inspectSweepProjectiveStripJets(singular[0]!,singular[1]!,'vMax','vMin',2,1,1000000)).toMatchObject({certified:false,exactIdentity:true,regularityCertified:false})
})
it('supports a common rational multispan basis, transposed axes and an affine moving strip',()=>{
 const multi=(turn:number):NurbsSurface=>{
  const s=quarter(turn),arc=s.controlPoints[0]!
  return {...s,degreeU:3,knotsU:[0,0,0,0,.5,1,1,1,1],
   controlPoints:[0,.125,.5,.875,1].map(z=>arc.map(([x,y])=>[2*x!+z,y!+z,2*z])),
   weights:[1,2,4,2,1].map(w=>[w,w*Math.SQRT1_2,w])}
 }
 const a=multi(0),b=multi(1)
 expect(inspectSweepProjectiveStripJets(a,b,'vMax','vMin',2,1,1000000).certified).toBe(true)
 const transpose=(s:NurbsSurface):NurbsSurface=>({...s,degreeU:s.degreeV,degreeV:s.degreeU,knotsU:s.knotsV,knotsV:s.knotsU,
  controlPoints:s.controlPoints[0]!.map((_,j)=>s.controlPoints.map(row=>row[j]!)),weights:s.weights[0]!.map((_,j)=>s.weights.map(row=>row[j]!))})
 expect(inspectSweepProjectiveStripJets(transpose(a),transpose(b),'uMax','uMin',2,1,1000000).certified).toBe(true)
 const wrong=structuredClone(b);wrong.knotsU[4]=.25
 expect(inspectSweepProjectiveStripJets(a,wrong,'vMax','vMin',2,1,1000000)).toMatchObject({certified:false,work:0})
})

it('requires every declared cyclic seam and shares exact work including closure',()=>{
 const patches=[0,1,2,3].map(quarter)
 const seams:SweepSeamDeclaration[]=patches.map((_,i)=>({patches:[i,(i+1)%4],boundaries:['vMax','vMin'],order:2,normalScale:1,jetTolerance:1e9}))
 const before=structuredClone({patches,seams})
 const report=inspectSweepProjectiveSeams(patches,seams,1000000)
 expect(report).toMatchObject({exactG1G2Certified:true,certifiedOrder:2,unresolvedSeams:[]})
 const openWork=report.seams.slice(0,3).reduce((sum,s)=>sum+s.work,0)
 expect(inspectSweepProjectiveSeams(patches,seams,openWork)).toMatchObject({exactG1G2Certified:false,certifiedOrder:null,exactWork:openWork,unresolvedSeams:[3]})
 const changed=structuredClone(patches);for(const row of changed[0]!.controlPoints)row[2]![1]=.125
 expect(inspectSweepProjectiveSeams(changed,seams,1000000).exactG1G2Certified).toBe(false)
 expect(inspectSweepProjectiveSeams(patches,[],100)).toMatchObject({exactG1G2Certified:false,certifiedOrder:null,exactWork:0})
 expect(()=>inspectSweepProjectiveSeams(patches,seams,-1)).toThrow()
 expect(()=>inspectSweepProjectiveSeams(patches,[{...seams[0]!,patches:[0,4]}],100)).toThrow()
 expect({patches,seams}).toEqual(before)
})
it('audits actual progressive sweep profile joins with affine laws on a shared station grid',async()=>{
 const {progressiveSweepNurbsProfiles}=await import('../src/services/nurbsConstructors')
 const profiles=[0,1,2,3].map(turn=>{const s=quarter(turn);return {degree:2,knots:[0,0,0,1,1,1],controlPoints:s.controlPoints[0]!,weights:[1,Math.SQRT1_2,1],periodic:false}})
 const path={degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[0,0,8]],weights:[1,1],periodic:false}
 const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
 const built=progressiveSweepNurbsProfiles(profiles,path,law(1),law(0),{orientation:'fixed',normal:[1,0,0],initialSections:3,maxSections:3,maxDeviation:1e-8,
  axisScale:{degree:1,knots:[0,0,1,1],values:[[2,1,1],[2,1,1]],weights:[1,1]},
  centerLaw:{degree:1,knots:[0,0,1,1],values:[[0,0,0],[.5,0,0]],weights:[1,1]}})
 expect(built.report.accepted).toBe(true)
 const patches=built.patches!,ranges=built.profilePatchRanges!
 expect(ranges).toHaveLength(4)
 const seams:SweepSeamDeclaration[]=[]
 for(let profile=0;profile<4;profile++){
  const range=ranges[profile]!,next=ranges[(profile+1)%4]!
  expect(range[1]-range[0]).toBe(next[1]-next[0])
  for(let i=0;i<range[1]-range[0];i++)seams.push({patches:[range[0]+i,next[0]+i],boundaries:['uMax','uMin'],order:2,normalScale:1,jetTolerance:0})
 }
 const audit=inspectSweepProjectiveSeams(patches,seams,1000000)
 expect(audit).toMatchObject({exactG1G2Certified:true,certifiedOrder:2,unresolvedSeams:[]})
 const kinked=structuredClone(patches)
 for(const surface of kinked)for(const row of surface.controlPoints)row[1]![2]+=0.5
 const refused=inspectSweepProjectiveSeams(kinked,seams,1000000)
 expect(refused.exactG1G2Certified).toBe(false)
 expect(refused.seams.every(seam=>seam.reason==='linear-along-jet-smoothness-unproved')).toBe(true)
 expect(inspectSweepProjectiveSeams(patches,seams,audit.exactWork-1).exactG1G2Certified).toBe(false)
 expect(built.report.continuousBound).toBe(false)
})
