import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string, transform=(source:string)=>source){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(source)).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('retains parameter endpoint ownership in interior and exterior clips',async()=>{
  const inside=(await measure('clip-interior-path')).spans
  expect(inside).toEqual([[{parameters:['1/2','1'],included:[false,true]}],[{parameters:['0','1/2'],included:[true,false]}]])
  const outside=(await measure('clip-exterior-path')).spans
  expect(outside).toEqual([[{parameters:['0','1/2'],included:[true,false]}],[{parameters:['1/2','1'],included:[false,true]}]])
  const parts=(await measure('partition-solid-path')).spans
  expect(parts[0].map((p:{location:string})=>p.location)).toEqual(['Exterior','Boundary','Interior'])
  expect(parts[0][1]).toEqual({location:'Boundary',parameters:['1/2','1/2'],included:[true,true]})
})
it('measures length inside, outside and along the surface',async()=>{
  expect((await measure('bound-path-material-length')).lengthBoundsMm).toEqual(['2','2'])
  expect(await measure('bound-path-length-breakdown')).toMatchObject({interiorMm:['2','2'],boundaryMm:['0','0'],exteriorMm:['2','2']})
  const along=(source:string)=>source.replace('[[0,"7/2",5],[2,"7/2",5],[4,"7/2",5]]','[[1,3,5],[1,4,5],[2,4,5]]')
  expect(await measure('bound-path-length-breakdown',along)).toMatchObject({interiorMm:['1','1'],boundaryMm:['1','1'],exteriorMm:['0','0']})
  const diagonal=(source:string)=>source.replace('[[0,"7/2",5],[2,"7/2",5],[4,"7/2",5]]','[[1,3,5],[2,4,5]]')
  const material=await measure('bound-path-material-length',diagonal)
  const breakdown=await measure('bound-path-length-breakdown',diagonal)
  expect(breakdown).toMatchObject({boundaryMm:['0','0'],exteriorMm:['0','0']})
  for(const bounds of [material.lengthBoundsMm,breakdown.interiorMm]){
    const fraction=(value:string)=>{
      const [n,d='1']=value.split('/')
      return {n:BigInt(n!),d:BigInt(d)}
    }
    const lo=fraction(bounds[0]),hi=fraction(bounds[1])
    expect(lo.n>=0n && lo.d>0n && hi.n>0n && hi.d>0n).toBe(true)
    expect(lo.n*lo.n<=2n*lo.d*lo.d && hi.n*hi.n>=2n*hi.d*hi.d).toBe(true)
    expect((hi.n*lo.d-lo.n*hi.d)*1000000n<=hi.d*lo.d).toBe(true)
  }
})
