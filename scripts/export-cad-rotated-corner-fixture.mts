import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {cadRoadmapParts} from '../benchmarks/cad-roadmap-fixtures'
import {createBrepBox,transformNurbsBrep,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {stringifyMeshJson} from '../src/services/meshJson'
const directory=resolve(process.argv[2]);mkdirSync(directory,{recursive:true})
const source=createBrepBox([-7,3,-2],[3,11,4]),a=.37,b=-.61
const model=transformNurbsBrep(source,[[Math.cos(a)*Math.cos(b),-Math.sin(a),Math.cos(a)*Math.sin(b),17],[Math.sin(a)*Math.cos(b),Math.cos(a),Math.sin(a)*Math.sin(b),-9],[-Math.sin(b),0,Math.cos(b),23],[0,0,0,1]])
const body={...cadRoadmapParts()[0].body,name:'Rotated cuboid',brep:model,mesh:tessellateNurbsBrep(model,12)}
const ids=source.edges.flatMap((e,i)=>e.vertices.includes(0)?[model.topologyIds!.edges[i]]:[])
writeFileSync(resolve(directory,'fixture.json'),stringifyMeshJson({version:1,sketches:[],bodies:[body]}))
writeFileSync(resolve(directory,'edge-id.txt'),ids[0]);writeFileSync(resolve(directory,'edge-ids.json'),JSON.stringify(ids))
