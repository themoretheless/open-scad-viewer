import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {prepareSolidProfile} from '../src/services/solidProfilePreparation'
import {sampleCurve,type AnalyticCurve} from '../src/services/directSketchGeometry'
import {buildDirectExtrusion} from '../src/services/directExtrusion'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
import type {DirectDocument} from '../src/services/directModeling'
const directory=resolve(process.argv[2]??'/tmp/arc-profile-step');mkdirSync(directory,{recursive:true})
const parts=[]
for(const [name,start,sweep,height,bounds] of [
 ['semicircle',0,180,5,[[-2,0,0],[2,2,5]]],
 ['clockwise-quarter',0,-90,3,[[0,-2,0],[2,0,3]]],
 ['major-arc',0,270,-4,[[-2,-2,-4],[2,2,0]]],
 ['oblique-arc',13,77,2,[[0,2*Math.sin(13*Math.PI/180),0],[2*Math.cos(13*Math.PI/180),2,2]]],
 ['fractional-arc',0,27.06,1,[[2*Math.cos(27.06*Math.PI/180),0,0],[2,2*Math.sin(27.06*Math.PI/180),1]]],
] as const){
 const analytic:AnalyticCurve={kind:'arc',center:[0,0],radius:2,start,sweep},samples=sampleCurve(analytic)
 const source:DirectDocument={version:1,sketches:[{id:'arc',name,closed:false,points:samples,analytic},{id:'chord',name:'Chord',closed:false,points:[samples.at(-1)!,samples[0]]}],bodies:[]}
 const prepared=prepareSolidProfile(source,['arc','chord'],0)
 if(!prepared.report.accepted)throw Error(JSON.stringify(prepared.report))
 const body=buildDirectExtrusion(prepared.document,{sketchIds:['arc'],height,offset:0,operation:'new',targetId:'',id:name})!
 const text=await exportSolidStepCurrent(body),file=name+'.step';writeFileSync(resolve(directory,file),text)
 writeFileSync(resolve(directory,name+'.json'),JSON.stringify({...prepared.document,bodies:[body]},(_key,value)=>ArrayBuffer.isView(value)?Array.from(value as unknown as ArrayLike<number>):value))
 const angle=sweep*Math.PI/180,volume=2*Math.abs(angle-Math.sin(angle))*Math.abs(height)
 parts.push({name,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:volume,boundsMm:bounds}})
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n')
console.log(directory)
