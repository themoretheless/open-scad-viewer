import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {createBrepBox,booleanNurbsBrep,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {pushPullFace,solidTopology} from '../src/services/directSolidTools'
import {solidExactEdgeFeature} from '../src/services/solidExactEdgeFeature'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
import type {DirectBody} from '../src/services/directModeling'

// Export the actual public command chain for independent OpenCascade checks.
const directory=resolve(process.argv[2]??'output/qualification/cad-feature-chain')
mkdirSync(directory,{recursive:true})
const parts=[]
for(const distance of [2,-2]){
 const brep=booleanNurbsBrep(createBrepBox([0,0,0],[12,10,10]),createBrepBox([8,0,0],[20,10,10]),'union')
 const joined:DirectBody={id:'chain',name:'Boolean cap fillet',brep,mesh:tessellateNurbsBrep(brep,2)}
 const top=solidTopology(joined.mesh).faces.map((face,index)=>({face,index})).filter(({face})=>face.normal[2]>.99).sort((a,b)=>b.face.center[2]-a.face.center[2])[0].index
 const pushed=pushPullFace(joined,top,distance)
 const edges=pushed.brep!.edges.flatMap((edge,index)=>{
  const [a,b]=edge.vertices.map(i=>pushed.brep!.vertices[i].point)
  return a[0]===b[0]&&a[1]===b[1]?[index]:[]
 })
 if(edges.length!==4)throw Error(`Expected four longitudinal edges, got ${edges.length}`)
 const rounded=solidExactEdgeFeature(pushed,edges,1,'fillet','brep').body
 const text=await exportSolidStepCurrent(rounded),file=`chain-${distance>0?'outward':'inward'}.step`
 writeFileSync(resolve(directory,file),text)
 parts.push({name:`Boolean → Push/Pull ${distance} mm → fillet R1`,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:(200-4+Math.PI)*(10+distance),boundsMm:[[0,0,0],[20,10,10+distance]]}})
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n')
console.log(`Exported ${parts.length} feature chains to ${directory}`)
