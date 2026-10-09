import {expect,it} from 'vitest'
import {streamProgressiveBrepProfileBody,createPeriodicBrepSectionLoft,createRationalBrepSectionLoft,createProgressiveBrepProfileBody,inspectNurbsBrep,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {circleNurbsCurve,circleNurbsArc,bezierNurbsCurve,type NurbsScaleLaw} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {exportDirectStepV5,importDirectStepV5} from '../src/services/cadNurbsStep'
const loops=(z:number,s=1)=>[[circleNurbsCurve([0,0,z],[0,0,1],3*s)],[reverseNurbsCurve(circleNurbsCurve([0,0,z],[0,0,1],s))]]
const law=(a:number,b:number):NurbsScaleLaw=>({degree:1,knots:[0,0,1,1],values:[a,b],weights:[1,1]})
it('retains rational hollow loft walls, cap holes and manifold incidence through WASM',()=>{
 const model=createRationalBrepSectionLoft([loops(0),loops(5,1.5),loops(10,2)])
 expect(inspectNurbsBrep(model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
 expect(model.faces).toHaveLength(18)
 expect(model.faces.filter(f=>f.holes.length===1)).toHaveLength(2)
 expect(model.faces[0]!.surface.weights.flat().some(w=>w!==1)).toBe(true)
 const mesh=tessellateNurbsBrep(model,8)
 expect(mesh.report.closed).toBe(true)
 expect(mesh.report.signedVolumeMm3).toBeGreaterThan(0)
 const invalid=loops(0);invalid[1]![0]=reverseNurbsCurve(invalid[1]![0]!)
 expect(()=>createRationalBrepSectionLoft([invalid,loops(10)])).toThrow(/orientation/)
})
it('caps accepted simultaneous scale/twist and refuses insufficient refinement',()=>{
 const opts={normal:[1,0,0] as [number,number,number],initialSections:3,maxSections:16,maxDeviation:.001}
 const path=bezierNurbsCurve([[0,0,0],[0,0,10]])
 const result=createProgressiveBrepProfileBody(loops(0),path,law(1,2),law(0,5),opts)
 expect(result.globalEmbeddingCertified).toBe(false)
 expect(result.approximation.report.accepted).toBe(true)
 expect(result.approximation.levels.length).toBeGreaterThan(1)
 expect(inspectNurbsBrep(result.model).boundaryEdgeCount).toBe(0)
 expect(()=>createProgressiveBrepProfileBody(loops(0),path,law(1,2),law(0,5),{...opts,maxSections:3})).toThrow(/budget/)
})

it('caps rational hollow sections on a bent NURBS path',()=>{
 const boundaries=[[circleNurbsCurve([10,0,0],[0,1,0],.3)],[reverseNurbsCurve(circleNurbsCurve([10,0,0],[0,1,0],.1))]]
 const path=circleNurbsArc([0,0,0],[0,0,1],10,0,45)
 const body=createProgressiveBrepProfileBody(boundaries,path,law(1,1),law(0,0),{normal:[0,0,1],maxDeviation:.02})
 expect(body.approximation.report.accepted).toBe(true)
 expect(inspectNurbsBrep(body.model).boundaryEdgeCount).toBe(0)
 expect(body.model.faces.filter(f=>f.holes.length)).toHaveLength(2)
 expect(tessellateNurbsBrep(body.model,8).report.closed).toBe(true)
})

it('orients reverse section travel outward rather than returning an inverted body',()=>{
 const model=createRationalBrepSectionLoft([loops(10),loops(0)])
 const mesh=tessellateNurbsBrep(model,8)
 expect(mesh.report.closed).toBe(true)
 expect(mesh.report.signedVolumeMm3).toBeGreaterThan(0)
})

it('roundtrips capped rational walls and holes through AP242 WASM transport',()=>{
 const source=createRationalBrepSectionLoft([loops(0),loops(10,2)])
 const exported=exportDirectStepV5(source)
 expect(exported.text).toContain('RATIONAL_B_SPLINE_SURFACE')
 const result=importDirectStepV5(exported.text).model
 expect(inspectNurbsBrep(result).boundaryEdgeCount).toBe(0)
 expect(result.faces.filter(f=>f.holes.length)).toHaveLength(2)
 expect(result.faces).toHaveLength(source.faces.length)
 expect(tessellateNurbsBrep(result,8).report.closed).toBe(true)
})

it('builds retained hollow body through Rush and the real editor entrypoint',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {parseOpenSCAD}=await import('../src/services/openscadParser')
 const source=readFileSync('examples/rush/progressive-hollow-body.r','utf8')
 const graph=compileRushFrontend(source)
 const node=graph.document.nodes.find(n=>n.op==='brep_progressive_sweep')!
 expect(node).toMatchObject({loops:expect.any(Array),inputs:expect.any(Array)})
 const built=buildOwnNurbs(graph.document,{action:'build'})
 expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,continuousBound:true,globalEmbeddingCertified:false})
 const scene=await parseOpenSCAD(source)
 expect(scene.meshes[0]!.indices.length).toBeGreaterThan(0)
 expect(scene.meshes[0]!.faceIdsAuthoritative).toBe(true)
 expect(scene.meshes[0]!.nativeGeometry?.kind).toBe('brep')
 const {sceneMeshesToSolidDocument}=await import('../src/services/solidBridge')
 const solid=sceneMeshesToSolidDocument(scene.meshes)
 expect(solid.bodies[0]!.brep!.faces.filter(f=>f.holes.length)).toHaveLength(2)
 expect(inspectNurbsBrep(solid.bodies[0]!.brep!).boundaryEdgeCount).toBe(0)
 expect(()=>compileRushFrontend(source.replace('5deg','5mm'))).toThrow()
})

it('retains anisotropic hollow profiles and moving centers through Rush and Solid',async()=>{
 const {readFileSync}=await import('node:fs')
 const {parseOpenSCAD}=await import('../src/services/openscadParser')
 const {sceneMeshesToSolidDocument}=await import('../src/services/solidBridge')
 const scene=await parseOpenSCAD(readFileSync('examples/rush/affine-hollow-body.r','utf8'))
 const body=sceneMeshesToSolidDocument(scene.meshes).bodies[0]!
 expect(body.brep!.faces.filter(f=>f.holes.length)).toHaveLength(2)
 expect(inspectNurbsBrep(body.brep!).boundaryEdgeCount).toBe(0)
 expect(tessellateNurbsBrep(body.brep!,4).report.closed).toBe(true)
})

it('retains a closed hollow sweep as periodic outer and inner shells without caps',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {parseOpenSCAD}=await import('../src/services/openscadParser')
 const {sceneMeshesToSolidDocument}=await import('../src/services/solidBridge')
 const source=readFileSync('examples/rush/closed-progressive-hollow-body.r','utf8')
 const graph=compileRushFrontend(source),node=graph.document.nodes.find(n=>n.op==='brep_progressive_sweep')!
 const built=buildOwnNurbs(graph.document,{action:'build'})
 expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,closedPath:true,seamContinuity:'C0',globalEmbeddingCertified:false})
 const scene=await parseOpenSCAD(source),body=sceneMeshesToSolidDocument(scene.meshes).bodies[0]!.brep!
 expect(body.shells).toHaveLength(2)
 expect(body.bodies[0]!.innerShells).toEqual([1])
 expect(body.faces.every(f=>f.holes.length===0)).toBe(true)
 expect(inspectNurbsBrep(body).boundaryEdgeCount).toBe(0)
 const mesh=tessellateNurbsBrep(body,4)
 expect(mesh.report.closed).toBe(true)
 expect(mesh.report.signedVolumeMm3).toBeGreaterThan(0)
 const step=exportDirectStepV5(body)
 expect(step.text).toContain('BREP_WITH_VOIDS')
 const restored=importDirectStepV5(step.text).model
 expect(restored.bodies[0]!.innerShells).toHaveLength(1)
 expect(inspectNurbsBrep(restored).boundaryEdgeCount).toBe(0)
})

it('sews authored repeated sections through the direct periodic WASM API',()=>{
 const base=[[circleNurbsCurve([5,0,0],[0,1,0],.5)],[reverseNurbsCurve(circleNurbsCurve([5,0,0],[0,1,0],.2))]]
 const rotated=(quarter:number)=>base.map(loop=>loop.map(c=>({...c,controlPoints:c.controlPoints.map(p=>{
  const [x,y,z]=p;return (quarter===1?[-y!,x!,z!]:quarter===2?[-x!,-y!,z!]:[y!,-x!,z!]) as [number,number,number]
 })})))
 const sections=[base,rotated(1),rotated(2),rotated(3),base]
 const body=createPeriodicBrepSectionLoft(sections)
 expect(body.vertices).toHaveLength(32)
 expect(body.faces).toHaveLength(32)
 expect(body.bodies[0]!.innerShells).toEqual([1])
 expect(inspectNurbsBrep(body).boundaryEdgeCount).toBe(0)
 expect(tessellateNurbsBrep(body,4).report.closed).toBe(true)
 expect(()=>createPeriodicBrepSectionLoft([base,rotated(1),rotated(2),rotated(3)])).toThrow(/identical.*seam/)
})

it('closes full-turn twist with anisotropic and offset laws and rejects a mismatching vector seam',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const extra='axis_scale: {degree: 2,knots: [0,0,0,1,1,1],values: [[1,1,1],[1.2,0.8,1],[1,1,1]],weights: [1,1,1]},center_law: {degree: 2,knots: [0,0,0,1,1,1],values: [[0,0,0],[0.1mm,0,0],[0,0,0]],weights: [1,1,1]},'
 const source=readFileSync('examples/rush/closed-progressive-hollow-body.r','utf8').replace('values: [0deg,0deg]','values: [0deg,360deg]').replace('normal: [0,0,1],initial_sections',extra+'normal: [0,0,1],initial_sections').replace('.brep_tessellate(4)','.brep_tessellate(2)')
 const graph=compileRushFrontend(source),node=graph.document.nodes.find(n=>n.op==='brep_progressive_sweep')!
 expect(buildOwnNurbs(graph.document,{action:'build'}).report.construction?.[node.id]).toMatchObject({accepted:true,closedPath:true})
 expect(()=>buildOwnNurbs(compileRushFrontend(source.replace('[1.2,0.8,1],[1,1,1]','[1.2,0.8,1],[2,1,1]')).document,{action:'build'})).toThrow(/endpoints must agree/)
})


it('retains authored-frame hollow body caps, rational walls and STEP topology',()=>{
 const vector=(values:[number,number,number][])=>({degree:1,knots:[0,0,1,1],values,weights:[1,1]})
 const result=createProgressiveBrepProfileBody(loops(0),bezierNurbsCurve([[0,0,0],[0,0,10]]),law(1,2),law(0,5),{
  orientation:'authored',frameAxis:vector([[0,0,1],[0,1,1]]),frameNormal:vector([[1,0,0],[1,0,0]]),
  normal:[1,0,0],initialSections:3,maxSections:129,maxDeviation:.005,
 })
 expect(result.approximation.report).toMatchObject({accepted:true,continuousBound:true})
 expect(inspectNurbsBrep(result.model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
 expect(result.model.faces.filter(f=>f.holes.length===1)).toHaveLength(2)
 const mesh=tessellateNurbsBrep(result.model,4)
 expect(mesh.report.closed).toBe(true)
 const roundtrip=importDirectStepV5(exportDirectStepV5(result.model).text)
 expect(inspectNurbsBrep(roundtrip.model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
 expect(result.globalEmbeddingCertified).toBe(false)
})


it('retains shared contact fit across public hollow-body B-rep and STEP',()=>{
 const result=createProgressiveBrepProfileBody(loops(0),bezierNurbsCurve([[0,0,0],[0,0,10]]),law(1,1),law(0,0),{
  normal:[1,0,0],orientationGuide:bezierNurbsCurve([[3,0,0],[6,0,10]]),
  contactAnchor:{profileIndex:0,parameter:0},initialSections:3,maxSections:129,maxDeviation:.001,
 })
 expect(result.approximation.report.accepted).toBe(true)
 expect(Math.max(...result.model.vertices.map(v=>v.point[0]))).toBeCloseTo(6,9)
 expect(Math.min(...result.model.vertices.map(v=>v.point[0]))).toBeCloseTo(-6,9)
 expect(inspectNurbsBrep(result.model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
 expect(result.model.faces.filter(f=>f.holes.length===1)).toHaveLength(2)
 expect(tessellateNurbsBrep(result.model,4).report.closed).toBe(true)
 const restored=importDirectStepV5(exportDirectStepV5(result.model).text).model
 expect(inspectNurbsBrep(restored)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
 expect(result.globalEmbeddingCertified).toBe(false)
 expect(()=>createProgressiveBrepProfileBody(loops(0),bezierNurbsCurve([[0,0,0],[0,0,10]]),law(1,1),law(0,5),{
  normal:[1,0,0],orientationGuide:bezierNurbsCurve([[3,0,0],[6,0,10]]),contactAnchor:{parameter:0},maxDeviation:.001,
 })).toThrow(/zero twist/)
})


it('streams rational body walls on the same face budget before returning an audited hollow body',async()=>{
 const options={normal:[1,0,0] as [number,number,number],initialSections:3,maxSections:257,maxDeviation:.001}
 const boundaries=loops(0),path=bezierNurbsCurve([[0,0,0],[0,0,10]])
 const expected=createProgressiveBrepProfileBody(boundaries,path,law(1,2),law(0,5),options)
 const stream=streamProgressiveBrepProfileBody(boundaries,path,law(1,2),law(0,5),options)
 const reports=[]
 for(;;){
  const level=await stream.next()
  if(level.done){expect(level.value).toEqual(expected);break}
  expect(level.value.preview).toBe(true)
  expect('model' in level.value).toBe(false)
  reports.push(level.value.report)
 }
 expect(reports).toEqual(expected.approximation.levels)
})

it('cancels body streaming before topology construction and refuses exhausted wall refinement',async()=>{
 const options={normal:[1,0,0] as [number,number,number],initialSections:3,maxSections:3,maxDeviation:1e-10}
 const path=bezierNurbsCurve([[0,0,0],[0,0,10]]),boundaries=loops(0)
 const stream=streamProgressiveBrepProfileBody(boundaries,path,law(1,2),law(0,90),options)
 expect((await stream.next()).value).toMatchObject({preview:true,report:{accepted:false}})
 await expect(stream.next()).rejects.toThrow(/budget/)
 const controller=new AbortController()
 const cancelled=streamProgressiveBrepProfileBody(boundaries,path,law(1,1),law(0,0),{...options,maxDeviation:.001},{signal:controller.signal})
 expect((await cancelled.next()).value).toMatchObject({preview:true,report:{accepted:true}})
 controller.abort(new Error('cancel body'))
 await expect(cancelled.next()).rejects.toThrow('cancel body')
})


it('clamps body stream levels to cap-reserved face limits and retains cap refusal after wall acceptance',async()=>{
 const path=bezierNurbsCurve([[0,0,0],[0,0,10]])
 const stream=streamProgressiveBrepProfileBody(loops(0),path,law(1,2),law(0,90),
  {normal:[1,0,0],initialSections:3,maxSections:1025,maxDeviation:1e-12})
 const counts=[]
 await expect((async()=>{
  for(;;){const level=await stream.next();if(level.done)break;counts.push(level.value.report.sections)}
 })()).rejects.toThrow(/budget/)
 // Eight exact Bezier boundary spans, 1024 faces, and two cap faces.
 expect(counts).toEqual([3,5,9,17,33,65,128])
 const wrongHole=[[circleNurbsCurve([0,0,0],[0,0,1],3)],[circleNurbsCurve([0,0,0],[0,0,1],1)]]
 const refused=streamProgressiveBrepProfileBody(wrongHole,path,law(1,1),law(0,0),
  {normal:[1,0,0],initialSections:3,maxSections:3,maxDeviation:.001})
 expect((await refused.next()).value).toMatchObject({preview:true,report:{accepted:true}})
 await expect(refused.next()).rejects.toThrow(/orientation/)
})


it.each(['progressive-hollow-body','authored-progressive-hollow-body','affine-hollow-body','contact-progressive-hollow-body','closed-progressive-hollow-body'])('streams %s while preserving native body definitions and cap refusal',async(name)=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs,buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
  const document=compileRushFrontend(readFileSync(`examples/rush/${name}.r`,'utf8')).document
  const node=document.nodes.find(n=>n.op==='brep_progressive_sweep')!
  const expected=buildOwnNurbs(document,{action:'build'}),reports:unknown[]=[]
  const result=await buildOwnNurbsAsync(document,{action:'build'},{onSweepPreview:(id,preview)=>{
   expect(id).toBe(node.id)
   expect(preview.preview).toBe(true)
   reports.push(preview.report)
  }})
  expect(result,name).toEqual(expected)
  expect(reports,name).toEqual((expected.report.construction![node.id] as {levels:unknown[]}).levels)
})


it('cancels Rush body parsing on a wall preview and recovers on the same parser queue',async()=>{
 const {readFileSync}=await import('node:fs')
 const {parseOpenSCAD,AbortedError}=await import('../src/services/openscadParser')
 const source=readFileSync('examples/rush/contact-progressive-hollow-body.r','utf8')
 let cancelled=false,levels=0
 await expect(parseOpenSCAD(source,{shouldAbort:()=>cancelled,onSweepPreview:(_id,preview)=>{
  expect(preview.preview).toBe(true)
  levels++
  cancelled=true
 }})).rejects.toBeInstanceOf(AbortedError)
 expect(levels).toBe(1)
 const recovered=await parseOpenSCAD(source)
 expect(recovered.meshes[0]!.nativeGeometry?.kind).toBe('brep')
})


it('accepts corrected Frenet for a straight rational body while strict Frenet still refuses',()=>{
 const path=bezierNurbsCurve([[0,0,0],[0,0,10]])
 const options={normal:[1,0,0] as [number,number,number],initialSections:3,maxSections:3,maxDeviation:.001}
 const result=createProgressiveBrepProfileBody(loops(0),path,law(1,1),law(0,0),{...options,orientation:'corrected_frenet'})
 expect(result.approximation.report.accepted).toBe(true)
 expect(inspectNurbsBrep(result.model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
 expect(()=>createProgressiveBrepProfileBody(loops(0),path,law(1,1),law(0,0),{...options,orientation:'frenet'})).toThrow(/normal/)
})
