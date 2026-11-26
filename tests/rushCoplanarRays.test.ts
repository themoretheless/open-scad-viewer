import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,before='',after=''){
  const text=readFileSync('examples/rush/'+name+'.r','utf8').replace(before,after)
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
function numeric(s:string){const [n,d='1']=s.split('/');return Number(n)/Number(d)}
it('retains source contour ownership and brackets four crossings',async()=>{
  const r=await build('coplanar-ray-contacts')
  expect(r.contacts.map((c:{contour:number})=>c.contour).sort()).toEqual([0,0,1,1])
  expect(r.contacts.every((c:{kind:string})=>c.kind==='Crossing')).toBe(true)
  expect((await build('coplanar-ray-contacts','[-1,1,0]','[-1,1,1]')).status).toBe('notCoplanar')
})
it('certifies the two material intervals, the hole and the unbounded exterior',async()=>{
  const r=await build('coplanar-ray-partition')
  expect(r.gaps.map((g:{location:string})=>g.location)).toEqual(['Outside','Inside','Outside','Inside'])
  expect(r.contactBands).toHaveLength(4)
  for(const [i,x] of [1,1.5,2.5,3].entries()){
    const [lo,hi]=r.contactBands[i].map(numeric)
    expect(lo).toBeLessThanOrEqual(x)
    expect(hi).toBeGreaterThanOrEqual(x)
  }
  expect(numeric(r.outsideAfter)).toBeGreaterThanOrEqual(3)
})
