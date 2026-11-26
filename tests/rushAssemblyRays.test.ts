import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function run(name:string, transform=(source:string)=>source) {
  const source=transform(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  return JSON.parse(scene.measurements!.at(-1)!.report)
}
function contains(bounds:number[],value:number) {
  expect(bounds[0]).toBeLessThanOrEqual(value)
  expect(bounds[1]).toBeGreaterThanOrEqual(value)
  expect(bounds[1]!-bounds[0]!).toBeLessThan(0.001)
}
it('preserves placed face hits and material boundaries through native STEP ray queries',async()=>{
  const hits=(await run('step-ray-crossings')).hits
  expect(hits).toHaveLength(2)
  expect(hits.map((h:{crossings:unknown[]})=>h.crossings.length)).toEqual([2,2])
  for(const name of ['step-first-ray-crossing','cull-far-step-ray-instances']){
    const first=(await run(name)).first
    expect(first.occurrence).toBe(0)
    contains(first.crossing.parameter,1)
  }
  expect((await run('step-ray-candidates')).candidates).toHaveLength(2)
  const intervals=(await run('step-ray-intervals')).intervals
  expect(intervals).toHaveLength(1)
  contains(intervals[0][0],1)
  contains(intervals[0][1],3)
  contains((await run('step-ray-length')).lengthMm,2)
  expect((await run('qualify-step-instance')).proven).toBe(true)
  await expect(run('step-first-ray-crossing',s=>s.replace('max_cells:100000','max_cells:1'))).rejects.toThrow()
},180000)
it('distinguishes union from overlap, measures separated solids and returns exact section areas',async()=>{
  for(const [name,lo,hi] of [['step-ray-coverage',1,4],['step-ray-multiplicity',2,3]] as const){
    const {possible,guaranteed}=await run(name)
    contains([possible[0][0],guaranteed[0][0]],lo)
    contains([guaranteed[0][1],possible[0][1]],hi)
  }
  contains((await run('step-pair-clearance')).distanceMm,3)
  expect((await run('step-clearance-checks')).pairs[0].status).toBe('BelowMinimum')
  const {minimum}=await run('step-minimum-clearance')
  contains([minimum.lowerMm,minimum.upperMm],3)
  expect(minimum.pair).toEqual([0,1])
  const sections=(await run('step-polyhedral-sections')).sections
  expect(sections.map((s:{areaSquaredMm4:string})=>s.areaSquaredMm4)).toEqual(['16','16'])
  expect(sections[0].loops[0].pointsMm).toHaveLength(4)
},180000)
