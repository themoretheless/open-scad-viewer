import {beforeAll,expect,it} from 'vitest'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import type {NurbsSurface} from '../src/services/nurbsSurface'
import {evaluateNurbsSurfaceOffset,boundNurbsSurfaceOffset,boundNurbsSurfaceOffsetJacobian,certifyNurbsOffsetContactSection,findNurbsOffsetCandidateBoxes} from '../src/services/nurbsSurfaceOffset'
beforeAll(async()=>{await warmGeometryKernel()})
const plane=():NurbsSurface=>({degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false})
function contains(bounds:number[][],point:number[]){point.forEach((x,k)=>{expect(bounds[k][0]).toBeLessThanOrEqual(x);expect(bounds[k][1]).toBeGreaterThanOrEqual(x)})}
it('evaluates and encloses source offsets through actual WASM without changing the source',()=>{
 const s=plane(),before=structuredClone(s)
 const p=evaluateNurbsSurfaceOffset(s,[.37,.62],.2)
 expect(p).toMatchObject({point:[.37,.62,.2],du:[1,0,0],dv:[0,1,0],certified:false,topologyAuthority:false})
 const r=boundNurbsSurfaceOffset(s,[[0,1],[0,1]],.2,1);contains(r.image!,p.point)
 const j=boundNurbsSurfaceOffsetJacobian(s,[[0,1],[0,1]],.2,1)
 contains(j.derivatives![0],p.du);contains(j.derivatives![1],p.dv)
 expect(j.continuityCertified).toBe(false);expect(j.offsetRegularityCertified).toBe(false)
 expect(s).toEqual(before)
})
it('returns a certified section center while retaining the missing whole-curve and trim gates',()=>{
 const a=plane(),b=plane();for(const row of b.controlPoints)for(const p of row){const z=p[1];p[1]=.5;p[2]=z}
 const options={a,b,distances:[.2,.2] as [number,number],fixedAxis:0 as const,fixed:.37,firstOther:[.25,.35] as [number,number],secondDomain:[[.32,.42],[.15,.25]] as [[number,number],[number,number]],maxSpans:2}
 const before=structuredClone(options),r=certifyNurbsOffsetContactSection(options)
 expect(r.status).toBe('unique-contact');expect(r.rootExistenceProven).toBe(true)
 contains(r.witness!.centerIntervalMm,[.37,.3,.2]);expect(r.witness!.contractionUpper).toBeLessThan(.5)
 expect(r.wholeCurveComplete).toBe(false);expect(r.trimMembershipProven).toBe(false);expect(r.topologyAuthority).toBe(false)
 expect(options).toEqual(before)
 expect(certifyNurbsOffsetContactSection({...options,b:a,secondDomain:[[.32,.42],[.25,.35]]}).status).toBe('unresolved')
})
it('keeps unvisited candidate boxes and refuses invalid second surfaces',()=>{
 const a=plane(),options={a,b:plane(),domains:[[[0,1],[0,1]],[[0,1],[0,1]]] as [[[number,number],[number,number]],[[number,number],[number,number]]],distances:[.2,.2] as [number,number],parameterTolerance:.01,maxBoxes:1,maxSpans:1}
 const r=findNurbsOffsetCandidateBoxes(options)
 expect(r.reason).toBe('work-limit');expect(r.visitedBoxes).toBe(1);expect(r.pendingBoxes).toHaveLength(2)
 expect(r.rootExistenceProven).toBe(false);expect(r.topologyAuthority).toBe(false)
 const invalid=plane();invalid.weights[0][0]=0
 expect(()=>findNurbsOffsetCandidateBoxes({...options,b:invalid})).toThrow()
})
