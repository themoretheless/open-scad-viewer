import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function check(kind:string,from='',to=''){
  const text=readFileSync('examples/rush/'+kind+'.r','utf8').replace(from,to)
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report).outcome
}
it('certifies both inclusive tolerances and distinguishes actual exceedances',async()=>{
  for(const kind of ['check-seam-gap','check-seam-angle']){
    expect(await check(kind)).toMatchObject({status:'exceeds',parameter:[2,6]})
  }
  expect((await check('check-seam-gap','0.5mm','1mm')).status).toBe('within')
  expect((await check('check-seam-angle','min_cosine_squared:0.8','min_cosine_squared:0.5')).status).toBe('within')
})
it('isolates strict violation bands and retains ownership of the end parameter',async()=>{
  for(const kind of ['isolate-seam-gap','isolate-seam-angle']){
    const {bands}=await check(kind)
    expect(bands).toHaveLength(1)
    expect(bands[0].start.included).toBe(false)
    const [lo,hi]=bands[0].start.bracket
    expect(hi-lo).toBeLessThanOrEqual(0.0001)
    expect(Math.abs((hi+lo)/2-4)).toBeLessThan(0.0001)
    expect(bands[0].end).toEqual({bracket:[6,6],included:true})
  }
})
