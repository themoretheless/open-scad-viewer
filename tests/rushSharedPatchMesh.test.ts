import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const program=readFileSync('examples/rush/shared-patch-mesh.r','utf8')
const build=(code:string)=>buildTextNurbsScene(compileRushFrontend(code).document)

it('welds nine seam vertices into one indexed mesh for two adjacent charts',async()=>{
  const scene=await build(program)
  expect(scene.meshes).toHaveLength(1)
  const report=JSON.parse(scene.measurements![0]!.report)
  expect(report).toMatchObject({vertices:153,triangles:256,agreementChecks:2})
  expect(report.errorUpperMm).toBeLessThanOrEqual(0.3)
  const mesh=scene.meshes[0]!
  expect(mesh.indices.length).toBe(768)
  expect(new Set(mesh.indices).size).toBe(153)
  const seam=new Set<number>()
  for(const i of mesh.indices) if(mesh.vertices[i*6]===1)seam.add(i)
  expect(seam.size).toBe(9)
  expect(scene.surfaceArea).toBeCloseTo(2,10)
})
it('refuses an incompatible seam correspondence and a grid over budget',async()=>{
  await expect(build(program.replace('1,0,0]]','1,0,1]]'))).rejects.toThrow(/tolerance|seam/i)
  await expect(build(program.replace('max_cells:1000','max_cells:1'))).rejects.toThrow(/budget/i)
})
