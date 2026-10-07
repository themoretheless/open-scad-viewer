import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {bezierNurbsCurve,circleNurbsCurve,miterNurbsProfileSections} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createMiterBrepProfileBody,inspectNurbsBrep,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
import {exportDirectStepV5,importDirectStepV5} from '../src/services/cadNurbsStep'

it('retains rational miter controls and agrees with independent right-angle extrusion geometry',()=>{
  const profile=bezierNurbsCurve([[1,0,0],[1.5,1,0],[2,0,0]],[1,.7,2])
  const sections=miterNurbsProfileSections([profile],[[0,0,0],[0,0,10],[10,0,10]],[1,0,0],2)
  expect(sections[0]![0]).toEqual(profile)
  for(const section of sections){expect(section[0]!.weights).toEqual(profile.weights);expect(section[0]!.knots).toEqual(profile.knots)}
  expect(sections[1]![0]!.controlPoints).toEqual([[1,0,9],[1.5,1,8.5],[2,0,8]])
  expect(sections[2]![0]!.controlPoints).toEqual([[10,0,9],[10,1,8.5],[10,0,8]])
  expect(()=>miterNurbsProfileSections([profile],[[0,0,0],[0,0,10],[10,0,10]],[1,0,0],1.1)).toThrow(/limit/)
  expect(()=>miterNurbsProfileSections([profile],[[0,0,0],[0,0,1],[1,0,1]],[1,0,0],2)).toThrow(/consume|reverse/)
})

it('preserves rational hollow walls and caps through spatial miter joints and STEP transport',()=>{
  const outer=circleNurbsCurve([0,0,0],[0,0,1],.5)
  const hole=reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))
  const body=createMiterBrepProfileBody([[outer],[hole]],[[0,0,0],[0,0,10],[10,0,10],[10,10,15]],[1,0,0],2)
  expect(body.report).toMatchObject({method:'polyline-miter-sections',sections:4,globalEmbeddingCertified:false,roundingCertified:false})
  expect(body.model.faces).toHaveLength(26)
  expect(body.model.faces.filter(f=>f.holes.length===1)).toHaveLength(2)
  expect(inspectNurbsBrep(body.model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
  const mesh=tessellateNurbsBrep(body.model,4)
  expect(mesh.report.closed).toBe(true)
  const idealVolume=Math.PI*(0.5**2-0.2**2)*(20+Math.hypot(10,5))
  expect(Math.abs(mesh.report.signedVolumeMm3-idealVolume)/idealVolume).toBeLessThan(0.03)
  const imported=importDirectStepV5(exportDirectStepV5(body.model).text).model
  expect(imported.faces).toHaveLength(26)
  expect(inspectNurbsBrep(imported).boundaryEdgeCount).toBe(0)
})

it('lowers miter limits and spatial sites through Rush, parser and Solid',async()=>{
  const source=readFileSync('examples/rush/miter-hollow-body.r','utf8')
  const compiled=compileRushFrontend(source)
  const node=compiled.document.nodes.find(n=>n.op==='brep_miter_sweep')!
  expect(node).toMatchObject({miter_limit:2,points:[[0,0,0],[0,0,10],[10,0,10],[10,10,15]]})
  const built=buildOwnNurbs(compiled.document,{action:'build'})
  expect(built.report.construction?.[node.id]).toMatchObject({sections:4,globalEmbeddingCertified:false,roundingCertified:false})
  expect(()=>compileRushFrontend(source.replace('miter_limit: 2','miter_limit: 2mm'))).toThrow()
  const {parseOpenSCAD}=await import('../src/services/openscadParser')
  const {sceneMeshesToSolidDocument}=await import('../src/services/solidBridge')
  const scene=await parseOpenSCAD(source)
  expect(()=>sceneMeshesToSolidDocument(scene.meshes)).toThrow(/Miter sweep Solid geometry could not be proved: boundary embedding/)
})

it('retains cyclic miter extrusion correspondence and shared rational seam controls',()=>{
  const profile=bezierNurbsCurve([[0,.1,.2],[0,.3,.4]],[1,2])
  const points:[number,number,number][]=[[0,0,0],[10,0,0],[10,10,0],[0,10,0]]
  const sections=miterNurbsProfileSections([profile],points,[0,0,1],2,true)
  expect(sections).toHaveLength(5)
  expect(sections[0]).toEqual(sections.at(-1))
  expect(sections[0]![0]!.controlPoints[0]).toEqual([.1,.1,.2])
  for(const section of sections)expect(section[0]!.weights).toEqual(profile.weights)
  const spatial:[number,number,number][]=[[0,0,0],[10,0,0],[10,10,4],[0,10,1],[0,5,-2]]
  expect(()=>miterNurbsProfileSections([profile],spatial,[0,0,1],4,true)).toThrow(/holonomy/)
})

it('preserves a hollow closed miter body without caps through Rush, Solid and STEP',async()=>{
  const source=readFileSync('examples/rush/closed-miter-hollow-body.r','utf8')
  const compiled=compileRushFrontend(source)
  const node=compiled.document.nodes.find(n=>n.op==='brep_miter_sweep')!
  const built=buildOwnNurbs(compiled.document,{action:'build'})
  expect(built.report.construction?.[node.id]).toMatchObject({closedPath:true,sections:5,globalEmbeddingCertified:false,profileRegularityCertified:true,wallRegularityCertified:true,continuousBound:false,
    retainedWallCharts:{allChartsCertified:true,unresolvedFaces:[]}})
  const charts=(built.report.construction?.[node.id] as {retainedWallCharts:{charts:{face:number}[];cells:number}}).retainedWallCharts
  expect(charts.charts.map(chart=>chart.face)).toEqual(Array.from({length:32},(_,face)=>face))
  expect(charts.cells).toBeLessThanOrEqual(20000)
  expect(compileRushFrontend(source.replace('closed: true','closed: false')).document.nodes.find(n=>n.op==='brep_miter_sweep')).toMatchObject({closed:false})
  expect(()=>compileRushFrontend(source.replace('closed: true','closed: 1mm'))).toThrow()
  const {parseOpenSCAD}=await import('../src/services/openscadParser')
  const {sceneMeshesToSolidDocument}=await import('../src/services/solidBridge')
  const scene=await parseOpenSCAD(source)
  const body=sceneMeshesToSolidDocument(scene.meshes).bodies[0]!.brep!
  expect(body.faces).toHaveLength(32)
  expect(body.shells).toHaveLength(2)
  expect(body.bodies[0]!.innerShells).toEqual([1])
  expect(body.faces.every(f=>f.holes.length===0)).toBe(true)
  expect(inspectNurbsBrep(body)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
  const {inspectSweepSeams}=await import('../src/services/nurbsSweepAudit')
  // Every path corner, including the cyclic join, has regular adjacent charts
  // but an intentionally sharp miter normal. Closed topology cannot certify G1.
  const cornerSeams=Array.from({length:4},(_,layer)=>({
    patches:[layer*8,((layer+1)%4)*8] as [number,number],
    boundaries:['vMax','vMin'] as ['vMax','vMin'],order:1 as const,
    normalScale:1,jetTolerance:1e-8,
  }))
  const cornerAudit=inspectSweepSeams(body.faces.map(face=>face.surface),cornerSeams)
  expect(cornerAudit.inspectedSeams).toBe(4)
  expect(cornerAudit.exactG1G2Certified).toBe(false)
  expect(cornerAudit.seams.every(seam=>!seam.withinJetBudget)).toBe(true)
  const {inspectSweepRetainedWallCharts}=await import('../src/services/nurbsSweepRetainedCharts')
  const folded=structuredClone(body)
  const closing=folded.faces[24]!.surface
  for(const row of closing.controlPoints)row[1]=[...row[0]!]
  expect(inspectSweepRetainedWallCharts(folded,[],20000).unresolvedFaces).toContain(24)
  const imported=importDirectStepV5(exportDirectStepV5(body).text).model
  expect(imported.shells).toHaveLength(2)
  expect(inspectNurbsBrep(imported).boundaryEdgeCount).toBe(0)
  const mesh=tessellateNurbsBrep(body,4)
  const idealVolume=Math.PI*(.5**2-.2**2)*40
  expect(Math.abs(mesh.report.signedVolumeMm3-idealVolume)/idealVolume).toBeLessThan(.03)
})

it('carries explicit bounded cap correction from Rush through parser to the retained Solid',async()=>{
 const source=readFileSync('examples/rush/miter-hollow-corrected.r','utf8')
 const compiled=compileRushFrontend(source)
 const node=compiled.document.nodes.find(n=>n.op==='brep_miter_sweep')!
 expect(node).toMatchObject({cap_correction_tolerance:1e-9,cap_correction_quantum:2**-40,cap_correction_max_work:1000000})
 const built=buildOwnNurbs(compiled.document,{action:'build'})
 expect(built.report.construction?.[node.id]).toMatchObject({
  sectionCorrection:{reason:'bounded-section-interpolation',exactPlanarSections:[0,3]},
  retainedCorrespondence:{exact:true,wallErrorUpper:0},roundingCertified:false,
 })
 const {parseOpenSCAD}=await import('../src/services/openscadParser')
 const {sceneMeshesToSolidDocument}=await import('../src/services/solidBridge')
 const {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS}=await import('../src/services/nurbsSweepEmbedding')
 const scene=await parseOpenSCAD(source)
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 expect(scene.meshes.map(mesh=>readSweepViewportEvidence(mesh.nativeGeometry)).filter(Boolean)).toEqual([expect.objectContaining({solidGeometryCertified:true,continuousBound:false,wallRegularityCertified:true,profileRegularityCertified:true})])
 const solid=sceneMeshesToSolidDocument(scene.meshes)
 expect(solid.bodies).toHaveLength(1)
 const model=solid.bodies[0]!.brep!
 expect(inspectSweepVolume(model,[24,25],DEFAULT_SWEEP_VOLUME_BUDGETS).solidGeometryCertified).toBe(true)
 expect(()=>buildOwnNurbs(compileRushFrontend(source.replace('cap_correction_max_work: 1000000','cap_correction_max_work: 0')).document,{action:'build'})).toThrow(/correction unproved/)
})
