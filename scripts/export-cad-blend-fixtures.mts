import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {solidExactEdgeFeature} from '../src/services/solidExactEdgeFeature'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
const directory=resolve('output/qualification/cad-roadmap-blends');mkdirSync(directory,{recursive:true})
const brep=createBrepBox([0,0,0],[10,8,6]),body={id:'part',name:'Part',brep,mesh:tessellateNurbsBrep(brep)}
const corner=brep.vertices.findIndex(v=>v.point.every((x,i)=>x===[10,8,6][i]))
const edges=brep.edges.flatMap((edge,i)=>edge.vertices.includes(corner)?[i]:[])
const vertical=edges.find(i=>{const [a,b]=brep.edges[i].vertices.map(v=>brep.vertices[v].point);return a[0]===b[0]&&a[1]===b[1]})!
const parts=[]
for(const mode of ['variable','corner'] as const)for(const radius of [.5,1]){
 const endRadius=radius+1
 const result=solidExactEdgeFeature(body,mode==='corner'?edges:[vertical],radius,'fillet','brep',{mode,endRadius})
 const text=await exportSolidStepCurrent(result.body),name=`${mode}-r${radius}`,file=name+'.step'
 writeFileSync(resolve(directory,file),text)
 const r=radius,factor=1-Math.PI/4
 const volume=mode==='variable'?480-factor*6*(r*r+r*endRadius+endRadius*endRadius)/3:480-(24-3*r)*factor*r*r-(1-Math.PI/6)*r*r*r
 const startAtBottom=brep.vertices[brep.edges[vertical].vertices[0]].point[2]===0
 const bottom=startAtBottom?r:endRadius,mid=(r+endRadius)/2
 parts.push({name,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:volume,boundsMm:[[0,0,0],[10,8,6]],...(mode==='variable'?{sectionZ:3,lowerVolumeMm3:240-factor*3*(bottom*bottom+bottom*mid+mid*mid)/3}:{})},certificate:result.evidence.certificate})
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n')
console.log(directory)
