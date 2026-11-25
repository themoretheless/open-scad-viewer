import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function inspect(kind: string, budget = 1000000) {
  const text = readFileSync('examples/rush/tensor-'+kind+'-compression.r','utf8')
  const scene = await buildTextNurbsScene(compileRushFrontend(text.replace('max_work:1000000','max_work:'+budget)).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
it('removes redundant knots and degrees from both patch directions', async () => {
  const full = await inspect('surface')
  expect(full.patches[0]).toMatchObject({completed:true,controlNet:[2,2],removedKnots:[2,2],reducedDegrees:[1,1]})
  const partial = await inspect('surface',1)
  expect(partial.patches[0]).toMatchObject({completed:false,controlNet:[5,5]})
})
it('returns the selected quadratic patch only after the complete batch succeeds', async () => {
  const full = await inspect('batch')
  expect(full.selectedPatch).toBe(1)
  expect(full.patches.map((p: {controlNet:number[]})=>p.controlNet)).toEqual([[2,2],[3,2]])
  await expect(inspect('batch',1)).rejects.toThrow()
})
