import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,before='',after=''){
 const text=readFileSync('examples/rush/'+name+'.r','utf8').replace(before,after)
 const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
 expect(scene.meshes).toHaveLength(1)
 const vertices=scene.meshes[0]!.vertices
 expect(vertices.length).toBeGreaterThan(0)
 for(let i=2;i<vertices.length;i+=6) expect(vertices[i]).toBeCloseTo(2,6)
 return {area:scene.surfaceArea,report:JSON.parse(scene.measurements![0]!.report)}
}
it('renders the cap in its source plane and subtracts the trimmed hole',async()=>{
 expect((await build('cap-section-wire')).area).toBeCloseTo(16,4)
 const withHole=await build('cap-section-holes')
 expect(withHole.area).toBeCloseTo(12,4)
 expect(withHole.report).toMatchObject({supportErrorUpper:'0',boundaryGapUpper:'0'})
 await expect(build('cap-section-wire','plane_offset_mm:2','plane_offset_mm:3')).rejects.toThrow()
})
it('selects the separate material island from the exact ring forest',async()=>{
 const r=await build('arrange-section-caps')
 expect(r.area).toBeCloseTo(1,4)
 expect(r.report).toMatchObject({parents:[null,0,1],regions:[[0,1],[2]],selectedRegion:1})
})
