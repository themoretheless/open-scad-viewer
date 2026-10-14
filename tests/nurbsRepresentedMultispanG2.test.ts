import {expect,it} from 'vitest'
import {inspectSweepExactStripJets,inspectSweepProjectiveStripJets} from '../src/services/nurbsSweepAudit'
import {bezierNurbsCurve,inspectProgressiveRetainedStationSeams} from '../src/services/nurbsConstructors'
import type {NurbsSurface} from '../src/services/nurbsSurface'

const next=(x:number,steps:number)=>{
 const view=new DataView(new ArrayBuffer(8));view.setFloat64(0,x)
 view.setBigUint64(0,view.getBigUint64(0)+BigInt(steps));return view.getFloat64(0)
}
const seam=next(28,2),points=[27,next(27.5,1),seam,next(28.5,1),29]
const surface=(y:number):NurbsSurface=>({degreeU:2,degreeV:1,
 knotsU:[0,0,0,.5,.5,1,1,1],knotsV:[0,0,1,1],
 controlPoints:points.map(x=>[[x,y,0],[x,y+1,0]]),weights:points.map(()=>[1,1]),periodicU:false,periodicV:false})

it('proves actual full-multiplicity chain G2 at represented binary speeds without moving poles',()=>{
 const a=surface(0),b=surface(1),before=structuredClone([a,b])
 const proof=inspectSweepProjectiveStripJets(a,b,'vMax','vMin',2,1,1000000)
 expect(proof).toMatchObject({certified:true,exactIdentity:true,regularityCertified:true,certifiedOrder:2})
 expect(inspectSweepProjectiveStripJets(a,b,'vMax','vMin',2,1,proof.work-1).certified).toBe(false)
 expect(inspectSweepProjectiveStripJets(a,b,'vMax','vMin',2,1,0).certified).toBe(false)
 const changed=structuredClone([a,b]);for(const s of changed)for(const p of s.controlPoints[3]!)p[2]=Number.EPSILON
 expect(inspectSweepProjectiveStripJets(changed[0]!,changed[1]!,'vMax','vMin',2,1,1000000).certified).toBe(false)
 expect([a,b]).toEqual(before)
})

it('keeps geometric G2 of unequal linear knot spans separate from strict homogeneous jets',()=>{
 const linear=(y:number):NurbsSurface=>({degreeU:1,degreeV:1,knotsU:[0,0,.25,1,1],knotsV:[0,0,1,1],
  controlPoints:[0,.5,1].map(x=>[[x,y,0],[x,y+1,0]]),weights:[[1,1],[1,1],[1,1]],periodicU:false,periodicV:false})
 const a=linear(0),b=linear(1)
 const proof=inspectSweepProjectiveStripJets(a,b,'vMax','vMin',2,1,1000000)
 expect(proof.certified).toBe(true)
 expect(inspectSweepExactStripJets(a,b,'vMax','vMin',2,1,1000000).certified).toBe(false)
 expect(inspectSweepProjectiveStripJets(a,b,'vMax','vMin',2,1,proof.work-1).certified).toBe(false)
 const changed=structuredClone([a,b]);for(const s of changed)for(const p of s.controlPoints[1]!)p[2]=Number.EPSILON
 expect(inspectSweepProjectiveStripJets(changed[0]!,changed[1]!,'vMax','vMin',2,1,1000000).certified).toBe(false)
})

it('carries represented binary speeds through retained progressive station and profile-chain audits',()=>{
 const chain={degree:2,knots:[0,0,0,.5,.5,1,1,1],controlPoints:points.map(x=>[x,0,0]),weights:points.map(()=>1),periodic:false}
 const path={degree:1,knots:[0,0,.25,.5,.75,1,1],controlPoints:[0,27,seam,29,34].map(z=>[0,0,z]),weights:Array(5).fill(1),periodic:false}
 const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]},twist={...scale,values:[0,0]}
 const options={orientation:'fixed' as const,normal:[1,0,0] as [number,number,number],initialSections:5,maxSections:5,maxDeviation:1}
 const before=structuredClone({chain,path})
 for(const profile of [bezierNurbsCurve([[1,0,0],[2,0,0]]),chain]){
  const proof=inspectProgressiveRetainedStationSeams([profile],path,scale,twist,options,5,2,1000000)
  expect(proof).toMatchObject({allStationSeamsCertified:true,sourceFrameSmoothnessCertified:false,solidCertified:false})
  expect(proof.seams).toHaveLength(3)
  expect(inspectProgressiveRetainedStationSeams([profile],path,scale,twist,options,5,2,proof.exactWork-1).allStationSeamsCertified).toBe(false)
 }
 expect({chain,path}).toEqual(before)
})
