import {readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'

// Exclusive sampled CPU time, not elapsed operation latency. Run the workload
// with Node --cpu-prof, then pass its .cpuprofile and an output JSON filename.
const [input,output]=process.argv.slice(2)
if(!input||!output)throw Error('Usage: node scripts/summarize-cpu-profile.mjs input.cpuprofile output.json')
const profile=JSON.parse(readFileSync(input,'utf8'))
if(!Array.isArray(profile.samples)||profile.samples.length!==profile.timeDeltas?.length)throw Error('Profile needs matching samples and timeDeltas')
const nodes=new Map(profile.nodes.map(node=>[node.id,node]))
const totals=new Map(),frames=new Map()
for(let i=0;i<profile.samples.length;i++){
 const node=nodes.get(profile.samples[i]),us=profile.timeDeltas[i]
 if(!node||!Number.isFinite(us)||us<0)throw Error('Invalid sampled node or duration')
 const {functionName:name,url='',lineNumber,columnNumber}=node.callFrame
 const category=name==='(garbage collector)'?'garbageCollection':url.startsWith('wasm:')?'wasm':url.includes('/valueBinaryCodec.ts')?'binaryCodec':url.includes('/meshJson.ts')?'meshJson':name==='structuredClone'?'structuredClone':url.includes('/directModeling.ts')?'document':name==='(idle)'?'idle':'other'
 totals.set(category,(totals.get(category)??0)+us)
 const source=url.startsWith(`file://${process.cwd()}/`)?url.slice(`file://${process.cwd()}/`.length):url
 const key=JSON.stringify({name,source,line:lineNumber+1,column:columnNumber+1})
 frames.set(key,(frames.get(key)??0)+us)
}
const totalUs=[...totals.values()].reduce((a,b)=>a+b,0)
const rows=map=>[...map].sort((a,b)=>b[1]-a[1]).map(([key,us])=>({key,sampledMs:us/1000,percent:totalUs?100*us/totalUs:0}))
const summary={scope:'Exclusive sampled CPU time over the supplied capture window; capture boundaries define scope. A main-thread profile excludes workers.',sampleCount:profile.samples.length,sampledMs:totalUs/1000,categories:rows(totals),topFrames:rows(frames).slice(0,40).map(({key,...rest})=>({...JSON.parse(key),...rest}))}
writeFileSync(resolve(output),JSON.stringify(summary,null,2)+'\n')
console.log(JSON.stringify(summary.categories,null,2))
