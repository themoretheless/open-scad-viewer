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
it('closes a full spatial period while keeping two distinct UV seam ends',async()=>{
  const r=await build('close-plane-strip')
  expect(r.xyz).toHaveLength(5)
  expect(r.xyz[0]).toEqual(r.xyz[4])
  expect([r.uv[0],r.uv[4]]).toEqual([['1/2','0'],['1/2','4']])
  await expect(build('close-plane-strip','transverse_spans:[1,4]','transverse_spans:[1,3]')).rejects.toThrow()
})
it('stores the closed cycle and constructs a surface from its last edge',async()=>{
  const r=await build('store-closed-plane-piece')
  expect(r).toMatchObject({selectedPiece:3,uvPeriod:'4'})
  expect(r.pieces).toHaveLength(4)
  for(let i=0;i<4;i++) expect(r.pieces[i].spatial.controlPoints.at(-1)).toEqual(r.pieces[(i+1)%4].spatial.controlPoints[0])
  expect(r.pieces[3].pcurve.controlPoints.at(-1)).toEqual([0.5,4])
})
