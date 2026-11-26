import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string, transform=(source:string)=>source){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(source)).document)
  return JSON.parse(scene.measurements![0]!.report)
}
type Point = [string,string,string]
function areaTerm(a:Point,b:Point){
  function value(s:string){const [n,d='1']=s.split('/');return Number(n)/Number(d)}
  return value(a[0])*value(b[1])-value(a[1])*value(b[0])
}
function twiceArea(loop:Point[]){
  return loop.reduce((sum,p,i)=>sum+areaTerm(p,loop[(i+1)%loop.length]!),0)
}
it('conserves area across cells, seam-free edges, loops and triangulations in every mode',async()=>{
  for(const [mode,area] of Object.entries({intersection:8,union:26,difference:12,symmetric_difference:18})){
    const change=(s:string)=>s.replace('boolean_mode:"intersection"','boolean_mode:"'+mode+'"')
    const {cellsMm}=await measure('decompose-profile-boolean',change)
    expect(cellsMm.reduce((s:number,c:Point[])=>s+twiceArea(c),0)).toBe(2*area)
    const {edgesMm}=await measure('extract-profile-boolean-edges',change)
    expect(edgesMm.reduce((s:number,[a,b]:[Point,Point])=>s+areaTerm(a,b),0)).toBe(2*area)
    const {loopsMm}=await measure('assemble-profile-boolean-loops',change)
    for(const loop of loopsMm)expect(loop[0]).toEqual(loop.at(-1))
    expect(loopsMm.reduce((s:number,c:Point[])=>s+twiceArea(c),0)).toBe(2*area)
    const {triangles,verticesMm}=await measure('triangulate-profile-boolean',change)
    const areas=triangles.map((tri:number[])=>twiceArea(tri.map(i=>verticesMm[i])))
    expect(areas.every((v:number)=>v>0)).toBe(true)
    expect(areas.reduce((s:number,v:number)=>s+v,0)).toBe(2*area)
  }
})
it('extrudes obliquely with positive volume and opposite edge incidences',async()=>{
  const {triangles,volumeMm3}=await measure('extrude-profile-boolean-shell')
  expect(volumeMm3).toBe('24')
  const edges=new Map<string,number[]>()
  for(const tri of triangles)for(let i=0;i<3;i++){
    const a=tri[i],b=tri[(i+1)%3],key=[Math.min(a,b),Math.max(a,b)].join(',')
    edges.set(key,[...(edges.get(key)??[]),Math.sign(b-a)])
  }
  expect(edges.size).toBeGreaterThan(0)
  for(const uses of edges.values())expect(uses.sort()).toEqual([-1,1])
})
