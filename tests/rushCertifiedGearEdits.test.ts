import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
import {readNativeGeometryData} from '../src/kernel/nativeGeometrySnapshot'
async function build(name:string,transform=(s:string)=>s){
  const source=transform(readFileSync('examples/rush/'+name+'.r','utf8'))
  return buildTextNurbsScene(compileRushFrontend(source).document)
}
it('revalidates the changed outline and companion hole before returning the replacement',async()=>{
  const scene=await build('certified-profile-edit')
  expect(JSON.parse(scene.measurements!.at(-1)!.report).nestingDepth).toEqual([0,1])
  await expect(build('certified-profile-edit',s=>s.replaceAll('5mm','1.5mm'))).rejects.toThrow()
},180000)
it('builds the edited herringbone tooth and delivers its STEP from the same solid',async()=>{
  for(const name of ['verified-gear-edit','edited-gear-step']){
    const scene=await build(name)
    const measurement=scene.measurements!.at(-1)!
    const r=JSON.parse(measurement.report)
    expect(r.winding).toEqual([1,-1])
    expect(r.controlAfter[0]-r.controlBefore[0]).toBeCloseTo(1e-8,13)
    const model=readNativeGeometryData(scene.meshes[0]!.nativeGeometry!).geometry
    expect(model.shells[0].closed).toBe(true)
    expect(model.edges.some(e=>e.curve.controlPoints.some(p=>
      Math.abs(p[2])<1e-12 && Math.abs(p[0]-r.controlAfter[0])<1e-12 && Math.abs(p[1]-r.controlAfter[1])<1e-12))).toBe(true)
    if(name==='edited-gear-step')expect(measurement.step).toMatch(/MANIFOLD_SOLID_BREP\(/)
  }
},240000)
