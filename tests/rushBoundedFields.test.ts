import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const example=(name:string)=>readFileSync('examples/rush/'+name+'.r','utf8')
async function reports(text:string){
  const s=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(s.meshes).toHaveLength(1)
  return s.measurements!.map(m=>JSON.parse(m.report))
}
it('proves whole-span storage bounds for derivative, speed, curvature and binormal carriers',async()=>{
  const rows=[...await reports(example('bounded-curve-fields')),...await reports(example('bounded-circle-curvature'))]
  expect(rows.map(r=>r.value)).toEqual(expect.arrayContaining([[1,1,0],[0,2,0],[2,0,0],[0,0,2],[0.25,0,0]]))
  for(const row of rows) expect(row.continuousErrorUpper[0]).toBeLessThanOrEqual(0.000001)
})
it('reports whole-cell error for partials, oriented normals and metric of a saddle',async()=>{
  const s=example('bounded-saddle-fields')
  const rows=await reports(s)
  expect(rows.map(r=>r.value)).toEqual(expect.arrayContaining([[1,0,0.25],[-0.25,-0.5,1],[1.0625,0.125,1.25],[0,0,1]]))
  for(const row of rows) expect(row.continuousErrorUpper[0]).toBeLessThanOrEqual(0.000001)
  await expect(reports(s.replace('max_work:1000000','max_work:1'))).rejects.toThrow()
})
