import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {circleNurbsCurve,roundPolylineNurbsCurve,transitionPolylineNurbsCurve} from '../src/services/nurbsConstructors'
import {createProgressiveBrepProfileBody,inspectNurbsBrep} from '../src/services/geometry/brep'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'

it('preserves the requested circular radius through the packaged host',()=>{
  const curve=roundPolylineNurbsCurve([[0,0,0],[10,0,0],[10,10,0]],2)
  const start=curve.knots[3]!,end=curve.knots[5]!
  for(let i=0;i<=20;i++){
    const p=evaluateNurbsCurve(curve,start+(end-start)*i/20).point
    expect(Math.hypot(p[0]!-8,p[1]!-2)).toBeCloseTo(2,11)
  }
  expect(curve.controlPoints[0]).toEqual([0,0,0])
  expect(curve.controlPoints.at(-1)).toEqual([10,10,0])
  expect(()=>roundPolylineNurbsCurve([[0,0,0],[1,0,0],[1,1,0]],2)).toThrow(/overlap/)
  expect(()=>roundPolylineNurbsCurve([[0,0,0],[1,0,0],[0,0,0]],0.1)).toThrow()
})

it('lowers and builds a spatial round path sweep through Rush',()=>{
  const source=readFileSync('examples/rush/round-path-progressive-sweep.r','utf8')
  const compiled=compileRushFrontend(source)
  expect(compiled.execution_target).toBe('own-nurbs')
  expect(compiled.document.nodes.find(n=>n.op==='round_polyline_curve')).toMatchObject({radius:2})
  const built=buildOwnNurbs(compiled.document,{action:'build'})
  expect(built.mesh).toBeDefined()
  const node=compiled.document.nodes.find(n=>n.op==='progressive_sweep')!
  expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,continuousBound:false})
  expect(()=>compileRushFrontend(source.replace('radius: 2mm','radius: 2deg'))).toThrow()
})

it.each([['round',roundPolylineNurbsCurve],['transition',transitionPolylineNurbsCurve]] as const)('builds capped B-rep walls along %s corners within the body face budget',(_name,constructor)=>{
  const path=constructor([[0,0,0],[0,0,10],[10,0,10],[10,10,10]],2)
  const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
  const body=createProgressiveBrepProfileBody([[circleNurbsCurve([0,0,0],[0,0,1],0.25)]],path,law(1),law(0),{
    normal:[1,0,0],initialSections:5,maxSections:1025,maxDeviation:0.01,
  })
  expect(body.approximation.report.accepted).toBe(true)
  expect(body.model.faces.length).toBeLessThanOrEqual(1024)
  expect(inspectNurbsBrep(body.model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
  expect(body.globalEmbeddingCertified).toBe(false)
})

it('retains the independent quintic transition shape and validates dimensional Rush input',()=>{
  const curve=transitionPolylineNurbsCurve([[0,0,0],[10,0,0],[10,10,0]],2)
  const knots=[...new Set(curve.knots)]
  const p=evaluateNurbsCurve(curve,(knots[1]!+knots[2]!)/2).point
  expect(p[0]).toBeCloseTo(9.625,11)
  expect(p[1]).toBeCloseTo(0.375,11)
  const source=readFileSync('examples/rush/transition-path-progressive-sweep.r','utf8')
  const compiled=compileRushFrontend(source)
  expect(compiled.document.nodes.find(n=>n.op==='transition_polyline_curve')).toMatchObject({setback:2})
  const built=buildOwnNurbs(compiled.document,{action:'build'})
  const node=compiled.document.nodes.find(n=>n.op==='progressive_sweep')!
  expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,continuousBound:false})
  expect(()=>compileRushFrontend(source.replace('setback: 2mm','setback: 2deg'))).toThrow()
  expect(()=>transitionPolylineNurbsCurve([[0,0,0],[1,0,0],[1,1,0]],2)).toThrow(/overlap/)
})

it.each([['round',roundPolylineNurbsCurve],['transition',transitionPolylineNurbsCurve]] as const)('preserves the cyclic %s path seam and hollow periodic body through Rush',async(name,constructor)=>{
  const curve=constructor([[0,0,0],[10,0,0],[10,10,0],[0,10,0]],2,true)
  expect(curve.controlPoints[0]).toEqual(curve.controlPoints.at(-1))
  expect(curve.periodic).toBe(false)
  expect(()=>constructor([[0,0,0],[10,0,0],[10,10,0],[0,10,0]],5,true)).toThrow(/overlap/)
  let source=readFileSync('examples/rush/closed-corner-progressive-hollow-body.r','utf8')
  if(name==='transition')source=source.replace('round_polyline_curve','transition_polyline_curve').replace('radius: 2mm','setback: 2mm')
  expect(()=>buildOwnNurbs(compileRushFrontend(source).document,{action:'build'})).toThrow(/refinement.*budget/)
  source=source.replace('max_deviation: 0.02mm','max_deviation: 1mm')
  const compiled=compileRushFrontend(source)
  const node=compiled.document.nodes.find(n=>n.op==='brep_progressive_sweep')!
  const built=buildOwnNurbs(compiled.document,{action:'build'})
  expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,closedPath:true,seamContinuity:'C0',globalEmbeddingCertified:false})
  const {parseOpenSCAD}=await import('../src/services/openscadParser')
  const {sceneMeshesToSolidDocument}=await import('../src/services/solidBridge')
  const scene=await parseOpenSCAD(source)
  const body=sceneMeshesToSolidDocument(scene.meshes).bodies[0]!.brep!
  expect(body.shells).toHaveLength(2)
  expect(body.bodies[0]!.innerShells).toEqual([1])
  expect(body.faces.every(f=>f.holes.length===0)).toBe(true)
  expect(inspectNurbsBrep(body)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
})
