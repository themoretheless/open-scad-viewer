import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const example=readFileSync('examples/rush/sew-periodic-source-face.r','utf8')

async function run(source:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.find(row=>row.operation==='sew_periodic_source_face')!.report)
}

it('keeps original curved UV edges and pairs the two sides of the crossed seam',async()=>{
  const report=await run(example)
  expect(report).toMatchObject({sourceBound:true,previewRole:'support_surface',solidBodyAuthored:false})
  expect(report.faces).toHaveLength(1)
  expect(report.faces[0]).toMatchObject({genus:0,eulerCharacteristic:1})
  expect(report.faces[0].boundaryCycles).toHaveLength(1)
  expect(report.seams.length).toBeGreaterThan(0)
  expect(report.edges.some((edge:{original:{kind:string}})=>edge.original.kind==='original_trace')).toBe(true)
  await expect(run(example.replace('max_edges:128','max_edges:1'))).rejects.toThrow()
},180000)

it('sews two independent wrapped axes into one toroidal face',async()=>{
  const source=[
    '// @rush/1',
    'carrier = torus_surface(center:[0mm,0mm,0mm],major_radius:3mm,radial_radius:1mm,axial_radius:1mm)',
    'sewn = carrier.sew_periodic_source_face(loops:[],wrapped_axes:"uv",base_uv:[0.5,0.5],base_winding:1,probe_radius_uv:0.0009765625,max_work:100000,max_edges:128)',
    'show sewn.tessellate(8,8)',
  ].join('\n')
  const report=await run(source)
  expect(report.seams).toHaveLength(2)
  expect(report.faces).toHaveLength(1)
  expect(report.faces[0]).toMatchObject({genus:1,eulerCharacteristic:0,boundaryCycles:[]})
},180000)
