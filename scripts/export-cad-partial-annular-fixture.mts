import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createBrepTube,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {stringifyMeshJson} from '../src/services/meshJson'
const directory=resolve(process.argv[2]);mkdirSync(directory,{recursive:true})
const brep=createBrepTube(20,5,6)
const body={id:'annular-preview',name:'Annular preview',brep,mesh:tessellateNurbsBrep(brep,2)}
const id=brep.topologyIds!.edges[2]
writeFileSync(resolve(directory,'fixture.json'),stringifyMeshJson({version:1,sketches:[],bodies:[body]}))
writeFileSync(resolve(directory,'edge-id.txt'),id)
writeFileSync(resolve(directory,'edge-ids.json'),JSON.stringify([id]))
