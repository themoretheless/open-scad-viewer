import {readFileSync,writeFileSync} from 'node:fs'
import {pathToFileURL} from 'node:url'
import {resolve} from 'node:path'
import {encodeBinary as after} from '../src/services/valueBinaryCodec'
import assert from 'node:assert/strict'
const [fixture,baseline,output]=process.argv.slice(2)
if(!fixture||!baseline||!output)throw Error('Usage: node --import tsx scripts/benchmark-cad-codec.mts fixture.json baseline-codec.ts report.json')
const {encodeBinary:before}=await import(pathToFileURL(resolve(baseline)).href)
const doc=JSON.parse(readFileSync(fixture,'utf8'))
const input=doc.bodies.slice(0,32)
assert.deepEqual(after(input),before(input))
const samples:Record<string,number[]>={before:[],after:[]}
for(let i=0;i<22;i++)for(const [name,fn] of i%2?[['after',after],['before',before]]:[['before',before],['after',after]]){
 const start=performance.now();(fn as typeof after)(input);if(i>=2)samples[name as string].push(performance.now()-start)
}
const result={scope:'32 materialized CAD bodies, alternating order, 2 warmups and 20 measured runs',byteIdentical:true,samplesMs:samples}
writeFileSync(output,JSON.stringify(result,null,2)+'\n')
for(const key of ['before','after'])console.log(key,samples[key].sort((a,b)=>a-b)[10])
