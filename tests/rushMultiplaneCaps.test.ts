import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

for (const name of ['close-section-planes','close-bounded-planes']) {
  it(name+' closes both ends of the triangular prism in plane order',async()=>{
    const source=readFileSync('examples/rush/'+name+'.r','utf8')
    const build=(text:string)=>buildTextNurbsScene(compileRushFrontend(text).document)
    const scene=await build(source)
    expect(scene.volume).toBeCloseTo(0.5,6)
    const report=JSON.parse(scene.measurements![0]!.report)
    expect(report).toMatchObject({closed:true,capFaces:[6,7],supportErrorUpper:['0','0'],sourceEdges:12,resultEdges:12})
    await expect(build(source.replace(/,\[0,0,1,1(?:,0,0)?\]/,''))).rejects.toThrow()
  })
}
