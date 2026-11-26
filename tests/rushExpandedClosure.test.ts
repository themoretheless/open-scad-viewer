import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,alter=(x:string)=>x){
  const source=alter(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  return {volume:scene.volume,report:JSON.parse(scene.measurements![0]!.report)}
}
it('sews selected or discovered rims on exact expanded supports and closes two planes atomically',async()=>{
  for(const name of ['attach-expanded-rims','discover-expanded-rims']){
    const {volume,report}=await build(name)
    expect(report).toMatchObject({capFaces:[3],closed:true,sourceEdges:6,resultEdges:6})
    expect(volume).toBeCloseTo(1/6,6)
  }
  const prism=await build('close-expanded-planes')
  expect(prism.report).toMatchObject({capFaces:[6,7],closed:true,sourceEdges:12,resultEdges:12})
  expect(prism.volume).toBeCloseTo(0.5,6)
  await expect(build('close-expanded-planes',x=>x.replace(',[0,0,1,1]',''))).rejects.toThrow()
})
it('proves positive material volume after correcting inward shell orientation',async()=>{
  for(const name of ['build-expanded-plane-solid','build-outward-plane-solid','build-outward-inferred-solid']){
    const {volume,report}=await build(name)
    expect(report).toMatchObject({orientationReversed:name.includes('outward'),bodyCount:1,boundaryProven:true,supportErrorUpper:['0']})
    expect(report.volumeMm3[0]).toBeLessThanOrEqual(1/6)
    expect(report.volumeMm3[1]).toBeGreaterThanOrEqual(1/6)
    expect(volume).toBeCloseTo(1/6,6)
  }
})
