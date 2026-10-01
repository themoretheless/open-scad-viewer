import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {transformSelection} from '../src/services/directSolidTools'
import {SolidInstanceBatchCache} from '../src/services/solidInstanceBatchCache'
const [fixture,output]=process.argv.slice(2)
if(!fixture||!output)throw Error('Usage: fixture.json report.json')
const first=parseDirectDocument(readFileSync(fixture,'utf8')),source=first.bodies.find(b=>!b.instance)
assert.ok(source)
const moved=transformSelection({version:1,sketches:[],bodies:[source]},[source.id],[1,0,0],[0,0,1],0,1)
const second={...first,bodies:first.bodies.map(b=>b.id===source.id?moved.bodies[0]:b)}
const texts=[serializeDirectDocument(first),serializeDirectDocument(second)],cache=new SolidInstanceBatchCache(),get=cache.get.bind(cache)
let hits=0,misses=0
cache.get=key=>{const result=get(key);if(result)hits++;else misses++;return result}
const rows=[]
for(const version of [0,1,0,1]){
 hits=misses=0;const start=performance.now(),document=parseDirectDocument(texts[version],cache),elapsedMs=performance.now()-start
 assert.equal(serializeDirectDocument(document),texts[version])
 rows.push({version,elapsedMs,hits,misses,entries:cache.size,estimatedRetainedBytes:cache.retainedBytes})
}
assert.equal(rows[2].misses,0);assert.equal(rows[3].misses,0)
writeFileSync(output,JSON.stringify({scope:'Node alternating compact instance versions; cache capacity and identity proof, not UI latency or RSS',rows},null,2)+'\n')
console.log(rows)
