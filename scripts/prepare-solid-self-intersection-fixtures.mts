import {readFileSync,mkdirSync,writeFileSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {resolve} from 'node:path'
import {tessellateNurbsBrep,createBrepBox,composeStepV9Occurrences,type NurbsBrep} from '../src/services/geometry/brep'
const directory=resolve(process.argv[2]??'/tmp/solid-self-intersection-fixtures')
mkdirSync(directory,{recursive:true})

for(const [name,model] of [
 ...Array.from({length:4},(_,i)=>[`annular-${i}`,JSON.parse(gunzipSync(readFileSync(`docs/qualification/variable-annular-transitions-2026-10-07/fixtures/case-${i}.json.gz`)).toString())] as const),
 ['crossing',composeStepV9Occurrences([createBrepBox([0,0,0],[1,1,1]),createBrepBox([0.5,0.5,0.5],[1.5,1.5,1.5])])] as const,
]) {
 const mesh=tessellateNurbsBrep(model as NurbsBrep,8)
 const document={version:1,sketches:[],bodies:[{id:name,name,brep:model,mesh:{positions:Array.from(mesh.positions),indices:Array.from(mesh.indices)}}]}
 writeFileSync(resolve(directory,name+'.json'),JSON.stringify(document))
}
console.log(directory)
