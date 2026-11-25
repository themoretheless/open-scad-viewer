import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=readFileSync('examples/rush/lift-uv-edge.r','utf8')
async function build(text:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return {report:JSON.parse(scene.measurements![0]!.report),vertices:Array.from(scene.meshes[0]!.vertices)}
}
it('builds a spatial quadratic on each source knot cell with a common seam',async()=>{
  for(const piece of [0,1]){
    const {report,vertices}=await build(source.replace('piece:1','piece:'+piece))
    expect(report).toMatchObject({pieceCount:2,degree:2,domain:piece===0?[3,5]:[5,7]})
    expect(vertices.some((v,i)=>i%6===0 && v===2 && vertices[i+1]===2 && vertices[i+2]===4)).toBe(true)
  }
})
it('refuses an out-of-chart trace and an invalid output index',async()=>{
  await expect(build(source.replace('[[0,0],[4,4]]','[[0,0],[5,4]]'))).rejects.toThrow(/lift refused/i)
  await expect(build(source.replace('piece:1','piece:2'))).rejects.toThrow(/outside the lifted trace/i)
})
