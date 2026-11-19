import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const source=readFileSync('examples/rush/rational-trim-range-length.r','utf8')
async function measure(program:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(program).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('retains exact reversed rational endpoints through Rush without converting them to floating point',async()=>{
  const scale=10n**60n
  const ratio=(4n*scale)+'/'+(5n*scale)
  const program=source.replace('4/5',ratio)
  const node=compileRushFrontend(program).document.nodes.find(n=>n.op==='surface_trim_range_length')
  expect(node).toMatchObject({range_start:ratio,range_end:'1/3'})
  for(const text of [source,program]){
    const p=await measure(text)
    expect(p.length.withinTolerance).toBe(true)
    expect(p.length.boundsMm[0]).toBeLessThanOrEqual(7/15)
    expect(p.length.boundsMm[1]).toBeGreaterThanOrEqual(7/15)
  }
})
it('distinguishes an exhausted budget from an invalid denominator',async()=>{
  const p=await measure(source.replace('max_cells:100','max_cells:3'))
  expect(p).toMatchObject({length:null,minimumCells:4,refusalStage:'measure'})
  await expect(measure(source.replace('1/3','1/0'))).rejects.toThrow()
})
