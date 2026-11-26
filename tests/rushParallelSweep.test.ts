import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string, transform=(source:string)=>source){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(source)).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('measures the shared axial interval and builds a solid with the same volume',async()=>{
  expect(await measure('measure-parallel-sweep-overlap')).toMatchObject({range:['1/3','1'],volumeMm3:'8'})
  expect(await measure('build-parallel-sweep-overlap')).toMatchObject({volumeMm3:'8'})
  const mesh=await measure('mesh-parallel-sweep-overlap')
  expect(mesh.volumeMm3).toBe('8')
  const edges=new Map<string,number[]>()
  for(const triangle of mesh.triangles as number[][]){
    triangle.forEach((a,i)=>{
      const b=triangle[(i+1)%3]!
      const key=[Math.min(a,b),Math.max(a,b)].join(',')
      edges.set(key,[...(edges.get(key)??[]),Math.sign(b-a)])
    })
  }
  expect(edges.size).toBeGreaterThan(0)
  for(const directions of edges.values())expect(directions.sort()).toEqual([-1,1])
})
it('reports zero shared material for separated sweeps',async()=>{
  const separate=(s:string)=>s.replace('other_sweep_mm:[0,0,-3]','other_sweep_mm:[0,0,3]')
  expect(await measure('measure-parallel-sweep-overlap',separate)).toMatchObject({range:null,volumeMm3:'0'})
  expect(await measure('mesh-parallel-sweep-overlap',separate)).toMatchObject({triangles:[],volumeMm3:'0'})
})
