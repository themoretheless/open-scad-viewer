import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {combineSketchProfiles,withRetainedProfile,sketchProfile} from '../src/services/retainedSketchProfile'
import {offsetSketch} from '../src/services/directSketchGeometry'
import {prepareSolidProfile} from '../src/services/solidProfilePreparation'
import {buildDirectExtrusion} from '../src/services/directExtrusion'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
import type {DirectSketch} from '../src/services/directModeling'
const directory=resolve(process.argv[2]??'/tmp/profile-offset-step');mkdirSync(directory,{recursive:true})
const disk:DirectSketch={id:'disk',name:'Disk',closed:true,points:[],analytic:{kind:'circle',center:[0,0],radius:2,start:0,sweep:360}}
const retainedDisk=withRetainedProfile(disk,sketchProfile(disk))
const plate=combineSketchProfiles([{id:'plate',name:'Offset plate',closed:true,points:[[0,0],[8,0],[8,6],[0,6]]},{id:'hole',name:'Hole',closed:true,points:[],analytic:{kind:'circle',center:[4,3],radius:1,start:0,sweep:360}}],'difference')
const semicircle=prepareSolidProfile({version:1,sketches:[{...disk,id:'arc',closed:false,analytic:{...disk.analytic!,kind:'arc',sweep:180}},{id:'line',name:'Diameter',closed:false,points:[[-2,0],[2,0]]}],bodies:[]},['arc','line'],0).document.sketches[0]
writeFileSync(resolve(directory,'source.json'),JSON.stringify({version:1,sketches:[plate],bodies:[]}))
const neckSketch:DirectSketch={id:'neck',name:'Neck',closed:true,points:[[-3,-2],[-1,-2],[-1,-.2],[1,-.2],[1,-2],[3,-2],[3,2],[1,2],[1,.2],[-1,.2],[-1,2],[-3,2]]}
const neck=withRetainedProfile(neckSketch,sketchProfile(neckSketch))
const neckExtra=.12-.2*Math.sqrt(.05)-.09*Math.asin(2/3)
const specs:{name:string;source:DirectSketch;distance:number;area:number;bounds:number[][];solids?:number}[]=[
 {name:'neck-split',source:neck,distance:-.3,area:2*(4.76+neckExtra),bounds:[[-2.7,-1.7,0],[2.7,1.7,2]],solids:2},
 {name:'disk-grown',source:retainedDisk,distance:1,area:9*Math.PI,bounds:[[-3,-3,0],[3,3,2]]},
 {name:'disk-eroded',source:retainedDisk,distance:-.5,area:2.25*Math.PI,bounds:[[-1.5,-1.5,0],[1.5,1.5,2]]},
 {name:'plate-grown',source:plate,distance:.5,area:62,bounds:[[-.5,-.5,0],[8.5,6.5,2]]},
 {name:'plate-eroded',source:plate,distance:-.5,area:35-2.25*Math.PI,bounds:[[.5,.5,0],[7.5,5.5,2]]},
 {name:'semicircle-grown',source:semicircle,distance:.5,area:3.25*Math.PI+2,bounds:[[-2.5,-.5,0],[2.5,2.5,2]]},
 {name:'semicircle-eroded',source:semicircle,distance:-.5,area:2.25*Math.acos(1/3)-.5*Math.sqrt(2),bounds:[[-Math.sqrt(2),.5,0],[Math.sqrt(2),1.5,2]]},
]
const parts=[]
for(const spec of specs){
 const sketch=offsetSketch(spec.source,spec.distance),document={version:1 as const,sketches:[sketch],bodies:[]}
 const body=buildDirectExtrusion(document,{sketchIds:[sketch.id],height:2,offset:0,operation:'new',targetId:'',id:spec.name})!
 const text=await exportSolidStepCurrent(body),file=spec.name+'.step';writeFileSync(resolve(directory,file),text)
 writeFileSync(resolve(directory,spec.name+'.json'),JSON.stringify({...document,bodies:[body]},(_key,value)=>ArrayBuffer.isView(value)?Array.from(value as unknown as ArrayLike<number>):value))
 parts.push({name:spec.name,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:spec.area*2,boundsMm:spec.bounds,solids:spec.solids??1}})
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n');console.log(directory)
