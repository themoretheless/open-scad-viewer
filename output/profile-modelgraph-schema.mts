import {readFileSync,writeFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {createHash} from 'node:crypto'
import {cpus} from 'node:os'
import {compileModelGraph,modelGraphSchema} from './modelGraph-runtime-reference.ts'
const path='examples/skadis-box/skadis-dovetail.modelgraph.scad'; const source=readFileSync(path,'utf8')
const doc=JSON.parse(readFileSync('output/modelgraph-schema-profile-skadis.json','utf8'));const nodes=doc.nodes
const runs:any={schema:()=>modelGraphSchema.parse(doc),total_graph:()=>compileModelGraph(doc),json_roundtrip:()=>JSON.parse(JSON.stringify(doc))}
const raw:Record<string,number[]>={}; for(const [name,fn] of Object.entries(runs)){raw[name]=[];for(let i=0;i<5;i++)(fn as any)()}
for(let batch=0;batch<6;batch++)for(const [name,fn] of batch%2?Object.entries(runs).reverse():Object.entries(runs)){const start=performance.now();for(let i=0;i<10;i++)(fn as any)();raw[name]!.push((performance.now()-start)/10)}
const median=(a:number[])=>{a=[...a].sort((a,b)=>a-b);return(a[2]!+a[3]!)/2}
const report={node:process.version,cpu:cpus()[0]?.model,source_sha256:createHash('sha256').update(source).digest('hex'),source_bytes:source.length,json_bytes:JSON.stringify(doc).length,node_count:nodes.length,method:'5 warmups, 6 alternating batches x10; per-run ms; excludes geometry execution',median_ms:Object.fromEntries(Object.entries(raw).map(([k,v])=>[k,median(v)])),raw}
writeFileSync('output/modelgraph-schema-profile-current.json',JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2))
