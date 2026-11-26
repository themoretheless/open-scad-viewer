import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

for (const name of ['build-section-solid','build-bounded-section-solid']) {
  it(name+' proves one outward material body and encloses its volume',async()=>{
    const source=readFileSync('examples/rush/'+name+'.r','utf8')
    const build=(text:string)=>buildTextNurbsScene(compileRushFrontend(text).document)
    const scene=await build(source)
    const report=JSON.parse(scene.measurements![0]!.report)
    expect(report).toMatchObject({bodyCount:1,boundaryProven:true,capFaces:[3],supportErrorUpper:['0']})
    expect(report.volumeMm3[0]).toBeLessThanOrEqual(1/6)
    expect(report.volumeMm3[1]).toBeGreaterThanOrEqual(1/6)
    expect(report.volumeMm3[1]-report.volumeMm3[0]).toBeLessThanOrEqual(0.0001)
    expect(scene.volume).toBeCloseTo(1/6,6)
    await expect(build(source.replace('[0,0,-1,0','[0,0,-1,1'))).rejects.toThrow()
    await expect(build(source.replace('max_volume_cells:10000','max_volume_cells:1'))).rejects.toThrow()
  })
}
