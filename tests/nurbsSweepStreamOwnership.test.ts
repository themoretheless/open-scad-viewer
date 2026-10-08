import {expect,it} from 'vitest'
import {bezierNurbsCurve,circleNurbsCurve,progressiveMiterNurbsProfiles,progressiveSweepNurbsProfiles,streamProgressiveMiterNurbsProfiles,streamProgressiveNurbsProfiles,type NurbsScaleLaw} from '../src/services/nurbsConstructors'
import {createProgressiveMiterBrepProfileBody,createProgressiveBrepProfileBody,streamProgressiveMiterBrepProfileBody,streamProgressiveBrepProfileBody} from '../src/services/geometry/brep'
const law=(a:number,b=a):NurbsScaleLaw=>({degree:1,knots:[0,0,1,1],values:[a,b],weights:[1,1]})
const line=()=>bezierNurbsCurve([[.1,0,0],[.2,0,0]])
const points=():[number,number,number][]=>[[0,0,0],[0,0,10]]
const path=()=>bezierNurbsCurve(points())
const miterOptions=()=>({normal:[1,0,0] as [number,number,number],maxDeviation:.001,maxSteps:64})
const surfaceOptions=()=>({normal:[1,0,0] as [number,number,number],maxDeviation:.01,initialSections:3,maxSections:65})
async function finish<P,R>(stream:AsyncGenerator<P,R,void>):Promise<R>{for(;;){const next=await stream.next();if(next.done)return next.value}}
it('keeps accepted miter preview geometry and diagnostics separate from the final certificate',async()=>{
 const profiles=[line()],p=points(),scale=law(1),twist=law(0),opts=miterOptions()
 const expected=progressiveMiterNurbsProfiles(profiles,p,scale,twist,opts)
 const stream=streamProgressiveMiterNurbsProfiles(profiles,p,scale,twist,opts),first=await stream.next()
 expect(first.done).toBe(false);if(first.done)throw Error('Expected preview')
 expect(first.value.report.accepted).toBe(true)
 first.value.sections[0]![0]!.controlPoints[0]![0]+=100
 first.value.report.certifiedErrorUpper=999
 first.value.report.budget=999
 expect(await finish(stream)).toEqual(expected)
})
it('holds the original miter curves, laws, path and nested options across refinement yields',async()=>{
 const profiles=[line()],p=points(),scale=law(1),twist=law(0,90),opts=miterOptions()
 const expected=progressiveMiterNurbsProfiles(profiles,p,scale,twist,opts)
 const stream=streamProgressiveMiterNurbsProfiles(profiles,p,scale,twist,opts),first=await stream.next()
 expect(first.done).toBe(false);if(first.done)throw Error('Expected preview')
 expect(first.value.report.accepted).toBe(false)
 profiles[0]!.controlPoints[0]![0]=999;p[1]![2]=999;scale.values[0]=-1;twist.values[1]=999;opts.normal[0]=0;opts.maxSteps=1
 expect(await finish(stream)).toEqual(expected)
})
it('keeps accepted surface preview patches and report out of the final result',async()=>{
 const profiles=[line()],curve=path(),scale=law(1),twist=law(0),opts=surfaceOptions()
 const expected=progressiveSweepNurbsProfiles(profiles,curve,scale,twist,opts)
 const stream=streamProgressiveNurbsProfiles(profiles,curve,scale,twist,opts),first=await stream.next()
 expect(first.done).toBe(false);if(first.done)throw Error('Expected preview')
 expect(first.value.report.accepted).toBe(true)
 first.value.patches[0]!.controlPoints[0]![0]![0]+=100
 first.value.report.accepted=false
 opts.maxSections=1
 expect(await finish(stream)).toEqual(expected)
})
it('holds original surface profiles, path, laws and options across refinement yields',async()=>{
 const profiles=[line()],curve=path(),scale=law(1),twist=law(0,90),opts=surfaceOptions()
 const expected=progressiveSweepNurbsProfiles(profiles,curve,scale,twist,opts)
 const stream=streamProgressiveNurbsProfiles(profiles,curve,scale,twist,opts),first=await stream.next()
 expect(first.done).toBe(false);if(first.done)throw Error('Expected preview')
 expect(first.value.report.accepted).toBe(false)
 profiles[0]!.controlPoints[0]![0]=999;curve.controlPoints[1]![2]=999;scale.values[0]=-1;twist.values[1]=999;opts.normal[0]=0;opts.maxSections=1
 expect(await finish(stream)).toEqual(expected)
})
it('constructs miter caps and all post-yield audits from the original body request',async()=>{
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)]],p=points(),scale=law(1),twist=law(0),opts=miterOptions()
 const expected=createProgressiveMiterBrepProfileBody(loops,p,scale,twist,opts)
 const stream=streamProgressiveMiterBrepProfileBody(loops,p,scale,twist,opts),first=await stream.next()
 expect(first.done).toBe(false);if(first.done)throw Error('Expected preview')
 loops[0]![0]!.controlPoints[0]![0]=999;p[1]![2]=999;scale.values[0]=-1;opts.normal[0]=0
 first.value.report.accepted=false
 expect(await finish(stream)).toEqual(expected)
})
it('constructs surface body topology from the original request after preview yields',async()=>{
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)]],curve=path(),scale=law(1),twist=law(0),opts=surfaceOptions()
 const expected=createProgressiveBrepProfileBody(loops,curve,scale,twist,opts)
 const stream=streamProgressiveBrepProfileBody(loops,curve,scale,twist,opts),first=await stream.next()
 expect(first.done).toBe(false);if(first.done)throw Error('Expected preview')
 loops[0]![0]!.controlPoints[0]![0]=999;curve.controlPoints[1]![2]=999;scale.values[0]=-1;opts.normal[0]=0
 expect(await finish(stream)).toEqual(expected)
})
