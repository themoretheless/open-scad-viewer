import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {combineSketchProfiles} from '../src/services/retainedSketchProfile'
import {buildDirectExtrusion} from '../src/services/directExtrusion'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
import type {DirectSketch} from '../src/services/directModeling'
const directory=resolve(process.argv[2]??'/tmp/profile-regions-step');mkdirSync(directory,{recursive:true})
const plate=():DirectSketch=>({id:'plate',name:'Plate',closed:true,points:[[-4,-3],[4,-3],[4,3],[-4,3]]})
const circle=(id='circle',x=0,radius=2):DirectSketch=>({id,name:id,closed:true,points:[],analytic:{kind:'circle',center:[x,0],radius,start:0,sweep:360}})
const half=():DirectSketch=>({id:'half',name:'Right half cutter',closed:true,points:[[0,-3],[3,-3],[3,3],[0,3]]})
const specs:{name:string;operation:'difference'|'intersection';source:DirectSketch[];area:number;height:number;bounds:number[][]}[]=[
 {name:'circular-hole',operation:'difference',source:[plate(),circle()],area:48-4*Math.PI,height:3,bounds:[[-4,-3,0],[4,3,3]]},
 {name:'common-disk',operation:'intersection',source:[plate(),circle()],area:4*Math.PI,height:3,bounds:[[-2,-2,0],[2,2,3]]},
 {name:'trimmed-right-half',operation:'intersection',source:[circle(),half()],area:2*Math.PI,height:3,bounds:[[0,-2,0],[2,2,3]]},
 {name:'trimmed-left-half',operation:'difference',source:[circle(),half()],area:2*Math.PI,height:-2,bounds:[[-2,-2,-2],[0,2,0]]},
 {name:'two-holes',operation:'difference',source:[plate(),circle('left',-2,.5),circle('right',2,.5)],area:48-Math.PI/2,height:3,bounds:[[-4,-3,0],[4,3,3]]},
]
const parts=[]
for(const spec of specs){
 const sketch=combineSketchProfiles(spec.source,spec.operation),document={version:1 as const,sketches:[sketch],bodies:[]}
 const body=buildDirectExtrusion(document,{sketchIds:[sketch.id],height:spec.height,offset:0,operation:'new',targetId:'',id:spec.name})!
 const text=await exportSolidStepCurrent(body),file=spec.name+'.step';writeFileSync(resolve(directory,file),text)
 writeFileSync(resolve(directory,spec.name+'.json'),JSON.stringify({...document,bodies:[body]},(_key,value)=>ArrayBuffer.isView(value)?Array.from(value as unknown as ArrayLike<number>):value))
 parts.push({name:spec.name,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:spec.area*Math.abs(spec.height),boundsMm:spec.bounds}})
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n')
console.log(directory)
