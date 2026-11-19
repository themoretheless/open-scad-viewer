import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const source=readFileSync('examples/rush/principal-axis-probes.r','utf8')
async function measure(program:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(program).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('certifies the separated principal axis and bounds distance and angular deviation',async()=>{
  const p=await measure(source)
  expect(p.principalAxes.slice(1)).toEqual([null,null])
  const axis=p.principalAxes[0]
  expect(Math.abs(axis.direction[2])).toBeGreaterThan(0.9)
  expect(axis.sinAngleUpper).toBeLessThan(0.5)
  for(const [bounds,target] of [[axis.distanceBoundsMm,0.84375],[axis.angleSineBounds,Math.SQRT1_2]] as const){
    expect(bounds[0]).toBeLessThanOrEqual(target)
    expect(bounds[1]).toBeGreaterThanOrEqual(target)
  }
},120000)
it('refuses invalid probes and withholds unproved directions under limited work',async()=>{
  expect((await measure(source.replace('max_surface_cells:20000','max_surface_cells:1'))).principalAxes).toBeNull()
  await expect(measure(source.replace('probe_direction:[1,0,1]','probe_direction:[0,0,0]'))).rejects.toThrow()
  await expect(measure(source.replace('principal_axes:true','principal_axes:false'))).rejects.toThrow()
},120000)
