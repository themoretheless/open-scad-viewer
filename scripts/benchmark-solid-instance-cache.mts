import {readFileSync} from 'node:fs'
import assert from 'node:assert/strict'
import {performance} from 'node:perf_hooks'
import {parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {SolidInstanceBatchCache} from '../src/services/solidInstanceBatchCache'
const text=readFileSync(process.argv[2],'utf8'),expected=serializeDirectDocument(parseDirectDocument(text))
for(const limit of [16_000_000,64_000_000]){
 const cache=new SolidInstanceBatchCache(limit),get=cache.get.bind(cache)
 let hits=0,misses=0
 cache.get=key=>{const value=get(key);if(value)hits++;else misses++;return value}
 for(let iteration=0;iteration<2;iteration++){
  hits=0;misses=0;const start=performance.now(),doc=parseDirectDocument(text,cache),elapsedMs=performance.now()-start
  assert.equal(serializeDirectDocument(doc),expected)
  console.log(JSON.stringify({limit,iteration,elapsedMs,hits,misses,entries:cache.size,estimatedRetainedBytes:cache.retainedBytes}))
 }
}
