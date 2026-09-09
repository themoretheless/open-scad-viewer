import {readFileSync,writeFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {compileModelGraphText as rust} from '../src/services/modelGraphText'
import {compileModelGraphText as ts} from './modelgraph-text-typescript-reference'
const samples=['examples/skadis-box/skadis-dovetail.modelgraph.scad','examples/modelgraph-text/generic-functions.scad','examples/modelgraph-text/range-pattern.scad'];
const results=[]
for(const file of samples){
 const source=readFileSync(file,'utf8');
 const expected=ts(source),actual=rust(source)
 if(JSON.stringify(expected.document)!==JSON.stringify(actual.document)) {
  writeFileSync('output/migration-expected.json',JSON.stringify(expected.document,null,2));writeFileSync('output/migration-actual.json',JSON.stringify(actual.document,null,2));
  // Property order is not semantic; use the standard deep comparator.
  const {isDeepStrictEqual}=await import('node:util');if(!isDeepStrictEqual(expected.document,actual.document)) throw new Error(`Mismatch ${file}`)
 }
 for(let i=0;i<5;i++){ts(source);rust(source)}
 const times={typescript:[],rust:[]} as Record<string,number[]>
 for(let round=0;round<6;round++)for(const [name,fn] of (round%2?[['rust',rust],['typescript',ts]]:[['typescript',ts],['rust',rust]]) as [string,typeof ts][]){const start=performance.now();for(let i=0;i<5;i++)fn(source);times[name].push((performance.now()-start)/5)}
 const med=(xs:number[])=>[...xs].sort((a,b)=>a-b)[3]
 results.push({file,typescript_ms:med(times.typescript),rust_ms:med(times.rust),raw:times})
}
console.log(JSON.stringify(results,null,2));writeFileSync('output/modelgraph-migration-benchmark.json',JSON.stringify({node:process.version,arch:process.arch,platform:process.platform,results},null,2))
