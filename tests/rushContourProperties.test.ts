import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function join(name:string, transform=(s:string)=>s){
  const source=transform(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.at(-1)!.report)
}

function contains(interval:number[],value:number){
  expect(interval[0]).toBeLessThanOrEqual(value)
  expect(interval[1]).toBeGreaterThanOrEqual(value)
}
it('preserves exact hole geometry and distinguishes even-odd and nonzero fill',async()=>{
  expect((await join('orient-fill-contour')).reverse).toEqual([false,true])
  const oriented=await join('orient-fill-contour',s=>s.replace('show result.surface_extrude','check = result.curve_winding_at_point(point:[0,0],parameter_tolerance:0.01,max_cells:10000)\nshow check.surface_extrude'))
  expect(oriented.winding).toBe(-1)
  contains((await join('contour-fill-area')).area,15)
  contains((await join('contour-nonzero-area')).area,16)
  contains((await join('contour-fill-perimeter')).perimeter,20)
  expect((await join('contour-fill-topology')).components).toEqual([{outer:0,holes:[1]}])
  contains((await join('contour-boundary-signed-distance')).signedDistance,0.5)
  await expect(join('orient-fill-contour',s=>s.replace('piece:1','piece:2'))).rejects.toThrow()
})
it('bounds centroid and second moments and reports nonunique axes of a square ring',async()=>{
  const centroid=(await join('contour-fill-centroid')).centroid
  for(const axis of centroid) contains(axis,0)
  const central=(await join('contour-central-moments')).centralMoments
  contains(central[0],255/12)
  contains(central[1],0)
  contains(central[2],255/12)
  for(const moment of (await join('contour-principal-moments')).moments) contains(moment,255/12)
  expect((await join('contour-principal-axes')).projectors).toBeNull()
})
