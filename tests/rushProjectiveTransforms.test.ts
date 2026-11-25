import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

for(const kind of ['curve','surface']) {
  it('applies finite-chart perspective to the '+kind+' and rejects a chart boundary',async()=>{
    const source=readFileSync('examples/rush/projective-'+kind+'.r','utf8')
    const build=async(text:string)=>buildTextNurbsScene(compileRushFrontend(text).document)
    const scene=await build(source)
    expect(JSON.parse(scene.measurements![0]!.report).finiteChart).toBe(true)
    const vertices=scene.meshes[0]!.vertices
    const xs=Array.from(vertices).filter((_,i)=>i%6===0)
    expect(Math.min(...xs)).toBeCloseTo(0)
    expect(Math.max(...xs)).toBeCloseTo(0.5)
    await expect(build(source.replace('[1,0,0,1]]','[-1,0,0,1]]'))).rejects.toThrow()
  })
}
