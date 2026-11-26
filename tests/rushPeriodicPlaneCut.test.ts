import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const example=readFileSync('examples/rush/split-periodic-plane-strip.r','utf8')
async function build(text:string){
 const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
 expect(scene.meshes).toHaveLength(1)
 return {area:scene.surfaceArea, report:JSON.parse(scene.measurements![0]!.report)}
}
it('renders both complementary bands on the periodic surface',async()=>{
 const a=await build(example),b=await build(example.replace('positive_side:false','positive_side:true'))
 expect(a.area).toBeCloseTo(4*Math.SQRT2,4)
 expect(b.area).toBeCloseTo(4*Math.SQRT2,4)
 expect(a.report.trimLoops).toEqual([a.report.negative])
 expect(b.report.trimLoops).toEqual([b.report.positive])
 expect(a.report.uvPeriod).toBe('4')
 expect(a.report.pieces).toHaveLength(4)
})
it('preserves the closed spatial seam and rejects an incomplete period',async()=>{
 const {report}=await build(example)
 expect(report.pieces[3].spatial.controlPoints.at(-1)).toEqual(report.pieces[0].spatial.controlPoints[0])
 await expect(build(example.replace('transverse_spans:[1,4]','transverse_spans:[1,3]'))).rejects.toThrow()
})
