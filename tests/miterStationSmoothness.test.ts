import {it,expect} from 'vitest'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {createRationalBrepSectionLoft,createMiterBrepProfileBody} from '../src/services/geometry/brep'
import {inspectMiterStationSmoothness} from '../src/services/miterStationSmoothness'
const straight=()=>createRationalBrepSectionLoft([0,5,10].map(z=>[[circleNurbsCurve([0,0,z],[0,0,1],.5)]]))
it('certifies all straight station joins independently of caps and refuses budget exhaustion',()=>{
 const model=straight(),caps=[model.faces.length-2,model.faces.length-1]
 const r=inspectMiterStationSmoothness(model,caps)
 expect(r).toMatchObject({extractionComplete:true,stationG1Certified:true,stationG2Certified:true,g2:{certifiedOrder:2}})
 expect(r.edgeIds).toHaveLength(4);expect(r.capEdges).toHaveLength(8)
 expect(inspectMiterStationSmoothness(model,caps,r.exactWork-1)).toMatchObject({stationG1Certified:false,stationG2Certified:false})
})
it('does not promote sharp miter corners or hide damaged/partial station boundaries',()=>{
 const body=createMiterBrepProfileBody([[circleNurbsCurve([0,0,0],[0,0,1],.5)]],[[0,0,0],[0,0,10],[10,0,10]],[1,0,0],2)
 expect(inspectMiterStationSmoothness(body.model,[body.model.faces.length-2,body.model.faces.length-1])).toMatchObject({extractionComplete:true,stationG1Certified:false,stationG2Certified:false})
 const model=straight(),face=model.faces[0]!,edge=model.loops[face.outer]!.coedges.find(c=>c.pcurve.controlPoints[0]![1]===c.pcurve.controlPoints[1]![1])!
 edge.pcurve.controlPoints[1]![0]=.5
 expect(inspectMiterStationSmoothness(model,[model.faces.length-2,model.faces.length-1])).toMatchObject({extractionComplete:false,stationG1Certified:false,stationG2Certified:false})
})

it('certifies unequal straight spans and distinguishes station G1 from unproved G2',()=>{
 const unequal=createRationalBrepSectionLoft([0,4,12].map(z=>[[circleNurbsCurve([0,0,z],[0,0,1],.5)]]))
 expect(inspectMiterStationSmoothness(unequal,[unequal.faces.length-2,unequal.faces.length-1])).toMatchObject({stationG1Certified:true,stationG2Certified:true})
 const model=createRationalBrepSectionLoft([0,6,12].map(z=>[[circleNurbsCurve([0,0,z],[0,0,1],.5)]]))
 for(const face of model.faces.slice(0,-2)){
  const s=face.surface
  s.controlPoints=s.controlPoints.map(row=>{
   const [a,b]=row as [number[],number[]]
   return [a,[a[0]!,a[1]!,a[2]!+2],[b[0]!+(a[2]===6?.25:0),b[1]!,b[2]!-2],b]
  })
  s.weights=s.weights.map(row=>[row[0]!,row[0]!,row[1]!,row[1]!]);s.degreeV=3;s.knotsV=[0,0,0,0,1,1,1,1]
 }
 const r=inspectMiterStationSmoothness(model,[model.faces.length-2,model.faces.length-1])
 expect(r).toMatchObject({extractionComplete:true,stationG1Certified:true,stationG2Certified:false,g1Audit:{certifiedOrder:1}})
 expect(r.exactWork).toBe(r.g2.exactWork+r.g1Audit!.exactWork)
})
it('carries applicable station G2 through Rush while retaining cap C0 and Solid',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileModelGraphText}=await import('../src/services/modelGraphText')
 const {buildOwnNurbs}=await import('../src/services/modelGraphNurbsKernel')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const graph=compileModelGraphText(readFileSync('examples/rush/progressive-miter-station-g2-hollow.r','utf8'))
 const built=buildOwnNurbs(graph.document,{action:'build',display:{segments:4,subdivisionLevels:0}})

 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({stationG1Certified:true,stationG2Certified:true,stationContinuity:'G2',capContinuity:'C0',solidGeometryCertified:true,continuousBound:true})
 const node=graph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const smooth=(built.report.construction![node.id] as any).profileSmoothness
 expect(smooth.totalExactWork).toBe(smooth.exactWork+smooth.station.exactWork)
 expect(smooth.totalExactWork).toBeLessThanOrEqual(smooth.maxWork)
})
it('includes closed-path closure and keeps sharp closed miter stations unpromoted',()=>{
 const body=createMiterBrepProfileBody([[circleNurbsCurve([0,0,0],[1,0,0],.5)]],[[0,0,0],[10,0,0],[10,10,0],[0,10,0]],[0,0,1],2,true)
 const r=inspectMiterStationSmoothness(body.model,[])
 expect(r).toMatchObject({extractionComplete:true,capEdges:[],unpairedEdges:[],stationG1Certified:false,stationG2Certified:false})
 expect(r.edgeIds).toHaveLength(16)
 expect(r.exactWork).toBeLessThanOrEqual(r.maxWork)
})
it('certifies reconstructed moving frames and hollow Solid while retaining explicit budget refusal',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileModelGraphText}=await import('../src/services/modelGraphText')
 const {buildOwnNurbs}=await import('../src/services/modelGraphNurbsKernel')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const source=readFileSync('examples/rush/miter-moving-frame-guide-affine-hollow-corrected.r','utf8')
  .replace('cap_correction_tolerance:','circle_correction_tolerance:1e-9mm,circle_correction_max_work:100000,cap_correction_tolerance:')
  .replace(/\.brep_tessellate\(\d+\)/,'.brep_smooth_miter_stations(wall_tolerance:1mm,quantum:0.0000000000004547473508864641mm,max_work:1000000,max_deviation:2mm).brep_tessellate(4)')
 const built=buildOwnNurbs(compileModelGraphText(source).document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({profileG2Certified:true,stationG1Certified:true,stationG2Certified:true,solidGeometryCertified:true,continuousBound:true})
 const node=compileModelGraphText(source).document.nodes.find(n=>n.op==='brep_smooth_miter_stations')!
 const report=(built.report.construction![node.id] as any)
 expect(report.profileSmoothness.totalExactWork).toBeLessThanOrEqual(report.profileSmoothness.maxWork)
 const {kind,...model}=built.report.definitions[node.id] as any
 expect(kind).toBe('brep')
 const {inspectMiterProfileSmoothness}=await import('../src/services/miterProfileSmoothness')
 const budget=report.profileSmoothness.totalExactWork-1
 const limited=inspectMiterProfileSmoothness(model,[model.faces.length-2,model.faces.length-1],budget)
 expect(limited.station.stationG2Certified).toBe(false)
 expect(limited.totalExactWork).toBeLessThanOrEqual(budget)
 expect(limited.station.g2.seams.some(s=>s.reason==='exact-jet-work-unresolved')).toBe(true)
 expect(inspectMiterStationSmoothness(model,[model.faces.length-2,model.faces.length-1])).toMatchObject({stationG1Certified:true,stationG2Certified:true})
 expect(()=>buildOwnNurbs(compileModelGraphText(source.replace('max_deviation:2mm','max_deviation:0.1mm')).document,{action:'build'})).toThrow()
})

it('refuses periodic and malformed pcurves before certifying station joins',()=>{
 for(const periodic of [true,false]){
  const model=straight(),face=model.faces[0]!
  const use=model.loops[face.outer]!.coedges.find(c=>c.pcurve.controlPoints[0]![1]===c.pcurve.controlPoints[1]![1])!
  if(periodic)use.pcurve.periodic=true
  else use.pcurve.knots=[0,1,0,1]
  expect(inspectMiterStationSmoothness(model,[model.faces.length-2,model.faces.length-1])).toMatchObject({extractionComplete:false,stationG1Certified:false,stationG2Certified:false})
 }
})
