import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {emptyDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {createSolidInstance} from '../src/services/solidInstances'
import {exportSolidBlenderSnapshot} from '../src/services/solidBlenderExchange'
const directory=resolve(process.argv[2]??'/tmp/solid-blender-qualification')
mkdirSync(directory,{recursive:true})
for(const width of [2,7]){
 const document=emptyDirectDocument(),brep=createBrepBox([0,0,0],[width,3,4])
 document.blenderProjectId='cad-qualification'
 document.bodies.push({id:'source',name:'Qualified part',brep,mesh:tessellateNurbsBrep(brep,1)})
 const linked=createSolidInstance(document,'source','instance',[[1,0,0,10],[0,1,0,0],[0,0,1,0],[0,0,0,1]])
 writeFileSync(resolve(directory,`width-${width}.json`),serializeDirectDocument(linked))
 writeFileSync(resolve(directory,`width-${width}.osv-blender.json`),exportSolidBlenderSnapshot(linked,'cad-qualification'))
}
console.log(directory)
