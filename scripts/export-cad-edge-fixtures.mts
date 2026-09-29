import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {solidTopology} from '../src/services/directSolidTools'
import {solidExactEdgeFeature} from '../src/services/solidExactEdgeFeature'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
const directory=resolve('output/qualification/cad-roadmap-edges')
mkdirSync(directory,{recursive:true})
const brep=createBrepBox([0,0,0],[10,8,6]),mesh=tessellateNurbsBrep(brep)
const body={id:'part',name:'Part',brep,mesh}
const vertical=solidTopology(mesh).edges.flatMap((e,i)=>{
 const a=mesh.positions.slice(e.a*3,e.a*3+3),b=mesh.positions.slice(e.b*3,e.b*3+3)
 return Math.abs(a[0]-b[0])<1e-8&&Math.abs(a[1]-b[1])<1e-8?[i]:[]
})
const parts=[]
for(const [name,edges,radius,kind,volume] of [
 ['fillet-four-r1',vertical,1,'fillet',480-(4-Math.PI)*6],
 ['fillet-one-r05',[vertical[0]],.5,'fillet',480-(1-Math.PI/4)*.25*6],
 ['chamfer-one-d1',[vertical[0]],1,'chamfer',477],
 ['fillet-four-r2',vertical,2,'fillet',480-(4-Math.PI)*24],
] as const){
 const result=solidExactEdgeFeature(body,[...edges],radius,kind)
 const text=await exportSolidStepCurrent(result.body),file=name+'.step'
 writeFileSync(resolve(directory,file),text)
 parts.push({name,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:volume,boundsMm:[[0,0,0],[10,8,6]]},certificate:result.evidence.certificate})
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n')
console.log(directory)
