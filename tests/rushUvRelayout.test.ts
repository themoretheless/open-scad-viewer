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
it('transfers a diagonal UV contour through the intersection of two knot lines',async()=>{
  expect(await build('transfer-uv-grid-piece')).toMatchObject({pieceCount:2,selectedPiece:1,domain:[4,6]})
  await expect(build('transfer-uv-grid-piece','max_work:1000000','max_work:1')).rejects.toThrow(/refused/i)
})
it('relocates every part of the trimmed surface and keeps both closed loops in order',async()=>{
  expect((await build('relayout-trim-surface')).loopPieces).toEqual([8,8])
  expect(await build('relayout-trim-piece')).toMatchObject({loopPieces:[8,8],selectedLoop:1,selectedPiece:1,domain:[1,2]})
  await expect(build('relayout-trim-surface','hole1,hole2,hole3','hole2,hole1,hole3')).rejects.toThrow(/OriginalLoopClosure/)
})
