import {expect,it} from 'vitest'
import {bezierNurbsCurve,inspectProgressiveMiterWalls,previewProgressiveMiterNurbsProfiles,type NurbsScaleLaw} from '../src/services/nurbsConstructors'
const law=(value:number):NurbsScaleLaw=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
const profiles=[bezierNurbsCurve([[.1,0,0],[.2,0,0]])]
const points:[number,number,number][]=[[0,0,0],[0,0,10]]
const options={normal:[1,0,0] as [number,number,number],maxSteps:4,maxDeviation:.001}
const budgets={clearance:0,distanceTolerance:.001,maxInjectivityCells:100,maxPairs:100,maxPairCells:100}
it('audits constructor preview sections with Rust-derived adjacency',()=>{
 const preview=previewProgressiveMiterNurbsProfiles(profiles,points,law(1),law(0),options,3)
 const before=structuredClone(preview.sections)
 const report=inspectProgressiveMiterWalls(profiles,points,law(1),law(0),options,preview.sections,budgets)
 expect(report).toMatchObject({chartsAndPairsCertified:true,globalEmbeddingCertified:false,
  c0Boundaries:[[0,1],[1,2]],unresolvedCharts:[]})
 expect(preview.sections).toEqual(before)
 const exhausted=inspectProgressiveMiterWalls(profiles,points,law(1),law(0),options,preview.sections,{...budgets,maxInjectivityCells:0})
 expect(exhausted).toMatchObject({chartsAndPairsCertified:false,declaredBoundariesC0:true,unresolvedCharts:[0,1,2]})
 const changed=structuredClone(preview.sections)
 changed[1]![0]!.weights[0]=2
 expect(()=>inspectProgressiveMiterWalls(profiles,points,law(1),law(0),options,changed,budgets)).toThrow(/correspondence/)
})
it('retains cyclic C0 ownership independently of unresolved global geometry',()=>{
 const sites:[number,number,number][]=[[0,0,0],[10,0,0],[10,10,0],[0,10,0]]
 const profiles=[bezierNurbsCurve([[0,.1,0],[0,.2,0]])]
 const opts={...options,normal:[0,0,1] as [number,number,number],closed:true}
 const preview=previewProgressiveMiterNurbsProfiles(profiles,sites,law(1),law(0),opts,1)
 const report=inspectProgressiveMiterWalls(profiles,sites,law(1),law(0),opts,preview.sections,{...budgets,maxInjectivityCells:0})
 expect(report.declaredBoundariesC0).toBe(true)
 expect(report.c0Boundaries).toEqual([[0,1],[0,3],[1,2],[2,3]])
 expect(report.globalEmbeddingCertified).toBe(false)
 expect(report.chartsAndPairsCertified).toBe(false)
 const broken=structuredClone(preview.sections)
 broken[broken.length-1]![0]!.controlPoints[0]![0]!+=.1
 expect(()=>inspectProgressiveMiterWalls(profiles,sites,law(1),law(0),opts,broken,budgets)).toThrow(/exact retained seam/)
})
it('retains the same bounded evidence in synchronous and streamed body construction',async()=>{
 const {circleNurbsCurve}=await import('../src/services/nurbsConstructors')
 const {createProgressiveMiterBrepProfileBody,streamProgressiveMiterBrepProfileBody}=await import('../src/services/geometry/brep')
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.2)]]
 const opts={...options,retainedWallMaxInjectivityCells:10000,wallAuditBudgets:{...budgets,maxInjectivityCells:0,maxPairs:0,maxPairCells:0}}
 const sync=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),opts)
 expect(sync.wallAudit).toMatchObject({chartsAndPairsCertified:false,globalEmbeddingCertified:false,injectivityCells:0})
 expect(sync.wallAudit.unresolvedCharts.length).toBeGreaterThan(0)
 const stream=streamProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),opts)
 for(;;){
  const level=await stream.next()
  if(level.done){expect(level.value.wallAudit).toEqual(sync.wallAudit);break}
 }
})
it('keeps profile-side ownership inside explicit closed loops',()=>{
 const square=(x:number)=>[
  [[x,0,0],[x+1,0,0]],[[x+1,0,0],[x+1,1,0]],
  [[x+1,1,0],[x,1,0]],[[x,1,0],[x,0,0]],
 ].map(points=>bezierNurbsCurve(points))
 const profiles=[...square(0),...square(3)]
 const preview=previewProgressiveMiterNurbsProfiles(profiles,points,law(1),law(0),options,1)
 const report=inspectProgressiveMiterWalls(profiles,points,law(1),law(0),options,preview.sections,budgets,[4,4])
 expect(report.chartsAndPairsCertified).toBe(true)
 expect(report.c0Boundaries).toHaveLength(8)
 expect(report.c0Boundaries.every(([a,b])=>(a<4)===(b<4))).toBe(true)
 expect(report.globalEmbeddingCertified).toBe(false)
 for(const sizes of [[2,2,4],[4],[0,8]]){
  expect(()=>inspectProgressiveMiterWalls(profiles,points,law(1),law(0),options,preview.sections,budgets,sizes)).toThrow()
 }
})
it('audits both actual retained cap domains and preserves zero-budget refusal',async()=>{
 const {circleNurbsCurve}=await import('../src/services/nurbsConstructors')
 const {createProgressiveMiterBrepProfileBody,streamProgressiveMiterBrepProfileBody}=await import('../src/services/geometry/brep')
 const {reverseNurbsCurve}=await import('../src/services/nurbsCurve')
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]]
 const opts={...options,retainedWallMaxInjectivityCells:10000,wallAuditBudgets:{...budgets,maxInjectivityCells:0,maxPairs:0,maxPairCells:0},contourAuditBudgets:{tolerance:.001,maxPairs:10000,maxCells:10000}}
 const body=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),opts)
 const {DEFAULT_SWEEP_VOLUME_BUDGETS}=await import('../src/services/nurbsSweepEmbedding')
 const shortVolume=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),{
  ...opts,volumeBudgets:{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1},
 })
 expect(shortVolume.volume).toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:false,nesting:null})
 expect(shortVolume.volume.nextPair).not.toBeNull()
 expect(shortVolume.volume.orientations.every(shell=>shell.outward===null&&shell.attempts===0)).toBe(true)
 expect(body.retainedWallCharts).toMatchObject({allChartsCertified:true,unresolvedFaces:[],globalEmbeddingCertified:false})
 expect(body.volume).toMatchObject({solidGeometryCertified:expect.any(Boolean),orientations:expect.any(Array)})
 expect(body.embedding).toMatchObject({solidGeometryCertified:false,totalPairs:body.model.faces.length*(body.model.faces.length-1)/2})
 expect(body.embedding!.caps.map(cap=>cap.face)).toEqual([body.model.faces.length-2,body.model.faces.length-1])
 expect(body.capDomains).toHaveLength(2)
 expect(body.capDomains!.map(cap=>cap.reason)).toEqual([null,null])
 expect(body.capPairs).toMatchObject({capFaces:[body.model.faces.length-2,body.model.faces.length-1],unresolvedPairs:[],globalEmbeddingCertified:false,audit:{pairs:{allPairsSeparated:true}}})
 expect(body.capContacts).toHaveLength(2)
 expect(body.capContacts!.every(contact=>contact.audit.allWallInteriorsExcluded&&!contact.audit.boundaryOwnershipCertified)).toBe(true)

 expect(body.capDomains).toMatchObject([0,1].map(()=>({capDomainCertified:true,capGeometryCertified:false,globalEmbeddingCertified:false})))
 const stream=streamProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),opts)
 for(;;){const next=await stream.next();if(next.done){expect(next.value.capDomains).toEqual(body.capDomains);expect(next.value.capContacts).toEqual(body.capContacts);expect(next.value.capPairs).toEqual(body.capPairs);expect(next.value.retainedWallCharts).toEqual(body.retainedWallCharts);expect(next.value.embedding).toEqual(body.embedding);expect(next.value.volume).toEqual(body.volume);break}}
 const noRetainedI=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),{...opts,retainedWallMaxInjectivityCells:0})
 expect(noRetainedI.retainedWallCharts.allChartsCertified).toBe(false)
 expect(noRetainedI.retainedWallCharts.unresolvedFaces.length).toBe(noRetainedI.model.faces.length-2)
 const exhausted=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),{...opts,contourAuditBudgets:{tolerance:.001,maxPairs:1000,maxCells:0}})
 expect(exhausted.capDomains!.every(cap=>!cap.capDomainCertified&&cap.reason==='contour-budget-exhausted')).toBe(true)
 const capPairBudget=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),{...opts,capPairAuditBudgets:{clearance:0,distanceTolerance:.001,maxInjectivityCells:1000,maxPairs:0,maxPairCells:1000}})
 expect(capPairBudget.capPairs!.unresolvedPairs.map(pair=>pair.faces)).toEqual([[capPairBudget.model.faces.length-2,capPairBudget.model.faces.length-1]])
 const capBudget=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),{...opts,capWallMaxWalls:0})
 expect(capBudget.capContacts!.every(contact=>!contact.audit.allWallInteriorsExcluded&&contact.audit.reason==='cap-wall-budget-exhausted')).toBe(true)

})
