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
it('maps a rational patch and its complete outer and inner contours together',async()=>{
  for (const [name, points] of [
    ['affine', [[20,-9],[20,3]]],
    ['transpose', [[3,5],[9,5]]],
    ['remap', [[24,-5],[12,-5]]],
  ] as const) {
    const flat = readFileSync('examples/rush/'+name+'-trim-surface.r','utf8')
      .replace('15mm,1mm','15mm,0mm').replace('0,2mm','0,0mm').replace('15mm,9mm','15mm,0mm')
      .replace('weights:[[1,2],[4,1]]','weights:[[1,1],[1,1]]')
    const scene = await buildTextNurbsScene(compileRushFrontend(flat).document)
    expect(scene.surfaceArea).toBeCloseTo(15*15-(6-5)*(9-3),4)
    const report = await build(name+'-trim-surface')
    expect(report.loopPieces).toEqual([4,4])
    expect(report.trimLoops[1][0]).toMatchObject({controlPoints:points,weights:[1,1],knots:[3,3,4,4]})
    expect(await build(name+'-trim-piece')).toMatchObject({selectedLoop:1,selectedPiece:0,domain:[3,4]})
  }
})
it('supports both axis orders and refuses a broken boundary',async()=>{
  const report=await build('remap-trim-surface','axis_order:1','axis_order:0')
  expect(report.trimLoops[1][0].controlPoints).toEqual([[20,-9],[20,3]])
  await expect(build('affine-trim-surface','hole0,hole1,hole2,hole3','hole0,hole2,hole1,hole3')).rejects.toThrow(/OriginalLoopClosure/)
})
