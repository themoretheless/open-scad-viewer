import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function run(name:string,transform=(s:string)=>s) {
  const source=transform(readFileSync('examples/rush/'+name+'.r','utf8'))
  try {
    const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
    return {volume:scene.volume,report:JSON.parse(scene.measurements!.at(-1)!.report)}
  } catch (error) {
    throw new Error('Rush example failed: '+name,{cause:error})
  }
}
it('returns editable regularized solids for material differences, clipping and layer selection',async()=>{
  for(const [name,volume] of [['step-cut-stock',4],['step-fuse-instances',12],['step-revision-delta',4],['step-split-instance',4],['step-stock-delta',4],['step-slice-layers',2],['step-clip-material',4],['step-free-space',68],['step-interference-regions',4],['step-interference-material',4],['step-group-interferences',4]] as const){
    expect((await run(name)).volume,name).toBeCloseTo(volume)
  }
  const middle=await run('step-slice-layers',s=>s.replace('piece:0','piece:1'))
  expect(middle.report.pieces).toBe(3)
  expect(middle.volume).toBeCloseTo(4)
  await expect(run('step-split-instance',s=>s.replace('piece:0','piece:2'))).rejects.toThrow()
},180000)
it('retains group membership, pair identities and shared definitions through revisions',async()=>{
  expect((await run('step-interference-groups')).report.groups).toEqual([[0,1]])
  const checks=(await run('step-group-clearances')).report.pairs
  expect(checks[0].pair).toEqual([0,1])
  expect(checks[0].status).toBe('BelowMinimum')
  const {minimum}=(await run('step-group-minimum-gap')).report
  expect(minimum.pair).toEqual([0,1])
  expect(minimum.lowerMm).toBeCloseTo(3)
  expect(minimum.upperMm).toBeCloseTo(3)
  const moved=(await run('move-step-group')).report
  expect(moved.occurrences.map((o:{placement:number[][]})=>o.placement[0]![3])).toEqual([7,8])
  const copied=(await run('copy-step-group')).report
  expect(copied.definitionCount).toBe(1)
  expect(copied.occurrenceCount).toBe(4)
  const variant=(await run('replace-step-variant')).report
  expect(variant.definitionCount).toBe(2)
  expect(variant.occurrences[0].definition).not.toBe(variant.occurrences[1].definition)
},180000)
