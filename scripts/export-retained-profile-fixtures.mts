import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {unionSketchProfiles} from '../src/services/retainedSketchProfile'
import {transformSketch} from '../src/services/directSketchGeometry'
import {buildDirectExtrusion} from '../src/services/directExtrusion'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
import type {DirectSketch} from '../src/services/directModeling'
const directory=resolve(process.argv[2]??'/tmp/retained-profile-step');mkdirSync(directory,{recursive:true})
const source:DirectSketch[]=[{id:'rect',name:'Rectangle',closed:true,points:[[-4,-3],[3,-3],[3,3],[-4,3]]},{id:'circle',name:'Circle',closed:true,points:[],analytic:{kind:'circle',center:[3,0],radius:2,start:0,sweep:360}}]
const original=unionSketchProfiles(source),scaled=transformSketch(original,[0,0],0,2,[0,0]),parts=[]
writeFileSync(resolve(directory,'source.json'),JSON.stringify({version:1,sketches:source,bodies:[]}))
for(const [name,sketch,height,bounds,volume] of [['retained-union',original,5,[[-4,-3,0],[5,3,5]],(42+2*Math.PI)*5],['retained-scaled',scaled,-5,[[-8,-6,-5],[10,6,0]],(42+2*Math.PI)*20]] as const){
 const document={version:1 as const,sketches:[sketch],bodies:[]}
 const body=buildDirectExtrusion(document,{sketchIds:[sketch.id],height,offset:0,operation:'new',targetId:'',id:name})!
 const text=await exportSolidStepCurrent(body),file=name+'.step';writeFileSync(resolve(directory,file),text)
 writeFileSync(resolve(directory,name+'.json'),JSON.stringify({...document,bodies:[body]},(_key,value)=>ArrayBuffer.isView(value)?Array.from(value as unknown as ArrayLike<number>):value))
 parts.push({name,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:volume,boundsMm:bounds}})
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n')
console.log(directory)
