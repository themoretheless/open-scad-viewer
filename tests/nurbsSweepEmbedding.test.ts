import {expect,it} from 'vitest'
import {createRationalBrepSectionLoft,createPeriodicBrepSectionLoft,createProgressiveMiterBrepProfileBody,transformNurbsBrep} from '../src/services/geometry/brep'
import {circleNurbsCurve,miterNurbsProfileSections} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createNativeGeometryArtifact} from '../src/core/nativeGeometry'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
import type {NurbsCurve} from '../src/services/nurbsCurve'
import {inspectSweepEmbedding,inspectSweepVolume,type SweepEmbeddingBudgets} from '../src/services/nurbsSweepEmbedding'
import {inspectSweepRetainedCorrespondence,inspectSweepRetainedCaps} from '../src/services/sweepRetainedCorrespondence'
import {inspectNativeSweepCapContacts} from '../src/services/nurbsSweepCapContacts'
const budgets:SweepEmbeddingBudgets={toleranceUv:1e-8,maxExactWork:1000000,
 maxTrimPairs:1000,maxTrimCells:10000,maxTrimDomainCells:100000,maxSpans:100,maxLinearCells:10000,
 maxPairs:100,maxCells:10000,maxDomainCells:100000,cellsPerPair:16,domainCellsPerPair:1000,
 capBudgets:{maxWalls:1024,maxExactWork:1000000,maxChartCells:1000,maxTrimPairs:100000,maxTrimCells:100000,maxTrimDomainCells:1000000}}
const section=(z:number):NurbsCurve[][]=>[[
 [[.5,0],[.5,.5],[0,.5]],[[0,.5],[-.5,.5],[-.5,0]],
 [[-.5,0],[-.5,-.5],[0,-.5]],[[0,-.5],[.5,-.5],[.5,0]],
].map(p=>({degree:2,knots:[0,0,0,1,1,1],controlPoints:p.map(xy=>[xy[0]!,xy[1]!,z]),weights:[1,Math.SQRT1_2,1],periodic:false}))]
it('keeps closed authored-frame guide affine cavity orientation separate from embedding',()=>{
 const scalar=(x:number)=>({degree:1,knots:[0,0,1,1],values:[x,x],weights:[1,1]})
 const body=createProgressiveMiterBrepProfileBody(
  [[circleNurbsCurve([0,0,0],[1,0,0],.25)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[1,0,0],.1))]],
  [[0,0,0],[10,0,0],[10,10,0],[0,10,0]],scalar(1),scalar(0),{
   normal:[0,0,1],closed:true,maxDeviation:10,maxSteps:8,retainedWallMaxInjectivityCells:10000,
   frameAxis:{degree:1,knots:[0,0,.25,.5,.75,1,1],values:[[1,-1,0],[1,1,0],[-1,1,0],[-1,-1,0],[1,-1,0]],weights:[1,1,1,1,1]},
   frameNormal:{degree:1,knots:[2,2,5,5],values:[[0,0,1],[0,0,1]],weights:[1,1]},
   orientationGuide:{degree:1,knots:[0,0,.25,.5,.75,1,1],controlPoints:[[0,0,100],[10,0,100],[10,10,100],[0,10,100],[0,0,100]],weights:[1,1,1,1,1],periodic:false},
   axisScale:{degree:1,knots:[7,7,9,9],values:[[2,1,1],[2,1,1]],weights:[1,1]},
  })
 const model=body.model,before=structuredClone(model)
 const limits={...budgets,maxTrimPairs:10000,maxTrimCells:100000,maxTrimDomainCells:1000000,
  maxSpans:1000,maxPairs:10000,maxCells:100000,maxDomainCells:1000000,cellsPerPair:1000,domainCellsPerPair:10000,
  nestingPairs:100,nestingCells:100000,nestingDomainCells:1000000,orientationCells:100000,orientationDomainCells:1000000,orientationSpans:100}
 expect(body.approximation.report).toMatchObject({authoredFramesApplied:true,orientationGuideApplied:true,affineLawsApplied:true})
 const positive=inspectSweepVolume(model,[],limits)
 expect(positive).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,
  nesting:{rolesConsistent:true,parents:[null,0]},orientations:[{outward:true},{outward:false}]})
 const inverted=structuredClone(model),inner=inverted.bodies[0]!.innerShells[0]!
 for(const use of inverted.shells[inner]!.faces)use.reversed=!use.reversed
 const invertedBefore=structuredClone(inverted)
 const refused=inspectSweepVolume(inverted,[],limits)
 expect(refused).toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:true,
  nesting:{rolesConsistent:true,parents:[null,0]},orientations:[{outward:true},{outward:true}]})
 const wrongRoles=structuredClone(model)
 wrongRoles.bodies=[{outerShell:0,innerShells:[]},{outerShell:inner,innerShells:[]}]
 // The body count changed; use the supported legacy geometry form rather
 // than retaining the old one-body topology identity table.
 delete wrongRoles.topologyIds
 const wrongRolesBefore=structuredClone(wrongRoles)
 const wrongOwnership=inspectSweepVolume(wrongRoles,[],limits)
 expect(wrongOwnership).toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:true,
  nesting:{rolesConsistent:false,parents:[null,0]},orientations:[{outward:null},{outward:null}]})
 expect(wrongRoles).toEqual(wrongRolesBefore)
 const exhaustedNesting=inspectSweepVolume(model,[],{...limits,nestingCells:2,nestingDomainCells:2})
 expect(exhaustedNesting).toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:true,
  nesting:{rolesConsistent:null,parents:null},orientations:[{outward:null},{outward:null}]})
 expect(exhaustedNesting.nesting!.cells).toBeLessThanOrEqual(2)
 expect(exhaustedNesting.nesting!.domainCells).toBeLessThanOrEqual(2)
 const exhaustedOrientation=inspectSweepVolume(model,[],{...limits,orientationCells:1,orientationDomainCells:1})
 expect(exhaustedOrientation).toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:true,
  nesting:{rolesConsistent:true,parents:[null,0]}})
 expect(exhaustedOrientation.orientations.some(shell=>shell.outward===null)).toBe(true)
 expect(exhaustedOrientation.orientationCells).toBeLessThanOrEqual(1)
 expect(exhaustedOrientation.orientationDomainCells).toBeLessThanOrEqual(1)
 expect(inverted).toEqual(invertedBefore)
 expect(model).toEqual(before)
})
it('proves retained oblique caps from exact original planarity and refuses warped charts',()=>{
 const endpoints:[NurbsCurve[][],NurbsCurve[][]]=[section(0),section(5)]
 const original=createRationalBrepSectionLoft(endpoints)
 const model=transformNurbsBrep(original,[[1,0,0,0],[0,1,0,0],[1,1,1,0],[0,0,0,1]])
 const placed=structuredClone(endpoints)
 for(const rings of placed)for(const ring of rings)for(const curve of ring)for(const point of curve.controlPoints)point[2]=point[0]!+point[1]!+point[2]!
 const before=structuredClone({model,placed})
 for(const face of [4,5])expect(inspectNativeSweepCapContacts(model,face,[4,5],budgets.capBudgets)).toMatchObject({capCertified:true,planarControlHullCertified:true})
 expect(inspectSweepRetainedCaps(model,placed,budgets.capBudgets)).toMatchObject({exact:true,capErrorUpper:0})
 expect(inspectSweepRetainedCaps(model,placed,{...budgets.capBudgets,maxExactWork:0})).toMatchObject({exact:false,capErrorUpper:null,exactWork:0})
 const warped=structuredClone(model)
 warped.faces[5]!.surface.controlPoints[1]![1]![2]+=8*Number.EPSILON
 expect(inspectNativeSweepCapContacts(warped,5,[4,5],budgets.capBudgets).planarControlHullCertified).toBe(false)
 expect(inspectSweepRetainedCaps(warped,placed,budgets.capBudgets)).toMatchObject({exact:false,capErrorUpper:null,reason:'cap-region-unproved'})
 expect({model,placed}).toEqual(before)
})
it('proves retained scale/twist hollow volume within shared budgets',()=>{
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]]
 const law=(a:number,b:number)=>({degree:1,knots:[0,0,1,1],values:[a,b],weights:[1,1]})
 const body=createProgressiveMiterBrepProfileBody(loops,[[0,0,0],[0,0,10]],law(1,1.5),law(0,30),{normal:[1,0,0],maxDeviation:.02,maxSteps:16})
 const caps=[body.model.faces.length-2,body.model.faces.length-1]
 expect(body.retainedCorrespondence).toMatchObject({exact:true,wallErrorUpper:0})
 const sections=body.approximation.sections!.map(row=>row.map(c=>[c]))
 expect(inspectSweepRetainedCorrespondence(body.model,sections,false)).toMatchObject({exact:true,wallErrorUpper:0})
 const retainedCaps=inspectSweepRetainedCaps(body.model,[sections[0]!,sections.at(-1)!],budgets.capBudgets)
 expect(retainedCaps).toMatchObject({exact:true,capErrorUpper:0})
 expect(retainedCaps.exactWork).toBeGreaterThan(0)
 expect(retainedCaps.exactWork).toBeLessThanOrEqual(budgets.capBudgets.maxExactWork)
 expect(inspectSweepRetainedCaps(body.model,[sections[0]!,sections.at(-1)!],{...budgets.capBudgets,maxExactWork:0})).toMatchObject({exact:false,capErrorUpper:null})
 const firstCap=inspectNativeSweepCapContacts(body.model,caps[0]!,caps,budgets.capBudgets)
 expect(firstCap.capCertified).toBe(true)
 expect(firstCap.exactWork).toBeGreaterThan(0)
 expect(inspectSweepRetainedCaps(body.model,[sections[0]!,sections.at(-1)!],{...budgets.capBudgets,maxExactWork:firstCap.exactWork})).toMatchObject({exact:false,capErrorUpper:null,exactWork:firstCap.exactWork})
 const wrongEnd=structuredClone(sections.at(-1)!)
 wrongEnd[0]![0]!.controlPoints[0]![0]+=Number.EPSILON
 expect(inspectSweepRetainedCaps(body.model,[sections[0]!,wrongEnd],budgets.capBudgets)).toMatchObject({exact:false,reason:'cap-contour-mismatch'})
 expect(inspectSweepRetainedCorrespondence(body.model,sections,false,1)).toMatchObject({exact:false,reason:'work-limit',wallErrorUpper:null})
 const altered=structuredClone(body.model)
 altered.faces[0]!.surface.controlPoints[0]![0]![0]+=Number.EPSILON
 expect(inspectSweepRetainedCorrespondence(altered,sections,false)).toMatchObject({exact:false,reason:'retained-wall-mismatch',wallErrorUpper:null})
 const b={...budgets,maxTrimPairs:10000,maxTrimCells:100000,maxTrimDomainCells:1000000,maxPairs:10000,maxCells:100000,maxDomainCells:1000000,cellsPerPair:1000,domainCellsPerPair:10000}
 const report=inspectSweepEmbedding(body.model,caps,b)
 expect(report).toMatchObject({exactBoundaryCertified:true,trimCertified:true,allFacesInjective:true,boundaryEmbeddingCertified:true,allPairsClassified:true,nextPair:null})
 expect(report.caps.every(c=>c.capCertified&&c.unresolvedWalls.length===0)).toBe(true)
 const volume=inspectSweepVolume(body.model,caps,{...b,nestingPairs:1000,nestingCells:100000,nestingDomainCells:1000000,orientationCells:100000,orientationDomainCells:1000000,orientationSpans:100})
 expect(volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,nesting:{rolesConsistent:true,parents:[null]},orientations:[{shell:0,outward:true,expectedOutward:true}]})
 expect(inspectSweepEmbedding(body.model,caps,{...b,maxExactWork:1}).exactBoundaryCertified).toBe(false)
})
it('proves retained boundary embedding with independent unresolved budget evidence',()=>{
 const model=createRationalBrepSectionLoft([section(0),section(5)]),before=structuredClone(model)
 const report=inspectSweepEmbedding(model,[4,5],budgets)
 expect(report).toMatchObject({boundaryEmbeddingCertified:true,solidGeometryCertified:false,
  exactBoundaryCertified:true,trimCertified:true,allFacesInjective:true,allPairsClassified:true,totalPairs:15,nextPair:null,unresolvedFaces:[]})
 expect(report.pairs).toHaveLength(report.individualPairs)
 expect(report.individualPairs+report.groupedPairs).toBe(report.totalPairs)
 expect(report.caps.map(c=>c.face)).toEqual([4,5])
 const exhausted=inspectSweepEmbedding(model,[4,5],{...budgets,capBudgets:{...budgets.capBudgets,maxExactWork:0}})
 expect(exhausted.boundaryEmbeddingCertified).toBe(false)
 expect(exhausted.caps.every(c=>!c.capCertified)).toBe(true)
 expect(exhausted.allPairsClassified).toBe(true)
 expect(exhausted.individualPairs+exhausted.groupedPairs).toBe(exhausted.totalPairs)
 const prefix=inspectSweepEmbedding(model,[4,5],{...budgets,maxPairs:1})
 expect(prefix.boundaryEmbeddingCertified).toBe(false)
 expect(prefix.pairs).toHaveLength(1)
 expect(prefix.nextPair).not.toBeNull()
 expect(()=>inspectSweepEmbedding(model,[4,4],budgets)).toThrow()
 const withoutCaps=inspectSweepEmbedding(model,[],budgets)
 expect(withoutCaps).toMatchObject({boundaryEmbeddingCertified:true,solidGeometryCertified:false,allPairsClassified:true})
 expect(withoutCaps.caps).toEqual([])
 expect(model).toEqual(before)
})
it('retains volume stage evidence and refuses reversed material orientation',()=>{
 const model=createRationalBrepSectionLoft([section(0),section(5)]),before=structuredClone(model)
 const volume={...budgets,nestingPairs:100,nestingCells:100000,nestingDomainCells:1000000,
  orientationCells:100000,orientationDomainCells:1000000,orientationSpans:100}
 const report=inspectSweepVolume(model,[4,5],volume)
 expect(report).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,
  nesting:{rolesConsistent:true,parents:[null]},orientations:[{shell:0,expectedOutward:true,outward:true}]})
 const short=inspectSweepVolume(model,[4,5],{...volume,orientationCells:1,orientationDomainCells:1})
 expect(short).toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:true})
 expect(short.orientationCells).toBeLessThanOrEqual(1)
 expect(short.orientationDomainCells).toBeLessThanOrEqual(1)
 const inward=structuredClone(model)
 for(const face of inward.shells[0]!.faces)face.reversed=!face.reversed
 const wrong=inspectSweepVolume(inward,[4,5],volume)
 expect(wrong).toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:true,
  orientations:[{shell:0,expectedOutward:true,outward:false}]})
 const noBoundary=inspectSweepVolume(model,[4,5],{...volume,capBudgets:{...volume.capBudgets,maxExactWork:0}})
 expect(noBoundary).toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:false,nesting:null})
 expect(noBoundary.orientations.every(s=>s.attempts===0&&s.outward===null)).toBe(true)
 expect(model).toEqual(before)
 const artifact=createNativeGeometryArtifact('sweep','brep',{geometry:model,
  sweepEvidence:{volume:{solidGeometryCertified:true}}},{nodes:[{id:'sweep',op:'brep_progressive_miter_sweep',closed:false}],root:'sweep'})
 expect(inspectProgressiveSweepSolidAdmission(artifact,model)?.solidGeometryCertified).toBe(true)
 const ordinary=createNativeGeometryArtifact('sweep','brep',{geometry:model,sweepEvidence:{volume:{solidGeometryCertified:false}}},{nodes:[{id:'sweep',op:'brep_miter_sweep',closed:false}],root:'sweep'})
 expect(inspectProgressiveSweepSolidAdmission(ordinary,model)?.solidGeometryCertified).toBe(true)
 expect(()=>inspectProgressiveSweepSolidAdmission(ordinary,inward)).toThrow(/snapshot binding/)
 expect(()=>inspectProgressiveSweepSolidAdmission(artifact,inward)).toThrow(/snapshot binding/)
 const translated=transformNurbsBrep(model,[[1,0,0,13],[0,1,0,-7],[0,0,1,3],[0,0,0,1]])
 const derived=createNativeGeometryArtifact('placed','brep',{geometry:translated},{nodes:[
  {id:'placed',op:'transform',input:'sweep'},
  {id:'sweep',op:'brep_progressive_miter_sweep',closed:false},
 ],root:'placed'})
 expect(inspectProgressiveSweepSolidAdmission(derived,translated)?.solidGeometryCertified).toBe(true)
 const ordinaryPlaced=createNativeGeometryArtifact('placed','brep',{geometry:translated,sweepEvidence:{volume:{solidGeometryCertified:true}}},{nodes:[{id:'placed',op:'transform',input:'sweep'},{id:'sweep',op:'brep_miter_sweep',closed:false}],root:'placed'})
 expect(inspectProgressiveSweepSolidAdmission(ordinaryPlaced,translated)?.solidGeometryCertified).toBe(true)
 expect(()=>inspectProgressiveSweepSolidAdmission(ordinaryPlaced,inward)).toThrow(/snapshot binding/)
 expect(()=>inspectProgressiveSweepSolidAdmission(derived,inward)).toThrow(/snapshot binding/)
 const cyclic=createNativeGeometryArtifact('placed','brep',{geometry:model},{nodes:[
  {id:'placed',op:'transform',input:'placed'},
 ],root:'placed'})
 expect(()=>inspectProgressiveSweepSolidAdmission(cyclic,model)).toThrow(/Cyclic/)
 const unrelated=createNativeGeometryArtifact('cube','brep',{geometry:model},{nodes:[{id:'cube',op:'brep_box'}],root:'cube'})
 expect(inspectProgressiveSweepSolidAdmission(unrelated,model)).toBeNull()
})
it('proves periodic hollow material roles through packaged no-cap volume audit',()=>{
 const profiles=[circleNurbsCurve([0,0,0],[1,0,0],.5),reverseNurbsCurve(circleNurbsCurve([0,0,0],[1,0,0],.2))]
 const rows=miterNurbsProfileSections(profiles,[[0,0,0],[10,0,0],[10,10,0],[0,10,0]],[0,0,1],4,true)
 const model=createPeriodicBrepSectionLoft(rows.map(row=>[[row[0]!],[row[1]!]])),before=structuredClone(model)
 const report=inspectSweepVolume(model,[],{...budgets,maxTrimPairs:10000,maxTrimCells:100000,
  maxTrimDomainCells:1000000,maxSpans:1000,maxPairs:10000,maxCells:100000,
  maxDomainCells:1000000,cellsPerPair:1000,domainCellsPerPair:10000,
  nestingPairs:100,nestingCells:100000,nestingDomainCells:1000000,
  orientationCells:100000,orientationDomainCells:1000000,orientationSpans:100})
 expect(report).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,
  nesting:{rolesConsistent:true,parents:[null,0]},orientations:[{outward:true},{outward:false}]})
 expect(model).toEqual(before)
},120000)
