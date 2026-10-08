import {expect,it} from 'vitest'
import {composeSweepBoundaryCertificate} from '../src/services/sweepBoundaryCertificate'

it('distinguishes complete boundary bounds from acceptance and incomplete cap proofs',()=>{
 expect(composeSweepBoundaryCertificate(.1,[.2,.3],false,.4)).toMatchObject({continuousBound:true,withinBudget:true,errorUpper:.3,scope:'boundary-set-hausdorff',reason:null})
 expect(composeSweepBoundaryCertificate(.1,[.2,.3],false,.25)).toMatchObject({continuousBound:true,withinBudget:false,errorUpper:.3,reason:'boundary-budget-exceeded'})
 expect(composeSweepBoundaryCertificate(.1,null,false,.4)).toMatchObject({continuousBound:false,withinBudget:null,errorUpper:null,reason:'filled-cap-bound-unproved'})
 expect(composeSweepBoundaryCertificate(.1,null,true,.4)).toMatchObject({continuousBound:true,withinBudget:true,errorUpper:.1,filledCapErrorUpper:null})
 for(const wall of [null,-1,NaN,Infinity])expect(composeSweepBoundaryCertificate(wall,[0,0],false,.4)).toMatchObject({continuousBound:false,errorUpper:null,reason:'wall-bound-unproved'})
 for(const budget of [0,-1,NaN,Infinity])expect(composeSweepBoundaryCertificate(0,[0,0],false,budget)).toMatchObject({continuousBound:false,errorUpper:null,reason:'invalid-budget'})
})

it('owns its cap bound pair rather than retaining caller-mutable evidence',()=>{
 const caps:[number,number]=[.2,.3],r=composeSweepBoundaryCertificate(.1,caps,false,.4)
 caps[1]=100
 expect(r.filledCapErrorUpper).toEqual([.2,.3])
 expect(r.errorUpper).toBe(.3)
})

it('fails complete construction certification when original cap or retained region work is exhausted',async()=>{
 const {circleNurbsCurve}=await import('../src/services/nurbsConstructors')
 const {createProgressiveMiterBrepProfileBody}=await import('../src/services/geometry/brep')
 const {DEFAULT_SWEEP_VOLUME_BUDGETS}=await import('../src/services/nurbsSweepEmbedding')
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[circleNurbsCurve([0,0,0],[0,0,-1],.2)]]
 const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const twist={...law,values:[0,0]}
 const points:[number,number,number][]=[[0,0,0],[0,0,10]]
 const options={normal:[1,0,0] as [number,number,number],maxSteps:16,maxDeviation:.01}
 const good=createProgressiveMiterBrepProfileBody(loops,points,law,twist,options)
 expect(good.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,reason:null})
 for(const opt of [
  {...options,capDomainBudgets:{tolerance:.001,maxPairs:1000,maxCells:0,maxExactWork:1000000}},
  {...options,volumeBudgets:{...DEFAULT_SWEEP_VOLUME_BUDGETS,capBudgets:{...DEFAULT_SWEEP_VOLUME_BUDGETS.capBudgets,maxExactWork:0}}},
  {...options,capProjectionBudgets:{maxCells:0,maxExactWork:1000000}},
 ]){
  const refused=createProgressiveMiterBrepProfileBody(loops,points,law,twist,opt)
  expect(refused.approximation.report.accepted).toBe(true)
  expect(refused.boundaryCertificate).toMatchObject({continuousBound:false,errorUpper:null,reason:'filled-cap-bound-unproved'})
 }
 // A source profile outside the initial normal plane violates the authored
 // family contract; it must be refused before a boundary certificate exists.
 const oblique=structuredClone(loops)
 for(const loop of oblique)for(const curve of loop)for(const p of curve.controlPoints)p[2]=p[0]!
 expect(()=>createProgressiveMiterBrepProfileBody(oblique,points,law,twist,options)).toThrow(/initial normal plane/)
})

it('bounds nonparallel filled caps in the moving-axis guide affine family and refuses the full budget',async()=>{
 const {createProgressiveMiterBrepProfileBody,streamProgressiveMiterBrepProfileBody}=await import('../src/services/geometry/brep')
 const {reverseNurbsCurve}=await import('../src/services/nurbsCurve')
 const ring=(r:number)=>[
  [[r,0],[r,r],[0,r]],[[0,r],[-r,r],[-r,0]],
  [[-r,0],[-r,-r],[0,-r]],[[0,-r],[r,-r],[r,0]],
 ].map(p=>({degree:2,knots:[0,0,0,1,1,1],controlPoints:p.map(([x,y])=>[x!,y!,0]),weights:[1,Math.SQRT1_2,1],periodic:false}))
 const loops=[ring(.5),ring(.25).reverse().map(reverseNurbsCurve)]
 const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]},twist={...law,values:[0,0]}
 const vector=(values:[number,number,number][],lo:number,hi:number)=>({degree:1,knots:[lo,lo,hi,hi],values,weights:[1,1]})
 const points:[number,number,number][]=[[0,0,0],[0,0,10]]
 const options={normal:[1,0,0] as [number,number,number],maxSteps:16,maxDeviation:.6,retainedWallMaxInjectivityCells:10000,
  frameAxis:vector([[0,0,1],[0,1,1]],2,5),frameNormal:vector([[0,1,0],[0,1,0]],11,13),
  orientationGuide:{degree:1,knots:[7,7,9,9],controlPoints:[[1,0,0],[1,0,10]],weights:[1,1],periodic:false},
  axisScale:vector([[2,1,1],[2,1,1]],17,19),
  capCorrection:{quantum:2**-40,tolerance:.6,maxWork:1000000},
 }
 const good=createProgressiveMiterBrepProfileBody(loops,points,law,twist,options)
 expect(good.capParallelism?.parallel).toEqual([true,false])
 expect(good.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,reason:null})
 expect(good.filledCapErrorUpper![1]).toBeGreaterThan(.5)
 expect(good.filledCapErrorUpper![1]).toBeLessThan(.6)
 // Independent ideal endpoint formula: n=X, b=(0,1,-1)/sqrt(2).
 // Interior annulus points project into the corrected cap at constant Z=10.
 for(const radius of [.3,.4])for(let i=0;i<24;i++){
  const y=radius*Math.sin(i*Math.PI/12)
  const verticalDistance=Math.abs(y/Math.SQRT2)
  expect(verticalDistance).toBeLessThan(good.filledCapErrorUpper![1])
 }
 expect(good.retainedWallErrorUpper).toBeLessThan(.45)
 const tight={...options,maxDeviation:.45}
 expect(()=>createProgressiveMiterBrepProfileBody(loops,points,law,twist,tight)).toThrow(/complete boundary error/)
 const stream=streamProgressiveMiterBrepProfileBody(loops,points,law,twist,tight)
 await expect((async()=>{for await(const preview of stream)void preview})()).rejects.toThrow(/complete boundary error/)
})

it('carries the nonparallel cap budget and explicit chart work through Rush and JSON',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileModelGraphText}=await import('../src/services/modelGraphText')
 const {buildOwnNurbs,buildOwnNurbsAsync}=await import('../src/services/modelGraphNurbsKernel')
 const source=readFileSync('examples/rush/miter-moving-frame-guide-affine-hollow-corrected.r','utf8')
 const graph=compileModelGraphText(source).document
 const node=graph.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 expect(node).toMatchObject({retained_wall_max_injectivity_cells:10000})
 const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction![node.id]).toMatchObject({continuousBound:true,boundaryCertificate:{withinBudget:true,scope:'boundary-set-hausdorff'},capParallelism:{parallel:[true,false]}})
 const payload=JSON.parse(built.nativeGeometry!.geometryJson)
 expect(payload.sweepEvidence.boundaryCertificate.continuousBound).toBe(true)
 expect(payload.sweepEvidence.volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null,nesting:{rolesConsistent:true},orientations:[{outward:true,expectedOutward:true}]})
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,payload.geometry)?.solidGeometryCertified).toBe(true)
 const broken=structuredClone(payload.geometry)
 broken.faces[0].surface.controlPoints[1][1][2]+=1e-12
 expect(()=>inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,broken)).toThrow(/exact boundary agreement/)
 expect(payload.sweepEvidence.volume.solidGeometryCertified).toBe(true) // A stale positive flag cannot admit changed geometry.
 const tight=compileModelGraphText(source.replace('max_deviation: 0.6mm','max_deviation: 0.45mm')).document
 expect(()=>buildOwnNurbs(tight,{action:'build'})).toThrow(/complete boundary error/)
 await expect(buildOwnNurbsAsync(tight,{action:'build'})).rejects.toThrow(/complete boundary error/)
 const exhausted=compileModelGraphText(source.replace('retained_wall_max_injectivity_cells: 10000','retained_wall_max_injectivity_cells: 0')).document
 expect(()=>buildOwnNurbs(exhausted,{action:'build'})).toThrow(/retained wall regularity unproved/)
 expect(()=>compileModelGraphText(source.replace('retained_wall_max_injectivity_cells: 10000','retained_wall_max_injectivity_cells: 10mm'))).toThrow()
})
