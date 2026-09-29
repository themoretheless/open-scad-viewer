import 'fake-indexeddb/auto'
import {mkdirSync, readFileSync, writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {emptyDirectDocument} from '../src/services/directModeling'
import {prepareSolidStepImport, exportSolidStepCurrent} from '../src/services/solidStepExchange'
import {pushPullFace, solidTopology} from '../src/services/directSolidTools'
const source=resolve('docs/qualification/cad-roadmap-2026-09-28/cap-step-exchange')
const directory=resolve(process.argv[2]??'output/qualification/cad-step-edit-cycle')
mkdirSync(directory,{recursive:true})
const manifest=JSON.parse(readFileSync(resolve(source,'manifest.json'),'utf8'))
for(const part of manifest.parts){
 const input=readFileSync(resolve(source,part.file),'utf8')
 const imported=await prepareSolidStepImport(input,emptyDirectDocument())
 const body=imported.document.bodies[0]
 const top=solidTopology(body.mesh).faces.map((face,index)=>({face,index})).filter(({face})=>face.normal[2]>.99).sort((a,b)=>b.face.center[2]-a.face.center[2])[0].index
 const text=await exportSolidStepCurrent(pushPullFace(body,top,1))
 writeFileSync(resolve(directory,part.file),text)
 part.sourceSha256=createHash('sha256').update(input).digest('hex')
 part.sha256=createHash('sha256').update(text).digest('hex')
 part.expected.boundsMm[1][2]+=1
 part.expected.volumeMm3=({bracket:7150,enclosure:7680,flange:3000*Math.PI} as Record<string,number>)[part.name]
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify(manifest,null,2)+'\n')
