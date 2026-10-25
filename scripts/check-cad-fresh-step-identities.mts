/** Preserve identity bindings on newly built, edited and rounded STEP controls. */
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFile,writeFile} from 'node:fs/promises'
import {resolve} from 'node:path'
import identity from '../src/generated/geometry-kernels/identity'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {importDirectStepV9} from '../src/services/cadNurbsStep'
import {tessellateNurbsBrep} from '../src/services/geometry/brep'
import {parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
assert(process.argv[2],'Provide the fresh part qualification directory')
const root=resolve(process.argv[2]),hash=(bytes:string|Uint8Array)=>createHash('sha256').update(bytes).digest('hex')
const manifest=JSON.parse(await readFile(resolve(root,'manifest.json'),'utf8'))
assert.equal(hash(await readFile('public/wasm/geometry-kernel.wasm')),identity.sha256)
assert.equal(manifest.wasmSha256,identity.sha256,'Use the same qualified package as the part exporter')
await warmGeometryKernel()
for(const name of ['bracket','flange','enclosure']){
 const source=parseDirectDocument(await readFile(resolve(root,name+'-restored.json'),'utf8'))
 const entry=manifest.parts.find((p:{name:string})=>p.name===name+'-edited-rounded')
 assert(entry)
 const step=await readFile(resolve(root,entry.file),'utf8');assert.equal(hash(step),entry.sha256)
 const imported=importDirectStepV9(step)
 assert(imported.model.topologyIds,'Import must retain authored topology identities')
 const roundtrip={...source,bodies:[...source.bodies,{id:'imported-'+name,name:'Retained AP242 STEP',brep:imported.model,mesh:tessellateNurbsBrep(imported.model,8)}]}
 const text=serializeDirectDocument(parseDirectDocument(serializeDirectDocument(roundtrip)))
 await writeFile(resolve(root,name+'-step-roundtrip.json'),text,{flag:'wx'})
}
assert.equal(hash(await readFile('public/wasm/geometry-kernel.wasm')),identity.sha256)
console.log(JSON.stringify({imported:3,wasmSha256:identity.sha256,identityBindingsQualified:false,scope:'Imported controls ready for independent identity-to-representation binding comparison.'}))
