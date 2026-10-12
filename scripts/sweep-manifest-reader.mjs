import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,mkdtempSync,rmSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {join} from 'node:path'
import {spawnSync} from 'node:child_process'
import {fileURLToPath} from 'node:url'

const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
const stringLimit=500_000_000
export const sweepManifestReaderProvenance=Object.freeze(Object.fromEntries(
 ['sweep-manifest-reader.mjs','split-sweep-manifest.py'].map(file=>[file,sha(readFileSync(new URL(file,import.meta.url)))])
))
function checkReaderSources(){
 for(const [file,hash] of Object.entries(sweepManifestReaderProvenance))
  assert.equal(sha(readFileSync(new URL(file,import.meta.url))),hash,'Manifest reader source changed during qualification')
}
/** Preserve all manifest values while avoiding V8's single-string ceiling.
 * Original bytes remain the authority for oracle/report SHA binding.
 */
export function readSweepManifest(path,bytes=readFileSync(path),{partition=false}={}){
 if(!partition&&bytes.length<=stringLimit)return JSON.parse(bytes.toString('utf8'))
 const temporary=mkdtempSync(join(tmpdir(),'osv-sweep-manifest-'))
 try{
  checkReaderSources()
  const result=spawnSync('python3',[fileURLToPath(new URL('./split-sweep-manifest.py',import.meta.url)),path,temporary],{encoding:'utf8'})
  if(result.error)throw result.error
  assert.equal(result.status,0,`Manifest partition failed: ${result.stderr}`)
  checkReaderSources()
  const index=JSON.parse(readFileSync(join(temporary,'index.json'),'utf8'))
  assert.equal(index.schema,'sweep-manifest-partition/1')
  assert.equal(index.originalSha256,sha(bytes),'Manifest changed while partitioning')
  assert.ok(Number.isSafeInteger(index.caseCount)&&index.caseCount>=0)
  assert.equal(index.records.length,index.caseCount)
  assert.equal(Object.hasOwn(index.root,'cases'),false)
  const cases=index.records.map((record,i)=>{
   assert.equal(record.file,`case-${String(i).padStart(6,'0')}.json`)
   const data=readFileSync(join(temporary,record.file))
   assert.equal(sha(data),record.sha256)
   assert.ok(data.length<=stringLimit,'Individual sweep case exceeds V8 string ceiling')
   return JSON.parse(data.toString('utf8'))
  })
  return {...index.root,cases}
 }finally{rmSync(temporary,{recursive:true,force:true})}
}
