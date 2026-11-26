import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,before='',after=''){
  const text=readFileSync('examples/rush/'+name+'.r','utf8').replace(before,after)
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
it('certifies the same root on both sides of an original knot',async()=>{
  const r=await build('stitch-plane-strip')
  expect(r.cells.map((c:any)=>c.span)).toEqual([[1,1],[1,2]])
  expect(r.junctions).toEqual([{transverse:7,solved:['3','3']}])
  await expect(build('stitch-plane-strip','transverse_spans:[1,2]','transverse_spans:[2,1]')).rejects.toThrow()
})
it('retains a single shared spatial and UV vertex in the original knot clocks',async()=>{
  expect(await build('trace-plane-strip')).toMatchObject({status:'complete',cellVertices:[[0,1],[1,2]],cellErrorUpper:['0','0'],uv:[['3','5'],['3','7'],['3','10']],xyz:[['1/2','0','0'],['1/2','2','0'],['1/2','5','0']]})
})
