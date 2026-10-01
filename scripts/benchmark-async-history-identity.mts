import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {DirectHistory,parseDirectDocument} from '../src/services/directModeling'
import {pathToFileURL} from 'node:url'
import {resolve} from 'node:path'
const [fixture,output,baselinePath]=process.argv.slice(2)
if(!fixture||!output||!baselinePath)throw Error('Usage: fixture.json output.json baseline-directModeling.ts (place baseline alongside service dependencies)')
const {DirectHistory:Baseline}=await import(pathToFileURL(resolve(baselinePath)).href)
const text=readFileSync(fixture,'utf8'),rows=[]
for(let iteration=0;iteration<6;iteration++)for(const [variant,History] of (iteration%2?[['current',DirectHistory],['baseline',Baseline]]:[['baseline',Baseline],['current',DirectHistory]]) as const){
 const doc=parseDirectDocument(text);doc.blenderProjectId='identity'
 const h=new History(doc),edit=h.document;edit.bodies[0].name='changed';h.commit(edit)
 await h.restoreAsync('undo',async (value:string)=>parseDirectDocument(value))
 assert.equal(h.storageStats.materializedStates,0)
 const candidate=structuredClone(doc);delete candidate.blenderProjectId;candidate.bodies[0].name='next'
 const start=performance.now();assert.equal(await h.commitAsync(async()=>candidate),true);const elapsedMs=performance.now()-start
 assert.equal(h.document.blenderProjectId,'identity')
 if(iteration)rows.push({variant,iteration,elapsedMs})
}
writeFileSync(output,JSON.stringify({scope:'Node commitAsync after compact async undo, 1000 linked instances; candidate prepared before timing; no browser/FPS claim',rows},null,2)+'\n')
console.log(rows)
