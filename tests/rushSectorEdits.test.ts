import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function run(name:string,transform=(s:string)=>s) {
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(readFileSync('examples/rush/'+name+'.r','utf8'))).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.at(-1)!.report)
}
it('returns the edited rational piece in all three sector modes',async()=>{
  for(const name of ['edit-profile-sector','edit-closed-sector','edit-tangent-sector']){
    const r=await run(name)
    expect(r.controlPoints).toEqual([[1,0,0],[3,0,0],[2,2,0]])
    expect(r.weights).toEqual([1,0.5,1])
  }
},180000)
it('refuses an open join and a tangent change at a previously regular join',async()=>{
  await expect(run('edit-closed-sector',s=>s.replace('[[1,0,0],[3,0,0]','[[1.1,0,0],[3,0,0]'))).rejects.toThrow(/join/i)
  await expect(run('edit-tangent-sector',s=>s.replace('[3,0,0]','[3,0.001,0]'))).rejects.toThrow(/G1/i)
},180000)
