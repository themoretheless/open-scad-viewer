import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=readFileSync('examples/rush/transfer-uv-clock.r','utf8')
async function build(text:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
it('transfers both UV spans and retains their original seam parameter',async()=>{
  for(const piece of [0,1]){
    const report=await build(source.replace('piece:0','piece:'+piece))
    expect(report).toMatchObject({pieceCount:2,selectedPiece:piece,degree:2,domain:[3+piece,4+piece]})
  }
})
it('withholds every piece if a later span leaves the chart or work runs out',async()=>{
  await expect(build(source.replace('[7,2],[2,-3]','[7,2],[8,-3]'))).rejects.toThrow()
  await expect(build(source.replace('max_work:1000000','max_work:1'))).rejects.toThrow()
})
