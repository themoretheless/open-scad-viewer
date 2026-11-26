import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const example = readFileSync('examples/rush/approximate-function-surface.r','utf8')

async function build(source=example) {
  const scene = await buildTextNurbsScene(compileRushFrontend(source).document)
  const report = JSON.parse(scene.measurements!.find(row=>row.operation==='approximate_function_surface')!.report)
  return {scene,report}
}

it('bounds the deviation of the editable bilinear patch from its coordinate formulas',async()=>{
  const {scene,report}=await build()
  expect(scene.meshes).toHaveLength(1)
  expect(report).toMatchObject({withinTolerance:true,requiresAuthoredGlobalDerivativeBounds:true,
    certificateOutcome:'WithinTolerance',solidTopologyCertified:false})
  expect(report.errorUpperMm).toBeLessThanOrEqual(0.01)
  expect(report.cells).toBeGreaterThan(1)
  const vertices=scene.meshes[0]!.vertices
  for(let i=0;i<vertices.length;i+=6){
    const u=vertices[i]!/8, v=vertices[i+1]!/6
    expect(Math.abs(vertices[i+2]!-u*u-v*v)).toBeLessThanOrEqual(report.errorUpperMm+1e-7)
  }
},180000)

it('reports budget exhaustion and rejects a pole between interpolation stations',async()=>{
  const coarse=await build(example.replace('max_cells:256','max_cells:1'))
  expect(coarse.report).toMatchObject({cells:1,withinTolerance:false,gridStop:'CellBudget'})
  await expect(build(example.replace('u*u+v*v','1/(u-0.37)'))).rejects.toThrow()
},180000)
