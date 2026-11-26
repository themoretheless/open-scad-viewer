import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {readNativeGeometryData} from '../src/kernel/nativeGeometrySnapshot'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const source = readFileSync('examples/rush/assemble-owned-trimmed-shell.r','utf8')
const build=(program:string)=>buildTextNurbsScene(compileRushFrontend(program).document)

it('replays owned trim incidence and admits only the verified shell',async()=>{
  const scene=await build(source)
  const report=JSON.parse(scene.measurements!.find(r=>r.operation==='assemble_owned_trimmed_shell')!.report)
  expect(report).toMatchObject({
    admitted:true,closure:'WithinTolerance',trimRegionsValid:true,allChartsSpanwiseRegular:true,
    connected:true,vertexLinksValid:true,solidBodyAuthored:false,globalEmbeddingCertified:false,
  })
  const model=readNativeGeometryData(scene.meshes[0]!.nativeGeometry!).geometry
  expect(model.faces).toHaveLength(6)
  expect(model.edges).toHaveLength(12)
  expect(model.shells).toHaveLength(1)
  expect(model.shells[0]!.closed).toBe(true)
  expect(model.bodies).toHaveLength(0)
  await expect(build(source.replace('regularity_cells:10000','regularity_cells:1'))).rejects.toThrow(/admit.*trimmed shell/)
},180000)
