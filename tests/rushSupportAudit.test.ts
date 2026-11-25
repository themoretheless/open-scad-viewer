import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const program=readFileSync('examples/rush/affine-surface-trace.r','utf8')
async function inspect(text:string){
  const s=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(s.meshes).toHaveLength(1)
  return s.measurements!.map(r=>JSON.parse(r.report))
}
it('proves affine ranks on the original controls and verifies the entire lifted UV trace',async()=>{
  const rows=await inspect(program)
  for(const dimension of [1,2]){
    const r=rows.find(r=>r.dimension===dimension)
    expect(r.complete).toBe(true)
    expect(r.witnessIndices).toHaveLength(dimension+1)
  }
  expect(rows.some(r=>r.status==='WithinTolerance')).toBe(true)
})
it('reports a witnessed mismatch when the UV trace is shifted onto the opposite boundary',async()=>{
  const rows=await inspect(program.replace('[[0,0],[1,0]]','[[0,1],[1,1]]'))
  const mismatch=rows.find(r=>r.status==='Mismatch')
  expect(mismatch.witnessDistanceMm[0]).toBeGreaterThan(0.05)
  expect(mismatch.witnessDistanceMm[0]).toBeLessThanOrEqual(1)
  expect(mismatch.witnessDistanceMm[1]).toBeGreaterThanOrEqual(1)
})
