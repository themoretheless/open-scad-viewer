import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const author=(s:string)=>readFileSync('examples/rush/'+s+'.r','utf8')
async function scene(s:string){return buildTextNurbsScene(compileRushFrontend(s).document)}
it('constructs derivative, squared speed and binormal carriers of a quadratic graph',async()=>{
  const s=author('exact-curve-fields')
  const data=await scene(s)
  const values=data.measurements!.map(row=>JSON.parse(row.report).value)
  expect(values).toEqual(expect.arrayContaining([[1,1,0],[0,2,0],[2,0,0],[0,0,2]]))
  const velocity=s.slice(0,s.indexOf('acceleration =')).replace('return_field:false','return_field:true')+'show velocity.surface_extrude(vector:[0,0,1mm]).tessellate(8,4)'
  const mesh=(await scene(velocity)).meshes[0]!
  expect(Array.from(mesh.vertices).filter((_,i)=>i%6===0).every(x=>Math.abs(x-1)<1e-6)).toBe(true)
})
it('certifies the constant squared curvature of a rational circle on its original domain',async()=>{
  const s=author('exact-circle-curvature')
  const data=await scene(s)
  expect(JSON.parse(data.measurements![0]!.report)).toMatchObject({spanCount:1,domain:[-2,2],value:[0.25,0,0]})
  await expect(scene(s.replace('parameter:0','parameter:3'))).rejects.toThrow()
})
