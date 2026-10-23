import {readFile,writeFile} from 'node:fs/promises'
import ts from 'typescript'
import {createHash} from 'node:crypto'
import {performance} from 'node:perf_hooks'
const source=await readFile(new URL('../src/services/solidGeometryDisplayQueue.ts',import.meta.url),'utf8')
const compiled=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText
const {SolidGeometryDisplayQueue}=await import('data:text/javascript;base64,'+Buffer.from(compiled).toString('base64'))
const items=Array.from({length:1000},(_,i)=>({id:String(i),curve:{degree:3,knots:Array.from({length:104},(_,j)=>j),weights:Array(100).fill(1),controlPoints:Array.from({length:100},(_,j)=>[i,j,Math.sin(j)])}}))
let jobs=0
const queue=new SolidGeometryDisplayQueue({run:async()=>{jobs++;return [1]},cancel(){}},item=>JSON.stringify(item.curve),item=>item,()=>8,2048,64_000_000)
const gaps=[];let last=performance.now()
const heartbeat=setInterval(()=>{const now=performance.now();gaps.push(now-last);last=now},1)
const start=performance.now();const result=await queue.prepare(items);const elapsedMs=performance.now()-start
await new Promise(resolve=>setTimeout(resolve,5));clearInterval(heartbeat)
const sorted=gaps.sort((a,b)=>a-b)
const report={sourceSha256:createHash('sha256').update(source).digest('hex'),node:process.version,scope:'Node main-thread geometry-key collection and cache scan; no browser rendering or worker transport',items:items.length,controlPointsPerCurve:100,jobs,errors:result.errors,elapsedMs,heartbeatCount:gaps.length,maxHeartbeatGapMs:sorted.at(-1),p95HeartbeatGapMs:sorted[Math.floor(sorted.length*.95)],retainedWeight:queue.retainedWeight}
if(jobs!==1000||result.errors.length||gaps.length<2)throw Error('Incomplete collection benchmark')
const out=process.argv[2];if(out)await writeFile(out,JSON.stringify(report,null,2)+'\n')
console.log(report)
