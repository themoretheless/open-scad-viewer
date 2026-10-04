import assert from 'node:assert/strict'
import {readFile,writeFile,mkdir} from 'node:fs/promises'
import {createHash} from 'node:crypto'
import {gzipSync,brotliCompressSync,constants} from 'node:zlib'
import path from 'node:path'
const [fixtures,mouse,keyboard,output]=process.argv.slice(2)
assert.ok(fixtures&&mouse&&keyboard&&output,'Usage: fixtures mouse keyboard output')
await mkdir(output,{recursive:true})
const report={scope:'controlled bracket/enclosure/flange histories; all-command and general BRep completion unproven',parts:[],files:[]}
const archive=new Map()
async function read(root,file){
 const bytes=await readFile(path.join(root,file)),value=JSON.parse(bytes)
 const key=path.basename(root)+'/'+file
 const sha256=createHash('sha256').update(bytes).digest('hex')
 report.files.push({file:key,sha256,bytes:bytes.length})
 archive.set(sha256,{sha256,json:bytes.toString('utf8')})
 return value
}
for(const name of ['bracket','enclosure','flange']){await read(fixtures,name+'/initial.json');await read(fixtures,name+'/expected.json')}
for(const [interaction,root] of [['mouse',mouse],['keyboard',keyboard]]){
 const result=await read(root,'result.json');assert.equal(result.ok,true);assert.equal(result.edits,20);assert.equal(result.cancelledPreviewsPerPart,7);assert.deepEqual(result.errors,[])
 for(const name of ['bracket','enclosure','flange']){
  const states=[]
  for(let i=0;i<=20;i++)states.push(await read(root,`${name}-${i}.json`))
  assert.equal(new Set(states.map(v=>JSON.stringify(v))).size,21)
  for(let i=0;i<20;i++)assert.deepEqual(await read(root,`${name}-undo-${i}.json`),states[i])
  for(let i=1;i<=20;i++)assert.deepEqual(await read(root,`${name}-redo-${i}.json`),states[i])
  for(let i=1;i<=7;i++)assert.deepEqual(await read(root,`${name}-cancel-preview-${i}.json`),states[(i-1)*3])
  assert.deepEqual(await read(root,`${name}-reload.json`),states[20])
  const expected=JSON.parse(await readFile(path.join(fixtures,name,'expected.json'),'utf8'))
  for(let i=0;i<=20;i++){
   const body=states[i].bodies.find(b=>b.id===name);assert.ok(body?.brep)
   for(const kind of ['vertices','edges','loops','faces','shells','bodies']){
    const ids=body.brep.topologyIds[kind];assert.equal(ids.length,body.brep[kind].length);assert.equal(new Set(ids).size,ids.length)
   }
   for(let axis=0;axis<3;axis++)for(const fn of [Math.min,Math.max]){
    const actual=fn(...body.brep.vertices.map(v=>v.point[axis])),target=fn(...expected[i].bodies[0].brep.vertices.map(v=>v.point[axis]))
    assert.ok(Math.abs(actual-target)<1e-6,`${interaction}/${name}/${i} bounds`)
   }
  }
  report.parts.push({interaction,name,committedEdits:20,cancelledPreviews:7,undoStates:20,redoStates:20,reloadExact:true,topologyIdsUnique:true,boundsToleranceMm:1e-6})
 }
}
const sourceBytes=Buffer.from([...archive.values()].map(v=>JSON.stringify(v)).join('\n')+'\n')
const brotli=process.argv.includes('--brotli'),archiveName=brotli?'documents.jsonl.br':'documents.jsonl.gz'
const packed=brotli?brotliCompressSync(sourceBytes,{params:{[constants.BROTLI_PARAM_QUALITY]:6,[constants.BROTLI_PARAM_LGWIN]:24}}):gzipSync(sourceBytes)
report.archive={file:archiveName,encoding:brotli?'brotli':'gzip',bytes:packed.length,sourceBytes:sourceBytes.length,sha256:createHash('sha256').update(packed).digest('hex')}
await writeFile(path.join(output,archiveName),packed)
await writeFile(path.join(output,'history-audit.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({passed:true,cases:report.parts.length,documents:report.files.length,uniquePayloads:archive.size}))
