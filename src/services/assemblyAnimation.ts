import {parseDirectDocument,serializeDirectDocument,type DirectDocument} from './directModeling'
import {applySolidSceneEdit} from './solidSceneEdit'
import {callGeometryRust} from './geometry/kernel'
export interface AssemblyPose {id:string;translation:[number,number,number];angle:number;rotation?:[number,number,number];scale:number}
export interface AssemblyKeyframe {time:number;poses:AssemblyPose[];camera:{yaw:number;pitch:number}}
export interface AssemblyAnimation {version:1;frames:AssemblyKeyframe[]}
export function validateAssemblyAnimation(document:DirectDocument,animation:AssemblyAnimation):void {
 if(!animation||animation.version!==1||!Array.isArray(animation.frames)||!animation.frames.length||animation.frames.length>64)throw Error('Expected 1–64 animation keyframes.')
 const ids=animation.frames[0].poses.map(p=>p.id)
 if(ids.length>16||new Set(ids).size!==ids.length||ids.some(id=>!document.bodies.some(b=>b.id===id&&!b.instance)))throw Error('Animation requires up to 16 independent bodies.')
 let previous=-1
 for(const frame of animation.frames){
  if(!Number.isFinite(frame.time)||frame.time<0||frame.time>120||frame.time<=previous)throw Error('Frame times must increase within 0–120 seconds.')
  previous=frame.time
  if(!frame.camera||![frame.camera.yaw,frame.camera.pitch].every(v=>Number.isFinite(v)&&Math.abs(v)<=Math.PI*100)||Math.abs(frame.camera.pitch)>Math.PI/2)throw Error('Invalid frame camera.')
  if(frame.poses.length!==ids.length||frame.poses.some((p,i)=>p.id!==ids[i]||!Array.isArray(p.translation)||p.translation.length!==3||p.translation.some(v=>!Number.isFinite(v)||Math.abs(v)>100000)||!Number.isFinite(p.angle)||Math.abs(p.angle)>36000||(p.rotation!==undefined&&(!Array.isArray(p.rotation)||p.rotation.length!==3||p.rotation.some(v=>!Number.isFinite(v)||Math.abs(v)>36000)))||!Number.isFinite(p.scale)||p.scale<.001||p.scale>100))throw Error('All frames must use the same ordered body tracks and valid poses.')
 }
}
export function sampleAssemblyAnimation(document:DirectDocument,animation:AssemblyAnimation,time:number):{document:DirectDocument;camera:{yaw:number;pitch:number}} {
 validateAssemblyAnimation(document,animation)
 const ids=animation.frames[0].poses.map(p=>p.id),count=ids.length
 const values=animation.frames.map(f=>[...f.poses.flatMap(p=>[...p.translation,...(p.rotation??[0,0,p.angle]).map(a=>a*Math.PI/180),p.scale]),f.camera.yaw,f.camera.pitch])
 const sampled=callGeometryRust<number[]>('cad_keyframe_sample',{times:animation.frames.map(f=>f.time),values,time,angles:[...ids.flatMap((_,i)=>[i*7+3,i*7+4,i*7+5]),count*7]})
 let next=parseDirectDocument(serializeDirectDocument(document))
 for(const [i,id] of ids.entries()){
  for(const [axis,k] of [['x',3],['y',4],['z',5]] as const){
   if(sampled[i*7+k]===0&&axis!=='z')continue
   next=applySolidSceneEdit(next,{operation:'transform',id,ids:[id],createdId:'',x:axis==='z'?sampled[i*7]:0,y:axis==='z'?sampled[i*7+1]:0,z:axis==='z'?sampled[i*7+2]:0,axis,angle:sampled[i*7+k]*180/Math.PI,scale:axis==='z'?sampled[i*7+6]:1})
  }
 }
 return {document:parseDirectDocument(serializeDirectDocument(next)),camera:{yaw:sampled[count*7],pitch:sampled[count*7+1]}}
}
