import {expect,it} from 'vitest'
import {bezierNurbsCurve,circleNurbsCurve,progressiveMiterNurbsProfiles,previewProgressiveMiterNurbsProfiles,streamProgressiveMiterNurbsProfiles,inspectProgressiveMiterCapProjection,inspectProgressiveMiterIdealCapDomains,miterNurbsProfileSections,type NurbsScaleLaw,type NurbsVectorLaw} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {inspectProgressiveMiterWalls} from '../src/services/nurbsConstructors'
import {createProgressiveMiterBrepProfileBody,streamProgressiveMiterBrepProfileBody,inspectNurbsBrep} from '../src/services/geometry/brep'
const law=(a:number,b=a):NurbsScaleLaw=>({degree:1,knots:[0,0,1,1],values:[a,b],weights:[1,1]})
const profile=()=>bezierNurbsCurve([[0.1,0,0],[0.2,0,0]])
const straight:[number,number,number][]=[[0,0,0],[0,0,10]]
const options={normal:[1,0,0] as [number,number,number],maxSteps:256,maxDeviation:1e-4}
it('requires exact periodic active-domain closure before wall-loop analysis',()=>{
 const periodic={degree:2,knots:Array.from({length:9},(_,i)=>i),controlPoints:[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0],[0,1,0]],weights:Array(6).fill(1),periodic:true}
 const level=previewProgressiveMiterNurbsProfiles([periodic],straight,law(1),law(0),options,1)
 const budgets={clearance:0,distanceTolerance:1e-6,maxInjectivityCells:0,maxPairs:0,maxPairCells:0}
 expect(()=>inspectProgressiveMiterWalls([periodic],straight,law(1),law(0),options,level.sections,budgets,[1])).not.toThrow()
 const almost=structuredClone(periodic)
 almost.knots[1]=1+Number.EPSILON
 const sections=structuredClone(level.sections)
 for(const section of sections)section[0].knots=[...almost.knots]
 expect(()=>inspectProgressiveMiterWalls([almost],straight,law(1),law(0),options,sections,budgets,[1])).toThrow('exact active-domain endpoint closure')
 // Matching original basis stencils prove a shared seam even when its
 // binary64 midpoint is not representable. Zero budgets isolate ownership.
 const nonrepresentable=structuredClone(periodic)
 nonrepresentable.controlPoints[1]![0]=1+Number.EPSILON
 nonrepresentable.controlPoints[5]=[...nonrepresentable.controlPoints[1]!]
 const nonrepresentableLevel=previewProgressiveMiterNurbsProfiles([nonrepresentable],straight,law(1),law(0),options,1)
 expect(()=>inspectProgressiveMiterWalls([nonrepresentable],straight,law(1),law(0),options,nonrepresentableLevel.sections,budgets,[1])).not.toThrow()
 const weightMismatch=structuredClone(nonrepresentableLevel.sections)
 weightMismatch[1]![0]!.weights[5]=2
 expect(()=>inspectProgressiveMiterWalls([nonrepresentable],straight,law(1),law(0),options,weightMismatch,budgets,[1])).toThrow()

})
it('certifies original unclamped hollow material domains through the native miter adapter',()=>{
 const outer={degree:2,knots:Array.from({length:9},(_,i)=>i),controlPoints:[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0],[0,1,0]],weights:Array(6).fill(1),periodic:false}
 const hole={...structuredClone(outer),controlPoints:outer.controlPoints.map(p=>p.map(x=>x*.25))}
 const profiles=[outer,hole],before=structuredClone(profiles)
 const report=inspectProgressiveMiterIdealCapDomains(profiles,straight,law(1),law(0),options,[1,1])
 expect(report).toMatchObject({idealCapDomainsCertified:true,localDomainCertified:true,continuousBound:false,reason:null})
 expect(profiles).toEqual(before)
})
it('retains periodic profile identity through the native miter regularity route',()=>{
 const periodic={degree:2,knots:Array.from({length:9},(_,i)=>i),controlPoints:[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0],[0,1,0]],weights:Array(6).fill(1),periodic:true}
 const before=structuredClone(periodic)
 const level=previewProgressiveMiterNurbsProfiles([periodic],straight,law(1),law(0),options,1)
 expect(level.report).toMatchObject({accepted:true,profileRegularityCertified:true,wallRegularityCertified:true})
 expect(level.sections).toHaveLength(2)
 for(const section of level.sections){
  expect(section[0]).toMatchObject({degree:2,knots:before.knots,weights:before.weights,periodic:true})
 }
 expect(periodic).toEqual(before)
})
it('combines a spatial orientation guide with affine miter laws',()=>{
 const orientationGuide={degree:1,knots:[31,31,41,41],controlPoints:[[1,0,0],[1,1,10]],weights:[1,1],periodic:false}
 const axisScale:NurbsVectorLaw={degree:1,knots:[17,17,19,19],values:[[1,1,1],[2,1,1]],weights:[1,1]}
 const centerLaw:NurbsVectorLaw={degree:2,knots:[7,7,7,9,9,9],values:[[0,0,0],[0,0,.125],[0,1,.25]],weights:[1,1,1]}
 const profiles=[profile()];const opts={...options,maxSteps:64,maxDeviation:.01,orientationGuide,axisScale,centerLaw}
 const before=structuredClone({profiles,opts})
 const fine=previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),opts,16)
 expect(fine.report).toMatchObject({accepted:true,orientationGuideApplied:true,authoredFramesApplied:false,affineLawsApplied:true,wallRegularityCertified:true,continuousErrorMethod:'interval-guide-frame-interpolation',continuousBound:false})
 const end=fine.sections.at(-1)![0]!.controlPoints[0]!
 expect(end[0]).toBeCloseTo((.2-1)/Math.SQRT2,14)
 expect(end[1]).toBeCloseTo((.2+1)/Math.SQRT2,14)
 expect(end[2]).toBeCloseTo(10.25,14)
 const crossing=structuredClone(orientationGuide);crossing.controlPoints=[[1,0,0],[-1,0,10]]
 expect(()=>previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),{...opts,orientationGuide:crossing},16)).toThrow()
 expect({profiles,opts}).toEqual(before)
})
it('combines a moving authored axis with a guide and preserves independent source domains',()=>{
 const frameAxis:NurbsVectorLaw={degree:1,knots:[2,2,5,5],values:[[0,0,1],[0,1,1]],weights:[1,1]}
 const frameNormal:NurbsVectorLaw={degree:1,knots:[11,11,13,13],values:[[0,1,0],[0,1,0]],weights:[1,1]}
 const orientationGuide={degree:1,knots:[7,7,11,11],controlPoints:[[1,0,0],[1,0,10]],weights:[1,1],periodic:false}
 const axisScale:NurbsVectorLaw={degree:1,knots:[17,17,19,19],values:[[1,1,1],[2,1,1]],weights:[1,1]}
 const profiles=[bezierNurbsCurve([[.1,.2,0],[.2,.2,0]])]
 const opts={...options,maxDeviation:.01,maxSteps:64,frameAxis,frameNormal,orientationGuide,axisScale}
 const before=structuredClone({profiles,opts})
 const fine=previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),opts,16)
 expect(fine.report).toMatchObject({accepted:true,authoredFramesApplied:true,orientationGuideApplied:true,affineLawsApplied:true,wallRegularityCertified:true,continuousErrorMethod:'interval-authored-axis-guide-frame-interpolation',continuousBound:false})
 for(let i=0;i<fine.sections.length;i++){
  const f=i/(fine.sections.length-1),h=Math.hypot(1,f)
  const actual=fine.sections[i]![0]!.controlPoints[0]!
  expect(actual[0]).toBeCloseTo(.1*(1+f),13)
  expect(actual[1]).toBeCloseTo(.2/h,13)
  expect(actual[2]).toBeCloseTo(10*f-.2*f/h,13)
 }
 const singular=structuredClone(orientationGuide);singular.controlPoints=[[0,0,1],[0,1,11]]
 expect(()=>previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),{...opts,orientationGuide:singular},16)).toThrow()
 expect({profiles,opts}).toEqual(before)
})
it('bounds moving axis, guide, twist and affine centers at interior retained parameters',()=>{
 const vector=(a:[number,number,number],b:[number,number,number],lo:number,hi:number):NurbsVectorLaw=>({degree:1,knots:[lo,lo,hi,hi],values:[a,b],weights:[1,1]})
 const frameAxis=vector([0,0,1],[0,.5,1],2,5)
 const frameNormal=vector([1,0,0],[1,0,0],7,9)
 const orientationGuide={degree:1,knots:[31,31,41,41],controlPoints:[[1,0,0],[1,0,10]],weights:[1,1],periodic:false}
 const axisScale=vector([1,1,1],[2,1,1],17,19)
 const centerLaw=vector([0,0,0],[0,.125,.25],23,29)
 const profiles=[profile()],twist=law(0,.25*180/Math.PI)
 const opts={...options,maxSteps:64,maxDeviation:.01,frameAxis,frameNormal,orientationGuide,axisScale,centerLaw}
 const before=structuredClone({profiles,twist,opts})
 const fine=previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),twist,opts,16)
 expect(fine.report).toMatchObject({accepted:true,authoredFramesApplied:true,orientationGuideApplied:true,affineLawsApplied:true,continuousBound:false})
 const upper=fine.report.certifiedErrorUpper!
 expect(upper).toBeGreaterThan(0)
 expect(upper).toBeLessThanOrEqual(opts.maxDeviation)
 // Independent closed form is a falsification oracle, not the continuous proof.
 for(let i=0;i<16;i++)for(const local of [0,.125,.375,.625,.875,1])for(const u of [0,.25,.5,.75,1]){
  const f=(i+local)/16,h=Math.hypot(1,.5*f),c=Math.cos(.25*f),s=Math.sin(.25*f)
  const r=(.1+.1*u)*(1+f),side=.125*f,longitudinal=.25*f
  const ideal=[r*c-side*s,(r*s+side*c+longitudinal*.5*f)/h,10*f+(-.5*f*(r*s+side*c)+longitudinal)/h]
  const a=fine.sections[i]![0]!.controlPoints,b=fine.sections[i+1]![0]!.controlPoints
  const retained=ideal.map((_,k)=>(1-local)*((1-u)*a[0]![k]!+u*a[1]![k]!)+local*((1-u)*b[0]![k]!+u*b[1]![k]!))
  expect(Math.hypot(...ideal.map((x,k)=>x-retained[k]!))).toBeLessThanOrEqual(upper)
 }
 const refused=previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),twist,{...opts,maxDeviation:1e-10},16)
 expect(refused.report.accepted).toBe(false)
 expect(refused.report.certifiedErrorUpper).toBeGreaterThan(refused.report.budget)
 const singular=structuredClone(frameAxis);singular.values=[[0,0,0],[0,0,0]]
 expect(()=>previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),twist,{...opts,frameAxis:singular},16)).toThrow('Miter direction must be finite and nonzero')
 expect({profiles,twist,opts}).toEqual(before)
})
it('combines affine and authored miter laws with shared continuous certificates',()=>{
 const vector=(a:[number,number,number],b:[number,number,number],lo:number,hi:number):NurbsVectorLaw=>({degree:1,knots:[lo,lo,hi,hi],values:[a,b],weights:[1,1]})
 const frameAxis=vector([0,0,1],[0,0,1],2,5)
 const frameNormal=vector([1,0,0],[1,1,0],11,13)
 const axisScale=vector([1,1,1],[2,1,1],17,19)
 const centerLaw:NurbsVectorLaw={degree:2,knots:[7,7,7,9,9,9],values:[[0,0,0],[0,0,.125],[0,1,.25]],weights:[1,1,1]}
 const profiles=[profile()];const opts={...options,maxSteps:64,maxDeviation:.01,frameAxis,frameNormal,axisScale,centerLaw}
 const before=structuredClone({profiles,opts})
 const fine=previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),opts,16)
 expect(fine.report).toMatchObject({accepted:true,authoredFramesApplied:true,affineLawsApplied:true,wallRegularityCertified:true,continuousBound:false})
 const end=fine.sections.at(-1)![0]!.controlPoints[0]!
 expect(end[0]).toBeCloseTo((.2-1)/Math.SQRT2,14)
 expect(end[1]).toBeCloseTo((.2+1)/Math.SQRT2,14)
 expect(end[2]).toBeCloseTo(10.25,14)
 expect({profiles,opts}).toEqual(before)
})
it('transports authored miter frames and certifies the actual retained surfaces',()=>{
 const frameAxis:NurbsVectorLaw={degree:1,knots:[2,2,5,5],values:[[0,0,1],[0,0,1]],weights:[1,1]}
 const frameNormal:NurbsVectorLaw={degree:1,knots:[7,7,9,9],values:[[1,0,0],[1,1,0]],weights:[1,1]}
 const profiles=[profile()]
 const opts={...options,maxSteps:16,maxDeviation:.001,frameAxis,frameNormal}
 const before=structuredClone({profiles,frameAxis,frameNormal})
 expect(previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),opts,1).report.accepted).toBe(false)
 const fine=previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),opts,16)
 expect(fine.report).toMatchObject({accepted:true,authoredFramesApplied:true,affineLawsApplied:false,wallRegularityCertified:true,continuousErrorMethod:'interval-authored-frame-interpolation',continuousBound:false})
 expect(fine.report.certifiedErrorUpper).toBeLessThanOrEqual(.001)
 expect(fine.sections.at(-1)![0]!.controlPoints[0]![0]).toBeCloseTo(.1/Math.SQRT2,14)
 expect(fine.sections.at(-1)![0]!.controlPoints[0]![1]).toBeCloseTo(.1/Math.SQRT2,14)
 expect(progressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),opts).report.accepted).toBe(true)
 expect(()=>previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),{...options,frameAxis},16)).toThrow(/frame_normal/)
 expect(()=>previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),{...opts,axisScale:frameAxis},16)).toThrow()
 expect({profiles,frameAxis,frameNormal}).toEqual(before)
})
it('preserves authored miter frames from Rush into actual Solid admission',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const source=readFileSync('examples/rush/miter-authored-frame-hollow.r','utf8')
 const compiled=compileRushFrontend(source)
 const node=compiled.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 expect(node).toMatchObject({frame_axis:{knots:[2,2,5,5],values:[[0,0,2],[0,0,2]]},frame_normal:{knots:[7,7,9,9],values:[[0,3,0],[0,3,0]]}})
 expect(()=>compileRushFrontend(source.replace('[[0,3,0],[0,3,0]]','[[0,3mm,0],[0,3,0]]'))).toThrow()
 const built=buildOwnNurbs(compiled.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,authoredFramesApplied:true,continuousBound:true,retainedCaps:{exact:true},volume:{solidGeometryCertified:true}})
 expect(built.nativeGeometry).toBeDefined()
 const model=JSON.parse(built.nativeGeometry!.geometryJson).geometry as import('../src/services/geometry/brep').NurbsBrep
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,model)).toMatchObject({solidGeometryCertified:true})
})
it.each(['miter-combined-frame-guide-affine-hollow.r','miter-combined-frame-guide-affine-hollow-corrected.r'])('admits authored-axis guide affine Rush %s with explicit combined provenance',async(file)=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const source=readFileSync('examples/rush/'+file,'utf8')
 const compiled=compileRushFrontend(source)
 const node=compiled.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(compiled.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,authoredFramesApplied:true,orientationGuideApplied:true,affineLawsApplied:true,continuousErrorMethod:'interval-authored-axis-guide-frame-interpolation',continuousBound:true,retainedCaps:{exact:true},volume:{solidGeometryCertified:true}})
 expect(built.nativeGeometry).toBeDefined()
 const model=JSON.parse(built.nativeGeometry!.geometryJson).geometry as import('../src/services/geometry/brep').NurbsBrep
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,model)).toMatchObject({solidGeometryCertified:true})
})
it('admits combined authored frame affine Rush bodies into Solid',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const source=readFileSync('examples/rush/miter-combined-frame-affine-hollow.r','utf8')
 const compiled=compileRushFrontend(source)
 const node=compiled.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 expect(node).toMatchObject({frame_axis:{knots:[2,2,5,5],values:[[0,0,2],[0,0,2]]},frame_normal:{knots:[7,7,9,9],values:[[0,3,0],[0,3,0]]}})
 expect(()=>compileRushFrontend(source.replace('[[0,3,0],[0,3,0]]','[[0,3mm,0],[0,3,0]]'))).toThrow()
 const built=buildOwnNurbs(compiled.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,authoredFramesApplied:true,affineLawsApplied:true,continuousBound:true,retainedCaps:{exact:true},volume:{solidGeometryCertified:true}})
 expect(built.nativeGeometry).toBeDefined()
 const model=JSON.parse(built.nativeGeometry!.geometryJson).geometry as import('../src/services/geometry/brep').NurbsBrep
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,model)).toMatchObject({solidGeometryCertified:true})
})
it('admits guide affine Rush bodies into Solid',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const source=readFileSync('examples/rush/miter-guide-affine-hollow.r','utf8')
 const compiled=compileRushFrontend(source)
 const node=compiled.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 expect(node).toHaveProperty('orientation_guide')
 const built=buildOwnNurbs(compiled.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,authoredFramesApplied:false,orientationGuideApplied:true,affineLawsApplied:true,continuousBound:true,retainedCaps:{exact:true},volume:{solidGeometryCertified:true}})
 expect(built.nativeGeometry).toBeDefined()
 const model=JSON.parse(built.nativeGeometry!.geometryJson).geometry as import('../src/services/geometry/brep').NurbsBrep
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,model)).toMatchObject({solidGeometryCertified:true})
})
it('certifies closed hollow affine shells and admits the actual body into Solid',async()=>{
 const points:[number,number,number][]=[[0,0,0],[10,0,0],[10,10,0],[0,10,0]]
 const loops=[[circleNurbsCurve([0,0,0],[1,0,0],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[1,0,0],.2))]]
 const axisScale:NurbsVectorLaw={degree:1,knots:[2,2,5,5],values:[[2,1,1],[2,1,1]],weights:[1,1]}
 const centerLaw:NurbsVectorLaw={degree:1,knots:[7,7,9,9],values:[[.125,0,0],[.125,0,0]],weights:[1,1]}
 const body=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),{normal:[0,0,1],closed:true,miterLimit:2,maxDeviation:.001,maxSteps:1,axisScale,centerLaw})
 expect(body.approximation.report).toMatchObject({accepted:true,affineLawsApplied:true,closedPath:true,seamContinuity:'C0',continuousBound:false})
 expect(body.retainedCorrespondence).toMatchObject({exact:true,wallErrorUpper:0})
 expect(body.model.faces).toHaveLength(32)
 expect(body.model.shells).toHaveLength(2)
 expect(body.model.bodies[0]!.innerShells).toEqual([1])
 expect(body.model.faces.every(face=>face.holes.length===0)).toBe(true)
 expect(body.volume,JSON.stringify(body.volume)).toMatchObject({solidGeometryCertified:true,nesting:{rolesConsistent:true,parents:[null,0]},orientations:[{shell:0,outward:true,expectedOutward:true},{shell:1,outward:false,expectedOutward:false}]})
 const {createNativeGeometryArtifact}=await import('../src/core/nativeGeometry')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const artifact=createNativeGeometryArtifact('closed-affine','brep',{geometry:body.model},{nodes:[{id:'closed-affine',op:'brep_progressive_miter_sweep',closed:true}],root:'closed-affine'})
 expect(inspectProgressiveSweepSolidAdmission(artifact,body.model)).toMatchObject({solidGeometryCertified:true})
 const inward=structuredClone(body.model)
 for(const face of inward.shells[0]!.faces)face.reversed=!face.reversed
 expect(()=>inspectProgressiveSweepSolidAdmission(artifact,inward)).toThrow(/snapshot binding/)
 const inwardArtifact=createNativeGeometryArtifact('closed-affine','brep',{geometry:inward},{nodes:[{id:'closed-affine',op:'brep_progressive_miter_sweep',closed:true}],root:'closed-affine'})
 expect(()=>inspectProgressiveSweepSolidAdmission(inwardArtifact,inward)).toThrow(/orientation/)
})
it('preserves affine miter dimensions and certificates from Rush into the native body',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const source=readFileSync('examples/rush/miter-affine-hollow.r','utf8')
 const compiled=compileRushFrontend(source)
 const node=compiled.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 expect(node).toMatchObject({axis_scale:{knots:[2,2,5,5],values:[[1,1,1],[2,1,1]]},center_law:{knots:[7,7,9,9],values:[[0,.25,0],[0,.5,0]]}})
 expect(()=>compileRushFrontend(source.replace('[[1,1,1],[2,1,1]]','[[1mm,1,1],[2,1,1]]'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('[[0mm,0.25mm,0mm],[0mm,0.5mm,0mm]]','[[0deg,0.25mm,0mm],[0mm,0.5mm,0mm]]'))).toThrow()
 const built=buildOwnNurbs(compiled.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,affineLawsApplied:true,continuousBound:true,retainedCorrespondence:{exact:true},retainedCaps:{exact:true},volume:{solidGeometryCertified:true}})
 expect(built.nativeGeometry).toBeDefined()
 const model=JSON.parse(built.nativeGeometry!.geometryJson).geometry as import('../src/services/geometry/brep').NurbsBrep
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,model)).toMatchObject({solidGeometryCertified:true})
})
it('transports affine miter laws with independent domains and certified refinement',()=>{
 const axisScale:NurbsVectorLaw={degree:1,knots:[2,2,5,5],values:[[1,1,1],[2,1,1]],weights:[1,1]}
 const centerLaw:NurbsVectorLaw={degree:2,knots:[7,7,7,9,9,9],values:[[0,0,0],[0,0,0],[0,1,0]],weights:[1,1,1]}
 const profiles=[bezierNurbsCurve([[1,0,0],[2,0,0]])]
 const opts={...options,maxDeviation:.01,axisScale,centerLaw}
 const before=structuredClone({profiles,axisScale,centerLaw})
 const coarse=previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),opts,1)
 expect(coarse.report).toMatchObject({accepted:false,affineLawsApplied:true,continuousBound:false})
 const fine=previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),opts,8)
 expect(fine.report).toMatchObject({accepted:true,affineLawsApplied:true,continuousErrorMethod:'interval-affine-law-interpolation',continuousBound:false})
 expect(fine.report.certifiedErrorUpper).toBeLessThanOrEqual(.01)
 expect(fine.sections.at(-1)![0]!.controlPoints).toEqual([[2,1,10],[4,1,10]])
 const result=progressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),opts)
 expect(result.report).toMatchObject({accepted:true,affineLawsApplied:true})
 expect(result.sections).not.toBeNull()
 const invalid=structuredClone(axisScale);invalid.values[0]![0]=0
 expect(()=>previewProgressiveMiterNurbsProfiles(profiles,straight,law(1),law(0),{...opts,axisScale:invalid},8)).toThrow()
 expect({profiles,axisScale,centerLaw}).toEqual(before)
})
it('qualifies closed affine law endpoints and refuses discontinuous or mismatched closure',()=>{
 const profiles=[bezierNurbsCurve([[0,0,.1],[0,0,.2]])]
 const points:[number,number,number][]=[[0,0,0],[10,0,0],[10,10,0],[0,10,0]]
 const axisScale:NurbsVectorLaw={degree:2,knots:[2,2,2,5,5,5],values:[[1,1,1],[2,1,1],[1,1,1]],weights:[1,1,1]}
 const centerLaw:NurbsVectorLaw={degree:2,knots:[7,7,7,9,9,9],values:[[0,0,0],[.2,0,0],[0,0,0]],weights:[1,1,1]}
 const opts={normal:[0,0,1] as [number,number,number],closed:true,miterLimit:2,maxDeviation:.01,maxSteps:16,axisScale,centerLaw}
 const before=structuredClone({profiles,points,axisScale,centerLaw})
 const level=previewProgressiveMiterNurbsProfiles(profiles,points,law(1),law(0),opts,8)
 expect(level.report).toMatchObject({accepted:true,affineLawsApplied:true,closedPath:true,continuousBound:false,seamContinuity:'C0',wallRegularityCertified:true})
 expect(level.sections[0]).toEqual(level.sections.at(-1))
 const badAxes=structuredClone(axisScale);badAxes.values[2]![0]=1.25
 const badCenter=structuredClone(centerLaw);badCenter.values[2]![0]=.125
 expect(()=>previewProgressiveMiterNurbsProfiles(profiles,points,law(1),law(0),{...opts,axisScale:badAxes},8)).toThrow(/endpoints/)
 expect(()=>previewProgressiveMiterNurbsProfiles(profiles,points,law(1),law(0),{...opts,centerLaw:badCenter},8)).toThrow(/endpoints/)
 const discontinuous:NurbsVectorLaw={degree:1,knots:[0,0,.5,.5,1,1],values:[[0,0,0],[0,0,0],[0,0,0],[0,0,0]],weights:[1,1,1,1]}
 expect(()=>previewProgressiveMiterNurbsProfiles(profiles,points,law(1),law(0),{...opts,centerLaw:discontinuous},8)).toThrow(/Interior multiplicity/)
 expect({profiles,points,axisScale,centerLaw}).toEqual(before)
})

it('refuses aliased whole-turn probes until the local phase hull is resolved',()=>{
  const opts={...options,maxDeviation:1e-3}
  const level=previewProgressiveMiterNurbsProfiles([profile()],straight,law(1),law(0,1440),opts,1)
  expect(level.report.sampledControlDeviation).toBeLessThan(1e-12)
  expect(level.report).toMatchObject({accepted:false,phaseResolved:false})
  const result=progressiveMiterNurbsProfiles([profile()],straight,law(1),law(0,1440),opts)
  expect(result.report).toMatchObject({accepted:true,phaseResolved:true})
  expect(result.report.continuousErrorUpper).toBeLessThanOrEqual(result.report.budget)
  expect(result.report.steps).toBeGreaterThanOrEqual(16)
})

it('retains the independently scaled full-turn geometry with explicit sampled reports',()=>{
  const level=previewProgressiveMiterNurbsProfiles([profile()],straight,law(1,2),law(0,360),options,100)
  for(const i of [0,17,50,93,100]){
    const f=i/100,point=level.sections[i]![0]!.controlPoints[1]!,angle=2*Math.PI*f
    expect(point[0]).toBeCloseTo(.2*(1+f)*Math.cos(angle),11)
    expect(point[1]).toBeCloseTo(.2*(1+f)*Math.sin(angle),11)
    expect(point[2]).toBeCloseTo(10*f,11)
  }
  expect(level.report).toMatchObject({continuousBound:false,roundingCertified:false,sections:101,stations:401})
  expect(level.report.continuousErrorUpper).toBeGreaterThanOrEqual(level.report.sampledControlDeviation)
  const result=progressiveMiterNurbsProfiles([profile()],straight,law(1,2),law(0,360),options)
  expect(result.report.accepted).toBe(true)
  expect(result.levels.length).toBeGreaterThan(1)
  const refused=progressiveMiterNurbsProfiles([profile()],straight,law(1),law(0,360),{...options,maxSteps:1})
  expect(refused.sections).toBeNull()
  expect(refused.report.accepted).toBe(false)
})

it('corrects nonzero skew-loop holonomy and builds retained periodic hollow topology',()=>{
  const points:[number,number,number][]=[[0,0,0],[10,0,0],[10,10,4],[0,10,1],[0,5,-2]]
  const loops=[[circleNurbsCurve([0,0,0],[1,0,0],.1)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[1,0,0],.04))]]
  expect(()=>miterNurbsProfileSections(loops.flat(),points,[0,0,1],4,true)).toThrow(/holonomy/)
  const body=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),{normal:[0,0,1],closed:true,maxDeviation:1e-4})
  expect(body.approximation.report.accepted).toBe(true)
  expect(Math.abs(body.approximation.report.holonomyCorrectionRadians)).toBeGreaterThan(1e-3)
  expect(body.approximation.sections![0]).toEqual(body.approximation.sections!.at(-1))
  expect(body.model.shells).toHaveLength(2)
  expect(body.model.bodies[0]!.innerShells).toEqual([1])
  expect(body.model.faces.every(f=>f.holes.length===0)).toBe(true)
  expect(body.model.faces.length).toBeLessThanOrEqual(1024)
  expect(inspectNurbsBrep(body.model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
  expect(body.globalEmbeddingCertified).toBe(false)
  expect(()=>createProgressiveMiterBrepProfileBody(loops,points,law(1,2),law(0),{normal:[0,0,1],closed:true,maxDeviation:1e-4})).toThrow(/scale endpoints/)
})

it('streams the native history without promoting previews and cancels between levels',async()=>{
  const profiles=[profile()],scale=law(1),twist=law(0,180)
  const direct=progressiveMiterNurbsProfiles(profiles,straight,scale,twist,options)
  const stream=streamProgressiveMiterNurbsProfiles(profiles,straight,scale,twist,options)
  const reports=[]
  for(;;){const level=await stream.next();if(level.done){expect(level.value).toEqual(direct);break}reports.push(level.value.report)}
  expect(reports).toEqual(direct.levels)
  const abort=new AbortController()
  const cancelled=streamProgressiveMiterNurbsProfiles(profiles,straight,scale,twist,options,{signal:abort.signal})
  const preview=await cancelled.next()
  expect(preview.done).toBe(false)
  if(!preview.done)expect(preview.value.report.accepted).toBe(false)
  abort.abort()
  await expect(cancelled.next()).rejects.toMatchObject({name:'AbortError'})
})


it('transfers certified retained scale/twist hollow Rush geometry to Solid',async()=>{
 const {readFileSync}=await import('node:fs')
 const {parseOpenSCAD}=await import('../src/services/openscadParser')
 const {sceneMeshesToSolidDocument}=await import('../src/services/solidBridge')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const source=readFileSync('examples/rush/progressive-miter-certified-hollow.r','utf8')
 const scene=await parseOpenSCAD(source)
 expect(scene.meshes.length).toBeGreaterThan(0)
 const evidence=scene.meshes.map(m=>readSweepViewportEvidence(m.nativeGeometry)).filter(e=>e!==null)
 expect(evidence.length).toBeGreaterThan(0)
 expect(evidence.every(e=>e.solidGeometryCertified===true)).toBe(true)
 expect(evidence.every(e=>e.continuousBound===true)).toBe(true)
 const solid=sceneMeshesToSolidDocument(scene.meshes)
 expect(solid.bodies).toHaveLength(1)
 expect(inspectNurbsBrep(solid.bodies[0]!.brep!)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
})

it('retains Rush rational topology after streamed previews and refuses dimensioned options',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const {parseOpenSCAD}=await import('../src/services/openscadParser')
 const {sceneMeshesToSolidDocument}=await import('../src/services/solidBridge')
 const source=readFileSync('examples/rush/progressive-miter-hollow-body.r','utf8')
 const compiled=compileRushFrontend(source),node=compiled.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 expect(node).toMatchObject({closed:true,max_steps:64,twist:{values:[0,360]}})
 expect(()=>compileRushFrontend(source.replace('max_steps: 64','max_steps: 64mm'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('values: [1,1.5,1]','values: [1mm,1.5mm,1mm]'))).toThrow()
 const previews:import('../src/services/nurbsConstructors').ProgressiveSweepPreview[]=[]
 const built=await buildOwnNurbsAsync(compiled.document,{action:'build'},{onSweepPreview:(_id,preview)=>{previews.push(preview)}})
 expect(previews.length).toBeGreaterThan(1)
 expect(previews[0]!.report.accepted).toBe(false)
 expect(previews.at(-1)!.report.accepted).toBe(true)
 expect(previews.every(p=>p.patches.length>0&&p.profilePatchRanges.length===2)).toBe(true)
 expect(built.report.construction?.[node.id]).toMatchObject({closedPath:true,phaseResolved:true,globalEmbeddingCertified:false,capDomains:null,capContacts:null,capPairs:null,embedding:null,volume:{solidGeometryCertified:expect.any(Boolean),orientations:expect.any(Array)},wallAudit:{globalEmbeddingCertified:false,declaredBoundariesC0:true}})
 const scene=await parseOpenSCAD(source)
 const solid=sceneMeshesToSolidDocument(scene.meshes)
 expect(solid.bodies).toHaveLength(1)
 expect(inspectNurbsBrep(solid.bodies[0]!.brep!)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
 let cancel=false
 await expect(buildOwnNurbsAsync(compiled.document,{action:'build'},{shouldAbort:()=>cancel,onSweepPreview:()=>{cancel=true}})).rejects.toMatchObject({name:'AbortError'})
 await expect(buildOwnNurbsAsync(compiled.document,{action:'build'})).resolves.toBeDefined()
 await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('max_steps: 64','max_steps: 1')).document,{action:'build'})).rejects.toThrow(/budget/)
})


it('does not promote aliased zero-error geometry while a frame limit is unproved',()=>{
 const result=progressiveMiterNurbsProfiles([profile()],[[0,0,0],[0,0,5],[0,0,10]],law(1),law(0),{normal:[1,0,0],miterLimit:1,maxSteps:2,maxDeviation:.001})
 expect(result.sections).toBeNull()
 expect(result.report).toMatchObject({accepted:false,phaseResolved:true,frameTransportCertified:false,frameTransportReason:'transport-enclosure-unresolved'})
 expect(result.report.sampledControlDeviation).toBeLessThan(1e-12)
 expect(()=>createProgressiveMiterBrepProfileBody([[circleNurbsCurve([0,0,0],[0,0,1],.1)]],[[0,0,0],[0,0,5],[0,0,10]],law(1),law(0),{normal:[1,0,0],miterLimit:1,maxSteps:2,maxDeviation:.001})).toThrow(/frame transport could not be proved/)
})

it('requires the certified rounding bound across the public WASM boundary',()=>{
 const translated=bezierNurbsCurve([[1e8+1,0,0],[1e8+2,0,0]])
 const points:[number,number,number][]=[[1e8,0,0],[1e8,0,10]]
 const refused=previewProgressiveMiterNurbsProfiles([translated],points,law(1),law(0),{...options,maxDeviation:1e-12},1)
 expect(refused.report.sampledControlDeviation).toBeLessThan(1e-12)
 expect(refused.report.certifiedErrorUpper).toBeGreaterThan(refused.report.budget)
 expect(refused.report).toMatchObject({accepted:false,errorCertificateReason:null})
 expect(refused.report.errorCertificateCells).toBeGreaterThan(0)
 const accepted=previewProgressiveMiterNurbsProfiles([translated],points,law(1),law(0),{...options,maxDeviation:1e-4},1)
 expect(accepted.report.accepted).toBe(true)
 expect(accepted.report.certifiedErrorUpper).toBeLessThanOrEqual(accepted.report.budget)
 expect(accepted.report.endpointContourErrorUpper).toHaveLength(2)
 for(const bound of accepted.report.endpointContourErrorUpper!){
  expect(bound).toBeGreaterThanOrEqual(0)
  expect(bound).toBeLessThanOrEqual(accepted.report.certifiedErrorUpper!)
 }
 expect(accepted.report.continuousBound).toBe(false)
})

it('reports independent profile and retained-wall regularity through WASM',()=>{
 const accepted=previewProgressiveMiterNurbsProfiles([profile()],straight,law(1),law(0),options,1)
 expect(accepted.report).toMatchObject({accepted:true,profileRegularityCertified:true,wallRegularityCertified:true,unresolvedWallPatches:[]})
 const collapsed=bezierNurbsCurve([[.1,0,0],[.1,0,0]])
 const refused=previewProgressiveMiterNurbsProfiles([collapsed],straight,law(1),law(0),options,1)
 expect(refused.report.certifiedErrorUpper).toBeLessThan(refused.report.budget)
 expect(refused.report).toMatchObject({accepted:false,profileRegularityCertified:false,wallRegularityCertified:null})
})

it('retains cap-pair evidence in the open Rush construction report',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const source=readFileSync('examples/rush/progressive-miter-hollow-body.r','utf8')
  .replace(/points: \[\[[^\n]+/,'points: [[0,0,0],[10mm,0,0]],')
  .replace('closed: true','closed: false').replace('values: [1,1.5,1]','values: [1,1,1]')
  .replace('values: [0deg,360deg]','values: [0deg,0deg]')
 const compiled=compileRushFrontend(source),node=compiled.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=await buildOwnNurbsAsync(compiled.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction?.[node.id]).toMatchObject({closedPath:false,globalEmbeddingCertified:false,volume:{solidGeometryCertified:expect.any(Boolean),orientations:expect.any(Array)},embedding:{solidGeometryCertified:false,pairs:expect.any(Array)},retainedWallCharts:{globalEmbeddingCertified:false,charts:expect.any(Array)},capPairs:{globalEmbeddingCertified:false,unresolvedPairs:[],audit:{pairs:{allPairsSeparated:true,separatedPairs:1}}}})
 expect('nativeGeometry' in built).toBe(true)
 if('nativeGeometry' in built) {
  const snapshot=JSON.parse(built.nativeGeometry.geometryJson)
  expect(snapshot.sweepEvidence.volume).toEqual((built.report.construction?.[node.id] as {volume:unknown}).volume)
  expect(snapshot.sweepEvidence.continuousBound).toBe(true)
 }
})

it('corrects progressive endpoint caps with bounded retained displacement and matching stream audits',async()=>{
 const outer=circleNurbsCurve([0,0,0],[0,0,1],.5)
 const hole=circleNurbsCurve([0,0,0],[0,0,-1],.2)
 const loops=[[outer],[hole]],before=structuredClone(loops)
 const opts={...options,maxSteps:16,maxDeviation:.01,capCorrection:{quantum:2**-40,tolerance:1e-8,maxWork:1000000}}
 const body=createProgressiveMiterBrepProfileBody(loops,straight,law(1),law(0),opts)
 expect(body.sectionCorrection?.reason).toBe('bounded-section-interpolation')
 expect(body.sectionCorrection?.wallDisplacementUpper).not.toBeNull()
 expect(body.sectionCorrection?.exactPlanarSections).toEqual([0,1])
 expect(body.retainedCorrespondence.exact).toBe(true)
 expect(body.approximation.report.continuousBound).toBe(false)
 const stream=streamProgressiveMiterBrepProfileBody(loops,straight,law(1),law(0),opts)
 let next=await stream.next();while(!next.done)next=await stream.next()
 expect(next.value.model).toEqual(body.model)
 expect(next.value.sectionCorrection).toEqual(body.sectionCorrection)
 expect(next.value.wallAudit).toEqual(body.wallAudit)
 expect(next.value.boundaryCertificate).toEqual(body.boundaryCertificate)
 expect(body.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,reason:null})
 expect(loops).toEqual(before)
 const {DEFAULT_SWEEP_VOLUME_BUDGETS}=await import('../src/services/nurbsSweepEmbedding')
 const noCapWork={...opts,volumeBudgets:{...DEFAULT_SWEEP_VOLUME_BUDGETS,capBudgets:{...DEFAULT_SWEEP_VOLUME_BUDGETS.capBudgets,maxExactWork:0}}}
 expect(()=>createProgressiveMiterBrepProfileBody(loops,straight,law(1),law(0),noCapWork)).toThrow('filled retained cap regions unproved')
 const denied=streamProgressiveMiterBrepProfileBody(loops,straight,law(1),law(0),noCapWork)
 await expect((async()=>{for await(const level of denied){void level}})()).rejects.toThrow('filled retained cap regions unproved')
 expect(()=>createProgressiveMiterBrepProfileBody(loops,straight,law(1),law(0),{...opts,capCorrection:{...opts.capCorrection,maxWork:0}})).toThrow('cap correction unproved')
})

it('transports progressive cap correction through Rush into construction evidence',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const source=readFileSync('examples/rush/miter-guide-hollow.r','utf8').replace('initial_steps:', 'cap_correction_tolerance: 0.00000001mm, cap_correction_max_work: 1000000, initial_steps:')
 const graph=compileRushFrontend(source).document
 const node=graph.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 expect(node).toMatchObject({cap_correction_tolerance:1e-8,cap_correction_max_work:1000000})
 const built=buildOwnNurbs(graph,{action:'build'})
 expect(built.report.construction?.[node.id]).toMatchObject({sectionCorrection:{reason:'bounded-section-interpolation',exactPlanarSections:[0,1]},continuousBound:true,retainedCorrespondence:{exact:true}})
 expect(()=>buildOwnNurbs(compileRushFrontend(source.replace('cap_correction_max_work: 1000000','cap_correction_max_work: 0')).document,{action:'build'})).toThrow(/correction unproved/)
 expect(()=>buildOwnNurbs(compileRushFrontend(source.replace('cap_correction_tolerance: 0.00000001mm, ','')).document,{action:'build'})).toThrow('requires a displacement tolerance')
 const {parseOpenSCAD}=await import('../src/services/openscadParser')
 const {sceneMeshesToSolidDocument}=await import('../src/services/solidBridge')
 const scene=await parseOpenSCAD(source)
 expect(sceneMeshesToSolidDocument(scene.meshes).bodies).toHaveLength(1)
})

it('composes correction with authored wall error and refuses an oversized combined frame/affine correction',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const source=readFileSync('examples/rush/miter-combined-frame-affine-hollow-corrected.r','utf8')
 const graph=compileRushFrontend(source).document
 const node=graph.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 const report=built.report.construction![node.id] as {retainedWallErrorUpper:number;certifiedErrorUpper:number;sectionCorrection:{wallDisplacementUpper:number}}
 expect(report.retainedWallErrorUpper).toBeGreaterThanOrEqual(report.sectionCorrection.wallDisplacementUpper)
 expect(report.retainedWallErrorUpper).toBeGreaterThanOrEqual(report.certifiedErrorUpper)
 expect(report.retainedWallErrorUpper).toBeLessThanOrEqual(.3)
 expect(JSON.parse(built.nativeGeometry!.geometryJson).sweepEvidence.retainedWallErrorUpper).toBe(report.retainedWallErrorUpper)
 expect(JSON.parse(built.nativeGeometry!.geometryJson).sweepEvidence.capProjection.normalDots).toHaveLength(2)
 expect(()=>buildOwnNurbs(compileRushFrontend(source.replace('max_deviation: 0.3mm','max_deviation: 0.01mm')).document,{action:'build'})).toThrow('retained wall error exceeds max_deviation')
})

it('transports original endpoint projection prerequisites with shared exact-work refusal',async()=>{
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[circleNurbsCurve([0,0,0],[0,0,-1],.2)]]
 const frameAxis:NurbsVectorLaw={degree:1,knots:[2,2,5,5],values:[[0,0,2],[0,0,2]],weights:[1,1]}
 const frameNormal:NurbsVectorLaw={degree:1,knots:[7,7,9,9],values:[[0,3,0],[0,3,0]],weights:[1,1]}
 const opts={...options,maxSteps:16,maxDeviation:.01,frameAxis,frameNormal}
 const body=createProgressiveMiterBrepProfileBody(loops,straight,law(1),law(0),opts)
 const caps=body.model.faces.slice(-2).map(face=>face.surface) as [import('../src/services/nurbsSurface').NurbsSurface,import('../src/services/nurbsSurface').NurbsSurface]
 const before=structuredClone({loops,caps,opts})
 const report=inspectProgressiveMiterCapProjection(loops.flat(),straight,law(1),law(0),opts,caps)
 expect(report).toMatchObject({reason:null,continuousBound:false,method:'original-endpoint-plane-projection'})
 expect(report.normalDots).toHaveLength(2)
 expect(body.capProjection).toEqual(report)
 const {inspectProgressiveMiterCapParallelism}=await import('../src/services/nurbsConstructors')
 const parallel=inspectProgressiveMiterCapParallelism(loops.flat(),straight,law(1),law(0),opts,caps)
 expect(parallel).toMatchObject({parallel:[true,true],reason:null,continuousBound:false,method:'original-endpoint-plane-parallelism'})
 expect(body.capParallelism).toEqual(parallel)
 const tiltedOpts={...opts,frameAxis:{...frameAxis,values:[[Number.EPSILON,0,2],[Number.EPSILON,0,2]] as [number,number,number][]}}
 expect(inspectProgressiveMiterCapParallelism(loops.flat(),straight,law(1),law(0),tiltedOpts,caps).parallel).toEqual([false,false])
 const unclampedOpts={...opts,frameAxis:{...frameAxis,knots:[1,2,5,6],weights:[.5,3]}}
 expect(inspectProgressiveMiterCapParallelism(loops.flat(),straight,law(1),law(0),unclampedOpts,caps).parallel).toEqual([true,true])
 const varyingUnclamped={...unclampedOpts,frameAxis:{...unclampedOpts.frameAxis,values:[[0,0,2],[Number.EPSILON,0,2]] as [number,number,number][]}}
 expect(inspectProgressiveMiterCapParallelism(loops.flat(),straight,law(1),law(0),varyingUnclamped,caps)).toMatchObject({parallel:null,reason:'original-endpoint-axis-unproved'})
 expect(inspectProgressiveMiterCapParallelism(loops.flat(),straight,law(1),law(0),opts,caps,parallel.cells-1).parallel).toBeNull()
 expect(inspectProgressiveMiterCapParallelism(loops.flat(),straight,law(1),law(0),opts,caps,10000,parallel.exactWork-1).parallel).toBeNull()
 expect(report.normalDots!.every(([lo,hi])=>lo>0||hi<0)).toBe(true)
 expect(inspectProgressiveMiterCapProjection(loops.flat(),straight,law(1),law(0),opts,caps,report.cells-1).normalDots).toBeNull()
 expect(inspectProgressiveMiterCapProjection(loops.flat(),straight,law(1),law(0),opts,caps,10000,report.exactWork-1).normalDots).toBeNull()
 const warped=structuredClone(caps);warped[1].controlPoints.at(-1)!.at(-1)![2]!+=.001
 expect(inspectProgressiveMiterCapProjection(loops.flat(),straight,law(1),law(0),opts,warped).normalDots).toBeNull()
 expect(inspectProgressiveMiterCapParallelism(loops.flat(),straight,law(1),law(0),opts,warped).parallel).toBeNull()
 expect({loops,caps,opts}).toEqual(before)
})

it('transports original ideal material domains and preserves them in Rush evidence',async()=>{
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[circleNurbsCurve([0,0,0],[0,0,-1],.2)]]
 const opts={...options,maxSteps:16,maxDeviation:.01}
 const r=inspectProgressiveMiterIdealCapDomains(loops.flat(),straight,law(1),law(0),opts,[1,1])
 expect(r).toMatchObject({idealCapDomainsCertified:true,localDomainCertified:true,reason:null,continuousBound:false})
 expect(inspectProgressiveMiterIdealCapDomains(loops.flat(),straight,law(1),law(0),opts,[1,1],{tolerance:.001,maxPairs:1000,maxCells:r.cells-1,maxExactWork:1000000}).idealCapDomainsCertified).toBe(false)
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 for(const file of ['miter-guide-affine-hollow-corrected.r','miter-combined-frame-affine-hollow-corrected.r']){
  const graph=compileRushFrontend(readFileSync('examples/rush/'+file,'utf8')).document
  const node=graph.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
  const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
  expect(built.report.construction![node.id]).toMatchObject({idealCapDomains:{idealCapDomainsCertified:true,continuousBound:false}})
  expect(JSON.parse(built.nativeGeometry!.geometryJson).sweepEvidence.idealCapDomains.idealCapDomainsCertified).toBe(true)
  const payload=JSON.parse(built.nativeGeometry!.geometryJson)
  const bounds=payload.sweepEvidence.filledCapErrorUpper as [number,number]
  expect(bounds).toHaveLength(2)
  expect(payload.sweepEvidence.boundaryErrorUpper).toBeGreaterThanOrEqual(payload.sweepEvidence.retainedWallErrorUpper)
  expect(bounds.every(bound=>bound<=payload.sweepEvidence.boundaryErrorUpper)).toBe(true)
  expect(payload.sweepEvidence.capParallelism.parallel).toEqual([true,true])
  expect(payload.sweepEvidence.boundaryErrorWithinBudget).toBe(true)
  const {evaluateNurbsSurface}=await import('../src/services/nurbsSurface')
  for(let end=0;end<2;end++){
   const cap=payload.geometry.faces.at(end===0?-2:-1).surface
   const ua=cap.knotsU[cap.degreeU],ub=cap.knotsU[cap.controlPoints.length]
   const va=cap.knotsV[cap.degreeV],vb=cap.knotsV[cap.controlPoints[0].length]
   const p=evaluateNurbsSurface(cap,ua+.75*(ub-ua),va+.5*(vb-va)).point
   // Independent analytic material sample: local annulus radius .25, axes
   // [2,1], transverse center .125 and authored axial offset .25. Explicit
   // correction puts the retained cap at z=0/10; ideal cap is z+.25.
   const radius=Math.hypot((p[1]-.125)/2,-p[0])
   expect(radius).toBeGreaterThan(.2);expect(radius).toBeLessThan(.5)
   expect(Math.abs(p[2]-(10*end+.25))).toBeLessThanOrEqual(bounds[end])
   expect(bounds[end]).toBeLessThan(.250001)
  }
 }
})
