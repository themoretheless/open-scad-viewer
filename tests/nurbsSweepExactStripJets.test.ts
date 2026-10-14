import {expect,it} from 'vitest'
import {inspectSweepExactStripJets,inspectSweepExactSeams,type SweepSeamDeclaration} from '../src/services/nurbsSweepAudit'
import type {NurbsSurface} from '../src/services/nurbsSurface'
const plane=(y:number):NurbsSurface=>({degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[0,1].map(x=>[[x,y,0],[x,y+1,0]]),weights:[[1,1],[1,1]],periodicU:false,periodicV:false})
it('includes the closing seam in exact cyclic C1 qualification and refuses C2 or a damaged closure',()=>{
 const base=[[1,0],[1,1],[1,1],[0,1]]
 const patches:NurbsSurface[]=Array.from({length:4},(_,turn)=>{
  const poles=base.map(([x,y])=>{
   for(let i=0;i<turn;i++)[x,y]=[-y!,x!]
   return [x!,y!]
  })
  return {degreeU:1,degreeV:3,knotsU:[0,0,1,1],knotsV:[0,0,0,0,1,1,1,1],controlPoints:[0,1].map(z=>poles.map(([x,y])=>[x!,y!,z])),weights:[[1,1,1,1],[1,1,1,1]],periodicU:false,periodicV:false}
 })
 const seams:SweepSeamDeclaration[]=patches.map((_,i)=>({patches:[i,(i+1)%4],boundaries:['vMax','vMin'],order:1,normalScale:1,jetTolerance:1e-8}))
 const positive=inspectSweepExactSeams(patches,seams,1000000)
 expect(positive).toMatchObject({exactG1G2Certified:true,certifiedOrder:1,unresolvedSeams:[]})
 expect(positive.seams).toHaveLength(4)
 expect(inspectSweepExactSeams(patches,seams.map(seam=>({...seam,order:2})),1000000)).toMatchObject({exactG1G2Certified:false,certifiedOrder:null,unresolvedSeams:[0,1,2,3]})
 const changed=structuredClone(patches)
 changed[3]!.controlPoints[0]![2]![1]+=Number.EPSILON
 expect(inspectSweepExactSeams(changed,seams,1000000)).toMatchObject({exactG1G2Certified:false,unresolvedSeams:[3]})
 const openWork=positive.seams.slice(0,3).reduce((sum,seam)=>sum+seam.work,0)
 expect(inspectSweepExactSeams(patches,seams,openWork)).toMatchObject({exactG1G2Certified:false,unresolvedSeams:[3],exactWork:openWork})
})
it('certifies the complete declared seam set only within a shared exact budget',()=>{
 const patches=[plane(0),plane(1),plane(2)]
 const seams:SweepSeamDeclaration[]=[0,1].map(i=>({patches:[i,i+1],boundaries:['vMax','vMin'],order:2,normalScale:1,jetTolerance:1e-8}))
 const before=structuredClone({patches,seams})
 const audit=inspectSweepExactSeams(patches,seams,1000000)
 expect(audit).toMatchObject({exactG1G2Certified:true,certifiedOrder:2,unresolvedSeams:[]})
 expect(inspectSweepExactSeams(patches,[seams[0]!,{...seams[1]!,order:1}],1000000)).toMatchObject({exactG1G2Certified:true,certifiedOrder:1})
 expect(audit.exactWork).toBe(audit.seams.reduce((sum,s)=>sum+s.work,0))
 const exhausted=inspectSweepExactSeams(patches,seams,audit.seams[0]!.work)
 expect(exhausted).toMatchObject({exactG1G2Certified:false,exactWork:audit.seams[0]!.work,unresolvedSeams:[1]})
 expect(exhausted.seams[1]).toMatchObject({certified:false,work:0})
 const changed=structuredClone(patches);changed[2]!.controlPoints[0]![1]![1]+=4*Number.EPSILON
 expect(inspectSweepExactSeams(changed,seams,1000000)).toMatchObject({exactG1G2Certified:false,unresolvedSeams:[1]})
 expect(inspectSweepExactSeams(patches,[],100)).toMatchObject({exactG1G2Certified:false,exactWork:0})
 expect(()=>inspectSweepExactSeams(patches,seams,-1)).toThrow()
 expect(()=>inspectSweepExactSeams(patches,[{...seams[0]!,patches:[0,9]}],100)).toThrow()
 expect({patches,seams}).toEqual(before)
})
it('certifies exact represented C2 and refuses one ULP, zero work and singular strips',()=>{
 const a=plane(0),b=plane(1),before=structuredClone([a,b])
 const inspect=(x:NurbsSurface,y:NurbsSurface,work=1000000)=>inspectSweepExactStripJets(x,y,'vMax','vMin',2,1,work)
 const positive=inspect(a,b)
 expect(positive).toMatchObject({certified:true,exactIdentity:true,regularityCertified:true,reason:'exact-regular-bezier-strip-jets'})
 expect(positive.work).toBeGreaterThan(0)
 expect(positive.work).toBeLessThanOrEqual(1000000)
 expect(inspect(a,b,0)).toMatchObject({certified:false,exactIdentity:false,work:0})
 const changed=structuredClone(b);changed.controlPoints[0]![1]![1]+=2*Number.EPSILON
 expect(inspect(a,changed)).toMatchObject({certified:false,exactIdentity:false,reason:'homogeneous-jets-different'})
 const singular=structuredClone([a,b]);for(const s of singular)s.controlPoints[1]=structuredClone(s.controlPoints[0]!)
 expect(inspect(singular[0]!,singular[1]!)).toMatchObject({certified:false,exactIdentity:true,regularityCertified:false})
 expect([a,b]).toEqual(before)
})
it('refuses unsupported multispan basis and invalid exact budgets',()=>{
 const a=plane(0),b=plane(1)
 const multi={...structuredClone(a),knotsU:[0,0,.5,1,1],controlPoints:[a.controlPoints[0]!,[[.5,0,0],[.5,1,0]],a.controlPoints[1]!],weights:[[1,1],[1,1],[1,1]]}
 expect(inspectSweepExactStripJets(multi,b,'vMax','vMin',1,1,1000000)).toMatchObject({certified:false,work:0,reason:'unsupported-bezier-strip-basis'})
 expect(()=>inspectSweepExactStripJets(a,b,'vMax','vMin',1,-1,100)).toThrow()
 expect(()=>inspectSweepExactStripJets(a,b,'vMax','vMin',1,1,1000001)).toThrow()
})

it('certifies exact rational multispan C2 in a shared basis and refuses mismatches, low smoothness and exhausted work',()=>{
 const multi=(y:number):NurbsSurface=>({degreeU:3,degreeV:1,
  knotsU:[0,0,0,0,.5,1,1,1,1],knotsV:[0,0,1,1],
  controlPoints:[0,.125,.5,.875,1].map(x=>[[x,y,0],[x,y+1,0]]),
  weights:[1,2,3,2,1].map(w=>[w,w]),periodicU:false,periodicV:false})
 const a=multi(0),b=multi(1),before=structuredClone([a,b])
 const audit=(x:NurbsSurface,y:NurbsSurface,work=1000000)=>inspectSweepExactStripJets(x,y,'vMax','vMin',2,1,work)
 const positive=audit(a,b)
 expect(positive).toMatchObject({certified:true,exactIdentity:true,regularityCertified:true,reason:'exact-regular-bspline-strip-jets'})
 const transpose=(s:NurbsSurface):NurbsSurface=>({degreeU:s.degreeV,degreeV:s.degreeU,knotsU:s.knotsV,knotsV:s.knotsU,
  controlPoints:s.controlPoints[0]!.map((_,j)=>s.controlPoints.map(row=>row[j]!)),weights:s.weights[0]!.map((_,j)=>s.weights.map(row=>row[j]!)),periodicU:false,periodicV:false})
 expect(inspectSweepExactStripJets(transpose(a),transpose(b),'uMax','uMin',2,1,1000000)).toMatchObject({certified:true,exactIdentity:true,regularityCertified:true})
 expect(audit(a,b,positive.work-1)).toMatchObject({certified:false})
 const unclamped=structuredClone([a,b]);for(const surface of unclamped)surface.knotsU=[-1,-.5,-.25,0,.5,1,1.25,1.5,2]
 expect(audit(unclamped[0]!,unclamped[1]!)).toMatchObject({certified:false,work:0})
 const basis=structuredClone(b);basis.knotsU[4]=.25
 expect(audit(a,basis)).toMatchObject({certified:false,work:0})
 const changed=structuredClone(b);changed.controlPoints[4]![1]![1]+=2*Number.EPSILON
 expect(audit(a,changed)).toMatchObject({certified:false,exactIdentity:false})
 const c1=(y:number):NurbsSurface=>({degreeU:2,degreeV:1,
  knotsU:[0,0,0,.5,1,1,1],knotsV:[0,0,1,1],
  controlPoints:[0,.25,.75,1].map(x=>[[x,y,0],[x,y+1,0]]),
  weights:Array.from({length:4},()=>[1,1]),periodicU:false,periodicV:false})
 expect(inspectSweepExactStripJets(c1(0),c1(1),'vMax','vMin',1,1,1000000)).toMatchObject({certified:true})
 expect(audit(c1(0),c1(1))).toMatchObject({certified:false,work:0})
 const singular=structuredClone([a,b]);for(const surface of singular)for(const row of surface.controlPoints)for(const pole of row)pole[0]=0
 expect(audit(singular[0]!,singular[1]!)).toMatchObject({certified:false,exactIdentity:true,regularityCertified:false})
 expect([a,b]).toEqual(before)
})
