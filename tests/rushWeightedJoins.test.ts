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

it('preserves the relative rational weights across a two-curve seam',async()=>{
  const r=await join('weighted-curve-join')
  expect(r.domain).toEqual([0,2])
  expect(r.weights.map((w:number)=>w/r.weights[0])).toEqual([1,2,10/3])
  await expect(join('weighted-curve-join',s=>s.replace('max_work:1000000','max_work:1'))).rejects.toThrow()
})

it('scales the entire three-curve chain in one exact operation',async()=>{
  const r=await join('weighted-chain-join')
  expect(r.domain).toEqual([0,3])
  expect(r.weights).toEqual([21/64,42/64,70/64,110/64])
  await expect(join('weighted-chain-join',s=>s.replace('inputs:[b,c]','inputs:[c,b]'))).rejects.toThrow()
})
