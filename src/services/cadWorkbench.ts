import {lightenSolid,type LighteningOptions} from './solidLightening'
import {textureSurface,type SurfaceTextureOptions} from './surfaceTexture'
import type {DirectDocument,DirectBody,DirectSketch} from './directModeling'
import {bodyPoints} from './directModeling'
import {booleanPolygonMeshes,inspectPolygonMesh,loftPolygonSections,sweepPolygonProfile,extrudePolygonProfile,type PolygonMesh} from './polygonKernel'
import {worldPoint,unit3,dot3,cross3,type Vec3} from './directSketchGeometry'
import {buildModelGraphThread} from './modelGraphThreads'
export type CadAction='lighten'|'texture'|'union'|'difference'|'intersection'|'loft'|'sweep'|'mirror'|'pattern'|'align'|'distribute'|'draft'|'hole'|'thread'|'joint'|'resize'
export interface CadOptions {action:CadAction;ids:string[];sketches:DirectSketch[];axis:Vec3;origin:Vec3;amount:number;count:number;width:number;height:number;depth:number;pitch:number;secondary:number;mode:string;pathId:string;profileIds:string[];texture?:SurfaceTextureOptions;lightening?:LighteningOptions}
export const bounds=(bodies:DirectBody[])=>{const min=[Infinity,Infinity,Infinity],max=[-Infinity,-Infinity,-Infinity];for(const b of bodies)for(let i=0;i<b.mesh.positions.length;i++){const k=i%3,v=b.mesh.positions[i];min[k]=Math.min(min[k],v);max[k]=Math.max(max[k],v)}if(!min.every(Number.isFinite))throw Error('Select bodies.');return {min,max}}
const change=(b:DirectBody,fn:(p:number[])=>number[],reverse=false):DirectBody=>({...b,mesh:{positions:bodyPoints(b).flatMap(fn),indices:reverse?Array.from({length:b.mesh.indices.length/3},(_,i)=>b.mesh.indices.slice(i*3,i*3+3).reverse()).flat():[...b.mesh.indices]}})
const shifted=(b:DirectBody,v:number[])=>change(b,p=>p.map((x,k)=>x+v[k]))
export function pathPoint(path:number[][],fraction:number):number[]{
 if(path.length<2)throw Error('A path requires at least two points.')
 const lengths=path.slice(1).map((p,i)=>Math.hypot(...p.map((v,k)=>v-path[i][k]))),total=lengths.reduce((a,b)=>a+b,0);if(total<1e-8)throw Error('Zero-length path.')
 let distance=Math.max(0,Math.min(1,fraction))*total
 for(let i=0;i<lengths.length;i++){if(distance<=lengths[i]||i===lengths.length-1){const t=lengths[i]?distance/lengths[i]:0;return path[i].map((v,k)=>v+(path[i+1][k]-v)*t)}distance-=lengths[i]}
 return path.at(-1)!
}
export function cadOperation(input:DirectDocument,o:CadOptions):DirectDocument{
 const d=structuredClone(input),selected=o.ids.map(id=>d.bodies.find(b=>b.id===id)).filter((b):b is DirectBody=>!!b),n=unit3(o.axis),axis=n.findIndex(v=>Math.abs(v)>.99)
 if(![...o.origin,o.amount,o.width,o.height,o.depth,o.pitch,o.secondary].every(Number.isFinite))throw Error('Enter finite parameters.')
 const replace=(b:DirectBody)=>{d.bodies[d.bodies.findIndex(old=>old.id===b.id)]=b}
 const add=(mesh:PolygonMesh,name:string)=>{const r=inspectPolygonMesh(mesh);if(!r.closed||r.signedVolumeMm3<=0)throw Error('Operation did not produce a closed solid.');d.bodies.push({id:crypto.randomUUID(),name,mesh:{positions:mesh.positions,indices:mesh.indices}})}
 if(['loft','sweep'].includes(o.action)){
  const profiles=o.profileIds.map(id=>o.sketches.find(s=>s.id===id)).filter((s):s is DirectSketch=>!!s)
  if(o.action==='loft'){if(profiles.length<2||profiles.some(s=>!s.closed))throw Error('Choose at least two closed sections in loft order.');const count=Math.max(...profiles.map(s=>s.points.length));const sections=profiles.map(s=>Array.from({length:count},(_,i)=>pathPoint([...s.points,s.points[0]].map(p=>worldPoint(p,s.plane)),i/count)));add(loftPolygonSections(sections),'Loft')}
  else {const p=profiles[0],path=o.sketches.find(s=>s.id===o.pathId);if(!p?.closed||!path||path.closed)throw Error('Choose a closed profile and an open path.');add(sweepPolygonProfile(p.points,path.points.map(p=>worldPoint(p,path.plane)),p.plane?.u??[1,0,0]),'Sweep')}
  return d
 }
 if(!selected.length)throw Error('Select bodies.')
 if(o.action==='lighten'){if(!o.lightening)throw Error('Configure lightening.');for(const b of selected)replace(lightenSolid(b,o.lightening));return d}
 if(o.action==='texture'){if(!o.texture)throw Error('Configure surface texture.');for(const b of selected)replace(textureSurface(b,o.texture));return d}
 if(['union','difference','intersection'].includes(o.action)){
  if(selected.length<2)throw Error('Select at least two bodies; the first is the base.')
  let mesh=selected[0].mesh;for(const b of selected.slice(1))mesh=booleanPolygonMeshes(mesh,b.mesh,o.action as 'union'|'difference'|'intersection')
  if(!mesh.indices.length)throw Error('The boolean result is empty.')
  replace({...selected[0],mesh});d.bodies=d.bodies.filter(b=>!o.ids.slice(1).includes(b.id));return d
 }
 if(o.action==='mirror'){for(const b of selected){const mirrored=change(b,p=>{const distance=dot3(p.map((v,k)=>v-o.origin[k]),n);return p.map((v,k)=>v-2*distance*n[k])},true);if(o.mode==='copy'){mirrored.id=crypto.randomUUID();d.bodies.push(mirrored)}else replace(mirrored)}return d}
 const bb=bounds(selected),center=bb.min.map((v,k)=>(v+bb.max[k])/2)
 if(o.action==='pattern'){
  if(!Number.isInteger(o.count)||o.count<2||o.count>100)throw Error('Pattern count must be 2–100.')
  const path=o.sketches.find(s=>s.id===o.pathId),pts=path?.points.map(p=>worldPoint(p,path.plane))
  for(let i=0;i<o.count;i++){const delta=pts?pathPoint(pts,i/(o.count-1)).map((v,k)=>v-center[k]):n.map(v=>v*o.amount*i);for(const b of selected){const copy=shifted(b,delta);if(i===0)replace(copy);else{copy.id=crypto.randomUUID();d.bodies.push(copy)}}}return d
 }
 if(o.action==='align'||o.action==='distribute'){
  if(axis<0||selected.length<2)throw Error('Choose a world axis and at least two bodies.')
  const key=(b:DirectBody)=>{const a=bounds([b]);return o.mode==='min'?a.min[axis]:o.mode==='max'?a.max[axis]:(a.min[axis]+a.max[axis])/2}
  const sorted=[...selected].sort((a,b)=>key(a)-key(b)),first=key(sorted[0]),last=key(sorted.at(-1)!)
  sorted.forEach((b,i)=>{const target=o.action==='align'?key(selected[0]):first+(last-first)*i/(sorted.length-1),v=[0,0,0];v[axis]=target-key(b);replace(shifted(b,v))});return d
 }
 if(o.action==='resize'){
  const desired=[o.width,o.height,o.depth];if(desired.some(v=>v<=0)||bb.max.some((v,k)=>v-bb.min[k]<1e-8))throw Error('Dimensions must be positive.')
  for(const b of selected)replace(change(b,p=>p.map((v,k)=>bb.min[k]+(v-bb.min[k])*desired[k]/(bb.max[k]-bb.min[k]))));return d
 }
 if(o.action==='draft'){
  const tangent=Math.tan(o.amount*Math.PI/180);if(Math.abs(o.amount)>60)throw Error('Draft angle must be between -60 and 60 degrees.')
  for(const b of selected){const base=bounds([b]),c=base.min.map((v,k)=>(v+base.max[k])/2),span=Math.max(...base.max.map((v,k)=>v-base.min[k]))/2;const next=change(b,p=>{const q=p.map((v,k)=>v-o.origin[k]),h=dot3(q,n),factor=1+h*tangent/span;if(factor<=.01)throw Error('Draft collapses the section.');return p.map((v,k)=>{const radial=v-c[k]-dot3(p.map((v,k)=>v-c[k]),n)*n[k];return v+radial*(factor-1)})});if(!inspectPolygonMesh(next.mesh).closed)throw Error('Invalid draft result.');replace(next)}return d
 }
 if(o.action==='joint'){
  if(selected.length!==2)throw Error('Choose parent and moving component.')
  const child=selected[1],r=o.amount*Math.PI/180
  replace(o.mode==='slider'?shifted(child,n.map(v=>v*o.amount)):change(child,p=>{const q=p.map((v,k)=>v-o.origin[k]),cross=cross3(n,q),dot=dot3(n,q);return q.map((v,k)=>o.origin[k]+v*Math.cos(r)+cross[k]*Math.sin(r)+n[k]*dot*(1-Math.cos(r)))}));return d
 }
 if(o.action==='hole'||o.action==='thread'){
  if(o.width<=0||o.depth<=0)throw Error('Diameter and depth must be positive.')
  const u=unit3(cross3(n,Math.abs(n[0])<.8?[1,0,0]:[0,1,0])),v=cross3(n,u),plane={origin:o.origin,u,v}
  let cutter:PolygonMesh
  if(o.action==='thread'){
   const built=buildModelGraphThread({diameter:o.width,pitch:o.pitch,length:o.depth,internal:false,wall:1,clearance:0,starts:1,left_handed:false,segments_per_turn:16})
   cutter={positions:Array.from({length:built.mesh.positions.length/3},(_,i)=>worldPoint(built.mesh.positions.slice(i*3,i*3+3),plane)).flat(),indices:built.mesh.indices}
  }else {
   const circle=(radius:number)=>Array.from({length:48},(_,i)=>[radius*Math.cos(i*Math.PI/24),radius*Math.sin(i*Math.PI/24)])
   const raw=extrudePolygonProfile({outer:circle(o.width/2)},[0,0,o.depth]);cutter={positions:Array.from({length:raw.positions.length/3},(_,i)=>worldPoint(raw.positions.slice(i*3,i*3+3),plane)).flat(),indices:raw.indices}
   if(o.mode==='counterbore'||o.mode==='countersink'){
    if(o.secondary<=o.width||o.height<=0||o.height>o.depth)throw Error('Counterbore diameter/depth must exceed the hole diameter and fit its depth.')
    const sections=[circle(o.secondary/2).map(p=>worldPoint([...p,0],plane)),circle(o.mode==='countersink'?o.width/2:o.secondary/2).map(p=>worldPoint([...p,o.height],plane))]
    cutter=booleanPolygonMeshes(cutter,loftPolygonSections(sections),'union')
   }
  }
  const base=selected[0],operation=o.action==='thread'&&o.mode==='external'?'union':'difference',mesh=booleanPolygonMeshes(base.mesh,cutter,operation)
  if(!mesh.report.closed||!mesh.indices.length)throw Error('Hole/thread did not produce a closed body.');replace({...base,mesh});return d
 }
 return d
}
