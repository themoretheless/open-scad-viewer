import 'fake-indexeddb/auto'
import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {emptyDirectDocument} from '../src/app/directModeling'
import {createBrepBox,tessellateNurbsBrep} from '../src/kernel/geometry/brep'
import {exportSolidStepAssembly} from '../src/kernel/solidStepAssembly'
import {prepareSolidStepImport} from '../src/kernel/solidStepExchange'
import {stringifyMeshJson} from '../src/exchange/meshJson'
const directory=resolve(process.argv[2]??'/tmp/cad-component-fixture');mkdirSync(directory,{recursive:true})
const document=emptyDirectDocument()
for(let i=0;i<3;i++){
 const brep=createBrepBox([i*10,0,0],[i*10+2,2,(i+1)*2])
 document.bodies.push({id:'part-'+i,name:'Part '+i,brep,mesh:tessellateNurbsBrep(brep,2)})
}
const imported=await prepareSolidStepImport(await exportSolidStepAssembly(document),emptyDirectDocument())
const body=imported.document.bodies[0],brep=body.brep!,surface=brep.bodies.pop()!
brep.topologyIds!.bodies.pop();brep.shells[surface.outerShell].closed=false
body.mesh=tessellateNurbsBrep(brep,2)
writeFileSync(resolve(directory,'fixture.json'),stringifyMeshJson(imported.document))
