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

it('changes periodic storage, rotates a weighted curve and opens its original interval',async()=>{
  expect(await join('store-periodic-curve')).toMatchObject({periodic:true,domain:[0,4]})
  expect(await join('rotate-periodic-knot')).toMatchObject({periodic:true,domain:[3,8]})
  expect(await join('insert-periodic-seam')).toMatchObject({periodic:true,domain:[0.5,4.5],controlCount:7})
  expect(await join('open-periodic-curve')).toMatchObject({periodic:false,knots:[0,0,0,1,2,3,4,4,4]})
})
it('trims and partitions a wrapped curve in the unwrapped parameter domain',async()=>{
  expect(await join('trim-periodic-curve')).toMatchObject({periodic:false,knots:[3.5,3.5,4,4.5,4.5]})
  expect(await join('trim-periodic-turns')).toMatchObject({periodic:false,domain:[3.5,13.25]})
  expect(await join('partition-periodic-curve')).toMatchObject({periodic:false,pieceCount:3,domain:[4.5,6]})
  await expect(join('partition-periodic-curve',s=>s.replace('part:1','part:3'))).rejects.toThrow()
})
