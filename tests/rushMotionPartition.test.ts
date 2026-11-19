import { readFileSync } from 'node:fs'
import { expect,it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

const program=readFileSync('examples/rush/motion-partition.r','utf8')
async function evaluate(source:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  return scene.measurements!.map(row=>JSON.parse(row.report))
}
it('covers both sides of a speed crossing without dropping unresolved cells',async()=>{
  const reports=await evaluate(program)
  expect(reports.length).toBe(3)
  for(const [i,report] of reports.entries()){
    const cells=report.cells
    expect(cells[0].parameterInterval[0]).toBe(0)
    expect(cells.at(-1).parameterInterval[1]).toBe(1)
    const states=[]
    for(const [j,cell] of cells.entries()){
      if(j>0)expect(cell.parameterInterval[0]).toBe(cells[j-1].parameterInterval[1])
      const state=i===0?cell.status:i===1?cell.statuses[0]:cell.axisStatuses[0][0]
      states.push(state)
      if(state==='within') expect(cell.parameterInterval[1]).toBeLessThanOrEqual(0.5)
      if(state==='exceeds') expect(cell.parameterInterval[0]).toBeGreaterThanOrEqual(0.5)
      if(i===2)expect(cell.axisStatuses[1]).toEqual(['within','within','within'])
    }
    expect(new Set(states)).toEqual(new Set(['within','exceeds','undetermined']))
  }
})
it('retains uncertainty under a minimal subdivision budget',async()=>{
  const [report]=await evaluate(program.replace('max_cells:128','max_cells:1'))
  expect(report.cells).toHaveLength(1)
  expect(report.cells[0]).toMatchObject({status:'undetermined',parameterInterval:[0,1]})
})

it('measures full-domain signed and unsigned travel and checks an interval reverse allowance',async()=>{
  const source=readFileSync('examples/rush/full-directional-travel.r','utf8')
  const [total,signed,checked]=await evaluate(source)
  const assertBound=(pair:number[],expected:number)=>{
    expect(pair[0]).toBeLessThanOrEqual(expected)
    expect(pair[1]).toBeGreaterThanOrEqual(expected)
    expect(pair[1]!-pair[0]!).toBeLessThan(0.001)
  }
  expect(total.parameterInterval).toEqual([2,4])
  assertBound(total.totalBoundsMm,8)
  assertBound(signed.forwardBoundsMm,5)
  assertBound(signed.reverseBoundsMm,3)
  assertBound(signed.signedTravelBoundsMm,2)
  expect(checked.reverseStatus).toBe('exceeds')
  expect((await evaluate(source.replace('reverse_limit:2mm','reverse_limit:4mm')))[2].reverseStatus).toBe('within')
})
