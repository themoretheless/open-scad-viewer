import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {autoGuidedLoftNurbsCurves,matchNurbsLoftEnds,naturalLoftNurbsCurves,lineNurbsCurve,circleNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {createNaturalBrepSectionLoft,inspectNurbsBrep,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
const section=(z:number)=>lineNurbsCurve([0,0,z],[1,0,z])
it('automatically discovers sorted reversed guides and nonuniform stations through WASM',()=>{
 const guides=[lineNurbsCurve([.75,0,2],[.75,0,0]),lineNurbsCurve([.25,0,0],[.25,0,2])]
 const built=autoGuidedLoftNurbsCurves([section(0),section(1),section(2)],[0,.25,1],guides,1e-8)
 expect(built.guide_order).toEqual([1,0]);expect(built.reversed).toEqual([false,true])
 expect(Math.max(...built.section_error_upper,...built.guide_error_upper)).toBeLessThanOrEqual(1e-8)
 expect(evaluateNurbsSurface(built.surface,.5,.25).point[2]).toBeCloseTo(1,8)
})
it('matches both G2 end seams while retaining complete sections',()=>{
 const sections=[section(0),section(1),section(2)]
 const base=naturalLoftNurbsCurves(sections,[0,.5,1])
 const a=naturalLoftNurbsCurves([section(-2),section(0)],[0,1])
 const b=naturalLoftNurbsCurves([section(2),section(4)],[0,1])
 const built=matchNurbsLoftEnds(base,sections,[0,.5,1],1e-8,{reference:a,boundary:'vMax',order:2,scale:1},{reference:b,boundary:'vMin',order:2,scale:1})
 expect(built.seams).toHaveLength(2)
 expect(Math.max(...built.section_error_upper)).toBeLessThanOrEqual(1e-8)
 expect(evaluateNurbsSurface(built.surface,.5,.5).point).toEqual([.5,0,1])
})
it('builds a capped cubic rational solid with an interpolated middle section',()=>{
 const sections=[0,5,10].map((z,i)=>[[circleNurbsCurve([0,0,z],[0,0,1],i===1?6:3)]])
 const model=createNaturalBrepSectionLoft(sections,[0,.5,1])
 expect(inspectNurbsBrep(model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
 const mesh=tessellateNurbsBrep(model,8)
 expect(mesh.report.closed).toBe(true);expect(mesh.report.signedVolumeMm3).toBeGreaterThan(0)
})
it.each(['auto-guided-loft.r','natural-loft-solid.r','g2-loft-surface.r'])('builds the authored %s through Rush',file=>{
 const compiled=(()=>{try{return compileModelGraphText(readFileSync(`examples/rush/${file}`,'utf8'))}catch(error){if(error instanceof Error)error.message+=` (${(error as Error&{path?:string}).path})`;throw error}})()
 const built=buildOwnNurbs(compiled.document,{action:'build'})
 expect(built.report).toBeDefined()
})

it('preserves the new capped loft as authoritative BRep through the editor Solid bridge',async()=>{
 const {parseOpenSCAD}=await import('../src/services/openscadParser')
 const {sceneMeshesToSolidDocument,meshDataToEditablePolygon}=await import('../src/services/solidBridge')
 const {meshObjectStats}=await import('../src/services/meshEditing')
 const scene=await parseOpenSCAD(readFileSync('examples/rush/natural-loft-solid.r','utf8'))
 const solid=sceneMeshesToSolidDocument(scene.meshes)
 expect(inspectNurbsBrep(solid.bodies[0]!.brep!)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
 expect(meshObjectStats(meshDataToEditablePolygon(scene.meshes[0]!)!).closed).toBe(true)
})

it('keeps parameter correspondence strict even with a large spatial budget',()=>{
 const guide=lineNurbsCurve([.25,0,0],[.75,0,2])
 expect(()=>autoGuidedLoftNurbsCurves([section(0),section(2)],[0,1],[guide],10)).toThrow(/inconsistent U stations/)
 const source=readFileSync('examples/rush/auto-guided-loft.r','utf8')
 expect(()=>compileModelGraphText(source.replace('budget: 0.000001mm','budget: 0.000001mm,parameter_tolerance: 0.00000001mm'))).toThrow()
})
