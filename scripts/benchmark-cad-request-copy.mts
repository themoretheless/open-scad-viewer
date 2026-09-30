import {readFileSync,writeFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {stringifyMeshJson} from '../src/services/meshJson'
const [fixture,output]=process.argv.slice(2)
if(!fixture||!output)throw Error('Usage: fixture.json report.json')
const document=JSON.parse(readFileSync(fixture,'utf8'))
const samples=[]
for(let i=0;i<12;i++){
 const start=performance.now()
 const copy=JSON.parse(stringifyMeshJson(document))
 if(!copy)throw Error('Missing result')
 if(i>=2)samples.push(performance.now()-start)
}
writeFileSync(output,JSON.stringify({scope:'Removed host JSON stringify/parse only; excludes postMessage, worker execution and rendering. Node microbenchmark, not browser latency.',fixture,inputBytes:readFileSync(fixture).length,samplesMs:samples,medianMs:[...samples].sort((a,b)=>a-b)[5]},null,2)+'\n')
