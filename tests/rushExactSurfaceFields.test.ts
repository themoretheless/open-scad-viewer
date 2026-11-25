import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=readFileSync('examples/rush/saddle-fields.r','utf8')
async function build(text:string){return buildTextNurbsScene(compileRushFrontend(text).document)}
it('returns exact partials, area normals and metric components for a bilinear saddle',async()=>{
  const scene=await build(source)
  const values=scene.measurements!.map(m=>JSON.parse(m.report).value)
  expect(values).toEqual(expect.arrayContaining([[1,0,0.25],[-0.25,-0.5,1],[1.0625,0.125,1.25],[0,0,1]]))
  const text=source.slice(0,source.indexOf('metric =')).replace('normal = du.surface_area_normal_field(patch:0,uv:[0.5,0.25],return_field:false','normal = du.surface_area_normal_field(patch:0,uv:[0.5,0.25],return_field:true')+'show normal.tessellate(8,8)'
  const mesh=(await build(text)).meshes[0]!
  expect(Array.from(mesh.vertices).filter((_,i)=>i%6===2).every(z=>Math.abs(z-1)<1e-6)).toBe(true)
})
it('rejects invalid mixed orders and requests outside the selected UV cell',async()=>{
  await expect(build(source.replace('orders:[1,1]','orders:[2,2]'))).rejects.toThrow()
  await expect(build(source.replace('uv:[0.5,0.25]','uv:[1.5,0.25]'))).rejects.toThrow()
})
