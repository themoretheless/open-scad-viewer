import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function join(name:string, transform=(s:string)=>s){
  const source=transform(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}

it('matches affine tangent and acceleration on independent parameter intervals',async()=>{
  expect(await join('join-affine-c1')).toMatchObject({seam:1,domain:[0,3]})
  expect(await join('join-affine-c2')).toMatchObject({seam:1,domain:[0,2]})
})
it('matches projective and stationary C2 seams and detects an incompatible affine join',async()=>{
  expect(await join('join-projective-c2')).toMatchObject({seam:1,domain:[0,3]})
  expect(await join('join-stationary-c2')).toMatchObject({seam:1,domain:[0,2]})
  await expect(join('join-projective-c2',s=>s.replace('join_projective_c2','join_affine_c2'))).rejects.toThrow()
})
