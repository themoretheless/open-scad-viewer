/** Warm/cold frontend timings exclude geometry execution. Optional historical TS module is supplied explicitly. */
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {cpus} from 'node:os'
import {resolve} from 'node:path'
import {pathToFileURL} from 'node:url'
import {performance} from 'node:perf_hooks'
import {isDeepStrictEqual} from 'node:util'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {compileTextRust} from '../src/services/geometryRustKernel'
const option=(key:string)=>process.argv.find(a=>a.startsWith(key+'='))?.slice(key.length+1)
const referencePath=option('--reference')
const reference=referencePath?(await import(pathToFileURL(resolve(referencePath)).href)).compileModelGraphText:undefined
const samples=['examples/skadis-box/skadis-dovetail.modelgraph.scad','examples/modelgraph-text/generic-functions.scad','examples/modelgraph-text/range-pattern.scad']
const firstSource=readFileSync(samples[0]!,'utf8')
const coldStart=performance.now();compileTextRust(firstSource);const coldFrontendMs=performance.now()-coldStart
const results=[]
const median=(xs:number[])=>{const sorted=[...xs].sort((a,b)=>a-b);return (sorted[sorted.length/2-1]!+sorted[sorted.length/2]!)/2}
for(const path of samples){
 const source=readFileSync(path,'utf8')
 if(reference){const a=reference(source),b=compileModelGraphText(source);if(!isDeepStrictEqual(a.document,b.document)||!isDeepStrictEqual(a.customizer,b.customizer))throw new Error(`Reference mismatch: ${path}`)}
 const runs:[string,(s:string)=>unknown][]=[['rust_frontend',compileTextRust],['rust_total',compileModelGraphText]]
 if(reference)runs.push(['typescript_total',reference])
 const raw:Record<string,number[]>={}
 for(const [name,fn] of runs){raw[name]=[];for(let i=0;i<10;i++)fn(source)}
 for(let batch=0;batch<8;batch++)for(const [name,fn] of batch%2?[...runs].reverse():runs){const start=performance.now();for(let i=0;i<10;i++)fn(source);raw[name]!.push((performance.now()-start)/10)}
 results.push({path,source_sha256:createHash('sha256').update(source).digest('hex'),median_ms:Object.fromEntries(Object.entries(raw).map(([name,values])=>[name,median(values)])),raw_batch_ms:raw})
}
const report={node:process.version,platform:process.platform,architecture:process.arch,cpu:cpus()[0]?.model,
 wasm_sha256:createHash('sha256').update(readFileSync('src/generated/geometry-kernels/kernel_bg.wasm')).digest('hex'),
 reference_sha256:referencePath?createHash('sha256').update(readFileSync(referencePath)).digest('hex'):undefined,
 cold_frontend_ms:coldFrontendMs,method:'10 warmups, 8 alternating batches of 10 runs; medians of per-run batch means; geometry execution excluded',results}
const encoded=JSON.stringify(report,null,2)+'\n';console.log(encoded)
const output=option('--output');if(output)writeFileSync(output,encoded)
