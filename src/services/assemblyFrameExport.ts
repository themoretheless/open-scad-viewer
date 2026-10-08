import {sampleAssemblyAnimation,validateAssemblyAnimation,type AssemblyAnimation} from './assemblyAnimation'
import type {DirectDocument} from './directModeling'
import {callGeometryRust} from './geometry/kernel'
export interface AssemblyFrameExport {frames:{time:number;svg:string}[];fps:number;width:number;height:number}
/** Fixed time sampling and native opaque mesh projection; no wall-clock capture. */
export function exportAssemblyFrames(document:DirectDocument,animation:AssemblyAnimation,fps:number):AssemblyFrameExport {
 validateAssemblyAnimation(document,animation)
 if(animation.frames.length<2||!Number.isInteger(fps)||fps<1||fps>30)throw Error('Expected at least two keyframes and 1–30 FPS.')
 const first=animation.frames[0].time,last=animation.frames.at(-1)!.time,steps=Math.ceil((last-first)*fps)
 if(steps+1>300)throw Error('Frame export is limited to 300 frames. Reduce FPS or duration.')
 const frames:{time:number;paths:string;bounds:number[]}[]=[],bounds=[Infinity,Infinity,-Infinity,-Infinity];let bytes=0
 for(let i=0;i<=steps;i++){
  const time=first+(last-first)*i/steps,sampled=sampleAssemblyAnimation(document,animation,time)
  const projected=callGeometryRust<{paths:string;bounds:number[]}>('cad_animation_frame',{bodies:sampled.document.bodies,yaw:sampled.camera.yaw,pitch:sampled.camera.pitch})
  bytes+=projected.paths.length;if(bytes>32*1024*1024)throw Error('Frame geometry exceeds 32 MiB.')
  projected.bounds.forEach((v,k)=>bounds[k]=k<2?Math.min(bounds[k],v):Math.max(bounds[k],v))
  frames.push({time,...projected})
 }
 const width=960,height=640,spanX=Math.max(bounds[2]-bounds[0],1),spanY=Math.max(bounds[3]-bounds[1],1),viewWidth=Math.max(spanX,spanY*width/height)*1.1,viewHeight=viewWidth*height/width
 const viewBox=`${(bounds[0]+bounds[2]-viewWidth)/2} ${(bounds[1]+bounds[3]-viewHeight)/2} ${viewWidth} ${viewHeight}`
 return {fps:steps/(last-first),width,height,frames:frames.map(f=>({time:f.time,svg:`<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="${viewBox}"><rect x="${(bounds[0]+bounds[2]-viewWidth)/2}" y="${(bounds[1]+bounds[3]-viewHeight)/2}" width="${viewWidth}" height="${viewHeight}" fill="#f4f5f7"/>${f.paths}</svg>`}))}
}
/** Stored ZIP: bounded, lossless archive of rasterized frames and their timestamps. */
export function frameArchive(files:{name:string;bytes:Uint8Array}[]):Uint8Array {
 if(!files.length||files.length>301)throw Error('Expected 1–301 archive entries.')
 const enc=new TextEncoder(),chunks:Uint8Array[]=[],central:Uint8Array[]=[];let offset=0,total=0
 const table=Uint32Array.from({length:256},(_,n)=>{for(let i=0;i<8;i++)n=(n>>>1)^((n&1)?0xedb88320:0);return n>>>0})
 const crc=(bytes:Uint8Array)=>{let v=0xffffffff;for(const b of bytes)v=(v>>>8)^table[(v^b)&255];return (v^0xffffffff)>>>0}
 const names=new Set<string>()
 for(const file of files){
  if(!/^[a-zA-Z0-9_.-]{1,80}$/.test(file.name)||names.has(file.name))throw Error('Invalid or duplicate frame filename.');names.add(file.name)
  total+=file.bytes.length;if(total>64*1024*1024)throw Error('Frame archive exceeds 64 MiB.')
  const name=enc.encode(file.name),sum=crc(file.bytes),local=new Uint8Array(30+name.length),l=new DataView(local.buffer)
  l.setUint32(0,0x04034b50,true);l.setUint16(4,20,true);l.setUint16(12,0x21,true);l.setUint32(14,sum,true);l.setUint32(18,file.bytes.length,true);l.setUint32(22,file.bytes.length,true);l.setUint16(26,name.length,true);local.set(name,30)
  chunks.push(local,file.bytes)
  const record=new Uint8Array(46+name.length),r=new DataView(record.buffer);r.setUint32(0,0x02014b50,true);r.setUint16(4,20,true);r.setUint16(6,20,true);r.setUint16(14,0x21,true);r.setUint32(16,sum,true);r.setUint32(20,file.bytes.length,true);r.setUint32(24,file.bytes.length,true);r.setUint16(28,name.length,true);r.setUint32(42,offset,true);record.set(name,46);central.push(record);offset+=local.length+file.bytes.length
 }
 const centralSize=central.reduce((n,c)=>n+c.length,0),end=new Uint8Array(22),e=new DataView(end.buffer)
 e.setUint32(0,0x06054b50,true);e.setUint16(8,files.length,true);e.setUint16(10,files.length,true);e.setUint32(12,centralSize,true);e.setUint32(16,offset,true)
 const result=new Uint8Array(offset+centralSize+22);let cursor=0;for(const c of [...chunks,...central,end]){result.set(c,cursor);cursor+=c.length}return result
}
