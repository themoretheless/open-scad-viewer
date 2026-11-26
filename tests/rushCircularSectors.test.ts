import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function run(name:string,transform=(s:string)=>s) {
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(readFileSync('examples/rush/'+name+'.r','utf8'))).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.at(-1)!.report)
}
it('rotates a seed and identifies the shortest repeated mixed-degree sector',async()=>{
  expect((await run('repeat-profile-segment')).rotationErrorMm).toBe(0)
  for(const name of ['compress-rotated-segments','compress-profile-sectors','discover-profile-sectors']){
    const r=await run(name)
    expect(r.sectorCount).toBe(4)
    expect(r.sourceErrorMm).toBeLessThanOrEqual(1e-6)
    if(name!=='compress-rotated-segments') expect(r.piecesPerSector).toBe(2)
  }
},180000)
it('restores a locally different control point exactly with both predictors',async()=>{
  for(const name of ['restore-lossless-sector','restore-bidirectional-sector']){
    const r=await run(name)
    expect(r.controlPoints).toEqual([[-2,-1,0],[-0.96875,-2,0],[0,-2,0]])
    expect(r.weights).toEqual([1,1,1])
    expect(r.residualBytes).toBeGreaterThan(0)
  }
},180000)
