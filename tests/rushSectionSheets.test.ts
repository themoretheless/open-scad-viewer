import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,before='',after=''){
 const text=readFileSync('examples/rush/'+name+'.r','utf8').replace(before,after)
 const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
 expect(scene.meshes).toHaveLength(1)
 return {area:scene.surfaceArea,report:JSON.parse(scene.measurements![0]!.report)}
}
it('constructs a B-rep face with an inner wire and eight original edges',async()=>{
 const r=await build('section-brep-sheets')
 expect(r.area).toBeCloseTo(12,4)
 expect(r.report).toMatchObject({faceRings:[[0,1]],edgeCount:8,shellsClosed:[false],supportErrorUpper:['0']})
})
it('certifies bounded support for an oblique nonbinary plane',async()=>{
 const r=await build('bounded-section-brep-sheets')
 const [n,d]=r.report.supportErrorUpper[0].split('/').map(BigInt)
 expect(n>0n && n*100000000000000n<=d).toBe(true)
 await expect(build('bounded-section-brep-sheets','support_tolerance_mm:0.00000000000001','support_tolerance_mm:0')).rejects.toThrow()
})
