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

it('matches later joins in the accumulated parameter and removes redundant joins',async()=>{
  expect(await join('join-projective-chain')).toMatchObject({joinsVerified:2,domain:[0,7]})
  expect(await join('compact-projective-chain')).toMatchObject({knots:[0,0,0,3,3,3],seams:[
    {parameter:1,removed:2,remaining:0,refusal:null},{parameter:2,removed:2,remaining:0,refusal:null}]})
})
it('verifies closure of four cubic pieces and rejects an open chain',async()=>{
  expect(await join('close-projective-chain')).toMatchObject({joinsVerified:3,domain:[0,4],closureOrder:2})
  await expect(join('close-projective-chain',s=>s.replace('inputs:[b,c,d]','inputs:[b,c]'))).rejects.toThrow()
})
