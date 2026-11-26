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

it('classifies a whole inner contour and measures the winding of each contour about the other',async()=>{
  const result=await join('contour-placement')
  expect(result.placements).toEqual([{filled:true,winding:1}])
  expect(result.distance[0]).toBeLessThanOrEqual(1.5)
  expect(result.distance[1]).toBeGreaterThanOrEqual(1.5)
  expect((await join('contour-nesting')).pairs[0].windings).toEqual([0,1])
  await expect(join('contour-placement',s=>s.replace('first_count:1','first_count:2'))).rejects.toThrow()
})
it('finds the immediate parent and identifies exactly the hole that needs reversal',async()=>{
  const hierarchy=await join('contour-hierarchy')
  expect(hierarchy.parents).toEqual([null,0])
  expect(hierarchy.depths).toEqual([0,1])
  expect((await join('contour-fill-orientations')).reverse).toEqual([false,true])
})
