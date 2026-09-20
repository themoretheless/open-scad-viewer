import assert from 'node:assert/strict'
import {readFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {pathToFileURL} from 'node:url'
import {resolve} from 'node:path'
import {encodeBinary,decodeBinary} from '../src/services/valueBinaryCodec'

if (!process.argv[2]) throw new Error('Pass the baseline valueBinaryCodec.ts path')
const baselineUrl=pathToFileURL(resolve(process.argv[2]))
const baseline=await import(baselineUrl.href)
const rows=[]
for(const count of [1000,100000]){
 const value=Array.from({length:count},(_,i)=>i%2 ? -i*4294967296+17 : i*4294967296+19)
 const bytes=encodeBinary(value)
 const methods={baseline:baseline.decodeBinary as typeof decodeBinary,candidate:decodeBinary}
 const times={baseline:[] as number[],candidate:[] as number[]}
 for(const fn of Object.values(methods))for(let i=0;i<5;i++)assert.deepEqual(fn(bytes),value)
 for(let i=0;i<21;i++)for(const key of i%2?['candidate','baseline'] as const:['baseline','candidate'] as const){
  const start=performance.now(),out=methods[key](bytes)
  times[key].push(performance.now()-start)
  assert.deepEqual(out,value)
 }
 rows.push({count,bytes:bytes.length,...Object.fromEntries(Object.entries(times).map(([key,values])=>[key,{medianMs:[...values].sort((a,b)=>a-b)[10],samplesMs:values}]))})
}
const sha=(url:URL)=>createHash('sha256').update(readFileSync(url)).digest('hex')
console.log(JSON.stringify({node:process.version,arch:process.arch,method:'5 warmups; 21 alternating pairs; signed and unsigned safe integers spanning both words; parity outside timing; allocation/GC included',baselineSha256:sha(baselineUrl),candidateSha256:sha(new URL('../src/services/valueBinaryCodec.ts',import.meta.url)),rows},null,2))
