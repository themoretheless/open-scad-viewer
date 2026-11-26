import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const example=readFileSync('examples/rush/split-cell-by-plane.r','utf8')
async function build(text:string){
 const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
 expect(scene.meshes).toHaveLength(1)
 return {area:scene.surfaceArea, report:JSON.parse(scene.measurements![0]!.report)}
}
it('renders complementary trimmed faces with the correct area and common section',async()=>{
 const a=await build(example), b=await build(example.replace('positive_side:false','positive_side:true'))
 expect(a.area).toBeCloseTo(2,5)
 expect(b.area).toBeCloseTo(6,5)
 expect(a.report.trimLoops).toEqual([a.report.negative])
 expect(b.report.trimLoops).toEqual([b.report.positive])
 expect(a.report.sectionCurve.controlPoints).toEqual([[1,0,0],[1,2,0]])
 expect(a.report.gapTolerance).toBe('0')
})
it('reverses material sides when the plane equation is negated',async()=>{
 const r=await build(example.replace('plane_normal:[1,0,0],plane_offset_mm:1','plane_normal:[-1,0,0],plane_offset_mm:-1'))
 expect(r.area).toBeCloseTo(6,5)
})
