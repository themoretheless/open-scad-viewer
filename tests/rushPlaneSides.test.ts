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
it('certifies a positive surface after exact subdivision of a mixed control net',async()=>{
  const original=await build('audit-plane-side')
  expect(original.cells[0]).toMatchObject({side:'Unresolved',residual:['-1/8','1']})
  const refined=await build('refine-plane-side')
  expect(refined.cells.map((r:any)=>r.cell.side)).toEqual(['Positive','Positive'])
  expect(refined.cells.map((r:any)=>r.cell.domain[0])).toEqual([[0,0.5],[0.5,1]])
})
it('certifies a unique transverse branch and rejects a zero plane normal',async()=>{
  const r=await build('audit-plane-graphs')
  expect(r.cells[0].status).toMatchObject({kind:'Graphs',axes:[{solveAxis:'U',position:'Interior',derivative:['2','2']},null]})
  await expect(build('audit-plane-graphs','plane_normal:[0,0,1]','plane_normal:[0,0,0]')).rejects.toThrow()
})
