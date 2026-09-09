import {expect, it, vi} from 'vitest'
import {evaluateModelGraphGeometry, requireModelGraphChecks} from '../src/services/modelGraphChecks'
import {parseOpenSCAD} from '../src/services/openscadParser'

it('measures target snapshots independently, reuses their build and retains assertion order', async () => {
  const shown = await parseOpenSCAD('cube([10,3,4]);')
  const build = vi.fn(async (source:string) => (await parseOpenSCAD(source)).meshes)
  const source = 'cube([2,3,4]);'
  const report = await evaluateModelGraphGeometry([
    {id:'before',target:'original',check:'width',expected:2,tolerance:0,message:'Original',source},
    {id:'after',target:'scaled',check:'width',expected:10,tolerance:0,message:'Scaled'},
    {id:'height',target:'original',check:'height',expected:4,tolerance:0,message:'Height',source},
  ], shown.meshes, build)
  expect(report.map(check => [check.id,check.status,check.actual])).toEqual([
    ['before','passed',2], ['after','passed',10], ['height','passed',4],
  ])
  expect(build).toHaveBeenCalledTimes(1)
  expect(report.every(check => !('source' in check))).toBe(true)
})

it('blocks exports for a failed hidden target and unavailable target builds', async () => {
  const shown = await parseOpenSCAD('cube([10,3,4]);')
  const report = await evaluateModelGraphGeometry([
    {id:'hidden',target:'hidden',check:'width',expected:10,tolerance:0,message:'Hidden',source:'cube([2,3,4]);'},
    {id:'missing',target:'missing',check:'width',expected:10,tolerance:0,message:'Missing',source:'unbuildable'},
  ], shown.meshes, async source => {
    if (source === 'unbuildable') throw new Error('Target unavailable')
    return (await parseOpenSCAD(source)).meshes
  })
  expect(report).toMatchObject([{id:'hidden',status:'failed',actual:2},{id:'missing',status:'unknown',reason:'Target unavailable'}])
  expect(() => requireModelGraphChecks(report)).toThrow('Geometry assertions')
})
